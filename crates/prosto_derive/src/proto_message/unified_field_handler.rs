use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Attribute;
use syn::Field;
use syn::Ident;
use syn::ItemEnum;
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
    let ty = &field.field.ty;
    let binding_ident = Ident::new(&format!("__proto_rs_field_{}_input", field.index), field.field.span());
    let access_expr = match &field.access {
        FieldAccess::Direct(tokens) => tokens.clone(),
        _ => field.access.access_tokens(base.clone()),
    };

    let init_expr = if is_option_type(ty) {
        quote! { (#access_expr).as_ref().map(|inner| inner) }
    } else if matches!(field.access, FieldAccess::Direct(_)) || is_value_encode_type(ty) {
        access_expr.clone()
    } else {
        quote! { &(#access_expr) }
    };

    let init = quote! {
        let #binding_ident: <#ty as ::proto_rs::ProtoWire>::EncodeInput<'_> = #init_expr;
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

pub fn build_clear_stmts(fields: &[FieldInfo<'_>], self_tokens: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .map(|info| {
            let access = info.access.access_tokens(self_tokens.clone());
            let ty = &info.field.ty;
            quote! { <#ty as ::proto_rs::ProtoWire>::clear(&mut #access) }
        })
        .collect()
}

pub fn build_is_default_checks(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
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

pub fn build_encoded_len_terms(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
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

pub fn build_encode_stmts(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
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
