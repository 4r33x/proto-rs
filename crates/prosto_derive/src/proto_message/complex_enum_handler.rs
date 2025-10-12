//! Handler for complex enums (with associated data) with ProtoExt support

use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;

use crate::utils::get_proto_rust_type;
use crate::utils::is_bytes_vec;
use crate::utils::is_complex_type;
use crate::utils::needs_into_conversion;
use crate::utils::parse_field_type;
use crate::utils::vec_inner_type;

pub fn handle_complex_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let original_variants: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let attrs: Vec<_> = v.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
            let ident = &v.ident;

            let fields = match &v.fields {
                Fields::Named(fields_named) => {
                    let filtered_fields: Vec<_> = fields_named
                        .named
                        .iter()
                        .map(|f| {
                            let field_attrs: Vec<_> = f.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
                            let field_ident = &f.ident;
                            let field_ty = &f.ty;
                            quote! { #(#field_attrs)* #field_ident: #field_ty }
                        })
                        .collect();
                    quote! { { #(#filtered_fields),* } }
                }
                Fields::Unnamed(fields_unnamed) => {
                    let filtered_fields: Vec<_> = fields_unnamed
                        .unnamed
                        .iter()
                        .map(|f| {
                            let field_attrs: Vec<_> = f.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
                            let field_ty = &f.ty;
                            quote! { #(#field_attrs)* #field_ty }
                        })
                        .collect();
                    quote! { ( #(#filtered_fields),* ) }
                }
                Fields::Unit => quote! {},
            };

            quote! { #(#attrs)* #ident #fields }
        })
        .collect();

    // Collect variant data for encoding/decoding
    let (encode_arms, decode_arms, encoded_len_arms) = generate_variant_arms(name, data);
    let last_variant = &data.variants.first().expect("Enum must have at least one variant");
    let last_variant_ident = &last_variant.ident;

    let default_value = match &last_variant.fields {
        Fields::Unit => quote! { Self::#last_variant_ident },
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            quote! { Self::#last_variant_ident(::proto_rs::ProtoExt::proto_default()) }
        }
        Fields::Named(fields) => {
            let field_defaults: Vec<_> = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    quote! { #ident: ::proto_rs::ProtoExt::proto_default() }
                })
                .collect();
            quote! { Self::#last_variant_ident { #(#field_defaults),* } }
        }
        _ => panic!("Unsupported variant structure"),
    };

    quote! {
        #(#attrs)*
        #vis enum #name #generics {
            #(#original_variants),*
        }

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            #[inline]
            fn proto_default() -> Self {
                #default_value
            }

            fn encode_raw(&self, buf: &mut impl ::bytes::BufMut) {
                match self {
                    #(#encode_arms)*
                }
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                match tag {
                    #(#decode_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            fn encoded_len(&self) -> usize {
                match self {
                    #(#encoded_len_arms)*
                }
            }

            fn clear(&mut self) {
                *self = Self::proto_default();
            }
        }
    }
}

fn generate_variant_arms(name: &syn::Ident, data: &DataEnum) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {
    let mut encode_arms = Vec::new();
    let mut decode_arms = Vec::new();
    let mut encoded_len_arms = Vec::new();

    // Process each variant
    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = (idx + 1) as u32;
        let variant_ident = &variant.ident;

        match &variant.fields {
            Fields::Unit => {
                // Unit variant - encode as empty message
                encode_arms.push(quote! {
                    #name::#variant_ident => {
                        ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                        ::proto_rs::encoding::encode_varint(0, buf);
                    }
                });

                decode_arms.push(quote! {
                    #tag => {
                        let len = ::proto_rs::encoding::decode_varint(buf)?;
                        if len != 0 {
                            return Err(::proto_rs::DecodeError::new("Expected empty message for unit variant"));
                        }
                        *self = #name::#variant_ident;
                        Ok(())
                    }
                });

                encoded_len_arms.push(quote! {
                    #name::#variant_ident => {
                        ::proto_rs::encoding::key_len(#tag) + 1
                    }
                });
            }
            Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
                let field = fields_unnamed.unnamed.first().unwrap();
                let field_ty = &field.ty;

                // Check if this is a Vec, Option, or Array that needs special handling
                if is_vec_or_array_type(field_ty) {
                    generate_repeated_variant_arms(name, variant_ident, tag, field_ty, &mut encode_arms, &mut decode_arms, &mut encoded_len_arms);
                } else {
                    // Regular message type
                    encode_arms.push(quote! {
                        #name::#variant_ident(inner) => {
                            ::proto_rs::encoding::message::encode(#tag, inner, buf);
                        }
                    });

                    decode_arms.push(quote! {
                        #tag => {
                            let mut temp = ::proto_rs::ProtoExt::proto_default();
                            ::proto_rs::encoding::message::merge(wire_type, &mut temp, buf, ctx)?;
                            *self = #name::#variant_ident(temp);
                            Ok(())
                        }
                    });

                    encoded_len_arms.push(quote! {
                        #name::#variant_ident(inner) => {
                            ::proto_rs::encoding::message::encoded_len(#tag, inner)
                        }
                    });
                }
            }
            Fields::Named(fields_named) => {
                // Named fields - encode as nested message (existing logic)
                generate_named_variant_arms(name, variant_ident, tag, fields_named, &mut encode_arms, &mut decode_arms, &mut encoded_len_arms);
            }
            _ => {
                panic!("Complex enum variants must have exactly one unnamed field or multiple named fields");
            }
        }
    }

    (encode_arms, decode_arms, encoded_len_arms)
}

fn is_vec_or_array_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Array(_) => true,
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "Vec" || segment.ident == "Option"
            } else {
                false
            }
        }
        _ => false,
    }
}

fn generate_repeated_variant_arms(
    name: &syn::Ident,
    variant_ident: &syn::Ident,
    tag: u32,
    field_ty: &syn::Type,
    encode_arms: &mut Vec<TokenStream>,
    decode_arms: &mut Vec<TokenStream>,
    encoded_len_arms: &mut Vec<TokenStream>,
) {
    let parsed = parse_field_type(field_ty);

    // For Vec<T>
    if parsed.is_repeated {
        if is_bytes_vec(field_ty) {
            // Vec<u8> is bytes
            encode_arms.push(quote! {
                #name::#variant_ident(inner) => {
                    ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                    ::proto_rs::encoding::encode_varint(inner.len() as u64, buf);
                    // Encode length-delimited wrapper
                    let len = inner.len();
                    ::proto_rs::encoding::encode_varint(len as u64, buf);
                    buf.put_slice(inner);
                }
            });

            decode_arms.push(quote! {
                #tag => {
                    let outer_len = ::proto_rs::encoding::decode_varint(buf)?;
                    if outer_len > buf.remaining() as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }

                    let mut temp = Vec::new();
                    ::proto_rs::encoding::bytes::merge(wire_type, &mut temp, buf, ctx)?;
                    *self = #name::#variant_ident(temp);
                    Ok(())
                }
            });

            encoded_len_arms.push(quote! {
                #name::#variant_ident(inner) => {
                    let len = inner.len();
                    ::proto_rs::encoding::key_len(#tag) +
                    ::proto_rs::encoding::encoded_len_varint(len as u64) +
                    len
                }
            });
        } else {
            // Vec<T> where T is a primitive or message
            let inner_ty = vec_inner_type(field_ty).unwrap();
            let encoding_type = get_encoding_type(&parsed);

            if is_complex_type(&inner_ty) {
                // Vec<Message>
                encode_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        // Encode as length-delimited list of messages
                        let mut msg_buf = Vec::new();
                        for item in inner {
                            ::proto_rs::encoding::message::encode(1, item, &mut msg_buf);
                        }
                        ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                        ::proto_rs::encoding::encode_varint(msg_buf.len() as u64, buf);
                        buf.put_slice(&msg_buf);
                    }
                });

                decode_arms.push(quote! {
                    #tag => {
                        let len = ::proto_rs::encoding::decode_varint(buf)?;
                        if len > buf.remaining() as u64 {
                            return Err(::proto_rs::DecodeError::new("buffer underflow"));
                        }

                        let mut temp = Vec::new();
                        let remaining = buf.remaining();
                        let limit = remaining - len as usize;

                        while buf.remaining() > limit {
                            let msg_len = ::proto_rs::encoding::decode_varint(buf)?;
                            if msg_len > buf.remaining() as u64 {
                                return Err(::proto_rs::DecodeError::new("buffer underflow"));
                            }
                            let mut msg_buf = buf.copy_to_bytes(msg_len as usize);
                            let msg = ::proto_rs::ProtoExt::decode(&mut msg_buf)?;
                            temp.push(msg);
                        }

                        *self = #name::#variant_ident(temp);
                        Ok(())
                    }
                });

                encoded_len_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        let mut total = 0;
                        for item in inner {
                            total += ::proto_rs::ProtoExt::encoded_len(item);
                        }
                        ::proto_rs::encoding::key_len(#tag) +
                        ::proto_rs::encoding::encoded_len_varint(total as u64) +
                        total
                    }
                });
            } else if needs_into_conversion(&inner_ty) {
                // Vec<u16> etc - needs conversion
                let target_type = get_proto_rust_type(&inner_ty);

                encode_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        let converted: Vec<#target_type> = inner.iter().map(|v| (*v).into()).collect();

                        // Encode as packed repeated or length-delimited
                        let mut packed_buf = Vec::new();
                        ::proto_rs::encoding::#encoding_type::encode_repeated(1, &converted, &mut packed_buf);

                        ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                        ::proto_rs::encoding::encode_varint(packed_buf.len() as u64, buf);
                        buf.put_slice(&packed_buf);
                    }
                });

                decode_arms.push(quote! {
                    #tag => {
                        let len = ::proto_rs::encoding::decode_varint(buf)?;
                        if len > buf.remaining() as u64 {
                            return Err(::proto_rs::DecodeError::new("buffer underflow"));
                        }

                        let mut temp_converted: Vec<#target_type> = Vec::new();
                        let mut limited_buf = buf.take(len as usize);

                        while limited_buf.has_remaining() {
                            let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(&mut limited_buf)?;
                            ::proto_rs::encoding::#encoding_type::merge_repeated(field_wire_type, &mut temp_converted, &mut limited_buf, ctx)?;
                        }

                        let temp: Vec<#inner_ty> = temp_converted.iter()
                            .map(|v| (*v).try_into())
                            .collect::<Result<_, _>>()
                            .map_err(|_| ::proto_rs::DecodeError::new("Type conversion error"))?;

                        *self = #name::#variant_ident(temp);
                        Ok(())
                    }
                });

                encoded_len_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        let converted: Vec<#target_type> = inner.iter().map(|v| (*v).into()).collect();
                        let packed_len = ::proto_rs::encoding::#encoding_type::encoded_len_repeated(1, &converted);
                        ::proto_rs::encoding::key_len(#tag) +
                        ::proto_rs::encoding::encoded_len_varint(packed_len as u64) +
                        packed_len
                    }
                });
            } else {
                // Vec<u32>, Vec<i32>, etc - no conversion needed
                encode_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        let mut packed_buf = Vec::new();
                        ::proto_rs::encoding::#encoding_type::encode_repeated(1, inner, &mut packed_buf);

                        ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                        ::proto_rs::encoding::encode_varint(packed_buf.len() as u64, buf);
                        buf.put_slice(&packed_buf);
                    }
                });

                decode_arms.push(quote! {
                    #tag => {
                        let len = ::proto_rs::encoding::decode_varint(buf)?;
                        if len > buf.remaining() as u64 {
                            return Err(::proto_rs::DecodeError::new("buffer underflow"));
                        }

                        let mut temp = Vec::new();
                        let mut limited_buf = buf.take(len as usize);

                        while limited_buf.has_remaining() {
                            let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(&mut limited_buf)?;
                            ::proto_rs::encoding::#encoding_type::merge_repeated(field_wire_type, &mut temp, &mut limited_buf, ctx)?;
                        }

                        *self = #name::#variant_ident(temp);
                        Ok(())
                    }
                });

                encoded_len_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        let packed_len = ::proto_rs::encoding::#encoding_type::encoded_len_repeated(1, inner);
                        ::proto_rs::encoding::key_len(#tag) +
                        ::proto_rs::encoding::encoded_len_varint(packed_len as u64) +
                        packed_len
                    }
                });
            }
        }
    } else {
        // Not a Vec - treat as message
        encode_arms.push(quote! {
            #name::#variant_ident(inner) => {
                ::proto_rs::encoding::message::encode(#tag, inner, buf);
            }
        });

        decode_arms.push(quote! {
            #tag => {
                let mut temp = ::proto_rs::ProtoExt::proto_default();
                ::proto_rs::encoding::message::merge(wire_type, &mut temp, buf, ctx)?;
                *self = #name::#variant_ident(temp);
                Ok(())
            }
        });

        encoded_len_arms.push(quote! {
            #name::#variant_ident(inner) => {
                ::proto_rs::encoding::message::encoded_len(#tag, inner)
            }
        });
    }
}

fn generate_named_variant_arms(
    name: &syn::Ident,
    variant_ident: &syn::Ident,
    tag: u32,
    fields_named: &syn::FieldsNamed,
    encode_arms: &mut Vec<TokenStream>,
    decode_arms: &mut Vec<TokenStream>,
    encoded_len_arms: &mut Vec<TokenStream>,
) {
    // Named fields - encode as nested message
    let field_bindings: Vec<_> = fields_named
        .named
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            quote! { #ident }
        })
        .collect();

    let field_encodes: Vec<_> = fields_named
        .named
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let ident = f.ident.as_ref().unwrap();
            let field_tag = (i + 1) as u32;
            quote! {
                ::proto_rs::encoding::message::encode(#field_tag, #ident, buf);
            }
        })
        .collect();

    encode_arms.push(quote! {
        #name::#variant_ident { #(#field_bindings),* } => {
            let msg_len = 0 #(+ ::proto_rs::ProtoExt::encoded_len(#field_bindings))*;

            ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
            ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
            #(#field_encodes)*
        }
    });

    let field_decodes: Vec<_> = fields_named
        .named
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let ident = f.ident.as_ref().unwrap();
            let field_tag = (i + 1) as u32;
            quote! {
                #field_tag => {
                    ::proto_rs::encoding::message::merge(wire_type, &mut #ident, buf, ctx)?;
                }
            }
        })
        .collect();

    let field_defaults: Vec<_> = fields_named
        .named
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            quote! { let mut #ident = <#ty as ::proto_rs::ProtoExt>::proto_default(); }
        })
        .collect();

    decode_arms.push(quote! {
        #tag => {
            let len = ::proto_rs::encoding::decode_varint(buf)?;
            let remaining = buf.remaining();
            if len > remaining as u64 {
                return Err(::proto_rs::DecodeError::new("buffer underflow"));
            }

            let limit = remaining - len as usize;
            #(#field_defaults)*

            while buf.remaining() > limit {
                let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                match field_tag {
                    #(#field_decodes)*
                    _ => ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, ctx)?,
                }
            }

            *self = #name::#variant_ident { #(#field_bindings),* };
            Ok(())
        }
    });

    encoded_len_arms.push(quote! {
        #name::#variant_ident { #(#field_bindings),* } => {
            let msg_len = 0 #(+ ::proto_rs::ProtoExt::encoded_len(#field_bindings))*;
            ::proto_rs::encoding::key_len(#tag) +
            ::proto_rs::encoding::encoded_len_varint(msg_len as u64) +
            msg_len
        }
    });
}

fn get_encoding_type(parsed: &crate::utils::ParsedFieldType) -> syn::Ident {
    let type_str = match parsed.proto_type.as_str() {
        "uint32" => "uint32",
        "uint64" => "uint64",
        "int32" => "int32",
        "int64" => "int64",
        "float" => "float",
        "double" => "double",
        "bool" => "bool",
        "string" => "string",
        "bytes" => "bytes",
        _ => "message",
    };
    syn::Ident::new(type_str, proc_macro2::Span::call_site())
}
