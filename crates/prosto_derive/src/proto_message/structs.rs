use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::GenericArgument;
use syn::Ident;
use syn::ItemStruct;
use syn::PathArguments;
use syn::Type;
use syn::parse_quote;
use syn::spanned::Spanned;

use super::build_validate_with_ext_impl;
use super::generic_bounds::add_proto_wire_bounds;
use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::FieldInfo;
use super::unified_field_handler::assign_tags;
use super::unified_field_handler::build_clear_stmts;
use super::unified_field_handler::build_decode_match_arms;
use super::unified_field_handler::build_post_decode_hooks;
use super::unified_field_handler::build_proto_default_expr;
use super::unified_field_handler::compute_decode_ty;
use super::unified_field_handler::compute_proto_ty;
use super::unified_field_handler::encode_conversion_expr;
use super::unified_field_handler::needs_encode_conversion;
use super::unified_field_handler::strip_proto_attrs;
use crate::parse::UnifiedProtoConfig;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::resolved_field_type;

pub(super) fn generate_struct_impl(
    input: &DeriveInput,
    item_struct: &ItemStruct,
    data: &syn::DataStruct,
    config: &UnifiedProtoConfig,
) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;

    let struct_item = sanitize_struct(item_struct.clone());

    let mut fields = match &data.fields {
        syn::Fields::Named(named) => named
            .named
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let config = parse_field_config(field);
                let effective_ty = resolved_field_type(field, &config);
                let parsed = parse_field_type(&effective_ty);
                let proto_ty = compute_proto_ty(field, &config, &parsed, &effective_ty);
                let decode_ty = compute_decode_ty(field, &config, &parsed, &proto_ty);
                FieldInfo {
                    index: idx,
                    field,
                    access: FieldAccess::Named(field.ident.as_ref().expect("named field missing ident")),
                    config,
                    tag: None,
                    parsed,
                    proto_ty,
                    decode_ty,
                }
            })
            .collect::<Vec<_>>(),
        syn::Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let config = parse_field_config(field);
                let effective_ty = resolved_field_type(field, &config);
                let parsed = parse_field_type(&effective_ty);
                let proto_ty = compute_proto_ty(field, &config, &parsed, &effective_ty);
                let decode_ty = compute_decode_ty(field, &config, &parsed, &proto_ty);
                FieldInfo {
                    index: idx,
                    field,
                    access: FieldAccess::Tuple(idx),
                    config,
                    tag: None,
                    parsed,
                    proto_ty,
                    decode_ty,
                }
            })
            .collect::<Vec<_>>(),
        syn::Fields::Unit => Vec::new(),
    };

    if config.transparent {
        assert!(fields.len() == 1, "#[proto_message(transparent)] requires a single-field struct");

        let field = fields.remove(0);
        let bounded_generics = add_proto_wire_bounds(generics, std::iter::once(&field));
        let bounded_generics = add_transparent_bounds(&bounded_generics, &field.field.ty);
        let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();
        let transparent_impl = generate_transparent_struct_impl(name, &impl_generics, &ty_generics, where_clause, &field, &data.fields);

        return quote! {
            #struct_item
            #transparent_impl
        };
    }

    let bounded_generics = add_proto_wire_bounds(generics, fields.iter());
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();

    let fields = assign_tags(fields);

    let proto_ext_impl = generate_proto_ext_impl(name, &impl_generics, &ty_generics, where_clause, &fields, config);
    let proto_wire_impl = generate_proto_wire_impl(name, &impl_generics, &ty_generics, where_clause, &fields, &data.fields, config, &bounded_generics);

    quote! {
        #struct_item
        #proto_ext_impl
        #proto_wire_impl
    }
}

fn add_transparent_bounds(generics: &syn::Generics, inner_ty: &Type) -> syn::Generics {
    let mut generics = generics.clone();
    let _type_params: BTreeSet<_> = generics.type_params().map(|param| param.ident.clone()).collect();
    let where_clause = generics.make_where_clause();
    where_clause.predicates.push(parse_quote!(#inner_ty: ::proto_rs::ProtoEncode));
    where_clause.predicates.push(parse_quote!(#inner_ty: ::proto_rs::ProtoDecode));
    where_clause.predicates.push(parse_quote!(#inner_ty: ::proto_rs::ProtoExt));
    generics
}

fn collect_type_params(ty: &Type, params: &BTreeSet<syn::Ident>, used: &mut BTreeSet<syn::Ident>) {
    match ty {
        Type::Path(type_path) => {
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let ident = &type_path.path.segments[0].ident;
                if params.contains(ident) {
                    used.insert(ident.clone());
                }
            }
            for segment in &type_path.path.segments {
                match &segment.arguments {
                    PathArguments::None => {}
                    PathArguments::AngleBracketed(args) => {
                        for arg in &args.args {
                            match arg {
                                GenericArgument::Type(inner_ty) => {
                                    collect_type_params(inner_ty, params, used);
                                }
                                GenericArgument::AssocType(assoc) => {
                                    collect_type_params(&assoc.ty, params, used);
                                }
                                GenericArgument::Constraint(constraint) => {
                                    for bound in &constraint.bounds {
                                        if let syn::TypeParamBound::Trait(trait_bound) = bound {
                                            for segment in &trait_bound.path.segments {
                                                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                                                    for arg in &args.args {
                                                        if let GenericArgument::Type(inner_ty) = arg {
                                                            collect_type_params(inner_ty, params, used);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                GenericArgument::Lifetime(_) | GenericArgument::Const(_) | GenericArgument::AssocConst(_) | _ => {}
                            }
                        }
                    }
                    PathArguments::Parenthesized(args) => {
                        for input in &args.inputs {
                            collect_type_params(input, params, used);
                        }
                        if let syn::ReturnType::Type(_, output) = &args.output {
                            collect_type_params(output, params, used);
                        }
                    }
                }
            }
        }
        Type::Reference(reference) => collect_type_params(&reference.elem, params, used),
        Type::Array(array) => collect_type_params(&array.elem, params, used),
        Type::Slice(slice) => collect_type_params(&slice.elem, params, used),
        Type::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_type_params(elem, params, used);
            }
        }
        Type::Paren(paren) => collect_type_params(&paren.elem, params, used),
        Type::Group(group) => collect_type_params(&group.elem, params, used),
        Type::Ptr(ptr) => collect_type_params(&ptr.elem, params, used),
        Type::BareFn(bare_fn) => {
            for input in &bare_fn.inputs {
                collect_type_params(&input.ty, params, used);
            }
            if let syn::ReturnType::Type(_, output) = &bare_fn.output {
                collect_type_params(output, params, used);
            }
        }
        Type::ImplTrait(impl_trait) => {
            for bound in &impl_trait.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    for segment in &trait_bound.path.segments {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    collect_type_params(inner_ty, params, used);
                                }
                            }
                        }
                    }
                }
            }
        }
        Type::TraitObject(trait_object) => {
            for bound in &trait_object.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    for segment in &trait_bound.path.segments {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    collect_type_params(inner_ty, params, used);
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn generate_transparent_struct_impl(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    field: &FieldInfo<'_>,
    original_fields: &syn::Fields,
) -> TokenStream2 {
    let inner_ty = &field.field.ty;
    let mut_self_access = field.access.access_tokens(quote! { self });
    let self_access = field.access.access_tokens(quote! { self });
    let value_access = field.access.access_tokens(quote! { value });

    let wrap_expr = match original_fields {
        syn::Fields::Unnamed(_) => quote! { Self(inner) },
        syn::Fields::Named(_) => {
            let ident = field.access.ident().expect("expected named field ident for transparent struct");
            quote! { Self { #ident: inner } }
        }
        syn::Fields::Unit => quote! { Self },
    };

    let default_expr = match original_fields {
        syn::Fields::Unnamed(_) => quote! { Self(<#inner_ty as ::proto_rs::ProtoDecoder>::proto_default()) },
        syn::Fields::Named(_) => {
            let ident = field.access.ident().expect("expected named field ident for transparent struct");
            quote! { Self { #ident: <#inner_ty as ::proto_rs::ProtoDecoder>::proto_default() } }
        }
        syn::Fields::Unit => quote! { Self },
    };

    quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = <#inner_ty as ::proto_rs::ProtoExt>::KIND;
        }

        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline(always)]
            fn proto_default() -> Self {
                #default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                <#inner_ty as ::proto_rs::ProtoDecoder>::clear(&mut #mut_self_access);
            }

            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                <#inner_ty as ::proto_rs::ProtoDecoder>::merge_field(
                    &mut #value_access,
                    tag,
                    wire_type,
                    buf,
                    ctx,
                )
            }
        }

        impl #impl_generics ::proto_rs::ProtoShadowDecode<Self> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<Self, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }

        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;
        }

        impl<'__proto_a> #impl_generics ::proto_rs::ProtoShadowEncode<'__proto_a, #name #ty_generics> for &'__proto_a #name #ty_generics #where_clause {
            #[inline(always)]
            fn from_sun(value: &'__proto_a #name #ty_generics) -> Self {
                value
            }
        }

        impl<'__proto_a> #impl_generics ::proto_rs::ProtoArchive for &'__proto_a #name #ty_generics #where_clause {
            type Archived<'__proto_x> = <<#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'__proto_a> as ::proto_rs::ProtoArchive>::Archived<'__proto_x>;

            #[inline(always)]
            fn is_default(&self) -> bool {
                let inner = <#inner_ty as ::proto_rs::ProtoEncode>::Shadow::from_sun(&#self_access);
                <_ as ::proto_rs::ProtoArchive>::is_default(&inner)
            }

            #[inline(always)]
            fn len(archived: &Self::Archived<'_>) -> usize {
                <<#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'__proto_a> as ::proto_rs::ProtoArchive>::len(archived)
            }

            #[inline(always)]
            unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                <<#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'__proto_a> as ::proto_rs::ProtoArchive>::encode(archived, buf)
            }

            #[inline(always)]
            fn archive(&self) -> Self::Archived<'_> {
                let inner = <#inner_ty as ::proto_rs::ProtoEncode>::Shadow::from_sun(&#self_access);
                inner.archive()
            }
        }

        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'__proto_a> = &'__proto_a Self;
        }
    }
}

fn generate_proto_archive_impl_for_ref(
    name: &syn::Ident,
    _impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    fields: &[FieldInfo<'_>],
    generics: &syn::Generics,
) -> TokenStream2 {
    // Generate field archiving expressions
    let field_archives: Vec<_> = fields.iter().filter_map(|info| {
        let tag = info.tag?;
        let field_name = Ident::new(&format!("f{}", tag), info.field.span());
        let access = info.access.access_tokens(quote! { self });
        let proto_ty = &info.proto_ty;

        let converted_value = if needs_encode_conversion(&info.config, &info.parsed) {
            encode_conversion_expr(info, &access)
        } else {
            access.clone()
        };

        Some(quote! {
            let #field_name = ::proto_rs::ArchivedProtoInner::<#tag, #proto_ty>::new(&#converted_value);
        })
    }).collect();

    let field_names: Vec<_> = fields.iter().filter_map(|info| {
        info.tag.map(|tag| Ident::new(&format!("f{}", tag), info.field.span()))
    }).collect();

    let len_sum = if field_names.is_empty() {
        quote! { 0 }
    } else {
        quote! { #(#field_names.len())+* }
    };

    let field_inits: Vec<_> = field_names.iter().map(|name| {
        quote! { #name }
    }).collect();

    let field_encodes: Vec<_> = field_names.iter().map(|name| {
        quote! { #name.encode(buf); }
    }).collect();

    let archived_struct_name = Ident::new(&format!("{}Archived", name), name.span());

    // Create generics for ProtoArchive impl with lifetime
    let mut archive_generics = generics.clone();
    archive_generics.params.insert(0, syn::parse_quote!('__proto_a));
    let (archive_impl_generics, _, archive_where_clause) = archive_generics.split_for_impl();

    // Generate the Archived struct
    let archived_struct_fields: Vec<_> = fields.iter().filter_map(|info| {
        let tag = info.tag?;
        let field_name = Ident::new(&format!("f{}", tag), info.field.span());
        let proto_ty = &info.proto_ty;
        Some(quote! { #field_name: ::proto_rs::ArchivedProtoInner<'__proto_a, #tag, #proto_ty> })
    }).collect();

    let archived_struct_def = quote! {
        #[allow(non_camel_case_types)]
        struct #archived_struct_name<'__proto_a> #archive_where_clause {
            #(#archived_struct_fields),*
        }
    };

    quote! {
        #archived_struct_def

        impl #archive_impl_generics ::proto_rs::ProtoArchive for &'__proto_a #name #ty_generics #archive_where_clause {
            type Archived<'__proto_x> = #archived_struct_name<'__proto_x>;

            #[inline(always)]
            fn is_default(&self) -> bool {
                // A message is default if all fields are default
                #(#field_archives)*
                #(if !#field_names.is_default() { return false; })*
                true
            }

            #[inline(always)]
            fn len(archived: &Self::Archived<'_>) -> usize {
                #len_sum
            }

            #[inline(always)]
            unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #(archived.#field_encodes)*
            }

            #[inline(always)]
            fn archive(&self) -> Self::Archived<'_> {
                #(#field_archives)*
                #archived_struct_name {
                    #(#field_inits),*
                }
            }
        }
    }
}

fn sanitize_struct(mut item: ItemStruct) -> ItemStruct {
    item.attrs = strip_proto_attrs(&item.attrs);
    match &mut item.fields {
        syn::Fields::Named(named) => {
            for field in &mut named.named {
                field.attrs = strip_proto_attrs(&field.attrs);
            }
        }
        syn::Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.unnamed {
                field.attrs = strip_proto_attrs(&field.attrs);
            }
        }
        syn::Fields::Unit => {}
    }
    item
}

fn generate_proto_ext_impl(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    _fields: &[FieldInfo<'_>],
    _config: &UnifiedProtoConfig,
) -> TokenStream2 {
    // New ProtoExt trait only has KIND constant
    quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
        }
    }
}

fn generate_proto_wire_impl(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fields: &[FieldInfo<'_>],
    original_fields: &syn::Fields,
    config: &UnifiedProtoConfig,
    bounded_generics: &syn::Generics,
) -> TokenStream2 {
    let proto_default_expr = build_proto_default_expr(fields, original_fields);
    let self_tokens = quote! { self };
    let clear_stmts = build_clear_stmts(fields, &self_tokens);
    let decode_arms = build_decode_match_arms(fields, &quote! { value });

    let post_decode_hooks = build_post_decode_hooks(fields);
    let message_validation = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! {
            #validator_path(&mut value)?;
        }
    } else {
        quote! {}
    };

    let post_decode_impl = if post_decode_hooks.is_empty() && config.validator.is_none() {
        quote! {}
    } else {
        quote! {
            #[inline(always)]
            fn post_decode(mut value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                #(#post_decode_hooks)*
                #message_validation
                Ok(value)
            }
        }
    };

    let validate_with_ext_impl = build_validate_with_ext_impl(config);

    // ProtoDecoder implementation (for the struct itself as ShadowDecoded)
    let proto_decoder_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline(always)]
            fn proto_default() -> Self {
                #proto_default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                #(#clear_stmts;)*
            }

            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                match tag {
                    #(#decode_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
        }
    };

    // ProtoShadowDecode implementation (Self decodes to Self)
    let proto_shadow_decode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoShadowDecode<Self> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<Self, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }
    };

    // ProtoDecode implementation
    let proto_decode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;

            #post_decode_impl
            #validate_with_ext_impl
        }
    };

    // ProtoShadowEncode implementation (&Self encodes from &Self)
    // Need to merge the lifetime with existing impl_generics
    let mut shadow_encode_generics = bounded_generics.clone();
    shadow_encode_generics.params.insert(0, syn::parse_quote!('__proto_a));
    let (shadow_encode_impl_generics, _, _) = shadow_encode_generics.split_for_impl();

    let proto_shadow_encode_impl = quote! {
        impl #shadow_encode_impl_generics ::proto_rs::ProtoShadowEncode<'__proto_a, #name #ty_generics> for &'__proto_a #name #ty_generics #where_clause {
            #[inline(always)]
            fn from_sun(value: &'__proto_a #name #ty_generics) -> Self {
                value
            }
        }
    };

    // ProtoArchive implementation (for &Self)
    let proto_archive_impl = generate_proto_archive_impl_for_ref(name, impl_generics, ty_generics, fields, &bounded_generics);

    // ProtoEncode implementation
    let proto_encode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'__proto_a> = &'__proto_a Self;
        }
    };

    quote! {
        #proto_decoder_impl
        #proto_shadow_decode_impl
        #proto_decode_impl
        #proto_shadow_encode_impl
        #proto_archive_impl
        #proto_encode_impl
    }
}
