//! Unified field handling for structs and enum variants.
//!
//! This module centralises the encode/decode/length generation logic so the
//! struct and enum handlers only need to forward `syn::Field` information. The
//! implementation intentionally mirrors prost's `Message` semantics to ensure we
//! stay 100% wire compatible with the canonical protobuf encoding.

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Field;
use syn::Ident;
use syn::Index;
use syn::Type;

use crate::utils::ParsedFieldType;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::type_info::is_bytes_array;
use crate::utils::type_info::is_bytes_vec;

// ---------------------------------------------------------------------------
// Field access abstraction (named field, tuple index)

#[derive(Clone)]
pub enum FieldAccess {
    Named(Ident),
    Tuple(Index),
}

impl FieldAccess {
    pub fn to_tokens(&self) -> TokenStream {
        match self {
            FieldAccess::Named(id) => quote! { #id },
            FieldAccess::Tuple(ix) => quote! { #ix },
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry points used by struct/enum handlers

pub fn generate_field_encode(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    let ty = &field.ty;
    let parsed = parse_field_type(ty);
    let access_tokens = access.to_tokens();

    if cfg.skip {
        return quote! {};
    }

    // Custom conversion via #[proto(into = "Type")] or #[proto(into_fn = "path")]
    if let Some(into_ty) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty).expect("invalid into type");
        let conv_fn = cfg.into_fn.as_deref().map(|f| format_ident!("{}", f));

        let value = if let Some(fun) = conv_fn {
            quote! { #fun(&self.#access_tokens) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from(self.#access_tokens.clone()) }
        };

        return encode_scalar_value(&value, tag, &parse_field_type(&into_ty));
    }

    if cfg.is_rust_enum || cfg.is_proto_enum {
        return encode_enum(&access_tokens, tag);
    }

    if let Type::Array(array) = ty {
        return encode_array(&access_tokens, tag, array);
    }

    if parsed.is_repeated {
        return encode_repeated(&access_tokens, tag, &parsed);
    }

    if parsed.is_option {
        return encode_option(&access_tokens, tag, &parsed);
    }

    if parsed.is_message_like || cfg.is_message {
        return encode_message(&access_tokens, tag);
    }

    encode_scalar(&access_tokens, tag, &parsed)
}

pub fn generate_field_decode(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    let ty = &field.ty;
    let parsed = parse_field_type(ty);
    let access_tokens = access.to_tokens();

    if cfg.skip {
        return quote! { /* skipped during decode */ };
    }

    if let Some(from_ty) = &cfg.from_type {
        let from_ty: Type = syn::parse_str(from_ty).expect("invalid from type");
        let conv_fn = cfg.from_fn.as_deref().map(|f| format_ident!("{}", f));
        let field_ty = ty.clone();

        let assign_expr = if let Some(fun) = conv_fn {
            quote! { #fun(__tmp) }
        } else {
            quote! { <#field_ty as ::core::convert::From<#from_ty>>::from(__tmp) }
        };

        return quote! {
            if #tag == tag {
                let mut __tmp: #from_ty = <#from_ty as ::proto_rs::ProtoExt>::proto_default();
                ::proto_rs::ProtoExt::merge_field(&mut __tmp, #tag, wire_type, buf, ctx.clone())?;
                self.#access_tokens = #assign_expr;
            }
        };
    }

    if cfg.is_rust_enum || cfg.is_proto_enum {
        return decode_enum(&access_tokens, tag, ty);
    }

    if let Type::Array(array) = ty {
        return decode_array(&access_tokens, tag, array);
    }

    if parsed.is_repeated {
        return decode_repeated(&access_tokens, tag, &parsed);
    }

    if parsed.is_option {
        return decode_option(&access_tokens, tag, &parsed);
    }

    if parsed.is_message_like {
        return decode_message(&access_tokens, tag);
    }

    decode_scalar(&access_tokens, tag, &parsed)
}

pub fn generate_field_encoded_len(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    if cfg.skip {
        return quote! { 0 };
    }

    let ty = &field.ty;
    let parsed = parse_field_type(ty);
    let access_tokens = access.to_tokens();

    if let Some(into_ty) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty).expect("invalid into type");
        let conv_fn = cfg.into_fn.as_deref().map(|f| format_ident!("{}", f));

        let value = if let Some(fun) = conv_fn {
            quote! { #fun(&self.#access_tokens) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from(self.#access_tokens.clone()) }
        };

        return encoded_len_scalar_value(&value, tag, &parse_field_type(&into_ty));
    }

    if cfg.is_rust_enum || cfg.is_proto_enum {
        return encoded_len_enum(&access_tokens, tag);
    }

    if let Type::Array(array) = ty {
        return encoded_len_array(&access_tokens, tag, array);
    }

    if parsed.is_repeated {
        return encoded_len_repeated(&access_tokens, tag, &parsed);
    }

    if parsed.is_option {
        return encoded_len_option(&access_tokens, tag, &parsed);
    }

    if parsed.is_message_like {
        return encoded_len_message(&access_tokens, tag);
    }

    encoded_len_scalar(&access_tokens, tag, &parsed)
}

// ---------------------------------------------------------------------------
// Scalar helpers

fn scalar_codec(parsed: &ParsedFieldType) -> Option<Ident> {
    match parsed.proto_type.as_str() {
        "message" => None,
        other => Some(Ident::new(other, Span::call_site())),
    }
}

fn needs_numeric_widening(parsed: &ParsedFieldType) -> bool {
    if !parsed.is_numeric_scalar {
        return false;
    }

    let elem = &parsed.elem_type;
    let proto = &parsed.proto_rust_type;
    quote!(#elem).to_string() != quote!(#proto).to_string()
}

fn decode_numeric_with_widening(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, codec: &Ident) -> TokenStream {
    let proto_ty = &parsed.proto_rust_type;
    let target_ty = &parsed.elem_type;

    quote! {
        if #tag == tag {
            let mut __tmp: #proto_ty = ::proto_rs::ProtoExt::proto_default();
            ::proto_rs::encoding::#codec::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
            self.#access = <#target_ty as ::core::convert::TryFrom<#proto_ty>>::try_from(__tmp)
                .map_err(|_| ::proto_rs::DecodeError::new("numeric conversion failed"))?;
        }
    }
}

fn encode_scalar(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if is_bytes_vec(&parsed.rust_type) {
        return quote! {
            if !self.#access.is_empty() {
                ::proto_rs::encoding::#codec::encode(#tag, &self.#access, buf);
            }
        };
    }

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            {
                let __value: #proto_ty = self.#access as #proto_ty;
                ::proto_rs::encoding::#codec::encode(#tag, &__value, buf);
            }
        }
    } else {
        quote! {
            ::proto_rs::encoding::#codec::encode(#tag, &self.#access, buf);
        }
    }
}

fn encode_scalar_value(value: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            {
                let __value: #proto_ty = (#value) as #proto_ty;
                ::proto_rs::encoding::#codec::encode(#tag, &__value, buf);
            }
        }
    } else {
        quote! {
            ::proto_rs::encoding::#codec::encode(#tag, &#value, buf);
        }
    }
}

fn decode_scalar(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if needs_numeric_widening(parsed) {
        decode_numeric_with_widening(access, tag, parsed, &codec)
    } else {
        quote! {
            if #tag == tag {
                ::proto_rs::encoding::#codec::merge(wire_type, &mut self.#access, buf, ctx.clone())?;
            }
        }
    }
}

fn encoded_len_scalar(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! { 0 };
    };

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            {
                let __value: #proto_ty = self.#access as #proto_ty;
                ::proto_rs::encoding::#codec::encoded_len(#tag, &__value)
            }
        }
    } else {
        quote! { ::proto_rs::encoding::#codec::encoded_len(#tag, &self.#access) }
    }
}

fn encoded_len_scalar_value(value: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! { 0 };
    };

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            {
                let __value: #proto_ty = (#value) as #proto_ty;
                ::proto_rs::encoding::#codec::encoded_len(#tag, &__value)
            }
        }
    } else {
        quote! { ::proto_rs::encoding::#codec::encoded_len(#tag, &#value) }
    }
}

// ---------------------------------------------------------------------------
// Message helpers

fn encode_message(access: &TokenStream, tag: u32) -> TokenStream {
    quote! {
        if ::proto_rs::ProtoExt::encoded_len(&self.#access) != 0 {
            ::proto_rs::encoding::message::encode(#tag, &self.#access, buf);
        }
    }
}

fn decode_message(access: &TokenStream, tag: u32) -> TokenStream {
    quote! {
        if #tag == tag {
            ::proto_rs::encoding::message::merge(wire_type, &mut self.#access, buf, ctx.clone())?;
        }
    }
}

fn encoded_len_message(access: &TokenStream, tag: u32) -> TokenStream {
    quote! { ::proto_rs::encoding::message::encoded_len(#tag, &self.#access) }
}

// ---------------------------------------------------------------------------
// Enum helpers (repr(i32))

fn encode_enum(access: &TokenStream, tag: u32) -> TokenStream {
    quote! {
        let __value: i32 = self.#access as i32;
        if __value != 0 {
            ::proto_rs::encoding::int32::encode(#tag, &__value, buf);
        }
    }
}

fn decode_enum(access: &TokenStream, tag: u32, enum_ty: &Type) -> TokenStream {
    quote! {
        if #tag == tag {
            let mut __tmp: i32 = 0;
            ::proto_rs::encoding::int32::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
            self.#access = <#enum_ty as ::core::convert::TryFrom<i32>>::try_from(__tmp)?;
        }
    }
}

fn encoded_len_enum(access: &TokenStream, tag: u32) -> TokenStream {
    quote! {
        if (self.#access as i32) != 0 {
            ::proto_rs::encoding::int32::encoded_len(#tag, &(self.#access as i32))
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Array helpers (fixed length, no allocations)

fn encode_array(access: &TokenStream, tag: u32, array: &syn::TypeArray) -> TokenStream {
    let elem_ty = &*array.elem;
    let elem_parsed = parse_field_type(elem_ty);

    if is_bytes_array(&Type::Array(array.clone())) {
        return quote! {
            if self.#access.iter().any(|&b| b != 0u8) {
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint(self.#access.len() as u64, buf);
                buf.put_slice(&self.#access);
            }
        };
    }

    if elem_parsed.is_message_like {
        return quote! {
            for __value in self.#access.iter() {
                ::proto_rs::encoding::message::encode(#tag, __value, buf);
            }
        };
    }

    let Some(codec) = scalar_codec(&elem_parsed) else {
        return quote! {};
    };

    if needs_numeric_widening(&elem_parsed) {
        let proto_ty = &elem_parsed.proto_rust_type;
        quote! {
            for __value in self.#access.iter() {
                let __converted: #proto_ty = (*__value) as #proto_ty;
                ::proto_rs::encoding::#codec::encode(#tag, &__converted, buf);
            }
        }
    } else {
        quote! {
            for __value in self.#access.iter() {
                ::proto_rs::encoding::#codec::encode(#tag, __value, buf);
            }
        }
    }
}

fn decode_array(access: &TokenStream, tag: u32, array: &syn::TypeArray) -> TokenStream {
    let elem_ty = &*array.elem;
    let elem_parsed = parse_field_type(elem_ty);

    if elem_parsed.is_message_like {
        return quote! {
            if #tag == tag {
                let mut __i = 0usize;
                if __i >= self.#access.len() {
                    return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                }
                let mut __tmp: #elem_ty = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
                ::proto_rs::encoding::message::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                self.#access[__i] = __tmp;
                __i += 1;
            }
        };
    }

    let Some(codec) = scalar_codec(&elem_parsed) else {
        return quote! {};
    };

    let wire = scalar_wire_type(&elem_parsed);
    let target_ty = &elem_parsed.elem_type;

    if needs_numeric_widening(&elem_parsed) {
        let proto_ty = &elem_parsed.proto_rust_type;
        quote! {
            if #tag == tag {
                let mut __i = 0usize;
                match wire_type {
                    ::proto_rs::encoding::WireType::LengthDelimited => {
                        let __len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                        let mut __limited = buf.take(__len);
                        while __limited.has_remaining() {
                            if __i >= self.#access.len() {
                                return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                            }
                            let mut __tmp: #proto_ty = ::proto_rs::ProtoExt::proto_default();
                            ::proto_rs::encoding::#codec::merge(#wire, &mut __tmp, &mut __limited, ctx.clone())?;
                            self.#access[__i] = <#target_ty as ::core::convert::TryFrom<#proto_ty>>::try_from(__tmp)
                                .map_err(|_| ::proto_rs::DecodeError::new("numeric conversion failed"))?;
                            __i += 1;
                        }
                    }
                    _ => {
                        if __i >= self.#access.len() {
                            return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                        }
                        let mut __tmp: #proto_ty = ::proto_rs::ProtoExt::proto_default();
                        ::proto_rs::encoding::#codec::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                        self.#access[__i] = <#target_ty as ::core::convert::TryFrom<#proto_ty>>::try_from(__tmp)
                            .map_err(|_| ::proto_rs::DecodeError::new("numeric conversion failed"))?;
                    }
                }
            }
        }
    } else {
        quote! {
            if #tag == tag {
                let mut __i = 0usize;
                match wire_type {
                    ::proto_rs::encoding::WireType::LengthDelimited => {
                        let __len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                        let mut __limited = buf.take(__len);
                        while __limited.has_remaining() {
                            if __i >= self.#access.len() {
                                return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                            }
                            let mut __tmp: #elem_ty = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
                            ::proto_rs::encoding::#codec::merge(#wire, &mut __tmp, &mut __limited, ctx.clone())?;
                            self.#access[__i] = __tmp;
                            __i += 1;
                        }
                    }
                    _ => {
                        if __i >= self.#access.len() {
                            return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                        }
                        let mut __tmp: #elem_ty = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
                        ::proto_rs::encoding::#codec::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                        self.#access[__i] = __tmp;
                    }
                }
            }
        }
    }
}

fn encoded_len_array(access: &TokenStream, tag: u32, array: &syn::TypeArray) -> TokenStream {
    if is_bytes_array(&Type::Array(array.clone())) {
        return quote! {
            if self.#access.iter().all(|&b| b == 0u8) {
                0
            } else {
                let l = self.#access.len();
                ::proto_rs::encoding::key_len(#tag)
                    + ::proto_rs::encoding::encoded_len_varint(l as u64)
                    + l
            }
        };
    }

    let elem_ty = &*array.elem;
    let elem_parsed = parse_field_type(elem_ty);
    let Some(codec) = scalar_codec(&elem_parsed) else {
        return quote! { 0 };
    };

    if needs_numeric_widening(&elem_parsed) {
        let proto_ty = &elem_parsed.proto_rust_type;
        quote! {
            {
                let mut __total = 0usize;
                for __value in self.#access.iter() {
                    let __converted: #proto_ty = (*__value) as #proto_ty;
                    __total += ::proto_rs::encoding::#codec::encoded_len(#tag, &__converted);
                }
                __total
            }
        }
    } else {
        quote! {
            {
                let mut __total = 0usize;
                for __value in self.#access.iter() {
                    __total += ::proto_rs::encoding::#codec::encoded_len(#tag, __value);
                }
                __total
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Repeated helpers (Vec<T>)

fn encode_repeated(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_message_like {
        return quote! { ::proto_rs::encoding::message::encode_repeated(#tag, &self.#access, buf); };
    }

    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if parsed.is_numeric_scalar {
        if needs_numeric_widening(parsed) {
            let proto_ty = &parsed.proto_rust_type;
            quote! {
                if !self.#access.is_empty() {
                    let mut __tmp: ::std::vec::Vec<#proto_ty> = ::std::vec::Vec::with_capacity(self.#access.len());
                    for value in self.#access.iter() {
                        __tmp.push((*value) as #proto_ty);
                    }
                    ::proto_rs::encoding::#codec::encode_packed(#tag, &__tmp, buf);
                }
            }
        } else {
            quote! { ::proto_rs::encoding::#codec::encode_packed(#tag, &self.#access, buf); }
        }
    } else {
        quote! { ::proto_rs::encoding::#codec::encode_repeated(#tag, &self.#access, buf); }
    }
}

fn decode_repeated(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_message_like {
        return quote! {
            if #tag == tag {
                ::proto_rs::encoding::message::merge_repeated(wire_type, &mut self.#access, buf, ctx.clone())?;
            }
        };
    }

    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if parsed.is_numeric_scalar && needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        let elem_ty = &parsed.elem_type;
        quote! {
            if #tag == tag {
                let mut __tmp: ::std::vec::Vec<#proto_ty> = ::std::vec::Vec::new();
                ::proto_rs::encoding::#codec::merge_repeated(wire_type, &mut __tmp, buf, ctx.clone())?;
                for value in __tmp {
                    self.#access.push(<#elem_ty as ::core::convert::TryFrom<#proto_ty>>::try_from(value)
                        .map_err(|_| ::proto_rs::DecodeError::new("numeric conversion failed"))?);
                }
            }
        }
    } else {
        quote! {
            if #tag == tag {
                ::proto_rs::encoding::#codec::merge_repeated(wire_type, &mut self.#access, buf, ctx.clone())?;
            }
        }
    }
}

fn encoded_len_repeated(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_message_like {
        return quote! { ::proto_rs::encoding::message::encoded_len_repeated(#tag, &self.#access) };
    }

    let Some(codec) = scalar_codec(parsed) else {
        return quote! { 0 };
    };

    if parsed.is_numeric_scalar {
        if needs_numeric_widening(parsed) {
            let proto_ty = &parsed.proto_rust_type;
            quote! {
                {
                    let mut __tmp: ::std::vec::Vec<#proto_ty> = ::std::vec::Vec::with_capacity(self.#access.len());
                    for value in self.#access.iter() {
                        __tmp.push((*value) as #proto_ty);
                    }
                    ::proto_rs::encoding::#codec::encoded_len_packed(#tag, &__tmp)
                }
            }
        } else {
            quote! { ::proto_rs::encoding::#codec::encoded_len_packed(#tag, &self.#access) }
        }
    } else {
        quote! { ::proto_rs::encoding::#codec::encoded_len_repeated(#tag, &self.#access) }
    }
}

// ---------------------------------------------------------------------------
// Option helpers

fn encode_option(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_message_like {
        return quote! {
            if let Some(value) = &self.#access {
                ::proto_rs::encoding::message::encode(#tag, value, buf);
            }
        };
    }

    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            if let Some(value) = &self.#access {
                let __value: #proto_ty = (*value) as #proto_ty;
                ::proto_rs::encoding::#codec::encode(#tag, &__value, buf);
            }
        }
    } else {
        quote! {
            if let Some(value) = &self.#access {
                ::proto_rs::encoding::#codec::encode(#tag, value, buf);
            }
        }
    }
}

fn decode_option(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let inner_ty = &parsed.elem_type;

    if parsed.is_message_like {
        return quote! {
            if #tag == tag {
                let mut __tmp: #inner_ty = <#inner_ty as ::proto_rs::ProtoExt>::proto_default();
                ::proto_rs::encoding::message::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                self.#access = Some(__tmp);
            }
        };
    }

    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            if #tag == tag {
                let mut __tmp: #proto_ty = ::proto_rs::ProtoExt::proto_default();
                ::proto_rs::encoding::#codec::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                let __converted = <#inner_ty as ::core::convert::TryFrom<#proto_ty>>::try_from(__tmp)
                    .map_err(|_| ::proto_rs::DecodeError::new("numeric conversion failed"))?;
                self.#access = Some(__converted);
            }
        }
    } else {
        quote! {
            if #tag == tag {
                let mut __tmp: #inner_ty = <#inner_ty as ::proto_rs::ProtoExt>::proto_default();
                ::proto_rs::encoding::#codec::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                self.#access = Some(__tmp);
            }
        }
    }
}

fn encoded_len_option(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_message_like {
        return quote! {
            self.#access
                .as_ref()
                .map_or(0, |value| ::proto_rs::encoding::message::encoded_len(#tag, value))
        };
    }

    let Some(codec) = scalar_codec(parsed) else {
        return quote! { 0 };
    };

    if needs_numeric_widening(parsed) {
        let proto_ty = &parsed.proto_rust_type;
        quote! {
            self.#access.as_ref().map_or(0, |value| {
                let __value: #proto_ty = (*value) as #proto_ty;
                ::proto_rs::encoding::#codec::encoded_len(#tag, &__value)
            })
        }
    } else {
        quote! {
            self.#access
                .as_ref()
                .map_or(0, |value| ::proto_rs::encoding::#codec::encoded_len(#tag, value))
        }
    }
}

// ---------------------------------------------------------------------------
// Utility helpers

fn scalar_wire_type(parsed: &ParsedFieldType) -> TokenStream {
    match parsed.proto_type.as_str() {
        "float" | "fixed32" | "sfixed32" => quote! { ::proto_rs::encoding::WireType::ThirtyTwoBit },
        "double" | "fixed64" | "sfixed64" => quote! { ::proto_rs::encoding::WireType::SixtyFourBit },
        "string" | "bytes" => quote! { ::proto_rs::encoding::WireType::LengthDelimited },
        _ => quote! { ::proto_rs::encoding::WireType::Varint },
    }
}
