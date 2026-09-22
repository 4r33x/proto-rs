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
use super::unified_field_handler::build_decode_match_arms;
use super::unified_field_handler::build_field_validator_hooks_for_base;
use super::unified_field_handler::build_post_decode_hooks;
use super::unified_field_handler::build_post_decode_hooks_for_base;
use super::unified_field_handler::build_proto_default_expr;
use super::unified_field_handler::encode_conversion_expr;
use super::unified_field_handler::encode_conversion_expr_direct;
use super::unified_field_handler::needs_encode_conversion;
use super::unified_field_handler::strip_proto_attrs;
use crate::parse::UnifiedProtoConfig;

pub(super) fn generate_struct_impl(
    input: &DeriveInput,
    item_struct: &ItemStruct,
    data: &syn::DataStruct,
    config: &UnifiedProtoConfig,
) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;

    let struct_item = sanitize_struct(item_struct.clone());

    let mut fields = data
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let access = field.ident.as_ref().map_or(FieldAccess::Tuple(index), FieldAccess::Named);
            FieldInfo::new(index, field, access)
        })
        .collect::<Vec<_>>();

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
            config,
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

    let has_sun_ir = config.suns.iter().any(|sun| sun.ir_ty.is_some());
    let shadow_impls = if has_sun_ir {
        TokenStream2::new()
    } else {
        generate_shadow_impls(
            name,
            &shadow_ident,
            &item_struct.vis,
            &data.fields,
            &fields,
            &bounded_generics,
            &ty_generics,
            config.suns.is_empty(),
        )
    };
    let proto_impls = generate_proto_impls(
        name,
        &shadow_ident,
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
    config: &UnifiedProtoConfig,
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
        syn::Fields::Unnamed(_) => quote! { Self(<#inner_ty as ::proto_rs::ProtoDefault>::proto_default()) },
        syn::Fields::Named(_) => {
            let ident = field.access.ident().expect("expected named field ident for transparent struct");
            quote! { Self { #ident: <#inner_ty as ::proto_rs::ProtoDefault>::proto_default() } }
        }
        syn::Fields::Unit => quote! { Self },
    };

    let shadow_ty = quote! { <#inner_ty as ::proto_rs::ProtoEncode>::Shadow<'a> };
    let field_validation_value = if let Some(validator_fn) = &field.config.validator {
        let validator_path = super::unified_field_handler::parse_path_string(field.field, validator_fn);
        let access = field.access.access_tokens(quote! { value });
        quote! { #validator_path(&mut #access)?; }
    } else {
        quote! {}
    };
    let field_validation_self = if let Some(validator_fn) = &field.config.validator {
        let validator_path = super::unified_field_handler::parse_path_string(field.field, validator_fn);
        let access = field.access.access_tokens(quote! { self });
        quote! { #validator_path(&mut #access)?; }
    } else {
        quote! {}
    };
    let message_validation_value = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! { #validator_path(&mut value)?; }
    } else {
        quote! {}
    };
    let message_validation_self = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! { #validator_path(self)?; }
    } else {
        quote! {}
    };
    let defer_hooks = if field.config.validator.is_some() || config.validator.is_some() {
        quote! { state.defer(); }
    } else {
        quote! {}
    };
    let mut shadow_generics = generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();
    quote! {
        #vis struct #shadow_ident #shadow_impl_generics ( #shadow_ty ) #shadow_where_clause;

        impl #shadow_impl_generics ::proto_rs::ProtoExt for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            const KIND: ::proto_rs::ProtoKind = <#shadow_ty as ::proto_rs::ProtoExt>::KIND;
            const WRAP_ROOT: bool = <#shadow_ty as ::proto_rs::ProtoExt>::WRAP_ROOT;
            const ENCODED_SIZE_HINT: ::proto_rs::EncodeSizeHint = <#shadow_ty as ::proto_rs::ProtoExt>::ENCODED_SIZE_HINT;
        }

        impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #name #ty_generics> for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline]
            fn from_sun(value: &'a #name #ty_generics) -> Self {
                Self(<#shadow_ty as ::proto_rs::ProtoShadowEncode<'a, #inner_ty>>::from_sun(&#mut_value_access))
            }
        }

        impl #shadow_impl_generics ::proto_rs::ProtoArchive for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline]
            fn is_default(&self) -> bool {
                <#shadow_ty as ::proto_rs::ProtoArchive>::is_default(&self.0)
            }

            #[inline]
            fn encoded_size_hint<const TAG: u32>(&self) -> ::proto_rs::EncodeSizeHint {
                <#shadow_ty as ::proto_rs::ProtoArchive>::encoded_size_hint::<TAG>(&self.0)
            }

            #[inline]
            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                <#shadow_ty as ::proto_rs::ProtoArchive>::archive::<TAG>(&self.0, w);
            }
        }

        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = <#inner_ty as ::proto_rs::ProtoExt>::KIND;
            const WRAP_ROOT: bool = <#inner_ty as ::proto_rs::ProtoExt>::WRAP_ROOT;
        }

        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline]
            fn finish(&mut self, state: &::proto_rs::DecodeState<'_>) -> Result<(), ::proto_rs::DecodeError> {
                <#inner_ty as ::proto_rs::ProtoDecoder>::finish(&mut #mut_self_access, &state.field(0))?;
                if state.pending() {
                    #field_validation_self
                    #message_validation_self
                }
                Ok(())
            }

            #[inline]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let state = ::proto_rs::DecodeState::default();
                Self::merge_field_with_state(value, tag, wire_type, buf, ctx, &state)?;
                Self::finish(value, &state)
            }
            #[inline]
            fn merge_field_with_state(
                value: &mut Self, tag: u32, wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext,
                state: &::proto_rs::DecodeState<'_>,
            ) -> Result<(), ::proto_rs::DecodeError> {
                <#inner_ty as ::proto_rs::ProtoDecoder>::merge_field_with_state(&mut #mut_value_access, tag, wire_type, buf, ctx, &state.field(0))
            }

            #[inline]
            fn merge(&mut self, wire_type: ::proto_rs::encoding::WireType, buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<(), ::proto_rs::DecodeError> {
                let state = ::proto_rs::DecodeState::default();
                self.merge_with_state(wire_type, buf, ctx, &state)?;
                self.finish(&state)
            }
            #[inline]
            fn merge_with_state(&mut self, wire_type: ::proto_rs::encoding::WireType, buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext, state: &::proto_rs::DecodeState<'_>) -> Result<(), ::proto_rs::DecodeError> {
                <#inner_ty as ::proto_rs::ProtoDecoder>::merge_with_state(&mut #mut_self_access, wire_type, buf, ctx, &state.field(0))?;
                #defer_hooks
                Ok(())
            }

            #[inline]
            fn decode(buf: impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<Self, ::proto_rs::DecodeError>
            where
                Self: ::proto_rs::ProtoDefault,
            {
                <Self as ::proto_rs::ProtoDecode>::decode(buf, ctx)
            }
        }

        impl #impl_generics ::proto_rs::ProtoDefault for #name #ty_generics #where_clause {
            #[inline]
            fn proto_default() -> Self {
                #default_expr
            }
        }

        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;

            #[inline]
            fn decode(mut buf: impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<Self, ::proto_rs::DecodeError> {
                let inner = <#inner_ty as ::proto_rs::ProtoDecode>::decode(buf, ctx)?;
                let mut value = #wrap_expr;
                #field_validation_value
                #message_validation_value
                Ok(value)
            }
        }

        impl #impl_generics ::proto_rs::ProtoShadowDecode<#name #ty_generics> for #name #ty_generics #where_clause {
            #[inline]
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

    // Tuple shadows omit skipped fields, so their positions differ from the source.
    let shadow_access = |index: usize, info: &FieldInfo<'_>| match info.access {
        FieldAccess::Tuple(_) => {
            let index = syn::Index::from(index);
            quote! { self.#index }
        }
        _ => info.access.access_tokens(quote! { self }),
    };
    let archive_fields = encoded_fields.iter().enumerate().rev().map(|(index, info)| {
        let tag = info.tag.expect("tag required");
        let shadow_ty = shadow_field_ty(info);
        let access = shadow_access(index, info);
        quote! { ::proto_rs::ArchivedProtoField::<#tag, #shadow_ty>::archive(&#access, w); }
    });

    let is_default_checks = encoded_fields.iter().enumerate().map(|(index, info)| {
        let access = shadow_access(index, info);
        quote! { ::proto_rs::ProtoArchive::is_default(&#access) }
    });

    let is_default_expr = if encoded_fields.is_empty() {
        quote! { true }
    } else {
        quote! { #( #is_default_checks )&&* }
    };

    let value_size_hints = encoded_fields.iter().enumerate().map(|(index, info)| {
        let tag = info.tag.expect("tag required");
        let access = shadow_access(index, info);
        quote! { ::proto_rs::ProtoArchive::encoded_size_hint::<#tag>(&#access) }
    });

    let size_hint_expr = encoded_fields.iter().fold(quote! { ::proto_rs::EncodeSizeHint::new(0, true) }, |hint, info| {
        let tag = info.tag.expect("tag required");
        let shadow_ty = shadow_field_ty(info);
        quote! {
            (#hint).add_field::<#tag>(
                <#shadow_ty as ::proto_rs::ProtoExt>::ENCODED_SIZE_HINT,
                <#shadow_ty as ::proto_rs::ProtoExt>::WIRE_TYPE,
            )
        }
    });

    quote! {
        #shadow_struct

        impl #shadow_impl_generics ::proto_rs::ProtoExt for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
            const ENCODED_SIZE_HINT: ::proto_rs::EncodeSizeHint = #size_hint_expr;
        }

        impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #proto_ident #ty_generics> for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline]
            fn from_sun(value: &'a #proto_ident #ty_generics) -> Self {
                #shadow_init
            }
        }

        impl #shadow_impl_generics ::proto_rs::ProtoArchive for #shadow_ident #shadow_ty_generics #shadow_where_clause {
            #[inline]
            fn is_default(&self) -> bool {
                #is_default_expr
            }

            #[inline]
            fn encoded_size_hint<const TAG: u32>(&self) -> ::proto_rs::EncodeSizeHint {
                let payload = ::proto_rs::EncodeSizeHint::EMPTY #( .add(#value_size_hints) )*;
                payload.for_field::<TAG>(<Self as ::proto_rs::ProtoExt>::WIRE_TYPE)
            }

            #[inline]
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
    let post_decode_hooks = build_post_decode_hooks(fields);
    let field_validator_hooks = build_field_validator_hooks_for_base(fields, &quote! { shadow });
    let merge_post_decode_hooks = build_post_decode_hooks_for_base(fields, &quote! { (*self) });
    let merge_field_validator_hooks = build_field_validator_hooks_for_base(fields, &quote! { (*self) });
    let defer_hooks = if !merge_post_decode_hooks.is_empty() || !merge_field_validator_hooks.is_empty() || config.validator.is_some() {
        quote! { state.defer(); }
    } else {
        quote! {}
    };
    let finish_fields = fields
        .iter()
        .filter(|info| info.tag.is_some() && !super::unified_field_handler::needs_decode_conversion(&info.config, &info.parsed))
        .map(|info| {
            let tag = info.tag.unwrap();
            let ty = &info.field.ty;
            let access = info.access.access_tokens(quote! { self });
            quote! { <#ty as ::proto_rs::ProtoFieldMerge>::finish_value(&mut #access, &state.field(#tag))?; }
        });
    let decode_post_decode_hooks = build_post_decode_hooks_for_base(fields, &quote! { value });
    let decode_field_validator_hooks = build_field_validator_hooks_for_base(fields, &quote! { value });
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
    let merge_message_validation = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! { #validator_path(self)?; }
    } else {
        quote! {}
    };
    let decode_message_validation = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! { #validator_path(&mut value)?; }
    } else {
        quote! {}
    };

    let post_decode_impl = if post_decode_hooks.is_empty() && field_validator_hooks.is_empty() && config.validator.is_none() {
        quote! {}
    } else {
        quote! {
            #[inline]
            fn post_decode(value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                let mut shadow = value;
                #(#post_decode_hooks)*
                #(#field_validator_hooks)*
                #message_validation
                Ok(shadow)
            }
        }
    };

    let shadow_ty = shadow_type_tokens(generics, shadow_ident);
    let shadow_ty_short = shadow_type_tokens_with_lifetime(generics, shadow_ident, quote! { '_ });
    let has_getters = fields.iter().any(|info| info.config.getter.is_some());
    let has_sun_ir = config.suns.iter().any(|sun| sun.ir_ty.is_some());
    let proto_size_hint = if has_sun_ir {
        quote! { ::proto_rs::EncodeSizeHint::UNKNOWN }
    } else {
        quote! { <#shadow_ty_short as ::proto_rs::ProtoExt>::ENCODED_SIZE_HINT }
    };
    let proto_archive_impl = if has_sun_ir {
        quote! {}
    } else {
        quote! {
            impl #impl_generics ::proto_rs::ProtoArchive for #name #ty_generics #where_clause {
                #[inline]
                fn is_default(&self) -> bool {
                    let shadow = <#shadow_ty_short as ::proto_rs::ProtoShadowEncode<'_, #name #ty_generics>>::from_sun(self);
                    ::proto_rs::ProtoArchive::is_default(&shadow)
                }


                #[inline]
                fn encoded_size_hint<const TAG: u32>(&self) -> ::proto_rs::EncodeSizeHint {
                    <Self as ::proto_rs::ProtoEncode>::size_hint::<TAG>(self)
                }

                #[inline]
                fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                    let shadow = <#shadow_ty_short as ::proto_rs::ProtoShadowEncode<'_, #name #ty_generics>>::from_sun(self);
                    <#shadow_ty_short as ::proto_rs::ProtoArchive>::archive::<TAG>(&shadow, w);
                }
            }
        }
    };
    let cheap_field_hints = fields.iter().filter_map(|info| {
        let tag = info.tag?;
        let ty = &info.field.ty;
        let access = info.access.access_tokens(quote! { self });
        Some(
            if needs_encode_conversion(&info.config, &info.parsed) || info.config.getter.is_some() {
                let ty = &info.proto_ty;
                quote! { <#ty as ::proto_rs::ProtoExt>::ENCODED_SIZE_HINT.for_field::<#tag>(<#ty as ::proto_rs::ProtoExt>::WIRE_TYPE) }
            } else {
                quote! { <#ty as ::proto_rs::ProtoEncode>::size_hint::<#tag>(&#access) }
            },
        )
    });
    let proto_encode_impl = if has_sun_ir {
        quote! {}
    } else {
        quote! {
            impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
                type Shadow<'a> = #shadow_ty;

                #[inline]
                fn size_hint<const TAG: u32>(&self) -> ::proto_rs::EncodeSizeHint {
                    let payload = ::proto_rs::EncodeSizeHint::EMPTY #(.add(#cheap_field_hints))*;
                    payload.for_field::<TAG>(<Self as ::proto_rs::ProtoExt>::WIRE_TYPE)
                }
            }
        }
    };
    let mut shadow_generics = generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, _shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();

    let sun_impls = if config.has_suns() {
        let sun_impls = config.suns.iter().map(|sun| {
            let target_ty = &sun.ty;
            let sun_ir_ty = sun.ir_ty.as_ref();
            let sun_ir_archive_impl = sun_ir_ty.map_or_default(|sun_ir_ty| {
                let mut sun_ir_archive_generics = shadow_generics.clone();
                sun_ir_archive_generics.make_where_clause().predicates.push(parse_quote!(#sun_ir_ty: 'a));
                let (sun_ir_archive_impl_generics, _sun_ir_archive_ty_generics, sun_ir_archive_where_clause) =
                    sun_ir_archive_generics.split_for_impl();
                let shadow_lifetime = quote! { '_ };
                let encoded_fields: Vec<_> = fields.iter().filter(|info| info.tag.is_some()).collect();
                let is_default_checks = encoded_fields.iter().map(|info| {
                    let base = quote! { self };
                    let (access_expr, _) = if has_getters && let Some(get) = &info.config.getter {
                        parse_getter_expr(get, &base, info.field)
                    } else {
                        (info.access.access_tokens(base), false)
                    };
                    let shadow_ty = shadow_field_ty_with_lifetime(info, &shadow_lifetime);
                    let shadow_init = if needs_encode_conversion(&info.config, &info.parsed) {
                        let ref_expr = quote! { #access_expr };
                        let converted = encode_conversion_expr(info, &ref_expr);
                        quote! { let __proto_shadow = #converted; }
                    } else {
                        let field_ty = &info.field.ty;
                        quote! {
                            let __proto_value = #access_expr;
                            let __proto_shadow =
                                <#shadow_ty as ::proto_rs::ProtoShadowEncode<#shadow_lifetime, #field_ty>>::from_sun(&__proto_value);
                        }
                    };
                    quote! {
                        {
                            #shadow_init
                            if !::proto_rs::ProtoArchive::is_default(&__proto_shadow) {
                                return false;
                            }
                        }
                    }
                });
                let archive_fields = encoded_fields.iter().rev().map(|info| {
                    let tag = info.tag.expect("tag required");
                    let base = quote! { self };
                    let (access_expr, _) = if has_getters && let Some(get) = &info.config.getter {
                        parse_getter_expr(get, &base, info.field)
                    } else {
                        (info.access.access_tokens(base), false)
                    };
                    let shadow_ty = shadow_field_ty_with_lifetime(info, &shadow_lifetime);
                    let shadow_init = if needs_encode_conversion(&info.config, &info.parsed) {
                        let ref_expr = quote! { #access_expr };
                        let converted = encode_conversion_expr(info, &ref_expr);
                        quote! { let __proto_shadow = #converted; }
                    } else {
                        let field_ty = &info.field.ty;
                        quote! {
                            let __proto_value = #access_expr;
                            let __proto_shadow =
                                <#shadow_ty as ::proto_rs::ProtoShadowEncode<#shadow_lifetime, #field_ty>>::from_sun(&__proto_value);
                        }
                    };
                    quote! {
                        {
                            #shadow_init
                            ::proto_rs::ArchivedProtoField::<#tag, #shadow_ty>::archive(&__proto_shadow, w);
                        }
                    }
                });
                let size_hint_fields = encoded_fields.iter().map(|info| {
                    let tag = info.tag.expect("tag required");
                    let base = quote! { self };
                    let (access_expr, _) = if has_getters && let Some(get) = &info.config.getter {
                        parse_getter_expr(get, &base, info.field)
                    } else {
                        (info.access.access_tokens(base), false)
                    };
                    let shadow_ty = shadow_field_ty_with_lifetime(info, &shadow_lifetime);
                    let shadow_init = if needs_encode_conversion(&info.config, &info.parsed) {
                        let ref_expr = quote! { #access_expr };
                        let converted = encode_conversion_expr(info, &ref_expr);
                        quote! { let __proto_shadow = #converted; }
                    } else {
                        let field_ty = &info.field.ty;
                        quote! {
                            let __proto_value = #access_expr;
                            let __proto_shadow =
                                <#shadow_ty as ::proto_rs::ProtoShadowEncode<#shadow_lifetime, #field_ty>>::from_sun(&__proto_value);
                        }
                    };
                    quote! {
                        {
                            #shadow_init
                            ::proto_rs::ProtoArchive::encoded_size_hint::<#tag>(&__proto_shadow)
                        }
                    }
                });
                quote! {
                    impl #sun_ir_archive_impl_generics ::proto_rs::ProtoArchive for #sun_ir_ty #sun_ir_archive_where_clause {
                        #[inline]
                        fn is_default(&self) -> bool {
                            #( #is_default_checks )*
                            true
                        }

                        #[inline]
                        fn encoded_size_hint<const TAG: u32>(&self) -> ::proto_rs::EncodeSizeHint {
                            let payload = ::proto_rs::EncodeSizeHint::EMPTY #( .add(#size_hint_fields) )*;
                            payload.for_field::<TAG>(<Self as ::proto_rs::ProtoExt>::WIRE_TYPE)
                        }

                        #[inline]
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
            });
            let sun_ir_ext_impl = sun_ir_ty.map_or_default(|sun_ir_ty| {
                let mut sun_ir_ext_generics = shadow_generics.clone();
                sun_ir_ext_generics.make_where_clause().predicates.push(parse_quote!(#sun_ir_ty: 'a));
                let (sun_ir_ext_impl_generics, _sun_ir_ext_ty_generics, sun_ir_ext_where_clause) = sun_ir_ext_generics.split_for_impl();
                quote! {
                    impl #sun_ir_ext_impl_generics ::proto_rs::ProtoExt for #sun_ir_ty #sun_ir_ext_where_clause {
                        const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
                    }
                }
            });
            let sun_post_decode = if post_decode_hooks.is_empty() && field_validator_hooks.is_empty() && config.validator.is_none() {
                quote! {}
            } else {
                quote! {
                    #[inline]
                    fn post_decode(value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                        let mut shadow = value;
                        #(#post_decode_hooks)*
                        #(#field_validator_hooks)*
                        #message_validation
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)
                    }
                }
            };
            let sun_shadow_encode_impl = if sun_ir_ty.is_none() && has_getters {
                let init = build_sun_shadow_encode_init(fields, original_fields);
                quote! {
                    impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #target_ty> for #name #ty_generics #shadow_where_clause {
                        #[inline]
                        fn from_sun(value: &'a #target_ty) -> Self {
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
            let sun_decode_shadow_init_self = if sun_ir_ty.is_some() {
                quote! {
                    let mut shadow = <#target_ty as ::proto_rs::DecodeIrBuilder<#name #ty_generics>>::build_ir(self)?;
                }
            } else {
                quote! { let mut shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self); }
            };
            quote! {
                impl #impl_generics ::proto_rs::ProtoExt for #target_ty #where_clause {
                    const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
                }

                #sun_shadow_encode_impl
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

                impl #impl_generics ::proto_rs::ProtoDefault for #target_ty #where_clause {
                    #[inline]
                    fn proto_default() -> Self {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoDefault>::proto_default();
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)
                            .expect("failed to build default sun value")
                    }
                }

                impl #impl_generics ::proto_rs::ProtoFieldMerge for #target_ty #where_clause {
                    #[inline]
                    fn finish_value(&mut self, state: &::proto_rs::DecodeState<'_>) -> Result<(), ::proto_rs::DecodeError> {
                        if !state.has_data() { return Ok(()); }
                        #sun_decode_shadow_init_self
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::finish(&mut shadow, state)?;
                        *self = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }

                    #[inline]
                    fn merge_value_with_state(
                        &mut self,
                        wire_type: ::proto_rs::encoding::WireType,
                        buf: &mut impl ::proto_rs::bytes::Buf,
                        ctx: ::proto_rs::encoding::DecodeContext,
                        state: &::proto_rs::DecodeState<'_>,
                    ) -> Result<(), ::proto_rs::DecodeError> {
                        #sun_decode_shadow_init_self
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::merge_with_state(&mut shadow, wire_type, buf, ctx, state)?;
                        *self = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }

                    #[inline]
                    fn merge_value(
                        &mut self,
                        wire_type: ::proto_rs::encoding::WireType,
                        buf: &mut impl ::proto_rs::bytes::Buf,
                        ctx: ::proto_rs::encoding::DecodeContext,
                    ) -> Result<(), ::proto_rs::DecodeError> {
                        #sun_decode_shadow_init_self
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::merge(&mut shadow, wire_type, buf, ctx)?;
                        *self = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }
                }

                impl #impl_generics ::proto_rs::ProtoArchive for #target_ty #where_clause {
                    #[inline]
                    fn is_default(&self) -> bool {
                        let shadow = <#sun_encode_shadow_archive as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#sun_encode_shadow_archive as ::proto_rs::ProtoArchive>::is_default(&shadow)
                    }


                    #[inline]
                    fn encoded_size_hint<const TAG: u32>(&self) -> ::proto_rs::EncodeSizeHint {
                        <Self as ::proto_rs::ProtoEncode>::size_hint::<TAG>(self)
                    }

                    #[inline]
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
            const ENCODED_SIZE_HINT: ::proto_rs::EncodeSizeHint = #proto_size_hint;
        }

        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline]
            fn finish(&mut self, state: &::proto_rs::DecodeState<'_>) -> Result<(), ::proto_rs::DecodeError> {
                if !state.has_data() { return Ok(()); }
                #(#finish_fields)*
                if state.pending() {
                    #(#merge_post_decode_hooks)*
                    #(#merge_field_validator_hooks)*
                    #merge_message_validation
                }
                Ok(())
            }

            #[inline]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let state = ::proto_rs::DecodeState::default();
                Self::merge_field_with_state(value, tag, wire_type, buf, ctx, &state)?;
                Self::finish(value, &state)
            }
            #[inline]
            fn merge_field_with_state(
                value: &mut Self, tag: u32, wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext,
                state: &::proto_rs::DecodeState<'_>,
            ) -> Result<(), ::proto_rs::DecodeError> {
                match tag {
                    #(#decode_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            #[inline]
            fn merge(&mut self, wire_type: ::proto_rs::encoding::WireType, buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<(), ::proto_rs::DecodeError> {
                let state = ::proto_rs::DecodeState::default();
                self.merge_with_state(wire_type, buf, ctx, &state)?;
                self.finish(&state)
            }
            #[inline]
            fn merge_with_state(&mut self, wire_type: ::proto_rs::encoding::WireType, buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext, state: &::proto_rs::DecodeState<'_>) -> Result<(), ::proto_rs::DecodeError> {
                self.merge_message_fields(wire_type, buf, ctx, state)?;
                #defer_hooks
                Ok(())
            }

            #[inline]
            fn decode(mut buf: impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<Self, ::proto_rs::DecodeError>
            where
                Self: ::proto_rs::ProtoDefault,
            {
                ctx.limit_reached()?;
                let mut value = <Self as ::proto_rs::ProtoDefault>::proto_default();
                Self::decode_into(&mut value, &mut buf, ctx)?;
                #(#decode_post_decode_hooks)*
                #(#decode_field_validator_hooks)*
                #decode_message_validation
                Ok(value)
            }
        }

        impl #impl_generics ::proto_rs::ProtoDefault for #name #ty_generics #where_clause {
            #[inline]
            fn proto_default() -> Self {
                #proto_default_expr
            }
        }

        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;
            #post_decode_impl
            #validate_with_ext_proto_impl
        }

        impl #impl_generics ::proto_rs::ProtoShadowDecode<#name #ty_generics> for #name #ty_generics #where_clause {
            #[inline]
            fn to_sun(self) -> Result<#name #ty_generics, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }

        #proto_encode_impl

        #proto_archive_impl

        #sun_impls
    }
}

fn shadow_field_ty(info: &FieldInfo<'_>) -> TokenStream2 {
    shadow_field_ty_with_lifetime(info, &quote! { 'a })
}

fn shadow_field_ty_with_lifetime(info: &FieldInfo<'_>, lifetime: &TokenStream2) -> TokenStream2 {
    if needs_encode_conversion(&info.config, &info.parsed) {
        let proto_ty = &info.proto_ty;
        quote! { #proto_ty }
    } else {
        let field_ty = &info.field.ty;
        quote! { <#field_ty as ::proto_rs::ProtoEncode>::Shadow<#lifetime> }
    }
}

fn shadow_field_init(info: &FieldInfo<'_>, use_getters: bool) -> TokenStream2 {
    shadow_field_init_with_lifetime(info, use_getters, &quote! { 'a }, &quote! { value })
}

fn shadow_field_init_with_lifetime(info: &FieldInfo<'_>, use_getters: bool, lifetime: &TokenStream2, base: &TokenStream2) -> TokenStream2 {
    let (access_expr, getter_is_ref) = if use_getters && let Some(getter) = &info.config.getter {
        parse_getter_expr(getter, base, info.field)
    } else {
        (info.access.access_tokens(base.clone()), false)
    };
    let ref_expr = if use_getters && info.config.getter.is_some() {
        if getter_is_ref {
            access_expr.clone()
        } else {
            quote! { &#access_expr }
        }
    } else {
        quote! { &#access_expr }
    };

    if needs_encode_conversion(&info.config, &info.parsed) {
        encode_conversion_expr(info, &ref_expr)
    } else {
        let field_ty = &info.field.ty;
        let shadow_ty = shadow_field_ty_with_lifetime(info, lifetime);
        quote! { <#shadow_ty as ::proto_rs::ProtoShadowEncode<#lifetime, #field_ty>>::from_sun(#ref_expr) }
    }
}

fn parse_getter_expr(getter: &str, base: &TokenStream2, field: &syn::Field) -> (TokenStream2, bool) {
    let base_str = base.to_string();
    let getter_expr = getter.replace('$', &base_str);
    let expr = syn::parse_str::<syn::Expr>(&getter_expr).unwrap_or_else(|_| {
        panic!(
            "invalid getter expression in #[proto(getter = ...)] on field {}",
            field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string)
        )
    });
    let is_ref = matches!(expr, syn::Expr::Reference(_));
    (quote! { #expr }, is_ref)
}

fn sun_field_init_with_base(
    info: &FieldInfo<'_>,
    base: &TokenStream2,
    apply_encode_conversion: bool,
    clone_getter_refs: bool,
) -> TokenStream2 {
    let (access_expr, getter_is_ref) = if let Some(getter) = &info.config.getter {
        parse_getter_expr(getter, base, info.field)
    } else {
        (info.access.access_tokens(base.clone()), false)
    };

    if apply_encode_conversion && needs_encode_conversion(&info.config, &info.parsed) {
        encode_conversion_expr_direct(info, &access_expr)
    } else if clone_getter_refs && getter_is_ref && !matches!(info.field.ty, syn::Type::Reference(_)) {
        quote! { (#access_expr).clone() }
    } else {
        access_expr
    }
}

fn build_sun_shadow_encode_init(fields: &[FieldInfo<'_>], original_fields: &syn::Fields) -> TokenStream2 {
    let base = quote! { value };
    build_sun_struct_init_with_base(fields, original_fields, &base, &quote! { Self }, true, false)
}

fn build_sun_struct_init_with_base(
    fields: &[FieldInfo<'_>],
    original_fields: &syn::Fields,
    base: &TokenStream2,
    struct_ty: &TokenStream2,
    apply_encode_conversion: bool,
    clone_getter_refs: bool,
) -> TokenStream2 {
    match original_fields {
        syn::Fields::Named(_) => {
            let inits = fields.iter().map(|info| {
                let ident = info.access.ident().expect("expected named field ident");
                let init = sun_field_init_with_base(info, base, apply_encode_conversion, clone_getter_refs);
                quote! { #ident: #init }
            });
            quote! { #struct_ty { #( #inits, )* } }
        }
        syn::Fields::Unnamed(_) => {
            let inits = fields.iter().map(|info| sun_field_init_with_base(info, base, apply_encode_conversion, clone_getter_refs));
            quote! { #struct_ty( #( #inits, )* ) }
        }
        syn::Fields::Unit => quote! { #struct_ty },
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
