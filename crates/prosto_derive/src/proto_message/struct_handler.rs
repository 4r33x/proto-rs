use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Fields;
use syn::Index;
use syn::Type;

use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::generate_field_decode;
use super::unified_field_handler::generate_field_encode;
use super::unified_field_handler::generate_field_encoded_len;
use crate::utils::parse_field_config;

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
fn generate_field_default(field_ty: &Type) -> TokenStream {
    if let Type::Path(type_path) = field_ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;

            // Handle Option<T> - always None
            if ident == "Option" {
                return quote! { None };
            }

            // Handle Vec<T> - always Vec::new()
            if ident == "Vec" {
                return quote! { Vec::new() };
            }
        }
    }

    // Default case - use ProtoExt::proto_default()
    quote! { <#field_ty as ::proto_rs::ProtoExt>::proto_default() }
}

/// Generate smart clear for a field
fn generate_field_clear(field_ident: &dyn quote::ToTokens, field_ty: &Type) -> TokenStream {
    if let Type::Path(type_path) = field_ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;

            // Handle Option<T>
            if ident == "Option" {
                return quote! { self.#field_ident = None; };
            }

            // Handle Vec<T>
            if ident == "Vec" {
                return quote! { self.#field_ident.clear(); };
            }
        }
    }

    // Default case
    quote! { self.#field_ident = <#field_ty as ::proto_rs::ProtoExt>::proto_default(); }
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

            fn encode_raw(&self, _buf: &mut impl ::bytes::BufMut) {}

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx)
            }

            fn encoded_len(&self) -> usize {
                0
            }

            fn clear(&mut self) {}
        }
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
    let default_values: Vec<_> = field_types.iter().map(|ty| generate_field_default(ty)).collect();

    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut encoded_len_fields = Vec::new();
    let mut clear_fields = Vec::new();

    for (idx, field) in fields.unnamed.iter().enumerate() {
        let field_config = parse_field_config(field);
        let field_num = field_config.custom_tag.unwrap_or(idx + 1);
        let tuple_idx = Index::from(idx);
        let field_ty = &field.ty;

        if !field_config.skip {
            let tag_u32 = field_num as u32;
            let field_access = FieldAccess::Tuple(tuple_idx.clone());

            encode_fields.push(generate_field_encode(field, field_access.clone(), tag_u32));

            let decode_body = generate_field_decode(field, field_access.clone(), tag_u32);
            decode_fields.push(quote! {
                #tag_u32 => {
                    #decode_body
                    Ok(())
                }
            });

            encoded_len_fields.push(generate_field_encoded_len(field, field_access, tag_u32));

            clear_fields.push(generate_field_clear(&tuple_idx, field_ty));
        }
    }

    quote! {
        #(#attrs)*
        #vis struct #name #generics(#(pub #field_types),*);

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            #[inline]
            fn proto_default() -> Self {
                Self(#(#default_values),*)
            }

            fn encode_raw(&self, buf: &mut impl ::bytes::BufMut) {
                #(#encode_fields)*
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
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
        }
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
    let default_field_values: Vec<_> = fields_named_idents
        .iter()
        .zip(fields_named_types.iter())
        .map(|(ident, ty)| {
            let default_value = generate_field_default(ty);
            quote! { #ident: #default_value }
        })
        .collect();

    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut encoded_len_fields = Vec::new();
    let mut clear_fields = Vec::new();
    let mut field_num = 0usize;

    for field in &fields.named {
        let ident = field.ident.as_ref().unwrap();
        let field_config = parse_field_config(field);

        let tag = field_config.custom_tag.unwrap_or_else(|| {
            field_num += 1;
            field_num
        });
        let tag_u32 = tag as u32;

        if !field_config.skip {
            let field_ty = &field.ty;
            let field_access = FieldAccess::Named(ident.clone());

            encode_fields.push(generate_field_encode(field, field_access.clone(), tag_u32));

            let decode_body = generate_field_decode(field, field_access.clone(), tag_u32);
            decode_fields.push(quote! {
                #tag_u32 => {
                    #decode_body
                    Ok(())
                }
            });

            encoded_len_fields.push(generate_field_encoded_len(field, field_access, tag_u32));

            clear_fields.push(generate_field_clear(ident, field_ty));
        }
    }

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

            fn encode_raw(&self, buf: &mut impl ::bytes::BufMut) {
                #(#encode_fields)*
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
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
        }
    }
}
