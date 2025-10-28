use std::collections::BTreeSet;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Field;
use syn::Fields;
use syn::Ident;
use syn::ItemEnum;
use syn::ItemStruct;
use syn::Lit;
use syn::spanned::Spanned;

use crate::emit_proto::generate_struct_proto;
use crate::parse::UnifiedProtoConfig;
use crate::utils::FieldConfig;
use crate::utils::is_option_type;
use crate::utils::parse_field_config;

pub fn proto_message_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ts: TokenStream2 = item.clone().into();
    let input: DeriveInput = syn::parse2(item_ts.clone()).expect("proto_message expects a type definition");

    let type_ident = input.ident.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input.data);
    let proto_name = config.sun.as_ref().map_or(type_ident.clone(), |sun| sun.message_ident.clone());

    let tokens = match input.data {
        Data::Struct(ref data) => {
            let proto = generate_struct_proto(&proto_name, &data.fields);
            config.register_and_emit_proto(&proto_name, &proto);

            let item_struct: ItemStruct = syn::parse2(item_ts).expect("failed to parse struct");
            generate_struct_impl(&input, &item_struct, data, &config)
        }
        Data::Enum(ref data) => {
            let is_simple_enum = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));
            let proto = if is_simple_enum {
                crate::emit_proto::generate_simple_enum_proto(&proto_name, data)
            } else {
                crate::emit_proto::generate_complex_enum_proto(&proto_name, data)
            };
            config.register_and_emit_proto(&proto_name, &proto);

            let item_enum: ItemEnum = syn::parse2(item_ts).expect("failed to parse enum");
            if is_simple_enum {
                generate_simple_enum_impl(&input, &item_enum, data, &config)
            } else {
                match generate_complex_enum_impl(&input, &item_enum, data, &config) {
                    Ok(tokens) => tokens,
                    Err(err) => return err.to_compile_error().into(),
                }
            }
        }
        Data::Union(_) => Error::new_spanned(&input.ident, "proto_message cannot be used on unions").to_compile_error(),
    };

    let proto_imports = config.imports_mat;
    quote! {
        #proto_imports
        #tokens
    }
    .into()
}

#[derive(Clone)]
struct FieldInfo<'a> {
    index: usize,
    field: &'a Field,
    access: FieldAccess<'a>,
    config: FieldConfig,
    tag: Option<u32>,
}

#[derive(Clone)]
enum VariantKind<'a> {
    Unit,
    Tuple { field: &'a Field },
    Struct { fields: Vec<FieldInfo<'a>> },
}

struct VariantInfo<'a> {
    ident: &'a Ident,
    tag: u32,
    kind: VariantKind<'a>,
    is_default: bool,
}

#[derive(Clone)]
enum FieldAccess<'a> {
    Named(&'a Ident),
    Tuple(usize),
    Direct(TokenStream2),
}

impl FieldAccess<'_> {
    fn ident(&self) -> Option<&Ident> {
        match self {
            FieldAccess::Named(id) => Some(id),
            FieldAccess::Tuple(_) | FieldAccess::Direct(_) => None,
        }
    }

    fn access_tokens(&self, base: TokenStream2) -> TokenStream2 {
        match self {
            FieldAccess::Named(ident) => quote! { #base.#ident },
            FieldAccess::Tuple(idx) => {
                let index = syn::Index::from(*idx);
                quote! { #base.#index }
            }
            FieldAccess::Direct(tokens) => tokens.clone(),
        }
    }
}

fn generate_struct_impl(input: &DeriveInput, item_struct: &ItemStruct, data: &syn::DataStruct, config: &UnifiedProtoConfig) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let struct_item = sanitize_struct(item_struct.clone());

    let fields = match &data.fields {
        Fields::Named(named) => named
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
        Fields::Unnamed(unnamed) => unnamed
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
        Fields::Unit => Vec::new(),
    };

    let fields = assign_tags(fields);

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        generate_proto_shadow_impl(name, generics)
    };

    let proto_ext_impl = generate_proto_ext_impl(name, &impl_generics, &ty_generics, where_clause, &fields, config);
    let proto_wire_impl = generate_proto_wire_impl(name, &impl_generics, &ty_generics, where_clause, &fields, config);

    quote! {
        #struct_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
    }
}

fn sanitize_enum(mut item: ItemEnum) -> ItemEnum {
    item.attrs = strip_proto_attrs(&item.attrs);
    for variant in &mut item.variants {
        variant.attrs = strip_proto_attrs(&variant.attrs);
    }
    item
}

fn generate_simple_enum_impl(input: &DeriveInput, item_enum: &ItemEnum, data: &syn::DataEnum, config: &UnifiedProtoConfig) -> TokenStream2 {
    let enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let ordered_variants: Vec<&syn::Variant> = (0..data.variants.len()).map(|idx| &data.variants[idx]).collect();
    let discriminants = match crate::utils::collect_discriminants_for_variants(&ordered_variants) {
        Ok(values) => values,
        Err(err) => return err.to_compile_error(),
    };

    let marked_default = match crate::utils::find_marked_default_variant(data) {
        Ok(value) => value,
        Err(err) => return err.to_compile_error(),
    };

    if let Some(idx) = marked_default
        && discriminants.get(idx).copied() != Some(0)
    {
        let variant = &data.variants[idx];
        return syn::Error::new(variant.span(), "enum #[default] variant must have discriminant 0").to_compile_error();
    }

    let Some(zero_index) = discriminants.iter().position(|&value| value == 0) else {
        return syn::Error::new(data.variants.span(), "proto enums must contain a variant with discriminant 0").to_compile_error();
    };

    let default_index = marked_default.unwrap_or(zero_index);
    let default_ident = &data.variants[default_index].ident;

    let raw_from_variant: Vec<_> = ordered_variants
        .iter()
        .zip(discriminants.iter())
        .map(|(variant, value)| {
            let ident = &variant.ident;
            quote! { Self::#ident => #value }
        })
        .collect();

    let raw_match_arms = &raw_from_variant;

    let try_from_arms: Vec<_> = ordered_variants
        .iter()
        .zip(discriminants.iter())
        .map(|(variant, value)| {
            let ident = &variant.ident;
            quote! { #value => Ok(Self::#ident) }
        })
        .collect();

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        quote! {
            impl #impl_generics ::proto_rs::ProtoShadow for #name #ty_generics #where_clause {
                type Sun<'a> = &'a Self;
                type OwnedSun = Self;
                type View<'a> = &'a Self;

                #[inline(always)]
                fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
                    Ok(self)
                }

                #[inline(always)]
                fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                    value
                }
            }
        }
    };

    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        quote! { #name #ty_generics }
    };

    let shadow_ty = quote! { #name #ty_generics };

    let proto_ext_impl = quote! {
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
                    1 => {
                        let mut raw = 0i32;
                        <i32 as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut raw, buf, ctx)?;
                        *value = <Self::Shadow<'_> as ::core::convert::TryFrom<i32>>::try_from(raw)?;
                        Ok(())
                    }
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
        }
    };

    let proto_wire_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = &'b Self;
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::SimpleEnum;

            #[inline(always)]
            fn proto_default() -> Self {
                Self::#default_ident
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                matches!(**value, Self::#default_ident)
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                let raw = match **value {
                    #(#raw_match_arms,)*
                };
                <i32 as ::proto_rs::ProtoWire>::encoded_len_impl_raw(&raw)
            }

            #[inline(always)]
            fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                let raw = match **value {
                    #(#raw_match_arms,)*
                };
                <i32 as ::proto_rs::ProtoWire>::encode_raw_unchecked(raw, buf);
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let mut raw = 0i32;
                <i32 as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut raw, buf, ctx)?;
                *value = Self::try_from(raw)?;
                Ok(())
            }
        }
    };

    let try_from_impl = quote! {
        impl #impl_generics ::core::convert::TryFrom<i32> for #name #ty_generics #where_clause {
            type Error = ::proto_rs::DecodeError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms,)*
                    _ => Err(::proto_rs::DecodeError::new("invalid enum value")),
                }
            }
        }
    };

    quote! {
        #enum_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
        #try_from_impl
    }
}

fn generate_complex_enum_impl(input: &DeriveInput, item_enum: &ItemEnum, data: &syn::DataEnum, config: &UnifiedProtoConfig) -> syn::Result<TokenStream2> {
    let enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let default_index = crate::utils::find_marked_default_variant(data)?.unwrap_or(0);
    let mut variants = collect_variant_infos(data)?;
    if variants.is_empty() {
        return Err(syn::Error::new(input.ident.span(), "proto_message enum must contain at least one variant"));
    }
    if default_index >= variants.len() {
        return Err(syn::Error::new(input.ident.span(), "#[default] variant index is out of bounds"));
    }
    variants[default_index].is_default = true;

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        generate_proto_shadow_impl(name, generics)
    };

    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        quote! { #name #ty_generics }
    };
    let shadow_ty = quote! { #name #ty_generics };

    let merge_field_arms = variants.iter().map(|variant| build_variant_merge_arm(name, variant)).collect::<Vec<_>>();

    let default_expr = build_variant_default_expr(&variants[default_index]);
    let is_default_match_arms = variants.iter().map(build_variant_is_default_arm).collect::<Vec<_>>();
    let encoded_len_arms = variants.iter().map(build_variant_encoded_len_arm).collect::<Vec<_>>();
    let encode_arms = variants.iter().map(build_variant_encode_arm).collect::<Vec<_>>();

    let proto_ext_impl = quote! {
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
                    #(#merge_field_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
        }
    };

    let proto_wire_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = &'b Self;
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;

            #[inline(always)]
            fn proto_default() -> Self {
                #default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                match **value {
                    #(#is_default_match_arms,)*
                }
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                match **value {
                    #(#encoded_len_arms,)*
                }
            }

            #[inline(always)]
            fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                match *value {
                    #(#encode_arms,)*
                }
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::check_wire_type(::proto_rs::encoding::WireType::LengthDelimited, wire_type)?;
                *value = Self::decode_length_delimited(buf, ctx)?;
                Ok(())
            }
        }
    };

    Ok(quote! {
        #enum_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
    })
}

fn collect_variant_infos(data: &syn::DataEnum) -> syn::Result<Vec<VariantInfo<'_>>> {
    let mut used_tags = BTreeSet::new();
    let mut variants = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = resolve_variant_tag(variant, idx + 1)?;
        if !used_tags.insert(tag) {
            return Err(syn::Error::new(variant.ident.span(), format!("duplicate proto(tag) attribute for enum variant: {tag}")));
        }

        let kind = match &variant.fields {
            Fields::Unit => VariantKind::Unit,
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new(variant.ident.span(), "complex enum tuple variants must contain exactly one field"));
                }

                let field = &fields.unnamed[0];
                let config = parse_field_config(field);
                if config.skip {
                    return Err(syn::Error::new(field.span(), "tuple enum variants cannot use #[proto(skip)]"));
                }
                if config.into_type.is_some()
                    || config.from_type.is_some()
                    || config.into_fn.is_some()
                    || config.from_fn.is_some()
                    || config.skip_deser_fn.is_some()
                    || config.is_rust_enum
                    || config.is_message
                    || config.is_proto_enum
                {
                    return Err(syn::Error::new(field.span(), "tuple enum variants do not support advanced #[proto] options"));
                }
                if let Some(custom) = config.custom_tag
                    && custom != 1
                {
                    return Err(syn::Error::new(field.span(), "tuple enum fields cannot override their protobuf tag"));
                }

                VariantKind::Tuple { field }
            }
            Fields::Named(fields_named) => {
                let mut infos: Vec<_> = fields_named
                    .named
                    .iter()
                    .enumerate()
                    .map(|(field_idx, field)| FieldInfo {
                        index: field_idx,
                        field,
                        access: FieldAccess::Direct({
                            let ident = field.ident.as_ref().expect("named variant field");
                            quote! { #ident }
                        }),
                        config: parse_field_config(field),
                        tag: None,
                    })
                    .collect();
                infos = assign_tags(infos);
                VariantKind::Struct { fields: infos }
            }
        };

        variants.push(VariantInfo {
            ident: &variant.ident,
            tag,
            kind,
            is_default: false,
        });
    }

    Ok(variants)
}

fn resolve_variant_tag(variant: &syn::Variant, default: usize) -> syn::Result<u32> {
    let mut custom_tag = None;

    for attr in &variant.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.get_ident().is_some_and(|ident| ident == "tag") {
                if custom_tag.is_some() {
                    return Err(syn::Error::new(meta.path.span(), "duplicate proto(tag) attribute for variant"));
                }

                let lit: Lit = meta.value()?.parse()?;
                let value = match lit {
                    Lit::Int(int_lit) => int_lit.base10_parse::<usize>()?,
                    Lit::Str(str_lit) => str_lit.value().parse::<usize>().map_err(|_| syn::Error::new(str_lit.span(), "proto tag must be a positive integer"))?,
                    _ => {
                        return Err(syn::Error::new(lit.span(), "proto tag must be specified as an integer"));
                    }
                };

                custom_tag = Some(value);
            }

            Ok(())
        })?;
    }

    let tag = custom_tag.unwrap_or(default);
    if tag == 0 {
        return Err(syn::Error::new(variant.ident.span(), "proto enum variant tags must be greater than or equal to 1"));
    }

    let tag_u32 = u32::try_from(tag).map_err(|_| syn::Error::new(variant.ident.span(), "proto tag overflowed u32"))?;
    Ok(tag_u32)
}

fn build_variant_default_expr(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    match &variant.kind {
        VariantKind::Unit => quote! { Self::#ident },
        VariantKind::Tuple { field, .. } => {
            let ty = &field.ty;
            quote! { Self::#ident(<#ty as ::proto_rs::ProtoWire>::proto_default()) }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! { Self::#ident {} }
            } else {
                let inits = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let ty = &info.field.ty;
                    quote! { #field_ident: <#ty as ::proto_rs::ProtoWire>::proto_default() }
                });
                quote! { Self::#ident { #(#inits),* } }
            }
        }
    }
}

fn build_variant_is_default_arm(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    match &variant.kind {
        VariantKind::Unit => {
            if variant.is_default {
                quote! { Self::#ident => true }
            } else {
                quote! { Self::#ident => false }
            }
        }
        VariantKind::Tuple { field, .. } => {
            if variant.is_default {
                let binding_ident = Ident::new(&format!("__proto_rs_variant_{}_value", ident.to_string().to_lowercase()), field.span());
                let ty = &field.ty;
                quote! {
                    Self::#ident(ref #binding_ident) => {
                        let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #binding_ident;
                        <#ty as ::proto_rs::ProtoWire>::is_default_impl(&#binding_ident)
                    }
                }
            } else {
                quote! { Self::#ident(..) => false }
            }
        }
        VariantKind::Struct { fields } => {
            if variant.is_default {
                if fields.is_empty() {
                    quote! { Self::#ident { .. } => true }
                } else {
                    let bindings = fields.iter().map(|info| {
                        let field_ident = info.field.ident.as_ref().expect("named field");
                        quote! { #field_ident: ref #field_ident }
                    });
                    let checks = build_is_default_checks(fields, &TokenStream2::new());
                    quote! {
                        Self::#ident { #(#bindings),* } => {
                            #(#checks;)*
                            true
                        }
                    }
                }
            } else {
                quote! { Self::#ident { .. } => false }
            }
        }
    }
}

fn build_variant_encoded_len_arm(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => quote! { Self::#ident => ::proto_rs::encoding::key_len(#tag) + 1 },
        VariantKind::Tuple { field, .. } => {
            let binding_ident = Ident::new(&format!("__proto_rs_variant_{}_value", ident.to_string().to_lowercase()), field.span());
            let ty = &field.ty;
            quote! {
                Self::#ident(ref #binding_ident) => {
                    let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #binding_ident;
                    <#ty as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&#binding_ident, #tag)
                }
            }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! {
                    Self::#ident { .. } => ::proto_rs::encoding::key_len(#tag) + 1
                }
            } else {
                let bindings = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    quote! { #field_ident: ref #field_ident }
                });
                let terms = build_encoded_len_terms(fields, &TokenStream2::new());
                quote! {
                    Self::#ident { #(#bindings),* } => {
                        let msg_len = 0 #(+ #terms)*;
                        ::proto_rs::encoding::key_len(#tag)
                            + ::proto_rs::encoding::encoded_len_varint(msg_len as u64)
                            + msg_len
                    }
                }
            }
        }
    }
}

fn build_variant_encode_arm(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => quote! {
            Self::#ident => {
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint(0, buf);
            }
        },
        VariantKind::Tuple { field, .. } => {
            let binding_ident = Ident::new(&format!("__proto_rs_variant_{}_value", ident.to_string().to_lowercase()), field.span());
            let ty = &field.ty;
            quote! {
                Self::#ident(ref #binding_ident) => {
                    let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #binding_ident;
                    if let Err(err) = <#ty as ::proto_rs::ProtoWire>::encode_with_tag(#tag, #binding_ident, buf) {
                        panic!("encode_raw_unchecked called without sufficient capacity: {err}");
                    }
                }
            }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! {
                    Self::#ident { .. } => {
                        ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                        ::proto_rs::encoding::encode_varint(0, buf);
                    }
                }
            } else {
                let bindings = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    quote! { #field_ident: ref #field_ident }
                });
                let terms = build_encoded_len_terms(fields, &TokenStream2::new());
                let encode_stmts = build_encode_stmts(fields, &TokenStream2::new());
                quote! {
                    Self::#ident { #(#bindings),* } => {
                        let msg_len = 0 #(+ #terms)*;
                        ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                        ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
                        #(#encode_stmts)*
                    }
                }
            }
        }
    }
}

fn build_variant_merge_arm(name: &Ident, variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => {
            quote! {
                #tag => {
                    ::proto_rs::encoding::check_wire_type(::proto_rs::encoding::WireType::LengthDelimited, wire_type)?;
                    ctx.limit_reached()?;
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    let remaining = buf.remaining();
                    if len > remaining as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    if len != 0 {
                        return Err(::proto_rs::DecodeError::new("expected empty variant payload"));
                    }
                    *value = #name::#ident;
                    Ok(())
                }
            }
        }
        VariantKind::Tuple { field, .. } => {
            let ty = &field.ty;
            quote! {
                #tag => {
                    let mut inner = <#ty as ::proto_rs::ProtoWire>::proto_default();
                    <#ty as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut inner, buf, ctx)?;
                    *value = #name::#ident(inner);
                    Ok(())
                }
            }
        }
        VariantKind::Struct { fields } => {
            let field_inits = fields.iter().map(|info| {
                let field_ident = info.field.ident.as_ref().expect("named field");
                let ty = &info.field.ty;
                quote! { let mut #field_ident = <#ty as ::proto_rs::ProtoWire>::proto_default(); }
            });
            let decode_match = fields
                .iter()
                .filter_map(|info| {
                    let field_tag = info.tag?;
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let ty = &info.field.ty;
                    Some(quote! {
                        #field_tag => {
                            <#ty as ::proto_rs::ProtoWire>::decode_into(field_wire_type, &mut #field_ident, buf, inner_ctx)?;
                        }
                    })
                })
                .collect::<Vec<_>>();
            let construct_expr = if fields.is_empty() {
                quote! { #name::#ident }
            } else {
                let assigns = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    quote! { #field_ident }
                });
                quote! { #name::#ident { #(#assigns),* } }
            };
            let decode_loop = if decode_match.is_empty() {
                quote! {
                    while buf.remaining() > limit {
                        let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, inner_ctx)?;
                    }
                }
            } else {
                quote! {
                    while buf.remaining() > limit {
                        let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        match field_tag {
                            #(#decode_match,)*
                            _ => ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, inner_ctx)?,
                        }
                    }
                }
            };
            quote! {
                #tag => {
                    ::proto_rs::encoding::check_wire_type(::proto_rs::encoding::WireType::LengthDelimited, wire_type)?;
                    ctx.limit_reached()?;
                    let inner_ctx = ctx.enter_recursion();
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    let remaining = buf.remaining();
                    if len > remaining as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    let limit = remaining - len as usize;
                    #(#field_inits)*
                    #decode_loop
                    if buf.remaining() != limit {
                        return Err(::proto_rs::DecodeError::new("delimited length exceeded"));
                    }
                    *value = #construct_expr;
                    Ok(())
                }
            }
        }
    }
}
fn sanitize_struct(mut item: ItemStruct) -> ItemStruct {
    item.attrs = strip_proto_attrs(&item.attrs);
    match &mut item.fields {
        Fields::Named(named) => {
            for field in &mut named.named {
                field.attrs = strip_proto_attrs(&field.attrs);
            }
        }
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.unnamed {
                field.attrs = strip_proto_attrs(&field.attrs);
            }
        }
        Fields::Unit => {}
    }
    item
}

fn strip_proto_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !attr.path().is_ident("proto_message") && !attr.path().is_ident("proto")).cloned().collect()
}

fn assign_tags(mut fields: Vec<FieldInfo<'_>>) -> Vec<FieldInfo<'_>> {
    let mut used = BTreeSet::new();
    let mut next = 1u32;

    for info in &mut fields {
        if info.config.skip {
            continue;
        }

        if info.config.into_type.is_some()
            || info.config.from_type.is_some()
            || info.config.into_fn.is_some()
            || info.config.from_fn.is_some()
            || info.config.skip_deser_fn.is_some()
            || info.config.is_rust_enum
            || info.config.is_message
            || info.config.is_proto_enum
        {
            panic!("proto_message rewrite does not yet support advanced field attributes");
        }

        let tag = if let Some(custom) = info.config.custom_tag {
            assert!(custom != 0, "proto field tags must be >= 1");
            let custom_u32: u32 = custom.try_into().expect("proto field tag overflowed u32");
            assert!(used.insert(custom_u32), "duplicate proto field tag: {custom}");
            custom_u32
        } else {
            while used.contains(&next) {
                next = next.checked_add(1).expect("proto field tag overflowed u32");
            }
            let assigned = next;
            used.insert(assigned);
            next = next.checked_add(1).expect("proto field tag overflowed u32");
            assigned
        };

        info.tag = Some(tag);
    }

    fields
}

fn generate_proto_shadow_impl(name: &Ident, generics: &syn::Generics) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::proto_rs::ProtoShadow for #name #ty_generics #where_clause {
            type Sun<'a> = &'a Self;
            type OwnedSun = Self;
            type View<'a> = &'a Self;

            #[inline(always)]
            fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
                Ok(self)
            }

            #[inline(always)]
            fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                value
            }
        }
    }
}

fn generate_proto_ext_impl(
    name: &Ident,
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

    let decode_arms = fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let field_ty = &info.field.ty;
            let access = info.access.access_tokens(quote! { value });
            Some(quote! {
                #tag => {
                    <#field_ty as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut #access, buf, ctx)
                }
            })
        })
        .collect::<Vec<_>>();

    let shadow_ty = quote! { #name #ty_generics };

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
        }
    }
}

fn generate_proto_wire_impl(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fields: &[FieldInfo<'_>],
    _config: &UnifiedProtoConfig,
) -> TokenStream2 {
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
            fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #(#encode_stmts)*
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::check_wire_type(::proto_rs::encoding::WireType::LengthDelimited, wire_type)?;
                <Self as ::proto_rs::ProtoExt>::merge_length_delimited(value, buf, ctx)
            }
        }
    }
}

fn build_proto_default_expr(fields: &[FieldInfo<'_>]) -> TokenStream2 {
    if fields.is_empty() {
        return quote! { Self };
    }

    if fields.iter().all(|f| matches!(f.access, FieldAccess::Tuple(_))) {
        let defaults = fields.iter().map(|info| {
            let ty = &info.field.ty;
            quote! { <#ty as ::proto_rs::ProtoWire>::proto_default() }
        });
        quote! { Self( #(#defaults),* ) }
    } else {
        let defaults = fields.iter().map(|info| {
            let ident = info.access.ident().expect("expected named field");
            let ty = &info.field.ty;
            quote! { #ident: <#ty as ::proto_rs::ProtoWire>::proto_default() }
        });
        quote! { Self { #(#defaults),* } }
    }
}

fn build_clear_stmts(fields: &[FieldInfo<'_>], self_tokens: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .map(|info| {
            let access = info.access.access_tokens(self_tokens.clone());
            let ty = &info.field.ty;
            quote! { <#ty as ::proto_rs::ProtoWire>::clear(&mut #access) }
        })
        .collect()
}

fn build_is_default_checks(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            info.tag?;
            let ty = &info.field.ty;
            let binding = encode_input_binding(info, base);
            let ident = binding.ident;
            let init = binding.init;
            Some(quote! {
                {
                    #init
                    if !<#ty as ::proto_rs::ProtoWire>::is_default_impl(&#ident) {
                        return false;
                    }
                }
            })
        })
        .collect()
}

fn build_encoded_len_terms(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let ty = &info.field.ty;
            let binding = encode_input_binding(info, base);
            let ident = binding.ident;
            let init = binding.init;
            Some(quote! {{
                #init
                <#ty as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&#ident, #tag)
            }})
        })
        .collect()
}

fn build_encode_stmts(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let ty = &info.field.ty;
            let binding = encode_input_binding(info, base);
            let ident = binding.ident;
            let init = binding.init;
            Some(quote! {
                {
                    #init
                    if let Err(err) = <#ty as ::proto_rs::ProtoWire>::encode_with_tag(#tag, #ident, buf) {
                        panic!("encode_raw_unchecked called without sufficient capacity: {err}");
                    }
                }
            })
        })
        .collect()
}

struct EncodeBinding {
    init: TokenStream2,
    ident: Ident,
}

fn encode_input_binding(field: &FieldInfo<'_>, base: &TokenStream2) -> EncodeBinding {
    let ty = &field.field.ty;
    let binding_ident = Ident::new(&format!("__proto_rs_field_{}_input", field.index), field.field.span());
    let access = match &field.access {
        FieldAccess::Direct(tokens) => tokens.clone(),
        _ => field.access.access_tokens(base.clone()),
    };
    let init_expr = if is_option_type(ty) {
        quote! { (#access).as_ref().map(|inner| inner) }
    } else {
        quote! { #access }
    };
    let init = quote! {
        let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #init_expr;
    };
    EncodeBinding { init, ident: binding_ident }
}
