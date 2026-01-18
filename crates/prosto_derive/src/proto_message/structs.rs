use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::GenericArgument;
use syn::ItemStruct;
use syn::PathArguments;
use syn::Type;
use syn::parse_quote;

use super::build_validate_with_ext_impl;
use super::generic_bounds::add_proto_wire_bounds;
use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::FieldInfo;
use super::unified_field_handler::archive_field_ty;
use super::unified_field_handler::assign_tags;
use super::unified_field_handler::build_archive_encode_stmts;
use super::unified_field_handler::build_archive_len_terms;
use super::unified_field_handler::build_clear_stmts;
use super::unified_field_handler::build_decode_match_arms;
use super::unified_field_handler::build_encode_stmts;
use super::unified_field_handler::build_encoded_len_terms;
use super::unified_field_handler::build_is_default_checks;
use super::unified_field_handler::build_post_decode_hooks;
use super::unified_field_handler::build_proto_default_expr;
use super::unified_field_handler::compute_decode_ty;
use super::unified_field_handler::compute_proto_ty;
use super::unified_field_handler::generate_delegating_proto_wire_impl;
use super::unified_field_handler::generate_proto_shadow_impl;
use super::unified_field_handler::generate_sun_proto_ext_impl;
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
        let archive_binding = super::unified_field_handler::encode_input_binding(&field, &quote! { value });
        let archive_prelude: Vec<_> = archive_binding.prelude.into_iter().collect();
        let archive_value = archive_binding.value;
        let archive_ty = quote! { <#inner_ty as ::proto_rs::ProtoShadow<#inner_ty>>::ProtoArchive };
        let archive_expr = quote! {{
            #( #archive_prelude )*
            let view = <#inner_ty as ::proto_rs::ProtoShadow<#inner_ty>>::from_sun(#archive_value);
            <#inner_ty as ::proto_rs::ProtoShadow<#inner_ty>>::to_archive(view)
        }};
        let proto_shadow_impl = generate_proto_shadow_impl(name, &bounded_generics, &archive_ty, &archive_expr);
        let transparent_impl = generate_transparent_struct_impl(name, &impl_generics, &ty_generics, where_clause, &field, &data.fields);

        return quote! {
            #struct_item
            #proto_shadow_impl
            #transparent_impl
        };
    }

    let bounded_generics = add_proto_wire_bounds(generics, fields.iter());
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();

    let fields = assign_tags(fields);

    let archive_name = format_ident!("{}ProtoArchive", name);
    let archive_field_vars: Vec<_> =
        fields.iter().map(|info| info.access.ident().cloned().unwrap_or_else(|| format_ident!("field_{}", info.index))).collect();
    let archive_field_defs: Vec<_> = fields
        .iter()
        .map(|info| {
            let ty = archive_field_ty(info);
            match info.access {
                FieldAccess::Named(ident) => quote! { #ident: #ty },
                FieldAccess::Tuple(_) | FieldAccess::Direct(_) => quote! { #ty },
            }
        })
        .collect();
    let archive_field_inits: Vec<_> = fields
        .iter()
        .zip(archive_field_vars.iter())
        .map(|(info, var)| {
            let binding = super::unified_field_handler::encode_input_binding(info, &quote! { value });
            let prelude = binding.prelude.into_iter();
            let value = binding.value;
            let proto_ty = &info.proto_ty;
            quote! {
                let #var = {
                    #( #prelude )*
                    let view = <#proto_ty as ::proto_rs::ProtoShadow<#proto_ty>>::from_sun(#value);
                    <#proto_ty as ::proto_rs::ProtoShadow<#proto_ty>>::to_archive(view)
                };
            }
        })
        .collect();
    let archive_struct_expr = match &data.fields {
        syn::Fields::Named(_) => {
            let assigns = fields.iter().zip(archive_field_vars.iter()).map(|(info, var)| {
                let ident = info.access.ident().expect("named field missing ident");
                quote! { #ident: #var }
            });
            quote! { #archive_name { #( #assigns, )* encoded_len: 0 } }
        }
        syn::Fields::Unnamed(_) => {
            quote! { #archive_name( #( #archive_field_vars, )* 0 ) }
        }
        syn::Fields::Unit => quote! { #archive_name { encoded_len: 0 } },
    };
    let archive_default_expr = match &data.fields {
        syn::Fields::Named(_) => {
            let assigns = fields.iter().map(|info| {
                let ident = info.access.ident().expect("named field missing ident");
                let ty = archive_field_ty(info);
                quote! { #ident: <#ty as ::proto_rs::ProtoWire>::proto_default() }
            });
            quote! { #archive_name { #( #assigns, )* encoded_len: 0 } }
        }
        syn::Fields::Unnamed(_) => {
            let defaults = fields.iter().map(|info| {
                let ty = archive_field_ty(info);
                quote! { <#ty as ::proto_rs::ProtoWire>::proto_default() }
            });
            quote! { #archive_name( #( #defaults, )* 0 ) }
        }
        syn::Fields::Unit => quote! { #archive_name { encoded_len: 0 } },
    };
    let archive_len_terms = build_archive_len_terms(&fields, &quote! { archive });
    let archive_encode_stmts = build_archive_encode_stmts(&fields, &quote! { value });

    let archive_struct = match &data.fields {
        syn::Fields::Named(_) => quote! {
            struct #archive_name #generics {
                #( #archive_field_defs, )*
                encoded_len: usize,
            }
        },
        syn::Fields::Unnamed(_) => quote! {
            struct #archive_name #generics(
                #( #archive_field_defs, )*
                usize,
            );
        },
        syn::Fields::Unit => quote! {
            struct #archive_name #generics {
                encoded_len: usize,
            }
        },
    };
    let archive_ctor = quote! {
        impl #impl_generics #archive_name #ty_generics #where_clause {
            fn from_shadow(value: &#name #ty_generics) -> Self {
                #( #archive_field_inits )*
                let mut archive = #archive_struct_expr;
                let mut encoded_len = 0usize;
                #( encoded_len += #archive_len_terms; )*
                archive.encoded_len = encoded_len;
                archive
            }
        }
    };
    let archive_wire_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #archive_name #ty_generics #where_clause {
            type EncodeInput<'a> = &'a Self;
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
            const WIRE_TYPE: ::proto_rs::encoding::WireType = ::proto_rs::encoding::WireType::LengthDelimited;

            #[inline(always)]
            fn proto_default() -> Self {
                #archive_default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                self.encoded_len = 0;
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.encoded_len == 0
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value.encoded_len
            }

            #[inline(always)]
            fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #( #archive_encode_stmts )*
            }

            #[inline(always)]
            fn decode_into(
                _wire_type: ::proto_rs::encoding::WireType,
                _value: &mut Self,
                _buf: &mut impl ::proto_rs::bytes::Buf,
                _ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                Err(::proto_rs::DecodeError::new("ProtoArchive does not support decoding"))
            }
        }
    };

    let proto_shadow_impl = generate_proto_shadow_impl(
        name,
        &bounded_generics,
        &quote! { #archive_name #ty_generics },
        &quote! {
            #archive_name::from_shadow(value)
        },
    );

    let proto_ext_impl = generate_proto_ext_impl(name, &impl_generics, &ty_generics, where_clause, &fields, config);
    let proto_wire_impl = generate_proto_wire_impl(name, &impl_generics, &ty_generics, where_clause, &fields, &data.fields, config);

    quote! {
        #struct_item
        #archive_struct
        #archive_ctor
        #archive_wire_impl
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
    }
}

fn add_transparent_bounds(generics: &syn::Generics, inner_ty: &Type) -> syn::Generics {
    let mut generics = generics.clone();
    let type_params: BTreeSet<_> = generics.type_params().map(|param| param.ident.clone()).collect();
    let where_clause = generics.make_where_clause();
    where_clause.predicates.push(parse_quote!(#inner_ty: ::proto_rs::ProtoWire));
    where_clause.predicates.push(parse_quote!(for<'a> #inner_ty: ::proto_rs::ProtoExt<Shadow<'a> = #inner_ty>));
    where_clause.predicates.push(parse_quote!(for<'a> #inner_ty: ::proto_rs::EncodeInputFromRef<'a>));
    if !type_params.is_empty() {
        let mut used = BTreeSet::new();
        collect_type_params(inner_ty, &type_params, &mut used);
        for ident in used {
            where_clause.predicates.push(parse_quote!(#ident: ::proto_rs::ProtoShadow<#ident>));
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

fn generate_transparent_struct_impl(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    field: &FieldInfo<'_>,
    original_fields: &syn::Fields,
) -> TokenStream2 {
    let inner_ty = &field.field.ty;
    let mut_value_access = field.access.access_tokens(quote! { value });
    let mut_self_access = field.access.access_tokens(quote! { self });

    let wrap_expr = match original_fields {
        syn::Fields::Unnamed(_) => quote! { Self(inner) },
        syn::Fields::Named(_) => {
            let ident = field.access.ident().expect("expected named field ident for transparent struct");
            quote! { Self { #ident: inner } }
        }
        syn::Fields::Unit => quote! { Self },
    };

    let default_expr = match original_fields {
        syn::Fields::Unnamed(_) => quote! { Self(<#inner_ty as ::proto_rs::ProtoWire>::proto_default()) },
        syn::Fields::Named(_) => {
            let ident = field.access.ident().expect("expected named field ident for transparent struct");
            quote! { Self { #ident: <#inner_ty as ::proto_rs::ProtoWire>::proto_default() } }
        }
        syn::Fields::Unit => quote! { Self },
    };

    let is_default_binding = super::unified_field_handler::encode_input_binding(field, &quote! { value });
    let is_default_prelude: Vec<_> = is_default_binding.prelude.into_iter().collect();
    let is_default_value = is_default_binding.value;

    let encoded_len_binding = super::unified_field_handler::encode_input_binding(field, &quote! { value });
    let encoded_len_prelude: Vec<_> = encoded_len_binding.prelude.into_iter().collect();
    let encoded_len_value = encoded_len_binding.value;

    let encode_raw_binding = super::unified_field_handler::encode_input_binding(field, &quote! { value });
    let encode_raw_prelude: Vec<_> = encode_raw_binding.prelude.into_iter().collect();
    let encode_raw_value = encode_raw_binding.value;

    quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            type Shadow<'b> = #name #ty_generics;

            #[inline(always)]
            fn merge_field(
                _value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx)
            }

            #[inline(always)]
            fn decode(mut buf: impl ::proto_rs::bytes::Buf) -> Result<Self, ::proto_rs::DecodeError> {
                if ::core::matches!(
                    <#inner_ty as ::proto_rs::ProtoWire>::KIND,
                    ::proto_rs::ProtoKind::Message
                ) {
                    let inner = <#inner_ty as ::proto_rs::ProtoExt>::decode(buf)?;
                    Ok(#wrap_expr)
                } else {
                    let mut inner = <#inner_ty as ::proto_rs::ProtoWire>::proto_default();
                    <#inner_ty as ::proto_rs::ProtoWire>::decode_into(
                        <#inner_ty as ::proto_rs::ProtoWire>::WIRE_TYPE,
                        &mut inner,
                        &mut buf,
                        ::proto_rs::encoding::DecodeContext::default(),
                    )?;
                    Ok(#wrap_expr)
                }
            }

            #[inline(always)]
            fn decode_length_delimited(
                mut buf: impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<Self, ::proto_rs::DecodeError> {
                if ::core::matches!(
                    <#inner_ty as ::proto_rs::ProtoWire>::KIND,
                    ::proto_rs::ProtoKind::Message
                ) {
                    let inner = <#inner_ty as ::proto_rs::ProtoExt>::decode_length_delimited(buf, ctx)?;
                    Ok(#wrap_expr)
                } else {
                    let mut inner = <#inner_ty as ::proto_rs::ProtoWire>::proto_default();
                    <#inner_ty as ::proto_rs::ProtoWire>::decode_into(
                        <#inner_ty as ::proto_rs::ProtoWire>::WIRE_TYPE,
                        &mut inner,
                        &mut buf,
                        ctx,
                    )?;
                    Ok(#wrap_expr)
                }
            }

            #[inline(always)]
            fn merge_length_delimited<B: ::proto_rs::bytes::Buf>(
                value: &mut Self::Shadow<'_>,
                buf: &mut B,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                if ::core::matches!(
                    <#inner_ty as ::proto_rs::ProtoWire>::KIND,
                    ::proto_rs::ProtoKind::Message
                ) {
                    <#inner_ty as ::proto_rs::ProtoExt>::merge_length_delimited(
                        &mut #mut_value_access,
                        buf,
                        ctx,
                    )
                } else {
                    <#inner_ty as ::proto_rs::ProtoWire>::decode_into(
                        <#inner_ty as ::proto_rs::ProtoWire>::WIRE_TYPE,
                        &mut #mut_value_access,
                        buf,
                        ctx,
                    )
                }
            }
        }

        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = &'b Self;
            const KIND: ::proto_rs::ProtoKind = <#inner_ty as ::proto_rs::ProtoWire>::KIND;

            #[inline(always)]
            fn proto_default() -> Self {
                #default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                <#inner_ty as ::proto_rs::ProtoWire>::clear(&mut #mut_self_access);
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                #( #is_default_prelude )*
                <#inner_ty as ::proto_rs::ProtoWire>::is_default_impl(&#is_default_value)
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                #( #encoded_len_prelude )*
                <#inner_ty as ::proto_rs::ProtoWire>::encoded_len_impl_raw(&#encoded_len_value)
            }

            #[inline(always)]
            fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #( #encode_raw_prelude )*
                <#inner_ty as ::proto_rs::ProtoWire>::encode_raw_unchecked(#encode_raw_value, buf);
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                <#inner_ty as ::proto_rs::ProtoWire>::decode_into(
                    wire_type,
                    &mut #mut_value_access,
                    buf,
                    ctx,
                )
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
    fields: &[FieldInfo<'_>],
    config: &UnifiedProtoConfig,
) -> TokenStream2 {
    let decode_arms = build_decode_match_arms(fields, &quote! { value });

    let shadow_ty = quote! { #name #ty_generics };
    let post_decode_hooks = build_post_decode_hooks(fields);

    // Generate message-level validation if validator is specified
    let message_validation = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! {
            #validator_path(&mut shadow)?;
        }
    } else {
        quote! {}
    };

    let has_validation = config.validator.is_some();
    let post_decode_impl = if post_decode_hooks.is_empty() && !has_validation {
        quote! {}
    } else {
        quote! {
            #[inline(always)]
            fn post_decode(mut shadow: Self::Shadow<'_>) -> Result<Self, ::proto_rs::DecodeError> {
                #(#post_decode_hooks)*
                #message_validation
                ::proto_rs::ProtoShadow::to_sun(shadow)
            }
        }
    };

    let validate_with_ext_impl = build_validate_with_ext_impl(config);

    if config.has_suns() {
        let impls: Vec<_> = config
            .suns
            .iter()
            .map(|sun| {
                let target_ty = &sun.ty;
                generate_sun_proto_ext_impl(&shadow_ty, target_ty, &decode_arms, &post_decode_impl, &validate_with_ext_impl)
            })
            .collect();

        quote! { #(#impls)* }
    } else {
        quote! {
            impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
                type Shadow<'b> = #shadow_ty;

                #[inline(always)]
                fn merge_field(
                    value: &mut Self::Shadow<'_>,
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

                #post_decode_impl
                #validate_with_ext_impl
            }
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
) -> TokenStream2 {
    let proto_default_expr = build_proto_default_expr(fields, original_fields);
    let self_tokens = quote! { self };
    let clear_stmts = build_clear_stmts(fields, &self_tokens);
    let encode_input_tokens = quote! { value };
    let is_default_checks = build_is_default_checks(fields, &encode_input_tokens);
    let encoded_len_terms = build_encoded_len_terms(fields, &encode_input_tokens);
    let encode_stmts = build_encode_stmts(fields, &encode_input_tokens);
    let wire_decode_arms = build_decode_match_arms(fields, &quote! { msg });

    let encode_input_ty = if let Some(sun) = config.suns.first() {
        let target_ty = &sun.ty;
        quote! { <Self as ::proto_rs::ProtoShadow<#target_ty>>::View<'b> }
    } else {
        quote! { <Self as ::proto_rs::ProtoShadow<Self>>::View<'b> }
    };

    let shadow_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = #encode_input_ty;
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;

            #[inline(always)]
            fn proto_default() -> Self {
                #proto_default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                #(#clear_stmts;)*
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                #(#is_default_checks;)*
                true
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                0 #(+ #encoded_len_terms)*
            }

            #[inline(always)]
            fn encode_raw_unchecked(
                value: Self::EncodeInput<'_>,
                buf: &mut impl ::proto_rs::bytes::BufMut,
            ) {
                #(#encode_stmts)*
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::check_wire_type(
                    ::proto_rs::encoding::WireType::LengthDelimited,
                    wire_type,
                )?;
                ctx.limit_reached()?;
                ::proto_rs::encoding::merge_loop(
                    value,
                    buf,
                    ctx.enter_recursion(),
                    |msg: &mut Self, buf, ctx| {
                        let (tag, wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        match tag {
                            #(#wire_decode_arms,)*
                            _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                        }
                    },
                )
            }
        }
    };

    if config.has_suns() {
        let shadow_ty = quote! { #name #ty_generics };
        let delegating_impls: Vec<_> = config.suns.iter().map(|sun| generate_delegating_proto_wire_impl(&shadow_ty, &sun.ty)).collect();

        quote! { #shadow_impl #(#delegating_impls)* }
    } else {
        quote! { #shadow_impl }
    }
}
