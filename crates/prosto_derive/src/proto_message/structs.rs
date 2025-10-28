use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::ItemStruct;

use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::FieldInfo;
use super::unified_field_handler::assign_tags;
use super::unified_field_handler::build_clear_stmts;
use super::unified_field_handler::build_decode_arm;
use super::unified_field_handler::build_encode_stmts;
use super::unified_field_handler::build_encoded_len_terms;
use super::unified_field_handler::build_is_default_checks;
use super::unified_field_handler::build_post_decode_method;
use super::unified_field_handler::build_proto_default_expr;
use super::unified_field_handler::generate_proto_shadow_impl;
use super::unified_field_handler::strip_proto_attrs;
use crate::parse::UnifiedProtoConfig;
use crate::utils::parse_field_config;

pub(super) fn generate_struct_impl(input: &DeriveInput, item_struct: &ItemStruct, data: &syn::DataStruct, config: &UnifiedProtoConfig) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let struct_item = sanitize_struct(item_struct.clone());

    let fields = match &data.fields {
        syn::Fields::Named(named) => named
            .named
            .iter()
            .enumerate()
            .map(|(idx, field)| FieldInfo {
                index: idx,
                field,
                access: FieldAccess::Named(field.ident.as_ref().expect("named field missing ident")),
                config: parse_field_config(field),
                tag: None,
            })
            .collect::<Vec<_>>(),
        syn::Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(idx, field)| FieldInfo {
                index: idx,
                field,
                access: FieldAccess::Tuple(idx),
                config: parse_field_config(field),
                tag: None,
            })
            .collect::<Vec<_>>(),
        syn::Fields::Unit => Vec::new(),
    };

    let fields = assign_tags(fields);

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        generate_proto_shadow_impl(name, generics)
    };

    let proto_ext_impl = generate_proto_ext_impl(name, &impl_generics, &ty_generics, where_clause, &fields, config);
    let proto_wire_impl = generate_proto_wire_impl(name, &impl_generics, &ty_generics, where_clause, &fields);

    quote! {
        #struct_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
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
    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        quote! { #name #ty_generics }
    };

    let decode_base = quote! { value };
    let decode_wire = quote! { wire_type };
    let decode_buf = quote! { buf };
    let decode_ctx = quote! { ctx };
    let decode_arms = fields
        .iter()
        .filter_map(|info| build_decode_arm(info, &decode_base, &decode_wire, &decode_buf, &decode_ctx))
        .collect::<Vec<_>>();

    let shadow_ty = quote! { #name #ty_generics };
    let post_decode = build_post_decode_method(fields);

    quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #target_ty #where_clause {
            type Shadow<'b> = #shadow_ty where Self: 'b;

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

            #post_decode
        }
    }
}

fn generate_proto_wire_impl(name: &syn::Ident, impl_generics: &syn::ImplGenerics, ty_generics: &syn::TypeGenerics, where_clause: Option<&syn::WhereClause>, fields: &[FieldInfo<'_>]) -> TokenStream2 {
    let proto_default_expr = build_proto_default_expr(fields);
    let self_tokens = quote! { self };
    let clear_stmts = build_clear_stmts(fields, &self_tokens);
    let encode_input_tokens = quote! { value };
    let is_default_checks = build_is_default_checks(fields, &encode_input_tokens);
    let encoded_len_terms = build_encoded_len_terms(fields, &encode_input_tokens);
    let encode_stmts = build_encode_stmts(fields, &encode_input_tokens);

    quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = &'b Self;
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
                <Self as ::proto_rs::ProtoExt>::merge_length_delimited(value, buf, ctx)
            }
        }
    }
}
