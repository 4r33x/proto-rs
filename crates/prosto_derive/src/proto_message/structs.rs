use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::GenericArgument;
use syn::ItemStruct;
use syn::PathArguments;
use syn::Type;
use syn::parse_quote;
use syn::visit_mut::VisitMut;

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
use super::unified_field_handler::is_value_encode_type;
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
        let transparent_impl = generate_transparent_struct_impl(
            name,
            &item_struct.vis,
            &bounded_generics,
            &impl_generics,
            &ty_generics,
            where_clause,
            &field,
            &data.fields,
        );

        return quote! {
            #struct_item
            #transparent_impl
        };
    }

    let bounded_generics = add_proto_wire_bounds(generics, fields.iter());
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();

    let fields = assign_tags(fields);

    let shadow_ident = syn::Ident::new(&format!("{name}Shadow"), name.span());
    let archived_ident = syn::Ident::new(&format!("{name}Archived"), name.span());

    let shadow_impls = generate_shadow_impls(
        name,
        &shadow_ident,
        &archived_ident,
        &item_struct.vis,
        &data.fields,
        &fields,
        &bounded_generics,
        &ty_generics,
        config.suns.is_empty(),
    );
    let proto_impls = generate_proto_impls(
        name,
        &shadow_ident,
        &archived_ident,
        &bounded_generics,
        &impl_generics,
        &ty_generics,
        where_clause,
        &fields,
        &data.fields,
        config,
    );

    quote! {
        #struct_item
        #shadow_impls
        #proto_impls
    }
}

fn add_transparent_bounds(generics: &syn::Generics, inner_ty: &Type) -> syn::Generics {
    let mut generics = generics.clone();
    let type_params: BTreeSet<_> = generics.type_params().map(|param| param.ident.clone()).collect();
    let where_clause = generics.make_where_clause();
    where_clause
        .predicates
        .push(parse_quote!(#inner_ty: ::proto_rs::ProtoEncode + ::proto_rs::ProtoDecode + ::proto_rs::ProtoDecoder + ::proto_rs::ProtoExt));
    where_clause
        .predicates
        .push(parse_quote!(for<'__proto> <#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'__proto>: ::proto_rs::ProtoArchive + ::proto_rs::ProtoExt));
    where_clause
        .predicates
        .push(parse_quote!(for<'__proto> <#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'__proto>: ::proto_rs::ProtoShadowEncode<'__proto, #inner_ty>));
    where_clause
        .predicates
        .push(parse_quote!(<#inner_ty as ::proto_rs::ProtoDecode>::ShadowDecoded: ::proto_rs::ProtoShadowDecode<#inner_ty>));
    if !type_params.is_empty() {
        let mut used = BTreeSet::new();
        collect_type_params(inner_ty, &type_params, &mut used);
        for ident in used {
            where_clause.predicates.push(
                parse_quote!(#ident: ::proto_rs::ProtoEncode + ::proto_rs::ProtoDecode + ::proto_rs::ProtoDecoder + ::proto_rs::ProtoExt),
            );
            where_clause
                .predicates
                .push(parse_quote!(for<'__proto> <#ident as ::proto_rs::ProtoEncode>::Shadow<'__proto>: ::proto_rs::ProtoArchive + ::proto_rs::ProtoExt));
        }
    }
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

#[allow(clippy::too_many_arguments)]
fn generate_transparent_struct_impl(
    name: &syn::Ident,
    vis: &syn::Visibility,
    generics: &syn::Generics,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    field: &FieldInfo<'_>,
    original_fields: &syn::Fields,
) -> TokenStream2 {
    let inner_ty = &field.field.ty;
    let mut_value_access = field.access.access_tokens(quote! { value });
    let mut_self_access = field.access.access_tokens(quote! { self });
    let shadow_ident = syn::Ident::new(&format!("{name}Shadow"), name.span());

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

    let shadow_ty = quote! { <#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'a> };
    let mut shadow_generics = generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();
    quote! {
        #vis struct #shadow_ident #shadow_impl_generics ( #shadow_ty ) #shadow_where_clause;

        impl #shadow_impl_generics ::proto_rs::ProtoExt for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            const KIND: ::proto_rs::ProtoKind = <#shadow_ty as ::proto_rs::ProtoExt>::KIND;
        }

        impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #name #ty_generics> for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline(always)]
            fn from_sun(value: &'a #name #ty_generics) -> Self {
                Self(<#shadow_ty as ::proto_rs::ProtoShadowEncode<'a, #inner_ty>>::from_sun(&#mut_value_access))
            }
        }

        impl #shadow_impl_generics ::proto_rs::ProtoArchive for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline(always)]
            fn is_default(&self) -> bool {
                <#shadow_ty as ::proto_rs::ProtoArchive>::is_default(&self.0)
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                <#shadow_ty as ::proto_rs::ProtoArchive>::archive::<TAG>(&self.0, w);
            }
        }

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
                <#inner_ty as ::proto_rs::ProtoDecoder>::merge_field(&mut #mut_value_access, tag, wire_type, buf, ctx)
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: ::proto_rs::encoding::WireType, buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<(), ::proto_rs::DecodeError> {
                <#inner_ty as ::proto_rs::ProtoDecoder>::merge(&mut #mut_self_access, wire_type, buf, ctx)
            }
        }

        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;

            #[inline(always)]
            fn decode(mut buf: impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<Self, ::proto_rs::DecodeError> {
                // For transparent types, we need to handle primitives vs messages differently:
                // - Primitives are encoded as raw values (no field tags)
                // - Messages are encoded with field tags
                if <#inner_ty as ::proto_rs::ProtoExt>::WIRE_TYPE.is_length_delimited() {
                    // Message type - decode using standard message decoding
                    let inner = <#inner_ty as ::proto_rs::ProtoDecode>::decode(buf, ctx)?;
                    Ok(#wrap_expr)
                } else {
                    // Primitive type - read raw value using merge
                    let mut inner = <#inner_ty as ::proto_rs::ProtoDecoder>::proto_default();
                    <#inner_ty as ::proto_rs::ProtoDecoder>::merge(&mut inner, <#inner_ty as ::proto_rs::ProtoExt>::WIRE_TYPE, &mut buf, ctx)?;
                    Ok(#wrap_expr)
                }
            }
        }

        impl #impl_generics ::proto_rs::ProtoShadowDecode<#name #ty_generics> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<#name #ty_generics, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }

        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'a> = #shadow_ident #shadow_ty_generics;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_shadow_impls(
    proto_ident: &syn::Ident,
    shadow_ident: &syn::Ident,
    _archived_ident: &syn::Ident,
    vis: &syn::Visibility,
    original_fields: &syn::Fields,
    fields: &[FieldInfo<'_>],
    generics: &syn::Generics,
    ty_generics: &syn::TypeGenerics,
    use_getters: bool,
) -> TokenStream2 {
    let mut shadow_generics = generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();

    let encoded_fields: Vec<_> = fields.iter().filter(|info| info.tag.is_some()).collect();

    let phantom_ident = syn::Ident::new("__proto_phantom", proto_ident.span());
    let mut shadow_field_defs = encoded_fields
        .iter()
        .map(|info| {
            let shadow_ty = shadow_field_ty(info);
            match info.access {
                FieldAccess::Named(ident) => quote! { #ident: #shadow_ty },
                FieldAccess::Direct(_) | FieldAccess::Tuple(_) => quote! { #shadow_ty },
            }
        })
        .collect::<Vec<_>>();

    let mut shadow_init_fields = encoded_fields
        .iter()
        .map(|info| {
            let init = shadow_field_init(info, use_getters);
            match info.access {
                FieldAccess::Named(ident) => quote! { #ident: #init },
                FieldAccess::Direct(_) | FieldAccess::Tuple(_) => quote! { #init },
            }
        })
        .collect::<Vec<_>>();

    let shadow_struct = match original_fields {
        syn::Fields::Named(_) => {
            shadow_field_defs.push(quote! { #phantom_ident: ::core::marker::PhantomData<&'a ()> });
            quote! {
                #vis struct #shadow_ident #shadow_impl_generics #shadow_where_clause {
                    #( #shadow_field_defs, )*
                }
            }
        }
        syn::Fields::Unnamed(_) => {
            shadow_field_defs.push(quote! { ::core::marker::PhantomData<&'a ()> });
            quote! {
                #vis struct #shadow_ident #shadow_impl_generics ( #( #shadow_field_defs, )* ) #shadow_where_clause;
            }
        }
        syn::Fields::Unit => quote! {
            #vis struct #shadow_ident #shadow_impl_generics #shadow_where_clause {
                #phantom_ident: ::core::marker::PhantomData<&'a ()>,
            }
        },
    };

    let shadow_init = match original_fields {
        syn::Fields::Named(_) => {
            shadow_init_fields.push(quote! { #phantom_ident: ::core::marker::PhantomData });
            quote! { Self { #( #shadow_init_fields, )* } }
        }
        syn::Fields::Unnamed(_) => {
            shadow_init_fields.push(quote! { ::core::marker::PhantomData });
            quote! { Self( #( #shadow_init_fields, )* ) }
        }
        syn::Fields::Unit => quote! { Self { #phantom_ident: ::core::marker::PhantomData } },
    };

    let archive_fields = encoded_fields.iter().rev().map(|info| {
        let tag = info.tag.expect("tag required");
        let shadow_ty = shadow_field_ty(info);
        let access = info.access.access_tokens(quote! { self });
        quote! { ::proto_rs::ArchivedProtoField::<#tag, #shadow_ty>::archive(&#access, w); }
    });

    let is_default_checks = encoded_fields.iter().map(|info| {
        let access = info.access.access_tokens(quote! { self });
        quote! { ::proto_rs::ProtoArchive::is_default(&#access) }
    });

    let is_default_expr = if encoded_fields.is_empty() {
        quote! { true }
    } else {
        quote! { #( #is_default_checks )&&* }
    };

    quote! {
        #shadow_struct

        impl #shadow_impl_generics ::proto_rs::ProtoExt for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
        }

        impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #proto_ident #ty_generics> for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline(always)]
            fn from_sun(value: &'a #proto_ident #ty_generics) -> Self {
                #shadow_init
            }
        }

        impl #shadow_impl_generics ::proto_rs::ProtoArchive for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline(always)]
            fn is_default(&self) -> bool {
                #is_default_expr
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                let mark = w.mark();
                #( #archive_fields )*
                if TAG != 0 {
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ::proto_rs::ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn generate_proto_impls(
    name: &syn::Ident,
    shadow_ident: &syn::Ident,
    _archived_ident: &syn::Ident,
    generics: &syn::Generics,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fields: &[FieldInfo<'_>],
    original_fields: &syn::Fields,
    config: &UnifiedProtoConfig,
) -> TokenStream2 {
    let decode_arms = build_decode_match_arms(fields, &quote! { value });
    let proto_default_expr = build_proto_default_expr(fields, original_fields);
    let clear_stmts = build_clear_stmts(fields, &quote! { self });
    let post_decode_hooks = build_post_decode_hooks(fields);
    let validate_with_ext_impl = build_validate_with_ext_impl(config);
    let validate_with_ext_proto_impl = if config.has_suns() {
        TokenStream2::new()
    } else {
        validate_with_ext_impl.clone()
    };

    let message_validation = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! { #validator_path(&mut shadow)?; }
    } else {
        quote! {}
    };

    let post_decode_impl = if post_decode_hooks.is_empty() && config.validator.is_none() {
        quote! {}
    } else {
        quote! {
            #[inline(always)]
            fn post_decode(value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                let mut shadow = value;
                #(#post_decode_hooks)*
                #message_validation
                Ok(shadow)
            }
        }
    };

    let shadow_ty = shadow_type_tokens(generics, shadow_ident);
    let shadow_ty_short = shadow_type_tokens_with_lifetime(generics, shadow_ident, quote! { '_ });
    let has_getters = fields.iter().any(|info| info.config.getter.is_some());
    let sun_shadow_encode_init = if has_getters {
        Some(build_sun_shadow_encode_init(fields, original_fields))
    } else {
        None
    };
    let sun_shadow_encode_init_for_shadow = build_shadow_encode_init(fields, original_fields, has_getters);
    let mut shadow_generics = generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, _shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();

    let sun_impls = if config.has_suns() {
        let sun_impls = config.suns.iter().map(|sun| {
            let target_ty = &sun.ty;
            let sun_ir_ty = sun.ir_ty.as_ref();
            let sun_shadow_encode_impl_for_shadow = sun_ir_ty
                .map(|sun_ir_ty| {
                    let sun_ir_lifetime = syn::Lifetime::new("'sun_ir", proc_macro2::Span::call_site());
                    let sun_ir_ty_param = replace_type_lifetimes(sun_ir_ty, &sun_ir_lifetime);
                    let mut sun_ir_generics = shadow_generics.clone();
                    sun_ir_generics.params.insert(1, parse_quote!('sun_ir));
                    let (sun_ir_impl_generics, _sun_ir_ty_generics, sun_ir_where_clause) = sun_ir_generics.split_for_impl();
                    let sun_shadow_encode_init_for_shadow = &sun_shadow_encode_init_for_shadow;
                    quote! {
                        impl #sun_ir_impl_generics ::proto_rs::ProtoShadowEncode<'a, #sun_ir_ty_param> for #shadow_ty #sun_ir_where_clause {
                            #[inline(always)]
                            fn from_sun(value: &'a #sun_ir_ty_param) -> Self {
                                #sun_shadow_encode_init_for_shadow
                            }
                        }
                    }
                })
                .unwrap_or_default();
            let sun_ir_archive_impl = sun_ir_ty
                .map(|sun_ir_ty| {
                    let sun_ir_ty_short = anonymize_type_lifetimes(sun_ir_ty);
                    quote! {
                        impl #shadow_impl_generics ::proto_rs::ProtoArchive for #sun_ir_ty #shadow_where_clause {
                            #[inline(always)]
                            fn is_default(&self) -> bool {
                                let shadow = <#shadow_ty_short as ::proto_rs::ProtoShadowEncode<'_, #sun_ir_ty_short>>::from_sun(self);
                                <#shadow_ty_short as ::proto_rs::ProtoArchive>::is_default(&shadow)
                            }

                            #[inline(always)]
                            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                                let shadow = <#shadow_ty_short as ::proto_rs::ProtoShadowEncode<'_, #sun_ir_ty_short>>::from_sun(self);
                                <#shadow_ty_short as ::proto_rs::ProtoArchive>::archive::<TAG>(&shadow, w)
                            }
                        }
                    }
                })
                .unwrap_or_default();
            let sun_ir_ext_impl = sun_ir_ty
                .map(|sun_ir_ty| {
                    quote! {
                        impl #shadow_impl_generics ::proto_rs::ProtoExt for #sun_ir_ty #shadow_where_clause {
                            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
                        }
                    }
                })
                .unwrap_or_default();
            let sun_post_decode = if post_decode_hooks.is_empty() && config.validator.is_none() {
                quote! {}
            } else {
                quote! {
                    #[inline(always)]
                    fn post_decode(value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                        let mut shadow = value;
                        #(#post_decode_hooks)*
                        #message_validation
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)
                    }
                }
            };
            let sun_shadow_encode_impl = if let Some(init) = &sun_shadow_encode_init {
                let sun_ir_binding = sun_ir_ty
                    .map(|sun_ir_ty| {
                        quote! {
                            let sun_ir = <#sun_ir_ty as ::proto_rs::ProtoShadowEncode<'a, #target_ty>>::from_sun(value);
                            let value = &sun_ir;
                        }
                    })
                    .unwrap_or_default();
                quote! {
                    impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #target_ty> for #name #ty_generics #shadow_where_clause {
                        #[inline(always)]
                        fn from_sun(value: &'a #target_ty) -> Self {
                            #sun_ir_binding
                            #init
                        }
                    }
                }
            } else {
                quote! {}
            };
            let sun_encode_shadow = if let Some(sun_ir_ty) = sun_ir_ty {
                quote! { #sun_ir_ty }
            } else {
                quote! { #name #ty_generics }
            };
            let sun_encode_shadow_archive = if let Some(sun_ir_ty) = sun_ir_ty {
                let sun_ir_ty_short = anonymize_type_lifetimes(sun_ir_ty);
                quote! { #sun_ir_ty_short }
            } else {
                quote! { #name #ty_generics }
            };
            quote! {
                impl #impl_generics ::proto_rs::ProtoExt for #target_ty #where_clause {
                    const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
                }

                #sun_shadow_encode_impl
                #sun_shadow_encode_impl_for_shadow
                #sun_ir_ext_impl
                #sun_ir_archive_impl

                impl #impl_generics ::proto_rs::ProtoEncode for #target_ty #where_clause {
                    type Shadow<'a> = #sun_encode_shadow;
                }

                impl #impl_generics ::proto_rs::ProtoDecode for #target_ty #where_clause {
                    type ShadowDecoded = #name #ty_generics;
                    #sun_post_decode
                    #validate_with_ext_impl
                }

                impl #impl_generics ::proto_rs::ProtoDecoder for #target_ty #where_clause {
                    #[inline(always)]
                    fn proto_default() -> Self {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoDecoder>::proto_default();
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)
                            .expect("failed to build default sun value")
                    }

                    #[inline(always)]
                    fn clear(&mut self) {
                        *self = Self::proto_default();
                    }

                    #[inline(always)]
                    fn merge_field(
                        value: &mut Self,
                        tag: u32,
                        wire_type: ::proto_rs::encoding::WireType,
                        buf: &mut impl ::proto_rs::bytes::Buf,
                        ctx: ::proto_rs::encoding::DecodeContext,
                    ) -> Result<(), ::proto_rs::DecodeError> {
                        let mut shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(value);
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::merge_field(&mut shadow, tag, wire_type, buf, ctx)?;
                        *value = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }

                    #[inline(always)]
                    fn merge(
                        &mut self,
                        wire_type: ::proto_rs::encoding::WireType,
                        buf: &mut impl ::proto_rs::bytes::Buf,
                        ctx: ::proto_rs::encoding::DecodeContext,
                    ) -> Result<(), ::proto_rs::DecodeError> {
                        let mut shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::merge(&mut shadow, wire_type, buf, ctx)?;
                        *self = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }
                }

                impl #impl_generics ::proto_rs::ProtoArchive for #target_ty #where_clause {
                    #[inline(always)]
                    fn is_default(&self) -> bool {
                        let shadow = <#sun_encode_shadow_archive as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#sun_encode_shadow_archive as ::proto_rs::ProtoArchive>::is_default(&shadow)
                    }

                    #[inline(always)]
                    fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                        let shadow = <#sun_encode_shadow_archive as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#sun_encode_shadow_archive as ::proto_rs::ProtoArchive>::archive::<TAG>(&shadow, w)
                    }
                }
            }
        });
        quote! { #( #sun_impls )* }
    } else {
        quote! {}
    };

    quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
        }

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

        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;
            #post_decode_impl
            #validate_with_ext_proto_impl
        }

        impl #impl_generics ::proto_rs::ProtoShadowDecode<#name #ty_generics> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<#name #ty_generics, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }

        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'a> = #shadow_ty;
        }

        impl #impl_generics ::proto_rs::ProtoArchive for #name #ty_generics #where_clause {
            #[inline(always)]
            fn is_default(&self) -> bool {
                let shadow = <#shadow_ty_short as ::proto_rs::ProtoShadowEncode<'_, #name #ty_generics>>::from_sun(self);
                ::proto_rs::ProtoArchive::is_default(&shadow)
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                let shadow = <#shadow_ty_short as ::proto_rs::ProtoShadowEncode<'_, #name #ty_generics>>::from_sun(self);
                <#shadow_ty_short as ::proto_rs::ProtoArchive>::archive::<TAG>(&shadow, w);
            }
        }

        #sun_impls
    }
}

fn shadow_field_ty(info: &FieldInfo<'_>) -> TokenStream2 {
    if needs_encode_conversion(&info.config, &info.parsed) {
        let proto_ty = &info.proto_ty;
        quote! { #proto_ty }
    } else {
        let field_ty = &info.field.ty;
        quote! { <#field_ty as ::proto_rs::ProtoEncode>::Shadow<'a> }
    }
}

fn shadow_field_init(info: &FieldInfo<'_>, use_getters: bool) -> TokenStream2 {
    let base = quote! { value };
    let access_expr = if use_getters && let Some(getter) = &info.config.getter {
        parse_getter_expr(getter, &base, info.field)
    } else {
        info.access.access_tokens(base)
    };
    let ref_expr = if use_getters && info.config.getter.is_some() {
        access_expr.clone()
    } else {
        quote! { &#access_expr }
    };

    if needs_encode_conversion(&info.config, &info.parsed) {
        encode_conversion_expr(info, &ref_expr)
    } else {
        let field_ty = &info.field.ty;
        let shadow_ty = shadow_field_ty(info);
        quote! { <#shadow_ty as ::proto_rs::ProtoShadowEncode<'a, #field_ty>>::from_sun(#ref_expr) }
    }
}

fn parse_getter_expr(getter: &str, base: &TokenStream2, field: &syn::Field) -> TokenStream2 {
    let base_str = base.to_string();
    let getter_expr = getter.replace('$', &base_str);
    syn::parse_str::<TokenStream2>(&getter_expr).unwrap_or_else(|_| {
        panic!(
            "invalid getter expression in #[proto(getter = ...)] on field {}",
            field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string)
        )
    })
}

fn sun_field_init(info: &FieldInfo<'_>) -> TokenStream2 {
    let base = quote! { value };
    let access_expr = if let Some(getter) = &info.config.getter {
        parse_getter_expr(getter, &base, info.field)
    } else {
        info.access.access_tokens(base)
    };
    let borrowed_expr = quote! { ::core::borrow::Borrow::borrow(&#access_expr) };

    if needs_encode_conversion(&info.config, &info.parsed) {
        encode_conversion_expr(info, &borrowed_expr)
    } else if is_value_encode_type(&info.field.ty) {
        quote! { *#borrowed_expr }
    } else {
        quote! { (*#borrowed_expr).clone() }
    }
}

fn build_sun_shadow_encode_init(fields: &[FieldInfo<'_>], original_fields: &syn::Fields) -> TokenStream2 {
    match original_fields {
        syn::Fields::Named(_) => {
            let inits = fields.iter().map(|info| {
                let ident = info.access.ident().expect("expected named field ident");
                let init = sun_field_init(info);
                quote! { #ident: #init }
            });
            quote! { Self { #( #inits, )* } }
        }
        syn::Fields::Unnamed(_) => {
            let inits = fields.iter().map(sun_field_init);
            quote! { Self( #( #inits, )* ) }
        }
        syn::Fields::Unit => quote! { Self },
    }
}

fn build_shadow_encode_init(fields: &[FieldInfo<'_>], original_fields: &syn::Fields, use_getters: bool) -> TokenStream2 {
    let phantom_ident = syn::Ident::new("__proto_phantom", proc_macro2::Span::call_site());
    match original_fields {
        syn::Fields::Named(_) => {
            let inits = fields.iter().map(|info| {
                let ident = info.access.ident().expect("expected named field ident");
                let init = shadow_field_init(info, use_getters);
                quote! { #ident: #init }
            });
            quote! { Self { #( #inits, )* #phantom_ident: ::core::marker::PhantomData } }
        }
        syn::Fields::Unnamed(_) => {
            let inits = fields.iter().map(|info| shadow_field_init(info, use_getters));
            quote! { Self( #( #inits, )* ::core::marker::PhantomData ) }
        }
        syn::Fields::Unit => quote! { Self { #phantom_ident: ::core::marker::PhantomData } },
    }
}

struct AnonLifetimes;
impl VisitMut for AnonLifetimes {
    fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
        *lifetime = syn::Lifetime::new("'_", lifetime.span());
    }
}

fn anonymize_type_lifetimes(ty: &Type) -> Type {
    let mut ty = ty.clone();

    let mut visitor = AnonLifetimes;
    visitor.visit_type_mut(&mut ty);
    ty
}

struct ReplaceLifetimes<'a> {
    replacement: &'a syn::Lifetime,
}
impl VisitMut for ReplaceLifetimes<'_> {
    fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
        *lifetime = self.replacement.clone();
    }
}
fn replace_type_lifetimes(ty: &Type, replacement: &syn::Lifetime) -> Type {
    let mut ty = ty.clone();

    let mut visitor = ReplaceLifetimes { replacement };
    visitor.visit_type_mut(&mut ty);
    ty
}

fn shadow_type_tokens(generics: &syn::Generics, shadow_ident: &syn::Ident) -> TokenStream2 {
    shadow_type_tokens_with_lifetime(generics, shadow_ident, quote! { 'a })
}

fn shadow_type_tokens_with_lifetime(generics: &syn::Generics, shadow_ident: &syn::Ident, lifetime: TokenStream2) -> TokenStream2 {
    let params: Vec<TokenStream2> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(ty) => {
                let ident = &ty.ident;
                Some(quote! { #ident })
            }
            syn::GenericParam::Const(konst) => {
                let ident = &konst.ident;
                Some(quote! { #ident })
            }
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    if params.is_empty() {
        quote! { #shadow_ident<#lifetime> }
    } else {
        quote! { #shadow_ident<#lifetime, #(#params),*> }
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
