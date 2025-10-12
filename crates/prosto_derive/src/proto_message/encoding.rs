// encoding.rs
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::utils::FieldConfig;
use crate::utils::ParsedFieldType;
use crate::utils::extract_option_inner_type;
use crate::utils::is_bytes_array;
use crate::utils::is_complex_type;
use crate::utils::parse_field_type;

/// Generate encoding logic for a single field
pub fn generate_field_encode(field_name: &syn::Ident, field_tag: usize, field_ty: &Type, field_config: &FieldConfig) -> TokenStream {
    if field_config.skip {
        return quote! {}; // Skip fields are not encoded
    }

    let tag = field_tag as u32;
    let parsed = parse_field_type(field_ty);

    // Handle custom conversions
    if let Some(into_fn) = &field_config.into_fn {
        let into_fn_ident: syn::Ident = syn::parse_str(into_fn).unwrap();
        let into_type: Type = syn::parse_str(field_config.into_type.as_ref().unwrap()).unwrap();
        return generate_custom_encode(field_name, tag, &into_type, &into_fn_ident);
    }

    // Handle rust_enum attribute
    if field_config.is_rust_enum {
        return generate_rust_enum_encode(field_name, tag, &parsed);
    }

    // Handle proto enum attribute
    if field_config.is_proto_enum {
        return generate_proto_enum_encode(field_name, tag, &parsed);
    }

    // Handle arrays
    if let Type::Array(type_array) = field_ty {
        let elem_ty = &*type_array.elem;

        // [u8; N] -> bytes
        if is_bytes_array(field_ty) {
            return quote! {
                if !self.#field_name.is_empty() {
                    ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                    ::proto_rs::encoding::encode_varint(self.#field_name.len() as u64, buf);
                    buf.put_slice(&self.#field_name[..]);
                }
            };
        }

        // Arrays of message types - iterate and encode each
        if field_config.is_message || is_complex_type(elem_ty) {
            return quote! {
                for item in &self.#field_name {
                    ::proto_rs::encoding::message::encode(#tag, item, buf);
                }
            };
        }

        // Arrays with type conversion
        if needs_conversion(elem_ty) {
            let target_type = get_conversion_target(elem_ty);
            let encoding_type = get_prost_encoding_type(&parsed);
            return quote! {
                {
                    let converted: Vec<#target_type> = self.#field_name.iter().map(|v| (*v).into()).collect();
                    ::proto_rs::encoding::#encoding_type::encode_repeated(#tag, &converted, buf);
                }
            };
        }

        // Arrays without conversion
        let encoding_type = get_prost_encoding_type(&parsed);
        return quote! {
            {
                let vec: Vec<_> = self.#field_name.to_vec();
                ::proto_rs::encoding::#encoding_type::encode_repeated(#tag, &vec, buf);
            }
        };
    }

    // Standard encoding based on type
    if parsed.is_repeated {
        generate_repeated_encode(field_name, tag, field_ty, field_config, &parsed)
    } else if parsed.is_option {
        generate_optional_encode(field_name, tag, field_ty, field_config, &parsed)
    } else if parsed.is_message_like {
        generate_message_encode(field_name, tag)
    } else {
        generate_primitive_encode(field_name, tag, field_ty)
    }
}

fn generate_custom_encode(field_name: &syn::Ident, tag: u32, into_type: &Type, into_fn: &syn::Ident) -> TokenStream {
    let parsed = parse_field_type(into_type);
    let encoding_type = get_prost_encoding_type(&parsed);

    quote! {
        {
            let converted = #into_fn(&self.#field_name);
            if converted != Default::default() {
                ::proto_rs::encoding::#encoding_type::encode(#tag, &converted, buf);
            }
        }
    }
}

fn generate_rust_enum_encode(field_name: &syn::Ident, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_option {
        quote! {
            if let Some(ref val) = self.#field_name {
                let val_i32 = (*val) as i32;
                if val_i32 != 0 {
                    ::proto_rs::encoding::int32::encode(#tag, &val_i32, buf);
                }
            }
        }
    } else if parsed.is_repeated {
        quote! {
            {
                let values: Vec<i32> = self.#field_name.iter().map(|v| *v as i32).collect();
                ::proto_rs::encoding::int32::encode_repeated(#tag, &values, buf);
            }
        }
    } else {
        quote! {
            {
                let val_i32 = self.#field_name as i32;
                if val_i32 != 0 {
                    ::proto_rs::encoding::int32::encode(#tag, &val_i32, buf);
                }
            }
        }
    }
}

fn generate_proto_enum_encode(field_name: &syn::Ident, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_option {
        quote! {
            if let Some(val) = self.#field_name {
                let val_i32 = val as i32;
                if val_i32 != 0 {
                    ::proto_rs::encoding::int32::encode(#tag, &val_i32, buf);
                }
            }
        }
    } else if parsed.is_repeated {
        quote! {
            {
                let values: Vec<i32> = self.#field_name.iter().map(|v| *v as i32).collect();
                ::proto_rs::encoding::int32::encode_repeated(#tag, &values, buf);
            }
        }
    } else {
        quote! {
            {
                let val_i32 = self.#field_name as i32;
                if val_i32 != 0 {
                    ::proto_rs::encoding::int32::encode(#tag, &val_i32, buf);
                }
            }
        }
    }
}

fn generate_repeated_encode(field_name: &syn::Ident, tag: u32, field_ty: &Type, field_config: &FieldConfig, parsed: &ParsedFieldType) -> TokenStream {
    use crate::utils::type_info::*;
    // Handle arrays before Vec
    if let Type::Array(type_array) = field_ty {
        let elem_ty = &*type_array.elem;

        // [u8; N] -> bytes (encode directly)
        if is_bytes_array(field_ty) {
            return quote! {
                if !self.#field_name.is_empty() {
                    ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                    ::proto_rs::encoding::encode_varint(self.#field_name.len() as u64, buf);
                    buf.put_slice(&self.#field_name[..]);
                }
            };
        }

        // Other arrays with type conversion -> iterate and convert
        if needs_conversion(elem_ty) {
            let target_type = get_conversion_target(elem_ty);
            let encoding_type = get_prost_encoding_type(parsed);
            return quote! {
                {
                    let converted: Vec<#target_type> = self.#field_name.iter().map(|v| (*v).into()).collect();
                    ::proto_rs::encoding::#encoding_type::encode_repeated(#tag, &converted, buf);
                }
            };
        }

        // Other arrays without conversion -> convert to vec
        let encoding_type = get_prost_encoding_type(parsed);
        return quote! {
            {
                let vec: Vec<_> = self.#field_name.to_vec();
                ::proto_rs::encoding::#encoding_type::encode_repeated(#tag, &vec, buf);
            }
        };
    }

    // Vec<u8> is bytes
    if is_bytes_vec(field_ty) {
        return quote! {
            if !self.#field_name.is_empty() {
                ::proto_rs::encoding::bytes::encode(#tag, &self.#field_name, buf);
            }
        };
    }

    // Message types
    if field_config.is_message || parsed.is_message_like {
        return quote! {
            for msg in &self.#field_name {
                ::proto_rs::encoding::message::encode(#tag, msg, buf);
            }
        };
    }

    // Primitive types - use packed encoding
    let encoding_type = get_prost_encoding_type(parsed);

    // Handle type conversions for repeated fields
    if needs_conversion(field_ty) {
        let target_type = get_conversion_target(field_ty);
        quote! {
            {
                let converted: Vec<#target_type> = self.#field_name.iter()
                    .map(|v| (*v).into())
                    .collect();
                ::proto_rs::encoding::#encoding_type::encode_repeated(#tag, &converted, buf);
            }
        }
    } else {
        quote! {
            ::proto_rs::encoding::#encoding_type::encode_repeated(#tag, &self.#field_name, buf);
        }
    }
}

fn generate_optional_encode(field_name: &syn::Ident, tag: u32, field_ty: &Type, field_config: &FieldConfig, parsed: &ParsedFieldType) -> TokenStream {
    if field_config.is_message || parsed.is_message_like {
        return quote! {
            if let Some(ref msg) = self.#field_name {
                ::proto_rs::encoding::message::encode(#tag, msg, buf);
            }
        };
    }

    let encoding_type = get_prost_encoding_type(parsed);

    if needs_conversion(field_ty) {
        quote! {
            if let Some(val) = self.#field_name {
                let converted = val.into();
                ::proto_rs::encoding::#encoding_type::encode(#tag, &converted, buf);
            }
        }
    } else {
        quote! {
            if let Some(ref val) = self.#field_name {
                ::proto_rs::encoding::#encoding_type::encode(#tag, val, buf);
            }
        }
    }
}

fn generate_message_encode(field_name: &syn::Ident, tag: u32) -> TokenStream {
    quote! {
        ::proto_rs::encoding::message::encode(#tag, &self.#field_name, buf);
    }
}

fn generate_primitive_encode(field_name: &syn::Ident, tag: u32, field_ty: &Type) -> TokenStream {
    let parsed = parse_field_type(field_ty);
    let encoding_type = get_prost_encoding_type(&parsed);

    // Handle type conversions (u8 -> u32, etc.)
    if needs_conversion(field_ty) {
        let target_type = get_conversion_target(field_ty);
        quote! {
            {
                let converted: #target_type = self.#field_name.into();
                if converted != 0 {
                    ::proto_rs::encoding::#encoding_type::encode(#tag, &converted, buf);
                }
            }
        }
    } else {
        // Check for default to avoid encoding default values
        match parsed.proto_type.as_str() {
            "string" | "bytes" => quote! {
                if !self.#field_name.is_empty() {
                    ::proto_rs::encoding::#encoding_type::encode(#tag, &self.#field_name, buf);
                }
            },
            "bool" => quote! {
                if self.#field_name {
                    ::proto_rs::encoding::#encoding_type::encode(#tag, &self.#field_name, buf);
                }
            },
            "float" | "double" => quote! {
                if self.#field_name != 0.0 {
                    ::proto_rs::encoding::#encoding_type::encode(#tag, &self.#field_name, buf);
                }
            },
            _ => quote! {
                if self.#field_name != 0 {
                    ::proto_rs::encoding::#encoding_type::encode(#tag, &self.#field_name, buf);
                }
            },
        }
    }
}

/// Generate decoding logic body for a single field (without the match arm pattern)
pub fn generate_field_decode_body(field_name: &syn::Ident, field_ty: &Type, field_config: &FieldConfig) -> TokenStream {
    if field_config.skip {
        return quote! {}; // Skip fields are not decoded
    }

    let parsed = parse_field_type(field_ty);

    // Handle custom conversions
    if let Some(from_fn) = &field_config.from_fn {
        let from_fn_ident: syn::Ident = syn::parse_str(from_fn).unwrap();
        let into_type: Type = syn::parse_str(field_config.into_type.as_ref().unwrap()).unwrap();
        return generate_custom_decode_body(field_name, &into_type, &from_fn_ident);
    }

    // Handle rust_enum attribute
    if field_config.is_rust_enum {
        return generate_rust_enum_decode_body(field_name, &parsed, field_ty);
    }

    // Handle proto enum attribute
    if field_config.is_proto_enum {
        return generate_proto_enum_decode_body(field_name, &parsed, field_ty);
    }

    // Handle arrays
    if let Type::Array(type_array) = field_ty {
        let elem_ty = &*type_array.elem;
        let array_len = if let syn::Expr::Lit(lit) = &type_array.len {
            if let syn::Lit::Int(int_lit) = &lit.lit {
                int_lit.base10_parse::<usize>().unwrap()
            } else {
                panic!("Array length must be integer literal")
            }
        } else {
            panic!("Array length must be literal")
        };

        // [u8; N] -> bytes
        if is_bytes_array(field_ty) {
            return quote! {
                let len = ::proto_rs::encoding::decode_varint(buf)?;
                if len > buf.remaining() as u64 {
                    return Err(::proto_rs::DecodeError::new("buffer underflow"));
                }
                let len = len as usize;
                let mut temp_vec = vec![0u8; len];
                buf.copy_to_slice(&mut temp_vec[..]);
                self.#field_name = temp_vec.as_slice().try_into()
                    .map_err(|_| ::proto_rs::DecodeError::new("Invalid byte array length"))?;
            };
        }

        // Arrays of message types - use MaybeUninit to avoid Default requirement
        if field_config.is_message || is_complex_type(elem_ty) {
            let array_len_lit = &type_array.len;
            return quote! {
                // Safe const initialization
                const UNINIT: ::core::mem::MaybeUninit<#elem_ty> = ::core::mem::MaybeUninit::uninit();
                let mut array: [::core::mem::MaybeUninit<#elem_ty>; #array_len_lit] = [UNINIT; #array_len_lit];
                let mut count = 0usize;

                // Decode first message (current tag)
                {
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    if len > buf.remaining() as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    let mut msg_buf = buf.copy_to_bytes(len as usize);
                    let msg: #elem_ty = ::proto_rs::ProtoExt::decode(&mut msg_buf)?;
                    array[count].write(msg);
                    count += 1;
                }

                // Read ahead for more elements with the same tag
                loop {
                    if !buf.has_remaining() {
                        break;
                    }

                    let chunk = buf.chunk();
                    if chunk.is_empty() {
                        break;
                    }

                    let mut peek_buf = &chunk[..];
                    match ::proto_rs::encoding::decode_key(&mut peek_buf) {
                        Ok((next_tag, _)) if next_tag == tag => {
                            // Same tag - consume the key and decode
                            let _ = ::proto_rs::encoding::decode_key(buf)?;

                            let len = ::proto_rs::encoding::decode_varint(buf)?;
                            if len > buf.remaining() as u64 {
                                return Err(::proto_rs::DecodeError::new("buffer underflow"));
                            }
                            let mut msg_buf = buf.copy_to_bytes(len as usize);
                            let msg: #elem_ty = ::proto_rs::ProtoExt::decode(&mut msg_buf)?;

                            if count >= #array_len_lit {
                                return Err(::proto_rs::DecodeError::new("Array overflow: too many elements"));
                            }
                            array[count].write(msg);
                            count += 1;
                        }
                        _ => break,
                    }
                }

                // Verify we got exactly the right number of elements - FIXED!
                if count != #array_len_lit {
                    return Err(::proto_rs::DecodeError::new(
                        format!("Array size mismatch: expected {}, got {}", #array_len_lit, count)
                    ));
                }

                // Safe to transmute now that array is fully initialized
                self.#field_name = unsafe {
                    ::core::mem::transmute_copy::<[::core::mem::MaybeUninit<#elem_ty>; #array_len_lit], [#elem_ty; #array_len_lit]>(&array)
                };
            };
        }

        // Arrays with type conversion
        if needs_conversion(elem_ty) {
            let target_type = get_conversion_target(elem_ty);
            let encoding_type = get_prost_encoding_type(&parse_field_type(elem_ty));
            return quote! {
                let mut temp_vec: Vec<#target_type> = Vec::new();
                ::proto_rs::encoding::#encoding_type::merge_repeated(wire_type, &mut temp_vec, buf, ctx)?;
                let converted: Vec<_> = temp_vec.iter()
                    .map(|v| (*v).try_into())
                    .collect::<Result<_, _>>()
                    .map_err(|_| ::proto_rs::DecodeError::new("Type conversion error"))?;
                self.#field_name = converted.as_slice().try_into()
                    .map_err(|_| ::proto_rs::DecodeError::new("Invalid array length"))?;
            };
        }

        // Arrays without conversion
        let encoding_type = get_prost_encoding_type(&parse_field_type(elem_ty));
        return quote! {
            let mut temp_vec = Vec::new();
            ::proto_rs::encoding::#encoding_type::merge_repeated(wire_type, &mut temp_vec, buf, ctx)?;
            self.#field_name = temp_vec.as_slice().try_into()
                .map_err(|_| ::proto_rs::DecodeError::new("Invalid array length"))?;
        };
    }

    // Standard decoding based on type
    if parsed.is_repeated {
        generate_repeated_decode_body(field_name, field_ty, field_config, &parsed)
    } else if parsed.is_option {
        generate_optional_decode_body(field_name, field_ty, field_config, &parsed)
    } else if parsed.is_message_like {
        generate_message_decode_body(field_name)
    } else {
        generate_primitive_decode_body(field_name, field_ty)
    }
}

fn generate_custom_decode_body(field_name: &syn::Ident, into_type: &Type, from_fn: &syn::Ident) -> TokenStream {
    let parsed = parse_field_type(into_type);
    let encoding_type = get_prost_encoding_type(&parsed);

    quote! {
        let mut temp = <#into_type as ::proto_rs::ProtoExt>::proto_default();
        ::proto_rs::encoding::#encoding_type::merge(wire_type, &mut temp, buf, ctx)?;
        self.#field_name = #from_fn(temp);
    }
}

fn generate_rust_enum_decode_body(field_name: &syn::Ident, parsed: &ParsedFieldType, field_ty: &Type) -> TokenStream {
    use crate::utils::type_info::extract_option_inner_type;

    let enum_type = if parsed.is_option { extract_option_inner_type(field_ty) } else { field_ty };

    if parsed.is_option {
        quote! {
            let mut temp: i32 = 0;
            ::proto_rs::encoding::int32::merge(wire_type, &mut temp, buf, ctx)?;
            self.#field_name = Some(<#enum_type>::try_from(temp)
                .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?);
        }
    } else if parsed.is_repeated {
        quote! {
            let mut temp_vec: Vec<i32> = Vec::new();
            ::proto_rs::encoding::int32::merge_repeated(wire_type, &mut temp_vec, buf, ctx)?;
            for val in temp_vec {
                self.#field_name.push(<#enum_type>::try_from(val)
                    .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?);
            }
        }
    } else {
        quote! {
            let mut temp: i32 = 0;
            ::proto_rs::encoding::int32::merge(wire_type, &mut temp, buf, ctx)?;
            self.#field_name = <#enum_type>::try_from(temp)
                .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?;
        }
    }
}

fn generate_proto_enum_decode_body(field_name: &syn::Ident, parsed: &ParsedFieldType, field_ty: &Type) -> TokenStream {
    use crate::utils::type_info::extract_option_inner_type;

    let enum_type = if parsed.is_option { extract_option_inner_type(field_ty) } else { field_ty };

    if parsed.is_option {
        quote! {
            let mut temp: i32 = 0;
            ::proto_rs::encoding::int32::merge(wire_type, &mut temp, buf, ctx)?;
            self.#field_name = Some(<#enum_type>::try_from(temp)
                .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?);
        }
    } else if parsed.is_repeated {
        quote! {
            let mut temp_vec: Vec<i32> = Vec::new();
            ::proto_rs::encoding::int32::merge_repeated(wire_type, &mut temp_vec, buf, ctx)?;
            for val in temp_vec {
                self.#field_name.push(<#enum_type>::try_from(val)
                    .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?);
            }
        }
    } else {
        quote! {
            let mut temp: i32 = 0;
            ::proto_rs::encoding::int32::merge(wire_type, &mut temp, buf, ctx)?;
            self.#field_name = <#enum_type>::try_from(temp)
                .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?;
        }
    }
}

fn generate_repeated_decode_body(field_name: &syn::Ident, field_ty: &Type, field_config: &FieldConfig, parsed: &ParsedFieldType) -> TokenStream {
    use crate::utils::type_info::*;

    // Vec<u8> is bytes
    if is_bytes_vec(field_ty) {
        return quote! {
            ::proto_rs::encoding::bytes::merge(wire_type, &mut self.#field_name, buf, ctx)?;
        };
    }

    // Message types - decode without requiring Default
    if field_config.is_message || parsed.is_message_like {
        let elem_ty = vec_inner_type(field_ty).unwrap();
        return quote! {
            // Read length-delimited message
            let len = ::proto_rs::encoding::decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(::proto_rs::DecodeError::new("buffer underflow"));
            }

            // Take exactly len bytes for this message - no allocation, reuse buffer
            let mut msg_buf = buf.copy_to_bytes(len as usize);

            // Decode the message using proto_default
            let msg = <#elem_ty as ::proto_rs::ProtoExt>::decode(&mut msg_buf)?;

            self.#field_name.push(msg);
        };
    }

    // Primitive types with conversion
    let encoding_type = get_prost_encoding_type(parsed);

    if needs_conversion(field_ty) {
        let target_type = get_conversion_target(field_ty);
        quote! {
            let mut temp_vec: Vec<#target_type> = Vec::new();
            ::proto_rs::encoding::#encoding_type::merge_repeated(wire_type, &mut temp_vec, buf, ctx)?;
            for val in temp_vec {
                self.#field_name.push(val.try_into()
                    .map_err(|_| ::proto_rs::DecodeError::new("Type conversion error"))?);
            }
        }
    } else {
        quote! {
            ::proto_rs::encoding::#encoding_type::merge_repeated(wire_type, &mut self.#field_name, buf, ctx)?;
        }
    }
}

fn generate_optional_decode_body(field_name: &syn::Ident, field_ty: &Type, field_config: &FieldConfig, parsed: &ParsedFieldType) -> TokenStream {
    if field_config.is_message || parsed.is_message_like {
        let inner_ty = extract_option_inner_type(field_ty);
        return quote! {
            // Read length-delimited message
            let len = ::proto_rs::encoding::decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(::proto_rs::DecodeError::new("buffer underflow"));
            }

            // Take exactly len bytes for this message
            let mut msg_buf = buf.copy_to_bytes(len as usize);

            // Decode the message using proto_default
            let msg = <#inner_ty as ::proto_rs::ProtoExt>::decode(&mut msg_buf)
                .map_err(|e| ::proto_rs::DecodeError::new(
                    "Failed to decode message"
                ))?;

            self.#field_name = Some(msg);
        };
    }

    let encoding_type = get_prost_encoding_type(parsed);

    if needs_conversion(field_ty) {
        let inner_ty = extract_option_inner_type(field_ty);
        quote! {
            let mut temp = <#inner_ty as ::proto_rs::ProtoExt>::proto_default();
            ::proto_rs::encoding::#encoding_type::merge(wire_type, &mut temp, buf, ctx)?;
            self.#field_name = Some(temp.try_into()
                .map_err(|_| ::proto_rs::DecodeError::new("Type conversion error"))?);
        }
    } else {
        let inner_ty = extract_option_inner_type(field_ty);
        quote! {
            let mut val = <#inner_ty as ::proto_rs::ProtoExt>::proto_default();
            ::proto_rs::encoding::#encoding_type::merge(wire_type, &mut val, buf, ctx)?;
            self.#field_name = Some(val);
        }
    }
}

fn generate_message_decode_body(field_name: &syn::Ident) -> TokenStream {
    quote! {
        ::proto_rs::encoding::message::merge(wire_type, &mut self.#field_name, buf, ctx)?;
    }
}

fn generate_primitive_decode_body(field_name: &syn::Ident, field_ty: &Type) -> TokenStream {
    let parsed = parse_field_type(field_ty);
    let encoding_type = get_prost_encoding_type(&parsed);

    if needs_conversion(field_ty) {
        let target_type = get_conversion_target(field_ty);
        quote! {
            let mut temp = <#target_type as ::proto_rs::ProtoExt>::proto_default();
            ::proto_rs::encoding::#encoding_type::merge(wire_type, &mut temp, buf, ctx)?;
            self.#field_name = temp.try_into()
                .map_err(|_| ::proto_rs::DecodeError::new("Type conversion error"))?;
        }
    } else {
        quote! {
            ::proto_rs::encoding::#encoding_type::merge(wire_type, &mut self.#field_name, buf, ctx)?;
        }
    }
}

/// Get prost encoding type name (e.g., "uint64", "string", "message")
fn get_prost_encoding_type(parsed: &ParsedFieldType) -> syn::Ident {
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
        "message" => "message",
        _ => "message",
    };
    syn::Ident::new(type_str, proc_macro2::Span::call_site())
}

/// Check if type needs conversion (u8 -> u32, u16 -> u32, etc.)
pub fn needs_conversion(ty: &Type) -> bool {
    use crate::utils::type_info::needs_into_conversion;
    needs_into_conversion(ty)
}

/// Get conversion target type
fn get_conversion_target(ty: &Type) -> TokenStream {
    use crate::utils::type_info::get_proto_rust_type;
    get_proto_rust_type(ty)
}
