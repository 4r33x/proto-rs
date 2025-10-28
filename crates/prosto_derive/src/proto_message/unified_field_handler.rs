use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Attribute;
use syn::Field;
use syn::Ident;
use syn::ItemEnum;
use syn::Type;
use syn::parse_quote;
use syn::parse_str;
use syn::spanned::Spanned;

use crate::utils::FieldConfig;
use crate::utils::is_option_type;
use crate::utils::vec_inner_type;

#[derive(Clone)]
pub struct FieldInfo<'a> {
    pub index: usize,
    pub field: &'a Field,
    pub access: FieldAccess<'a>,
    pub config: FieldConfig,
    pub tag: Option<u32>,
}

#[derive(Clone)]
pub enum FieldAccess<'a> {
    Named(&'a Ident),
    Tuple(usize),
    Direct(TokenStream2),
}

impl FieldAccess<'_> {
    pub fn ident(&self) -> Option<&Ident> {
        match self {
            FieldAccess::Named(id) => Some(id),
            FieldAccess::Tuple(_) | FieldAccess::Direct(_) => None,
        }
    }

    pub fn access_tokens(&self, base: TokenStream2) -> TokenStream2 {
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

pub fn strip_proto_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !attr.path().is_ident("proto_message") && !attr.path().is_ident("proto")).cloned().collect()
}

pub fn sanitize_enum(mut item: ItemEnum) -> ItemEnum {
    item.attrs = strip_proto_attrs(&item.attrs);
    for variant in &mut item.variants {
        variant.attrs = strip_proto_attrs(&variant.attrs);
    }
    item
}

pub fn assign_tags(mut fields: Vec<FieldInfo<'_>>) -> Vec<FieldInfo<'_>> {
    let mut used = BTreeSet::new();
    let mut next = 1u32;

    for info in &mut fields {
        if info.config.skip {
            info.tag = None;
            continue;
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

pub fn generate_proto_shadow_impl(name: &Ident, generics: &syn::Generics) -> TokenStream2 {
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

pub struct EncodeBinding {
    pub init: TokenStream2,
    pub ident: Ident,
}

pub fn encode_input_binding(field: &FieldInfo<'_>, base: &TokenStream2) -> EncodeBinding {
    let wire_ty = field_wire_type(field);
    let binding_ident = Ident::new(&format!("__proto_rs_field_{}_input", field.index), field.field.span());
    let access_expr = match &field.access {
        FieldAccess::Direct(tokens) => tokens.clone(),
        _ => field.access.access_tokens(base.clone()),
    };

    let value_expr = encode_value_expr(field, access_expr);

    let init_expr = encode_input_expr(&wire_ty, value_expr);

    let init = quote! {
        let #binding_ident: <#wire_ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #init_expr;
    };

    EncodeBinding { init, ident: binding_ident }
}

fn is_value_encode_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path)
    if type_path.qself.is_none()
        && type_path.path.segments.len() == 1
        && matches!(type_path.path.segments[0].ident.to_string().as_str(),
            "bool" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "f32" | "f64"
        ))
}

pub fn build_proto_default_expr(fields: &[FieldInfo<'_>]) -> TokenStream2 {
    if fields.is_empty() {
        return quote! { Self };
    }

    if fields.iter().all(|f| matches!(f.access, FieldAccess::Tuple(_))) {
        let defaults = fields.iter().map(|info| {
            let ty = &info.field.ty;
            default_expr_for_field(info, ty)
        });
        quote! { Self( #(#defaults),* ) }
    } else {
        let defaults = fields.iter().map(|info| {
            let ident = info.access.ident().expect("expected named field");
            let ty = &info.field.ty;
            let value = default_expr_for_field(info, ty);
            quote! { #ident: #value }
        });
        quote! { Self { #(#defaults),* } }
    }
}

pub fn build_clear_stmts(fields: &[FieldInfo<'_>], self_tokens: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .map(|info| {
            let access = info.access.access_tokens(self_tokens.clone());
            let ty = &info.field.ty;
            if needs_manual_handling(info) {
                quote! { #access = ::core::default::Default::default() }
            } else {
                quote! { <#ty as ::proto_rs::ProtoWire>::clear(&mut #access) }
            }
        })
        .collect()
}

pub fn build_is_default_checks(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            info.tag?;
            let wire_ty = field_wire_type(info);
            let binding = encode_input_binding(info, base);
            let ident = binding.ident;
            let init = binding.init;
            Some(quote! {
                {
                    #init
                    if !<#wire_ty as ::proto_rs::ProtoWire>::is_default_impl(&#ident) {
                        return false;
                    }
                }
            })
        })
        .collect()
}

pub fn build_encoded_len_terms(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let wire_ty = field_wire_type(info);
            let binding = encode_input_binding(info, base);
            let ident = binding.ident;
            let init = binding.init;
            Some(quote! {{
                #init
                <#wire_ty as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&#ident, #tag)
            }})
        })
        .collect()
}

pub fn build_encode_stmts(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let wire_ty = field_wire_type(info);
            let binding = encode_input_binding(info, base);
            let ident = binding.ident;
            let init = binding.init;
            Some(quote! {
                {
                    #init
                    if let Err(err) = <#wire_ty as ::proto_rs::ProtoWire>::encode_with_tag(#tag, #ident, buf) {
                        panic!("encode_raw_unchecked called without sufficient capacity: {err}");
                    }
                }
            })
        })
        .collect()
}

pub fn build_decode_arm(info: &FieldInfo<'_>, access: TokenStream2) -> TokenStream2 {
    let wire_ty = field_wire_type(info);

    if needs_conversion(info) {
        let tmp_ident = Ident::new(&format!("__proto_rs_field_{}_tmp", info.index), info.field.span());
        let assign_tokens = build_decode_assignment(info, &tmp_ident, access.clone(), &wire_ty);
        quote! {{
            let mut #tmp_ident: #wire_ty = <#wire_ty as ::proto_rs::ProtoWire>::proto_default();
            <#wire_ty as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut #tmp_ident, buf, ctx)?;
            #assign_tokens
            Ok(())
        }}
    } else {
        quote! {
            <#wire_ty as ::proto_rs::ProtoWire>::decode_into(
                wire_type,
                &mut #access,
                buf,
                ctx,
            )
        }
    }
}

pub fn build_post_decode_impl(fields: &[FieldInfo<'_>]) -> TokenStream2 {
    let hooks = fields
        .iter()
        .filter_map(|info| {
            if info.config.skip {
                if let Some(fun) = &info.config.skip_deser_fn {
                    let path: syn::Path = parse_str(fun).expect("invalid #[proto(skip = \"...\")] function path");
                    let access = info.access.access_tokens(quote! { shadow });
                    Some(quote! {
                        {
                            let __proto_rs_tmp = #path(&mut shadow);
                            #access = __proto_rs_tmp;
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if hooks.is_empty() {
        quote! {}
    } else {
        quote! {
            #[inline(always)]
            fn post_decode(mut shadow: Self::Shadow<'_>) -> Result<Self, ::proto_rs::DecodeError> {
                #(#hooks)*
                ::proto_rs::ProtoShadow::to_sun(shadow)
            }
        }
    }
}

fn field_wire_type(field: &FieldInfo<'_>) -> Type {
    let original_ty = &field.field.ty;

    if let Some(into_ty) = field.config.into_type.as_ref() {
        let ty: Type = parse_str(into_ty).expect("invalid #[proto(into = ...)] type");
        if option_inner_type(original_ty).is_some() {
            parse_quote! { ::core::option::Option<#ty> }
        } else {
            ty
        }
    } else if let Some(from_ty) = field.config.from_type.as_ref() {
        let ty: Type = parse_str(from_ty).expect("invalid #[proto(from = ...)] type");
        if option_inner_type(original_ty).is_some() {
            parse_quote! { ::core::option::Option<#ty> }
        } else {
            ty
        }
    } else {
        original_ty.clone()
    }
}

fn encode_value_expr(field: &FieldInfo<'_>, access_expr: TokenStream2) -> TokenStream2 {
    if field.config.skip {
        return access_expr;
    }

    if let Some(into_ty) = field.config.into_type.as_ref().or(field.config.from_type.as_ref()) {
        let ty_tokens: Type = parse_str(into_ty).expect("invalid #[proto(into)] type");
        if let Some(fun) = field.config.into_fn.as_ref() {
            let path: syn::Path = parse_str(fun).expect("invalid #[proto(into_fn)] path");
            if is_option_type(&field.field.ty) {
                quote! { (#access_expr).as_ref().map(|__proto_rs_value| #path(__proto_rs_value)) }
            } else {
                quote! { #path(&(#access_expr)) }
            }
        } else if is_option_type(&field.field.ty) {
            quote! {
                (#access_expr)
                    .as_ref()
                    .map(|__proto_rs_value| <#ty_tokens as ::core::convert::From<_>>::from(::core::clone::Clone::clone(__proto_rs_value)))
            }
        } else {
            quote! { <#ty_tokens as ::core::convert::From<_>>::from(::core::clone::Clone::clone(&(#access_expr))) }
        }
    } else {
        access_expr
    }
}

fn encode_input_expr(wire_ty: &Type, value_expr: TokenStream2) -> TokenStream2 {
    if is_option_type(wire_ty) {
        let inner = option_inner_type(wire_ty).expect("option missing inner type");
        if is_value_encode_type(&inner) {
            quote! { (#value_expr).as_ref().map(|inner| *inner) }
        } else {
            quote! { (#value_expr).as_ref().map(|inner| inner) }
        }
    } else if is_value_encode_type(wire_ty) {
        value_expr
    } else {
        quote! { &(#value_expr) }
    }
}

fn option_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(path) = ty
        && let Some(seg) = path.path.segments.last()
        && seg.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        Some(inner.clone())
    } else {
        None
    }
}

fn default_expr_for_field(info: &FieldInfo<'_>, ty: &Type) -> TokenStream2 {
    if needs_manual_handling(info) {
        if is_option_type(ty) {
            quote! { None }
        } else if vec_inner_type(ty).is_some() {
            quote! { ::proto_rs::alloc::vec::Vec::new() }
        } else {
            quote! { ::core::default::Default::default() }
        }
    } else {
        quote! { <#ty as ::proto_rs::ProtoWire>::proto_default() }
    }
}

fn needs_manual_handling(info: &FieldInfo<'_>) -> bool {
    info.config.skip || info.config.into_type.is_some() || info.config.from_type.is_some() || info.config.into_fn.is_some() || info.config.from_fn.is_some()
}

fn needs_conversion(info: &FieldInfo<'_>) -> bool {
    info.config.into_type.is_some() || info.config.from_type.is_some() || info.config.into_fn.is_some() || info.config.from_fn.is_some()
}

fn build_decode_assignment(info: &FieldInfo<'_>, tmp_ident: &Ident, target_access: TokenStream2, wire_ty: &Type) -> TokenStream2 {
    if let Some(fun) = info.config.from_fn.as_ref() {
        let path: syn::Path = parse_str(fun).expect("invalid #[proto(from_fn = ...)] path");
        quote! { #target_access = #path(#tmp_ident); }
    } else {
        let field_ty = &info.field.ty;
        quote! { #target_access = <#field_ty as ::core::convert::From<#wire_ty>>::from(#tmp_ident); }
    }
}
