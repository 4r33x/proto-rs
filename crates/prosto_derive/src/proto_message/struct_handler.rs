use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Fields;
use syn::Index;

use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::generate_field_decode;
use super::unified_field_handler::generate_field_encode;
use super::unified_field_handler::generate_field_encoded_len;
use crate::utils::is_option_type;
use crate::utils::parse_field_config;
use crate::utils::vec_inner_type;

pub fn handle_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    match &data.fields {
        Fields::Named(_) => handle_named_struct(input, data),
        Fields::Unnamed(_) => handle_tuple_struct(input, data),
        Fields::Unit => handle_unit_struct(input),
    }
}

fn strip_proto_attrs(attrs: &[syn::Attribute]) -> Vec<syn::Attribute> {
    attrs.iter().filter(|a| !a.path().is_ident("proto_message") && !a.path().is_ident("proto")).cloned().collect()
}

/// Generate smart default value for a field type
fn generate_field_default(field: &syn::Field) -> TokenStream {
    let field_ty = &field.ty;

    if is_option_type(field_ty) {
        return quote! { None };
    }

    if vec_inner_type(field_ty).is_some() {
        return quote! { Vec::new() };
    }

    let cfg = parse_field_config(field);
    if cfg.into_type.is_some() || cfg.from_type.is_some() || cfg.into_fn.is_some() || cfg.from_fn.is_some() || cfg.skip {
        quote! { ::core::default::Default::default() }
    } else {
        quote! { <#field_ty as ::proto_rs::ProtoExt>::proto_default() }
    }
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

    let cfg = parse_field_config(field);
    if cfg.into_type.is_some() || cfg.from_type.is_some() || cfg.into_fn.is_some() || cfg.from_fn.is_some() || cfg.skip {
        quote! { #access_tokens = ::core::default::Default::default(); }
    } else {
        quote! { #access_tokens = <#field_ty as ::proto_rs::ProtoExt>::proto_default(); }
    }
}

fn handle_unit_struct(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let attrs = strip_proto_attrs(&input.attrs);
    let vis = &input.vis;
    let generics = &input.generics;

    quote! {
        #(#attrs)*
        #vis struct #name #generics;

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            #[inline]
            fn proto_default() -> Self {
                Self
            }

            fn encode_raw(&self, _buf: &mut impl ::proto_rs::bytes::BufMut) {}

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx)
            }

            fn encoded_len(&self) -> usize {
                0
            }

            fn clear(&mut self) {}
        }

        impl #generics ::proto_rs::MessageField for #name #generics {}
    }
}

fn handle_tuple_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;
    let attrs = strip_proto_attrs(&input.attrs);
    let vis = &input.vis;
    let generics = &input.generics;

    let Fields::Unnamed(fields) = &data.fields else {
        panic!("Expected unnamed fields");
    };

    let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

    // Generate smart defaults
    let default_values: Vec<_> = fields.unnamed.iter().map(generate_field_default).collect();

    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut encoded_len_fields = Vec::new();
    let mut clear_fields = Vec::new();
    let mut post_decode_hooks = Vec::new();

    for (idx, field) in fields.unnamed.iter().enumerate() {
        let field_config = parse_field_config(field);
        let field_num = field_config.custom_tag.unwrap_or(idx + 1);
        let tuple_idx = Index::from(idx);
        let field_access = FieldAccess::Tuple(tuple_idx.clone());

        if field_config.skip {
            if let Some(fun) = &field_config.skip_deser_fn {
                let fun_path: syn::Path = syn::parse_str(fun).expect("invalid skip function path");
                let access_tokens = field_access.self_tokens();
                post_decode_hooks.push(quote! {
                    #access_tokens = #fun_path(self);
                });
            }
        } else {
            let tag_u32 = field_num as u32;

            let access_expr = field_access.self_tokens();
            encode_fields.push(generate_field_encode(field, access_expr.clone(), tag_u32));

            let decode_body = generate_field_decode(field, access_expr.clone(), tag_u32);
            decode_fields.push(quote! {
                #tag_u32 => {
                    #decode_body
                    Ok(())
                }
            });

            encoded_len_fields.push(generate_field_encoded_len(field, access_expr, tag_u32));
        }

        clear_fields.push(generate_field_clear(field, &field_access));
    }

    let post_decode_impl = if post_decode_hooks.is_empty() {
        quote! {}
    } else {
        quote! {
            fn post_decode(&mut self) {
                #(#post_decode_hooks)*
            }
        }
    };

    quote! {
        #(#attrs)*
        #vis struct #name #generics(#(pub #field_types),*);

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            #[inline]
            fn proto_default() -> Self {
                Self(#(#default_values),*)
            }

            fn encode_raw(&self, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #(#encode_fields)*
            }

            fn merge_field(
                &mut self,
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

            fn encoded_len(&self) -> usize {
                0 #(+ #encoded_len_fields)*
            }

            fn clear(&mut self) {
                #(#clear_fields)*
            }

            #post_decode_impl
        }

        impl #generics ::proto_rs::MessageField for #name #generics {}
    }
}

fn handle_named_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
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
            let default_value = generate_field_default(field);
            quote! { #ident: #default_value }
        })
        .collect();

    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut encoded_len_fields = Vec::new();
    let mut clear_fields = Vec::new();
    let mut post_decode_hooks = Vec::new();
    let mut next_tag = 1usize;

    for field in &fields.named {
        let ident = field.ident.as_ref().unwrap();
        let field_config = parse_field_config(field);
        let field_access = FieldAccess::Named(ident.clone());

        if field_config.skip {
            if let Some(fun) = &field_config.skip_deser_fn {
                let fun_path: syn::Path = syn::parse_str(fun).expect("invalid skip function path");
                let access_tokens = field_access.self_tokens();
                post_decode_hooks.push(quote! {
                    #access_tokens = #fun_path(self);
                });
            }
        }

        let tag = if field_config.skip {
            None
        } else {
            let assigned = match field_config.custom_tag {
                Some(tag) => {
                    if tag == 0 {
                        panic!("proto field tags must be >= 1");
                    }
                    next_tag = tag.checked_add(1).expect("proto field tag overflowed usize range");
                    tag
                }
                None => {
                    let tag = next_tag;
                    next_tag = next_tag.checked_add(1).expect("proto field tag overflowed usize range");
                    tag
                }
            };
            Some(assigned)
        };

        if let Some(tag) = tag {
            let tag_u32 = tag as u32;
            let access_expr = field_access.self_tokens();
            encode_fields.push(generate_field_encode(field, access_expr.clone(), tag_u32));

            let decode_body = generate_field_decode(field, access_expr.clone(), tag_u32);
            decode_fields.push(quote! {
                #tag_u32 => {
                    #decode_body
                    Ok(())
                }
            });
            encoded_len_fields.push(generate_field_encoded_len(field, access_expr, tag_u32));
        }

        clear_fields.push(generate_field_clear(field, &field_access));
    }

    let post_decode_impl = if post_decode_hooks.is_empty() {
        quote! {}
    } else {
        quote! {
            fn post_decode(&mut self) {
                #(#post_decode_hooks)*
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

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            #[inline]
            fn proto_default() -> Self {
                Self {
                    #(#default_field_values),*
                }
            }

            fn encode_raw(&self, buf: &mut impl ::proto_rs::bytes::BufMut) {
                #(#encode_fields)*
            }

            fn merge_field(
                &mut self,
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

            fn encoded_len(&self) -> usize {
                0 #(+ #encoded_len_fields)*
            }

            fn clear(&mut self) {
                #(#clear_fields)*
            }

            #post_decode_impl
        }

        impl #generics ::proto_rs::MessageField for #name #generics {}
    }
}
