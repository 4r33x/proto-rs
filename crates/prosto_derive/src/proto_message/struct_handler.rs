use alloc::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Fields;
use syn::Index;

use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::field_default_expr;
use super::unified_field_handler::generate_field_decode;
use super::unified_field_handler::generate_field_encode;
use super::unified_field_handler::generate_field_encoded_len;
use crate::parse::UnifiedProtoConfig;
use crate::utils::is_option_type;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::vec_inner_type;

pub fn handle_struct(input: DeriveInput, data: &syn::DataStruct, config: &UnifiedProtoConfig) -> TokenStream {
    match &data.fields {
        Fields::Named(_) => handle_named_struct(input, data, config),
        Fields::Unnamed(_) => handle_tuple_struct(input, data, config),
        Fields::Unit => handle_unit_struct(input, config),
    }
}

fn strip_proto_attrs(attrs: &[syn::Attribute]) -> Vec<syn::Attribute> {
    attrs.iter().filter(|a| !a.path().is_ident("proto_message") && !a.path().is_ident("proto")).cloned().collect()
}

/// Generate smart clear for a field
fn generate_field_clear(field: &syn::Field, access: &FieldAccess) -> TokenStream {
    let field_ty = &field.ty;
    let access_tokens = access.self_tokens();

    if is_option_type(field_ty) {
        return quote! { #access_tokens = None; };
    }

    if vec_inner_type(field_ty).is_some() {
        return quote! { #access_tokens.clear(); };
    }

    let parsed_ty = parse_field_type(field_ty);
    if parsed_ty.map_kind.is_some() || parsed_ty.set_kind.is_some() {
        return quote! { #access_tokens.clear(); };
    }

    let cfg = parse_field_config(field);
    if cfg.into_type.is_some() || cfg.from_type.is_some() || cfg.into_fn.is_some() || cfg.from_fn.is_some() || cfg.skip {
        quote! { #access_tokens = ::core::default::Default::default(); }
    } else {
        quote! { #access_tokens = <#field_ty as ::proto_rs::ProtoExt>::proto_default(); }
    }
}

fn handle_unit_struct(input: DeriveInput, config: &UnifiedProtoConfig) -> TokenStream {
    let name = &input.ident;
    let attrs = strip_proto_attrs(&input.attrs);
    let vis = &input.vis;
    let generics = &input.generics;

    let shadow_ty = quote! { #name #generics };
    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        shadow_ty.clone()
    };

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        quote! {
            impl #generics ::proto_rs::ProtoShadow for #name #generics {
                type Sun<'a> = &'a Self;
                type OwnedSun = Self;
                type View<'a> = &'a Self;

                fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
                    Ok(self)
                }

                fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                    value
                }
            }
        }
    };

    let encoded_len_binding = if config.sun.is_some() {
        quote! { let _value = value; }
    } else {
        quote! { let _value: &Self = *value; }
    };

    let encode_binding = if config.sun.is_some() {
        quote! { let _value = &value; }
    } else {
        quote! { let _value: &Self = value; }
    };

    let clear_impl = if config.sun.is_some() {
        quote! {
            fn clear(&mut self) {
                if let Ok(default) = Self::post_decode(Self::proto_default()) {
                    *self = default;
                }
            }
        }
    } else {
        quote! { fn clear(&mut self) {} }
    };

    quote! {
        #(#attrs)*
        #vis struct #name #generics;

        #proto_shadow_impl

        impl #generics ::proto_rs::ProtoExt for #target_ty {
            type Shadow<'a> = #shadow_ty;

            #[inline]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                #name
            }

            fn encoded_len(value: &::proto_rs::ViewOf<'_, Self>) -> usize {
                #encoded_len_binding
                0
            }

            fn encode_raw(value: ::proto_rs::ViewOf<'_, Self>, _buf: &mut impl ::proto_rs::bytes::BufMut) {
                #encode_binding
            }

            fn merge_field(
                _value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx)
            }

            #clear_impl
        }

        impl #generics ::proto_rs::MessageField for #target_ty {}
    }
}

fn handle_tuple_struct(input: DeriveInput, data: &syn::DataStruct, config: &UnifiedProtoConfig) -> TokenStream {
    let name = &input.ident;
    let attrs = strip_proto_attrs(&input.attrs);
    let vis = &input.vis;
    let generics = &input.generics;

    let Fields::Unnamed(fields) = &data.fields else {
        panic!("Expected unnamed fields");
    };

    let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

    // Generate smart defaults
    let default_values: Vec<_> = fields.unnamed.iter().map(field_default_expr).collect();

    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut encoded_len_fields = Vec::new();
    let mut clear_fields = Vec::new();
    let mut post_decode_hooks = Vec::new();

    let mut used_tags = BTreeSet::new();
    let mut next_tag = 1usize;

    for (idx, field) in fields.unnamed.iter().enumerate() {
        let field_config = parse_field_config(field);
        let tuple_idx = Index::from(idx);
        let field_access = FieldAccess::Tuple(tuple_idx.clone());

        if field_config.skip
            && let Some(fun) = &field_config.skip_deser_fn
        {
            let fun_path: syn::Path = syn::parse_str(fun).expect("invalid skip function path");
            let access_tokens = field_access.tokens_with_base(quote! { shadow });
            post_decode_hooks.push(quote! {
                {
                    let __proto_rs_tmp = #fun_path(&mut shadow);
                    #access_tokens = __proto_rs_tmp;
                }
            });
        }

        let tag = if field_config.skip {
            None
        } else if let Some(custom) = field_config.custom_tag {
            assert!((custom != 0), "proto field tags must be >= 1");
            assert!(used_tags.insert(custom), "duplicate proto field tag: {custom}");
            Some(custom)
        } else {
            while used_tags.contains(&next_tag) {
                next_tag = next_tag.checked_add(1).expect("proto field tag overflowed usize range");
            }
            let assigned = next_tag;
            used_tags.insert(assigned);
            next_tag = next_tag.checked_add(1).expect("proto field tag overflowed usize range");
            Some(assigned)
        };

        if let Some(tag) = tag {
            let tag_u32 = tag.try_into().unwrap();

            let encode_access = field_access.tokens_with_base(quote! { value });
            encode_fields.push(generate_field_encode(field, encode_access, tag_u32));

            let decode_access = field_access.tokens_with_base(quote! { shadow });
            let decode_body = generate_field_decode(field, decode_access, tag_u32);
            decode_fields.push(quote! {
                #tag_u32 => {
                    #decode_body
                    Ok(())
                }
            });

            let encoded_len_access = field_access.tokens_with_base(quote! { value });
            let encoded_len_tokens = generate_field_encoded_len(field, encoded_len_access, tag_u32);
            encoded_len_fields.push(encoded_len_tokens.tokens);
        }

        clear_fields.push(generate_field_clear(field, &field_access));
    }

    let post_decode_impl = if post_decode_hooks.is_empty() {
        quote! {}
    } else {
        quote! {
            fn post_decode(mut shadow: Self::Shadow<'_>) -> Result<Self, ::proto_rs::DecodeError> {
                #(#post_decode_hooks)*
                ::proto_rs::ProtoShadow::to_sun(shadow)
            }
        }
    };

    let shadow_ty = quote! { #name #generics };
    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        shadow_ty.clone()
    };

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        quote! {
            impl #generics ::proto_rs::ProtoShadow for #name #generics {
                type Sun<'a> = &'a Self;
                type OwnedSun = Self;
                type View<'a> = &'a Self;

                fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
                    Ok(self)
                }

                fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                    value
                }
            }
        }
    };

    let encoded_len_binding = if config.sun.is_some() {
        quote! {}
    } else {
        quote! { let value: &Self = *value; }
    };

    let encode_binding = if config.sun.is_some() {
        quote! { let value = &value; }
    } else {
        quote! { let value: &Self = value; }
    };

    let clear_impl = if config.sun.is_some() {
        quote! {
            fn clear(&mut self) {
                if let Ok(default) = Self::post_decode(Self::proto_default()) {
                    *self = default;
                }
            }
        }
    } else {
        quote! {
            fn clear(&mut self) {
                #(#clear_fields)*
            }
        }
    };

    quote! {
        #(#attrs)*
        #vis struct #name #generics(#(pub #field_types),*);

        #proto_shadow_impl

        impl #generics ::proto_rs::ProtoExt for #target_ty {
            type Shadow<'a> = #shadow_ty;

            #[inline]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                #shadow_ty(#(#default_values),*)
            }

            fn encoded_len(value: &::proto_rs::ViewOf<'_, Self>) -> usize {
                #encoded_len_binding
                0 #(+ #encoded_len_fields)*
            }

            fn encode_raw(value: ::proto_rs::ViewOf<'_, Self>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #encode_binding
                #(#encode_fields)*
            }

            fn merge_field(
                shadow: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                use ::proto_rs::bytes::Buf;
                match tag {
                    #(#decode_fields,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            #clear_impl

            #post_decode_impl
        }

        impl #generics ::proto_rs::MessageField for #target_ty {}
    }
}

fn handle_named_struct(input: DeriveInput, data: &syn::DataStruct, config: &UnifiedProtoConfig) -> TokenStream {
    let name = &input.ident;
    let attrs = strip_proto_attrs(&input.attrs);
    let vis = &input.vis;
    let generics = &input.generics;

    let Fields::Named(fields) = &data.fields else {
        panic!("Expected named fields");
    };

    let mut fields_named_idents = Vec::new();
    let mut fields_named_attrs = Vec::new();
    let mut fields_named_types = Vec::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let field_attrs = strip_proto_attrs(&field.attrs);

        fields_named_idents.push(ident);
        fields_named_attrs.push(field_attrs);
        fields_named_types.push(ty);
    }

    // Generate smart defaults
    let default_field_values: Vec<_> = fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let default_value = field_default_expr(field);
            quote! { #ident: #default_value }
        })
        .collect();

    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut encoded_len_fields = Vec::new();
    let mut clear_fields = Vec::new();
    let mut post_decode_hooks = Vec::new();
    let mut next_tag = 1usize;
    let mut used_tags = BTreeSet::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().unwrap();
        let field_config = parse_field_config(field);
        let field_access = FieldAccess::Named(ident.clone());

        if field_config.skip
            && let Some(fun) = &field_config.skip_deser_fn
        {
            let fun_path: syn::Path = syn::parse_str(fun).expect("invalid skip function path");
            let access_tokens = field_access.tokens_with_base(quote! { shadow });
            post_decode_hooks.push(quote! {
                {
                    let __proto_rs_tmp = #fun_path(&mut shadow);
                    #access_tokens = __proto_rs_tmp;
                }
            });
        }

        let tag = if field_config.skip {
            None
        } else {
            let assigned = if let Some(tag) = field_config.custom_tag {
                assert!((tag != 0), "proto field tags must be >= 1");
                assert!(used_tags.insert(tag), "duplicate proto field tag: {tag}");
                tag
            } else {
                while used_tags.contains(&next_tag) {
                    next_tag = next_tag.checked_add(1).expect("proto field tag overflowed usize range");
                }
                let tag = next_tag;
                used_tags.insert(tag);
                next_tag = next_tag.checked_add(1).expect("proto field tag overflowed usize range");
                tag
            };
            Some(assigned)
        };

        if let Some(tag) = tag {
            let tag_u32 = tag.try_into().unwrap();
            let encode_access = field_access.tokens_with_base(quote! { value });
            encode_fields.push(generate_field_encode(field, encode_access, tag_u32));

            let decode_access = field_access.tokens_with_base(quote! { shadow });
            let decode_body = generate_field_decode(field, decode_access, tag_u32);
            decode_fields.push(quote! {
                #tag_u32 => {
                    #decode_body
                    Ok(())
                }
            });
            let encoded_len_access = field_access.tokens_with_base(quote! { value });
            let encoded_len_tokens = generate_field_encoded_len(field, encoded_len_access, tag_u32);
            encoded_len_fields.push(encoded_len_tokens.tokens);
        }

        clear_fields.push(generate_field_clear(field, &field_access));
    }

    let post_decode_impl = if post_decode_hooks.is_empty() {
        quote! {}
    } else {
        quote! {
            fn post_decode(mut shadow: Self::Shadow<'_>) -> Result<Self, ::proto_rs::DecodeError> {
                #(#post_decode_hooks)*
                ::proto_rs::ProtoShadow::to_sun(shadow)
            }
        }
    };

    let shadow_ty = quote! { #name #generics };
    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        shadow_ty.clone()
    };

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        quote! {
            impl #generics ::proto_rs::ProtoShadow for #name #generics {
                type Sun<'a> = &'a Self;
                type OwnedSun = Self;
                type View<'a> = &'a Self;

                fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
                    Ok(self)
                }

                fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                    value
                }
            }
        }
    };

    let encoded_len_binding = if config.sun.is_some() {
        quote! {}
    } else {
        quote! { let value: &Self = *value; }
    };

    let encode_binding = if config.sun.is_some() {
        quote! { let value = &value; }
    } else {
        quote! { let value: &Self = value; }
    };

    let clear_impl = if config.sun.is_some() {
        quote! {
            fn clear(&mut self) {
                if let Ok(default) = Self::post_decode(Self::proto_default()) {
                    *self = default;
                }
            }
        }
    } else {
        quote! {
            fn clear(&mut self) {
                #(#clear_fields)*
            }
        }
    };

    quote! {
        #(#attrs)*
        #vis struct #name #generics {
            #(
                #(#fields_named_attrs)*
                pub #fields_named_idents: #fields_named_types,
            )*
        }

        #proto_shadow_impl

        impl #generics ::proto_rs::ProtoExt for #target_ty {
            type Shadow<'a> = #shadow_ty;

            #[inline]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                #shadow_ty {
                    #(#default_field_values),*
                }
            }

            fn encoded_len(value: &::proto_rs::ViewOf<'_, Self>) -> usize {
                #encoded_len_binding
                0 #(+ #encoded_len_fields)*
            }

            fn encode_raw(value: ::proto_rs::ViewOf<'_, Self>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #encode_binding
                #(#encode_fields)*
            }

            fn merge_field(
                shadow: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                use ::proto_rs::bytes::Buf;
                match tag {
                    #(#decode_fields,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            #clear_impl

            #post_decode_impl
        }

        impl #generics ::proto_rs::MessageField for #target_ty {}
    }
}
