use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Attribute;
use syn::Field;
use syn::Ident;
use syn::ItemEnum;
use syn::Path;
use syn::Type;
use syn::spanned::Spanned;

use crate::utils::FieldConfig;
use crate::utils::is_option_type;

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
    pub ty: Type,
}

pub fn encode_input_binding(field: &FieldInfo<'_>, base: &TokenStream2) -> Option<EncodeBinding> {
    if field.config.skip {
        return None;
    }

    let ty = encode_ty(field);
    let binding_ident = Ident::new(&format!("__proto_rs_field_{}_input", field.index), field.field.span());
    let access_expr = match &field.access {
        FieldAccess::Direct(tokens) => tokens.clone(),
        _ => field.access.access_tokens(base.clone()),
    };

    let init = build_encode_binding_init(field, ty.clone(), binding_ident.clone(), access_expr);

    Some(EncodeBinding { init, ident: binding_ident, ty })
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
        let defaults = fields.iter().map(|info| build_field_default(info));
        quote! { Self( #(#defaults),* ) }
    } else {
        let defaults = fields.iter().map(|info| {
            let ident = info.access.ident().expect("expected named field");
            let default_value = build_field_default(info);
            quote! { #ident: #default_value }
        });
        quote! { Self { #(#defaults),* } }
    }
}

pub fn build_clear_stmts(fields: &[FieldInfo<'_>], self_tokens: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .map(|info| {
            let access = info.access.access_tokens(self_tokens.clone());
            if uses_proto_wire(info) {
                let ty = &info.field.ty;
                quote! { <#ty as ::proto_rs::ProtoWire>::clear(&mut #access) }
            } else {
                quote! { #access = ::core::default::Default::default() }
            }
        })
        .collect()
}

pub fn build_is_default_checks(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            info.tag?;
            let binding = encode_input_binding(info, base)?;
            let ident = binding.ident;
            let init = binding.init;
            let ty = binding.ty;
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

pub fn build_encoded_len_terms(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let binding = encode_input_binding(info, base)?;
            let ident = binding.ident;
            let init = binding.init;
            let ty = binding.ty;
            Some(quote! {{
                #init
                <#ty as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&#ident, #tag)
            }})
        })
        .collect()
}

pub fn build_encode_stmts(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let binding = encode_input_binding(info, base)?;
            let ident = binding.ident;
            let init = binding.init;
            let ty = binding.ty;
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

pub fn build_decode_arm(info: &FieldInfo<'_>, base: &TokenStream2, wire_ident: &TokenStream2, buf_ident: &TokenStream2, ctx_ident: &TokenStream2) -> Option<TokenStream2> {
    let tag = info.tag?;
    let access = info.access.access_tokens(base.clone());

    if let Some(from_ty_raw) = info.config.from_type.as_ref().or(info.config.into_type.as_ref()) {
        let from_ty = parse_type(from_ty_raw);
        let assign_expr = if let Some(fun) = info.config.from_fn.as_ref() {
            let fun_path = parse_path(fun);
            quote! { #fun_path(__proto_rs_tmp) }
        } else {
            let field_ty = &info.field.ty;
            quote! { <#field_ty as ::core::convert::From<#from_ty>>::from(__proto_rs_tmp) }
        };

        Some(quote! {
            #tag => {
                let mut __proto_rs_tmp: #from_ty = <#from_ty as ::proto_rs::ProtoWire>::proto_default();
                <#from_ty as ::proto_rs::ProtoWire>::decode_into(
                    #wire_ident,
                    &mut __proto_rs_tmp,
                    #buf_ident,
                    #ctx_ident,
                )?;
                #access = #assign_expr;
                Ok(())
            }
        })
    } else {
        let field_ty = &info.field.ty;
        Some(quote! {
            #tag => {
                <#field_ty as ::proto_rs::ProtoWire>::decode_into(
                    #wire_ident,
                    &mut #access,
                    #buf_ident,
                    #ctx_ident,
                )
            }
        })
    }
}

pub fn build_post_decode_method(fields: &[FieldInfo<'_>]) -> TokenStream2 {
    let hooks = fields
        .iter()
        .filter_map(|info| {
            let fun = info.config.skip_deser_fn.as_ref()?;
            let fun_path = parse_path(fun);
            let access = info.access.access_tokens(quote! { shadow });
            Some(quote! {
                {
                    let __proto_rs_tmp = #fun_path(&shadow);
                    #access = __proto_rs_tmp;
                }
            })
        })
        .collect::<Vec<_>>();

    if hooks.is_empty() {
        quote! {}
    } else {
        quote! {
            #[inline(always)]
            fn post_decode(value: Self::Shadow<'_>) -> Result<Self, ::proto_rs::DecodeError> {
                let mut shadow = value;
                #(#hooks)*
                ::proto_rs::ProtoShadow::to_sun(shadow)
            }
        }
    }
}

fn encode_ty(field: &FieldInfo<'_>) -> Type {
    if let Some(into_ty) = field.config.into_type.as_ref() {
        parse_type(into_ty)
    } else {
        field.field.ty.clone()
    }
}

fn parse_type(raw: &str) -> Type {
    syn::parse_str::<Type>(raw).unwrap_or_else(|err| panic!("failed to parse type `{raw}`: {err}"))
}

fn parse_path(raw: &str) -> Path {
    syn::parse_str::<Path>(raw).unwrap_or_else(|err| panic!("failed to parse path `{raw}`: {err}"))
}

fn build_encode_binding_init(field: &FieldInfo<'_>, ty: Type, binding_ident: Ident, access_expr: TokenStream2) -> TokenStream2 {
    if let Some(into_ty_raw) = field.config.into_type.as_ref() {
        let into_ty = parse_type(into_ty_raw);
        let convert_expr = if let Some(fun) = field.config.into_fn.as_ref() {
            let fun_path = parse_path(fun);
            let ref_expr = access_as_ref(field, &access_expr);
            quote! { #fun_path(#ref_expr) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from((#access_expr).clone()) }
        };

        if is_value_encode_type(&ty) {
            quote! {
                let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #convert_expr;
            }
        } else {
            let converted_ident = Ident::new(&format!("__proto_rs_field_{}_converted", field.index), field.field.span());
            quote! {
                let #converted_ident = #convert_expr;
                let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = &(#converted_ident);
            }
        }
    } else if is_option_type(&field.field.ty) {
        let inner = option_inner_type(&field.field.ty).expect("option inner type");
        if is_value_encode_type(inner) {
            quote! {
                let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #access_expr;
            }
        } else {
            quote! {
                let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = (#access_expr).as_ref().map(|inner| inner);
            }
        }
    } else if matches!(field.access, FieldAccess::Direct(_)) || is_value_encode_type(&field.field.ty) {
        quote! {
            let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #access_expr;
        }
    } else {
        quote! {
            let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = &(#access_expr);
        }
    }
}

fn option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(path) = ty
        && let Some(seg) = path.path.segments.last()
        && seg.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        Some(inner)
    } else {
        None
    }
}

fn access_as_ref(field: &FieldInfo<'_>, expr: &TokenStream2) -> TokenStream2 {
    if matches!(field.access, FieldAccess::Direct(_)) {
        quote! { #expr }
    } else {
        quote! { &(#expr) }
    }
}

fn uses_proto_wire(field: &FieldInfo<'_>) -> bool {
    !(field.config.skip || field.config.into_type.is_some() || field.config.from_type.is_some() || field.config.into_fn.is_some() || field.config.from_fn.is_some())
}

pub fn build_field_default(field: &FieldInfo<'_>) -> TokenStream2 {
    if uses_proto_wire(field) {
        let ty = &field.field.ty;
        quote! { <#ty as ::proto_rs::ProtoWire>::proto_default() }
    } else {
        quote! { ::core::default::Default::default() }
    }
}
