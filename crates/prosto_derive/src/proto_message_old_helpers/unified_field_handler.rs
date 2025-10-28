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
use syn::GenericArgument;
use syn::Ident;
use syn::Index;
use syn::PathArguments;
use syn::Type;

use crate::utils::FieldConfig;
use crate::utils::MapKind;
use crate::utils::ParsedFieldType;
use crate::utils::is_option_type;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::type_info::is_bytes_array;
use crate::utils::vec_inner_type;

// ---------------------------------------------------------------------------
// Field access abstraction (named field, tuple index)

#[derive(Clone)]
pub enum FieldAccess {
    Named(Ident),
    Tuple(Index),
}

impl FieldAccess {
    pub fn self_tokens(&self) -> TokenStream {
        self.tokens_with_base(quote! { self })
    }

    pub fn tokens_with_base(&self, base: TokenStream) -> TokenStream {
        match self {
            FieldAccess::Named(id) => quote! { #base.#id },
            FieldAccess::Tuple(ix) => quote! { #base.#ix },
        }
    }
}

#[derive(Clone)]
pub struct EncodedLenTokens {
    pub tokens: TokenStream,
    pub uses_access: bool,
}

impl EncodedLenTokens {
    fn new(tokens: TokenStream, uses_access: bool) -> Self {
        Self { tokens, uses_access }
    }
}

// ---------------------------------------------------------------------------
// Public entry points used by struct/enum handlers

pub fn field_default_expr(field: &Field) -> TokenStream {
    let field_ty = &field.ty;

    if is_option_type(field_ty) {
        return quote! { None };
    }

    if vec_inner_type(field_ty).is_some() {
        return quote! { Vec::new() };
    }

    let parsed_ty = parse_field_type(field_ty);
    if parsed_ty.map_kind.is_some() || parsed_ty.set_kind.is_some() {
        return quote! { ::core::default::Default::default() };
    }

    let cfg = parse_field_config(field);
    if cfg.into_type.is_some() || cfg.from_type.is_some() || cfg.into_fn.is_some() || cfg.from_fn.is_some() || cfg.skip {
        quote! { ::core::default::Default::default() }
    } else {
        quote! { <#field_ty as ::proto_rs::ProtoExt>::proto_default() }
    }
}

pub fn generate_field_encode(field: &Field, access: TokenStream, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    let ty = &field.ty;
    let parsed = parse_field_type(ty);

    if cfg.skip {
        return quote! {};
    }

    // Custom conversion via #[proto(into = "Type")] or #[proto(into_fn = "path")]
    if let Some(into_ty) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty).expect("invalid into type");
        let into_parsed = parse_field_type(&into_ty);
        let conv_fn = cfg.into_fn.as_deref().map(|f| format_ident!("{}", f));

        let access_clone = access.clone();
        let value = if let Some(fun) = conv_fn {
            quote! { #fun(&(#access_clone)) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from((#access_clone).clone()) }
        };

        return encode_scalar_value(&value, tag, &into_ty, &into_parsed);
    }

    if (cfg.is_rust_enum || parsed.is_rust_enum || cfg.is_proto_enum) && !parsed.is_option && !parsed.is_repeated {
        return encode_enum(&access, tag, ty);
    }

    if let Some(kind) = parsed.map_kind {
        return encode_map(&access, tag, &parsed, kind);
    }

    if let Type::Array(array) = ty {
        return encode_array(&access, tag, array);
    }

    if parsed.set_kind.is_some() {
        return encode_set(&access, tag, &parsed);
    }

    if parsed.is_repeated {
        return encode_repeated(&access, tag, &parsed);
    }

    if parsed.is_option {
        return encode_option(&access, tag, &parsed, &cfg);
    }

    if cfg.is_message {
        return encode_message(&access, tag, ty);
    }

    encode_scalar(&access, tag, ty, &parsed)
}

pub fn generate_field_decode(field: &Field, access: TokenStream, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    let ty = &field.ty;
    let parsed = parse_field_type(ty);

    if cfg.skip {
        return quote! { /* skipped during decode */ };
    }

    if let Some(from_ty) = cfg.from_type.as_ref().or(cfg.into_type.as_ref()) {
        let from_ty: Type = syn::parse_str(from_ty).expect("invalid from type");
        let conv_fn = cfg.from_fn.as_deref().map(|f| format_ident!("{}", f));
        let field_ty = ty.clone();

        let access_clone = access.clone();
        let assign_expr = if let Some(fun) = conv_fn {
            quote! { #fun(__tmp) }
        } else {
            quote! { <#field_ty as ::core::convert::From<#from_ty>>::from(__tmp) }
        };

        return quote! {
            let mut __tmp: #from_ty = <#from_ty as ::proto_rs::ProtoExt>::proto_default();
            <#from_ty as ::proto_rs::ProtoExt>::merge_singular_field(
                wire_type,
                &mut __tmp,
                buf,
                ctx.clone(),
            )?;
            #access_clone = #assign_expr;
        };
    }

    if (cfg.is_rust_enum || parsed.is_rust_enum || cfg.is_proto_enum) && !parsed.is_option && !parsed.is_repeated {
        return decode_enum(&access, tag, ty);
    }

    if let Some(kind) = parsed.map_kind {
        return decode_map(&access, tag, &parsed, kind);
    }

    if let Type::Array(array) = ty {
        return decode_array(&access, tag, array);
    }

    if parsed.set_kind.is_some() {
        return decode_set(&access, tag, &parsed);
    }

    if parsed.is_repeated {
        return decode_repeated(&access, tag, &parsed);
    }

    if parsed.is_option {
        return decode_option(&access, tag, &parsed);
    }

    if cfg.is_message {
        return decode_message(&access, tag, ty);
    }

    decode_scalar(&access, tag, ty)
}

pub fn generate_field_encoded_len(field: &Field, access: TokenStream, tag: u32) -> EncodedLenTokens {
    let cfg = parse_field_config(field);
    if cfg.skip {
        return EncodedLenTokens::new(quote! { 0 }, false);
    }

    let ty = &field.ty;
    let parsed = parse_field_type(ty);

    if let Some(into_ty) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty).expect("invalid into type");
        let into_parsed = parse_field_type(&into_ty);
        let conv_fn = cfg.into_fn.as_deref().map(|f| format_ident!("{}", f));

        let access_clone = access.clone();
        let value = if let Some(fun) = conv_fn {
            quote! { #fun(&(#access_clone)) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from((#access_clone).clone()) }
        };

        return EncodedLenTokens::new(encoded_len_scalar_value(&value, tag, &into_ty, &into_parsed), true);
    }

    if (cfg.is_rust_enum || parsed.is_rust_enum || cfg.is_proto_enum) && !parsed.is_option && !parsed.is_repeated {
        return EncodedLenTokens::new(encoded_len_enum(&access, tag, ty), true);
    }

    if let Some(kind) = parsed.map_kind {
        return EncodedLenTokens::new(encoded_len_map(&access, tag, &parsed, kind), true);
    }

    if let Type::Array(array) = ty {
        return encoded_len_array(&access, tag, array);
    }

    if parsed.set_kind.is_some() {
        return EncodedLenTokens::new(encoded_len_set(&access, tag, &parsed), true);
    }

    if parsed.is_repeated {
        return EncodedLenTokens::new(encoded_len_repeated(&access, tag, &parsed), true);
    }

    if parsed.is_option {
        return EncodedLenTokens::new(encoded_len_option(&access, tag, &parsed, &cfg), true);
    }

    if cfg.is_message {
        return EncodedLenTokens::new(encoded_len_message(&access, tag, ty), true);
    }

    EncodedLenTokens::new(encoded_len_scalar(&access, tag, ty, &parsed), true)
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

fn encode_scalar(access: &TokenStream, tag: u32, ty: &Type, parsed: &ParsedFieldType) -> TokenStream {
    match parsed.proto_type.as_str() {
        "string" => quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::string::encode(#tag, &(#access), buf);
            }
        },
        "bytes" => quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::bytes::encode(#tag, &(#access), buf);
            }
        },
        "bool" => quote! {{
            let __proto_rs_value = ::core::clone::Clone::clone(&(#access));
            if __proto_rs_value {
                ::proto_rs::encoding::bool::encode(#tag, &__proto_rs_value, buf);
            }
        }},
        "double" => encode_numeric_scalar(access, tag, parsed, quote! { 0f64 }),
        "float" => encode_numeric_scalar(access, tag, parsed, quote! { 0f32 }),
        _ if parsed.is_numeric_scalar => encode_numeric_scalar(access, tag, parsed, quote! { 0 }),
        _ => quote! {
            <#ty as ::proto_rs::ProtoExt>::encode_singular_field(#tag, &(#access), buf);
        },
    }
}

fn encode_scalar_value(value: &TokenStream, tag: u32, ty: &Type, parsed: &ParsedFieldType) -> TokenStream {
    let tmp_ident = format_ident!("__proto_rs_value");
    let encode_body = encode_scalar(&quote! { #tmp_ident }, tag, ty, parsed);
    quote! {{
        let #tmp_ident: #ty = #value;
        #encode_body
    }}
}

fn encode_numeric_scalar(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, zero: TokenStream) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };
    let elem_ty = &parsed.elem_type;
    let proto_ty = &parsed.proto_rust_type;
    let convert = if needs_numeric_widening(parsed) {
        quote! {{
            let __proto_rs_raw: #elem_ty = ::core::clone::Clone::clone(&(#access));
            __proto_rs_raw as #proto_ty
        }}
    } else {
        quote! {
            ::core::clone::Clone::clone(&(#access))
        }
    };

    quote! {{
        let __proto_rs_value: #proto_ty = #convert;
        if __proto_rs_value != #zero {
            ::proto_rs::encoding::#codec::encode(#tag, &__proto_rs_value, buf);
        }
    }}
}

fn decode_scalar(access: &TokenStream, _tag: u32, ty: &Type) -> TokenStream {
    quote! {
        <#ty as ::proto_rs::ProtoExt>::merge_singular_field(
            wire_type,
            &mut (#access),
            buf,
            ctx.clone(),
        )?;
    }
}

fn encoded_len_scalar(access: &TokenStream, tag: u32, ty: &Type, parsed: &ParsedFieldType) -> TokenStream {
    match parsed.proto_type.as_str() {
        "string" => quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::string::encoded_len(#tag, &(#access))
            } else {
                0
            }
        },
        "bytes" => quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::bytes::encoded_len(#tag, &(#access))
            } else {
                0
            }
        },
        "bool" => quote! {{
            let __proto_rs_value = ::core::clone::Clone::clone(&(#access));
            if __proto_rs_value {
                ::proto_rs::encoding::bool::encoded_len(#tag, &__proto_rs_value)
            } else {
                0
            }
        }},
        "double" => encoded_len_numeric_scalar(access, tag, parsed, quote! { 0f64 }),
        "float" => encoded_len_numeric_scalar(access, tag, parsed, quote! { 0f32 }),
        _ if parsed.is_numeric_scalar => encoded_len_numeric_scalar(access, tag, parsed, quote! { 0 }),
        _ => quote! {
            <#ty as ::proto_rs::ProtoExt>::encoded_len_singular_field(#tag, &&(#access))
        },
    }
}

fn encoded_len_scalar_value(value: &TokenStream, tag: u32, ty: &Type, parsed: &ParsedFieldType) -> TokenStream {
    let tmp_ident = format_ident!("__proto_rs_value_len");
    let body = encoded_len_scalar(&quote! { #tmp_ident }, tag, ty, parsed);
    quote! {{
        let #tmp_ident: #ty = #value;
        #body
    }}
}

fn encoded_len_numeric_scalar(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, zero: TokenStream) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! { 0 };
    };
    let elem_ty = &parsed.elem_type;
    let proto_ty = &parsed.proto_rust_type;
    let convert = if needs_numeric_widening(parsed) {
        quote! {{
            let __proto_rs_raw: #elem_ty = ::core::clone::Clone::clone(&(#access));
            __proto_rs_raw as #proto_ty
        }}
    } else {
        quote! {
            ::core::clone::Clone::clone(&(#access))
        }
    };

    quote! {{
        let __proto_rs_value: #proto_ty = #convert;
        if __proto_rs_value != #zero {
            ::proto_rs::encoding::#codec::encoded_len(#tag, &__proto_rs_value)
        } else {
            0
        }
    }}
}

// ---------------------------------------------------------------------------
// Message helpers

fn encode_message(access: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    quote! {
        if <#ty as ::proto_rs::ProtoExt>::encoded_len(&(#access)) != 0 {
            ::proto_rs::encoding::message::encode::<#ty>(#tag, &(#access), buf);
        }
    }
}

fn decode_message(access: &TokenStream, _tag: u32, ty: &Type) -> TokenStream {
    quote! {
        ::proto_rs::encoding::message::merge::<#ty, _>(wire_type, &mut (#access), buf, ctx.clone())?;
    }
}

fn encoded_len_message(access: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    quote! { ::proto_rs::encoding::message::encoded_len::<#ty>(#tag, &&(#access)) }
}

// ---------------------------------------------------------------------------
// Enum helpers (repr(i32))

fn encode_enum(access: &TokenStream, tag: u32, _enum_ty: &Type) -> TokenStream {
    quote! {{
        let __proto_rs_enum = ::core::clone::Clone::clone(&(#access));
        let __value: i32 = __proto_rs_enum as i32;
        if __value != 0 {
            ::proto_rs::encoding::int32::encode(#tag, &__value, buf);
        }
    }}
}

fn decode_enum(access: &TokenStream, _tag: u32, enum_ty: &Type) -> TokenStream {
    quote! {
        let mut __tmp: i32 = 0;
        ::proto_rs::encoding::int32::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
        #access = <#enum_ty as ::core::convert::TryFrom<i32>>::try_from(__tmp)?;
    }
}

fn encoded_len_enum(access: &TokenStream, tag: u32, _enum_ty: &Type) -> TokenStream {
    quote! {{
        let __proto_rs_enum = ::core::clone::Clone::clone(&(#access));
        let __value: i32 = __proto_rs_enum as i32;
        if __value != 0 {
            ::proto_rs::encoding::int32::encoded_len(#tag, &__value)
        } else {
            0
        }
    }}
}

// ---------------------------------------------------------------------------
// Array helpers (fixed length, no allocations)

fn array_bytes_all_default(access: &TokenStream) -> TokenStream {
    quote! {{
        let __proto_rs_default = <u8 as ::proto_rs::ProtoExt>::proto_default();
        (#access).iter().all(|&__proto_rs_value| __proto_rs_value == __proto_rs_default)
    }}
}

fn array_all_default(access: &TokenStream, elem_ty: &Type, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_numeric_scalar {
        return quote! {{
            let __proto_rs_default = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
            (#access)
                .iter()
                .all(|__proto_rs_value| *__proto_rs_value == __proto_rs_default)
        }};
    }

    quote! {{
        let __proto_rs_default = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
        let __proto_rs_default_view = <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(&__proto_rs_default);
        let __proto_rs_default_len = <#elem_ty as ::proto_rs::ProtoExt>::encoded_len(&__proto_rs_default_view);
        (#access).iter().all(|__proto_rs_value| {
            let __proto_rs_view = <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
            <#elem_ty as ::proto_rs::ProtoExt>::encoded_len(&__proto_rs_view) == __proto_rs_default_len
        })
    }}
}

fn encode_array(access: &TokenStream, tag: u32, array: &syn::TypeArray) -> TokenStream {
    let elem_ty = &*array.elem;
    let elem_parsed = parse_field_type(elem_ty);

    if is_bytes_array(&Type::Array(array.clone())) {
        let all_default = array_bytes_all_default(access);
        return quote! {{
            if !#all_default {
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint((#access).len() as u64, buf);
                buf.put_slice(&(#access));
            }
        }};
    }

    let all_default = array_all_default(access, elem_ty, &elem_parsed);

    if elem_parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(&elem_parsed) else {
            return quote! {};
        };
        let proto_ty = &elem_parsed.proto_rust_type;
        let convert = if needs_numeric_widening(&elem_parsed) {
            quote! { (*__proto_rs_value) as #proto_ty }
        } else {
            quote! { *__proto_rs_value }
        };

        let body_len = quote! {
            {
                let mut __proto_rs_body_len = 0usize;
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    __proto_rs_body_len += (::proto_rs::encoding::#codec::encoded_len(1u32, &__proto_rs_converted)
                        - ::proto_rs::encoding::key_len(1u32));
                }
                __proto_rs_body_len
            }
        };

        let emit_values = match elem_parsed.proto_type.as_str() {
            "bool" => quote! {
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    ::proto_rs::encoding::encode_varint(u64::from(__proto_rs_converted), buf);
                }
            },
            "double" | "fixed64" | "sfixed64" | "float" | "fixed32" | "sfixed32" => quote! {
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    buf.put_slice(&__proto_rs_converted.to_le_bytes());
                }
            },

            _ => quote! {
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    ::proto_rs::encoding::encode_varint(__proto_rs_converted as u64, buf);
                }
            },
        };

        return quote! {{
            if !#all_default {
                let __proto_rs_body_len = #body_len;
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint(__proto_rs_body_len as u64, buf);
                #emit_values
            }
        }};
    }

    quote! {{
        if !#all_default {
            for __proto_rs_value in (#access).iter() {
                let __proto_rs_view =
                    <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
                <#elem_ty as ::proto_rs::ProtoExt>::encode_singular_field(#tag, __proto_rs_view, buf);
            }
        }
    }}
}

fn decode_array(access: &TokenStream, _tag: u32, array: &syn::TypeArray) -> TokenStream {
    let elem_ty = &*array.elem;
    let elem_parsed = parse_field_type(elem_ty);

    if is_bytes_array(&Type::Array(array.clone())) {
        return quote! {
            if wire_type != ::proto_rs::encoding::WireType::LengthDelimited {
                return Err(::proto_rs::DecodeError::new("invalid wire type for fixed array"));
            }
            let __len = ::proto_rs::encoding::decode_varint(buf)? as usize;
            if __len > (#access).len() {
                return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
            }
            {
                let (__filled, __rest) = (#access).split_at_mut(__len);
                buf.copy_to_slice(__filled);
                for __value in __rest.iter_mut() {
                    *__value = 0;
                }
            }
        };
    }

    if elem_parsed.is_message_like && !elem_parsed.is_rust_enum {
        return quote! {
            let mut __proto_rs_index = 0usize;
            if __proto_rs_index >= (#access).len() {
                return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
            }
            let mut __proto_rs_tmp: #elem_ty = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
            ::proto_rs::encoding::message::merge::<#elem_ty, _>(wire_type, &mut __proto_rs_tmp, buf, ctx.clone())?;
            (#access)[__proto_rs_index] = __proto_rs_tmp;
            __proto_rs_index += 1;
            while __proto_rs_index < (#access).len() {
                (#access)[__proto_rs_index] = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
                __proto_rs_index += 1;
            }
        };
    }

    if elem_parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(&elem_parsed) else {
            return quote! {};
        };
        let proto_ty = &elem_parsed.proto_rust_type;
        let assign_expr = if needs_numeric_widening(&elem_parsed) {
            let target_ty = &elem_parsed.elem_type;
            quote! {
                (#access)[__proto_rs_index] = <#target_ty as ::core::convert::TryFrom<#proto_ty>>::try_from(__proto_rs_tmp)
                    .map_err(|_| ::proto_rs::DecodeError::new("numeric conversion failed"))?;
            }
        } else {
            quote! {
                (#access)[__proto_rs_index] = __proto_rs_tmp;
            }
        };
        let wire = scalar_wire_type(&elem_parsed);

        return quote! {
            if wire_type != ::proto_rs::encoding::WireType::LengthDelimited {
                return Err(::proto_rs::DecodeError::new("packed array field must be length-delimited"));
            }
            let __proto_rs_len = ::proto_rs::encoding::decode_varint(buf)? as usize;
            let mut __proto_rs_limited = buf.take(__proto_rs_len);
            let mut __proto_rs_index = 0usize;
            while __proto_rs_limited.has_remaining() {
                if __proto_rs_index >= (#access).len() {
                    return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                }
                let mut __proto_rs_tmp: #proto_ty = <#proto_ty as ::proto_rs::ProtoExt>::proto_default();
                ::proto_rs::encoding::#codec::merge(#wire, &mut __proto_rs_tmp, &mut __proto_rs_limited, ctx.clone())?;
                #assign_expr
                __proto_rs_index += 1;
            }
            while __proto_rs_index < (#access).len() {
                (#access)[__proto_rs_index] = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
                __proto_rs_index += 1;
            }
        };
    }

    let Some(codec) = scalar_codec(&elem_parsed) else {
        return quote! {};
    };

    let wire = scalar_wire_type(&elem_parsed);
    let target_ty = &elem_parsed.elem_type;

    quote! {
        let mut __i = 0usize;
        match wire_type {
            ::proto_rs::encoding::WireType::LengthDelimited => {
                let __len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                let mut __limited = buf.take(__len);
                while __limited.has_remaining() {
                    if __i >= (#access).len() {
                        return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                    }
                    let mut __tmp: #target_ty = <#target_ty as ::proto_rs::ProtoExt>::proto_default();
                    ::proto_rs::encoding::#codec::merge(#wire, &mut __tmp, &mut __limited, ctx.clone())?;
                    (#access)[__i] = __tmp;
                    __i += 1;
                }
            }
            _ => {
                if __i >= (#access).len() {
                    return Err(::proto_rs::DecodeError::new("too many elements for fixed array"));
                }
                let mut __tmp: #target_ty = <#target_ty as ::proto_rs::ProtoExt>::proto_default();
                ::proto_rs::encoding::#codec::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                (#access)[__i] = __tmp;
            }
        }
        while __i < (#access).len() {
            (#access)[__i] = <#target_ty as ::proto_rs::ProtoExt>::proto_default();
            __i += 1;
        }
    }
}

fn encoded_len_array(access: &TokenStream, tag: u32, array: &syn::TypeArray) -> EncodedLenTokens {
    if is_bytes_array(&Type::Array(array.clone())) {
        let all_default = array_bytes_all_default(access);
        return EncodedLenTokens::new(
            quote! {{
                if #all_default {
                    0
                } else {
                    let __proto_rs_len = (#access).len();
                    ::proto_rs::encoding::key_len(#tag)
                        + ::proto_rs::encoding::encoded_len_varint(__proto_rs_len as u64)
                        + __proto_rs_len
                }
            }},
            true,
        );
    }

    let elem_ty = &*array.elem;
    let elem_parsed = parse_field_type(elem_ty);

    let all_default = array_all_default(access, elem_ty, &elem_parsed);

    let Some(codec) = scalar_codec(&elem_parsed) else {
        return EncodedLenTokens::new(quote! { 0 }, false);
    };

    if elem_parsed.is_numeric_scalar {
        let proto_ty = &elem_parsed.proto_rust_type;
        let convert = if needs_numeric_widening(&elem_parsed) {
            quote! { (*__proto_rs_value) as #proto_ty }
        } else {
            quote! { *__proto_rs_value }
        };

        return EncodedLenTokens::new(
            quote! {{
                if #all_default {
                    0
                } else {
                    let mut __proto_rs_body_len = 0usize;
                    for __proto_rs_value in (#access).iter() {
                        let __proto_rs_converted: #proto_ty = #convert;
                        __proto_rs_body_len += (::proto_rs::encoding::#codec::encoded_len(1u32, &__proto_rs_converted)
                            - ::proto_rs::encoding::key_len(1u32));
                    }
                    ::proto_rs::encoding::key_len(#tag)
                        + ::proto_rs::encoding::encoded_len_varint(__proto_rs_body_len as u64)
                        + __proto_rs_body_len
                }
            }},
            true,
        );
    }

    EncodedLenTokens::new(
        quote! {{
            if #all_default {
                0
            } else {
                let mut __proto_rs_len = 0usize;
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_view =
                        <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
                    __proto_rs_len += <#elem_ty as ::proto_rs::ProtoExt>::encoded_len_singular_field(#tag, &__proto_rs_view);
                }
                __proto_rs_len
            }
        }},
        true,
    )
}

// ---------------------------------------------------------------------------
// Repeated helpers (Vec<T>)

fn encode_repeated(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let elem_ty = &parsed.elem_type;
    if parsed.proto_type == "string" {
        return quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::string::encode_repeated(#tag, &(#access), buf);
            }
        };
    }

    if parsed.proto_type == "bytes" {
        return quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::bytes::encode_repeated(#tag, &(#access), buf);
            }
        };
    }

    if parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(parsed) else {
            return quote! {};
        };

        return quote! {
            if !(#access).is_empty() {
                ::proto_rs::encoding::#codec::encode_packed(#tag, &(#access), buf);
            }
        };
    }

    quote! {
        if !(#access).is_empty() {
            for __proto_rs_value in (#access).iter() {
                let __proto_rs_view =
                    <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
                <#elem_ty as ::proto_rs::ProtoExt>::encode_singular_field(#tag, __proto_rs_view, buf);
            }
        }
    }
}

fn decode_repeated(access: &TokenStream, _tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let elem_ty = &parsed.elem_type;
    if parsed.proto_type == "string" {
        return quote! {
            ::proto_rs::encoding::string::merge_repeated(
                wire_type,
                &mut (#access),
                buf,
                ctx.clone(),
            )?;
        };
    }

    if parsed.proto_type == "bytes" {
        return quote! {
            ::proto_rs::encoding::bytes::merge_repeated(
                wire_type,
                &mut (#access),
                buf,
                ctx.clone(),
            )?;
        };
    }

    if parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(parsed) else {
            return quote! {};
        };

        if needs_numeric_widening(parsed) {
            return quote! {
                if wire_type != ::proto_rs::encoding::WireType::LengthDelimited {
                    return Err(::proto_rs::DecodeError::new("packed numeric field must be length-delimited"));
                }
                let __proto_rs_len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                let mut __proto_rs_limited = buf.take(__proto_rs_len);
                while __proto_rs_limited.has_remaining() {
                    let __proto_rs_value = <#elem_ty as ::proto_rs::ProtoExt>::decode_length_delimited(&mut __proto_rs_limited)?;
                    (#access).push(__proto_rs_value);
                }
            };
        }

        return quote! {
            ::proto_rs::encoding::#codec::merge_repeated(
                wire_type,
                &mut (#access),
                buf,
                ctx.clone(),
            )?;
        };
    }

    quote! {
        if wire_type == ::proto_rs::encoding::WireType::LengthDelimited {
            let __proto_rs_len = ::proto_rs::encoding::decode_varint(buf)? as usize;
            let mut __proto_rs_limited = buf.take(__proto_rs_len);
            let __proto_rs_bytes = __proto_rs_limited.copy_to_bytes(__proto_rs_len);

            let mut __proto_rs_value = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
            if let Ok(()) = <#elem_ty as ::proto_rs::ProtoExt>::merge(
                &mut __proto_rs_value,
                __proto_rs_bytes.clone(),
            ) {
                let __proto_rs_owned = <#elem_ty as ::proto_rs::ProtoExt>::post_decode(__proto_rs_value)?;
                (#access).push(__proto_rs_owned);
            } else {
                let mut __proto_rs_cursor = __proto_rs_bytes.clone();
                while __proto_rs_cursor.has_remaining() {
                    let mut __proto_rs_elem = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
                    <#elem_ty as ::proto_rs::ProtoExt>::merge_singular_field(
                        ::proto_rs::encoding::WireType::Varint,
                        &mut __proto_rs_elem,
                        &mut __proto_rs_cursor,
                        ctx.clone(),
                    )?;
                    let __proto_rs_owned = <#elem_ty as ::proto_rs::ProtoExt>::post_decode(__proto_rs_elem)?;
                    (#access).push(__proto_rs_owned);
                }
            }
        } else {
            let mut __proto_rs_value = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
            <#elem_ty as ::proto_rs::ProtoExt>::merge_singular_field(
                wire_type,
                &mut __proto_rs_value,
                buf,
                ctx.clone(),
            )?;
            let __proto_rs_owned = <#elem_ty as ::proto_rs::ProtoExt>::post_decode(__proto_rs_value)?;
            (#access).push(__proto_rs_owned);
        }
    }
}

fn encoded_len_repeated(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let elem_ty = &parsed.elem_type;
    if parsed.proto_type == "string" {
        return quote! {
            if (#access).is_empty() {
                0
            } else {
                ::proto_rs::encoding::string::encoded_len_repeated(#tag, &(#access))
            }
        };
    }

    if parsed.proto_type == "bytes" {
        return quote! {
            if (#access).is_empty() {
                0
            } else {
                ::proto_rs::encoding::bytes::encoded_len_repeated(#tag, &(#access))
            }
        };
    }

    if parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(parsed) else {
            return quote! { 0 };
        };

        return quote! {
            if (#access).is_empty() {
                0
            } else {
                ::proto_rs::encoding::#codec::encoded_len_packed(#tag, &(#access))
            }
        };
    }

    quote! {{
        if (#access).is_empty() {
            0
        } else {
            let mut __proto_rs_len = 0usize;
            for __proto_rs_value in (#access).iter() {
                let __proto_rs_view =
                    <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
                __proto_rs_len +=
                    <#elem_ty as ::proto_rs::ProtoExt>::encoded_len_singular_field(#tag, &__proto_rs_view);
            }
            __proto_rs_len
        }
    }}
}

fn map_module(kind: MapKind) -> TokenStream {
    match kind {
        MapKind::HashMap => quote! { ::proto_rs::encoding::hash_map },
        MapKind::BTreeMap => quote! { ::proto_rs::encoding::btree_map },
    }
}

fn map_field_codecs(ty: &Type) -> (TokenStream, TokenStream) {
    let parsed = parse_field_type(ty);

    match parsed.proto_type.as_str() {
        "string" => (
            quote! { |tag, value, buf| ::proto_rs::encoding::string::encode(tag, value, buf) },
            quote! { |tag, value| ::proto_rs::encoding::string::encoded_len(tag, value) },
        ),
        "bytes" => (
            quote! { |tag, value, buf| ::proto_rs::encoding::bytes::encode(tag, value, buf) },
            quote! { |tag, value| ::proto_rs::encoding::bytes::encoded_len(tag, value) },
        ),
        "bool" => (
            quote! { |tag, value, buf| ::proto_rs::encoding::bool::encode(tag, value, buf) },
            quote! { |tag, value| ::proto_rs::encoding::bool::encoded_len(tag, value) },
        ),
        _ if parsed.is_numeric_scalar => {
            let Some(codec) = scalar_codec(&parsed) else {
                return (
                    quote! { |tag, value, buf| <#ty as ::proto_rs::ProtoExt>::encode_singular_field(tag, value, buf) },
                    quote! { |tag, value| <#ty as ::proto_rs::ProtoExt>::encoded_len_singular_field(tag, &value) },
                );
            };

            if needs_numeric_widening(&parsed) {
                let proto_ty = &parsed.proto_rust_type;
                (
                    quote! { |tag, value, buf| {
                        let __proto_rs_value: #proto_ty = (*value) as #proto_ty;
                        ::proto_rs::encoding::#codec::encode(tag, &__proto_rs_value, buf);
                    } },
                    quote! { |tag, value| {
                        let __proto_rs_value: #proto_ty = (*value) as #proto_ty;
                        ::proto_rs::encoding::#codec::encoded_len(tag, &__proto_rs_value)
                    } },
                )
            } else {
                (
                    quote! { |tag, value, buf| ::proto_rs::encoding::#codec::encode(tag, value, buf) },
                    quote! { |tag, value| ::proto_rs::encoding::#codec::encoded_len(tag, value) },
                )
            }
        }
        _ => (
            quote! { |tag, value, buf| <#ty as ::proto_rs::ProtoExt>::encode_singular_field(tag, value, buf) },
            quote! { |tag, value| <#ty as ::proto_rs::ProtoExt>::encoded_len_singular_field(tag, &value) },
        ),
    }
}

fn encode_map(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, kind: MapKind) -> TokenStream {
    let module = map_module(kind);
    let key_ty = parsed.map_key_type.as_ref().expect("map key type metadata missing");
    let value_ty = parsed.map_value_type.as_ref().expect("map value type metadata missing");
    let (key_encode, key_encoded_len) = map_field_codecs(key_ty);
    let (value_encode, value_encoded_len) = map_field_codecs(value_ty);

    quote! {
        if !(#access).is_empty() {
            #module::encode(
                #key_encode,
                #key_encoded_len,
                #value_encode,
                #value_encoded_len,
                #tag,
                &(#access),
                buf,
            );
        }
    }
}

fn decode_map(access: &TokenStream, _tag: u32, parsed: &ParsedFieldType, kind: MapKind) -> TokenStream {
    let module = map_module(kind);
    let key_ty = parsed.map_key_type.as_ref().expect("map key type metadata missing");
    let value_ty = parsed.map_value_type.as_ref().expect("map value type metadata missing");

    quote! {
        #module::merge(
            |wire_type, key, buf, ctx| <#key_ty as ::proto_rs::ProtoExt>::merge_singular_field(wire_type, key, buf, ctx),
            |wire_type, value, buf, ctx| <#value_ty as ::proto_rs::ProtoExt>::merge_singular_field(wire_type, value, buf, ctx),
            &mut (#access),
            buf,
            ctx.clone(),
        )?;
    }
}

fn encoded_len_map(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, kind: MapKind) -> TokenStream {
    let module = map_module(kind);
    let key_ty = parsed.map_key_type.as_ref().expect("map key type metadata missing");
    let value_ty = parsed.map_value_type.as_ref().expect("map value type metadata missing");
    let (_, key_encoded_len) = map_field_codecs(key_ty);
    let (_, value_encoded_len) = map_field_codecs(value_ty);

    quote! {
        #module::encoded_len(
            #key_encoded_len,
            #value_encoded_len,
            #tag,
            &(#access),
        )
    }
}

fn encode_set(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let elem_ty = &parsed.elem_type;
    if parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(parsed) else {
            return quote! {};
        };
        let proto_ty = &parsed.proto_rust_type;
        let convert = if needs_numeric_widening(parsed) {
            quote! { (*__proto_rs_value) as #proto_ty }
        } else {
            quote! { *__proto_rs_value }
        };

        let body_len = quote! {{
            let mut __proto_rs_body_len = 0usize;
            for __proto_rs_value in (#access).iter() {
                let __proto_rs_converted: #proto_ty = #convert;
                __proto_rs_body_len += (::proto_rs::encoding::#codec::encoded_len(1u32, &__proto_rs_converted)
                    - ::proto_rs::encoding::key_len(1u32));
            }
            __proto_rs_body_len
        }};

        let emit_values = match parsed.proto_type.as_str() {
            "bool" => quote! {
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    ::proto_rs::encoding::encode_varint(u64::from(__proto_rs_converted), buf);
                }
            },
            "double" | "fixed64" | "sfixed64" | "float" | "fixed32" | "sfixed32" => quote! {
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    buf.put_slice(&__proto_rs_converted.to_le_bytes());
                }
            },
            _ => quote! {
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    ::proto_rs::encoding::encode_varint(__proto_rs_converted as u64, buf);
                }
            },
        };

        return quote! {{
            if !(#access).is_empty() {
                let __proto_rs_body_len = #body_len;
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint(__proto_rs_body_len as u64, buf);
                #emit_values
            }
        }};
    }

    quote! {{
        if !(#access).is_empty() {
            for __proto_rs_value in (#access).iter() {
                let __proto_rs_view =
                    <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
                <#elem_ty as ::proto_rs::ProtoExt>::encode_singular_field(#tag, __proto_rs_view, buf);
            }
        }
    }}
}

fn decode_set(access: &TokenStream, _tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let elem_ty = &parsed.elem_type;
    if parsed.proto_type == "string" {
        return quote! {
            ::proto_rs::encoding::string::merge_repeated(
                wire_type,
                &mut (#access),
                buf,
                ctx.clone(),
            )?;
        };
    }

    if parsed.proto_type == "bytes" {
        return quote! {
            ::proto_rs::encoding::bytes::merge_repeated(
                wire_type,
                &mut (#access),
                buf,
                ctx.clone(),
            )?;
        };
    }

    if parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(parsed) else {
            return quote! {};
        };

        if needs_numeric_widening(parsed) {
            return quote! {
                if wire_type != ::proto_rs::encoding::WireType::LengthDelimited {
                    return Err(::proto_rs::DecodeError::new("packed numeric field must be length-delimited"));
                }
                let __proto_rs_len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                let mut __proto_rs_limited = buf.take(__proto_rs_len);
                while __proto_rs_limited.has_remaining() {
                    let __proto_rs_value = <#elem_ty as ::proto_rs::ProtoExt>::decode_length_delimited(&mut __proto_rs_limited)?;
                    ::proto_rs::RepeatedCollection::push(&mut (#access), __proto_rs_value);
                }
            };
        }

        return quote! {
            ::proto_rs::encoding::#codec::merge_repeated(
                wire_type,
                &mut (#access),
                buf,
                ctx.clone(),
            )?;
        };
    }

    quote! {
        let mut __proto_rs_value = <#elem_ty as ::proto_rs::ProtoExt>::proto_default();
        <#elem_ty as ::proto_rs::ProtoExt>::merge_singular_field(
            wire_type,
            &mut __proto_rs_value,
            buf,
            ctx.clone(),
        )?;
        let __proto_rs_owned = <#elem_ty as ::proto_rs::ProtoExt>::post_decode(__proto_rs_value)?;
        ::proto_rs::RepeatedCollection::push(&mut (#access), __proto_rs_owned);
    }
}

fn encoded_len_set(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let elem_ty = &parsed.elem_type;
    if parsed.is_numeric_scalar {
        let Some(codec) = scalar_codec(parsed) else {
            return quote! { 0 };
        };
        let proto_ty = &parsed.proto_rust_type;
        let convert = if needs_numeric_widening(parsed) {
            quote! { (*__proto_rs_value) as #proto_ty }
        } else {
            quote! { *__proto_rs_value }
        };

        return quote! {{
            if (#access).is_empty() {
                0
            } else {
                let mut __proto_rs_body_len = 0usize;
                for __proto_rs_value in (#access).iter() {
                    let __proto_rs_converted: #proto_ty = #convert;
                    __proto_rs_body_len += (::proto_rs::encoding::#codec::encoded_len(1u32, &__proto_rs_converted)
                        - ::proto_rs::encoding::key_len(1u32));
                }
                ::proto_rs::encoding::key_len(#tag)
                    + ::proto_rs::encoding::encoded_len_varint(__proto_rs_body_len as u64)
                    + __proto_rs_body_len
            }
        }};
    }

    quote! {{
        if (#access).is_empty() {
            0
        } else {
            let mut __proto_rs_len = 0usize;
            for __proto_rs_value in (#access).iter() {
                let __proto_rs_view =
                    <<#elem_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(__proto_rs_value);
                __proto_rs_len += <#elem_ty as ::proto_rs::ProtoExt>::encoded_len_singular_field(#tag, &__proto_rs_view);
            }
            __proto_rs_len
        }
    }}
}

// ---------------------------------------------------------------------------
// Option helpers

fn encode_option(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, cfg: &FieldConfig) -> TokenStream {
    let inner_ty = &parsed.elem_type;
    let inner_parsed = parse_field_type(inner_ty);

    if cfg.is_rust_enum || cfg.is_proto_enum || inner_parsed.is_rust_enum {
        return quote! {
            if let Some(value) = (#access).as_ref() {
                let __proto_rs_value: i32 = (*value) as i32;
                ::proto_rs::encoding::int32::encode(#tag, &__proto_rs_value, buf);
            }
        };
    }

    match inner_parsed.proto_type.as_str() {
        "string" => quote! {
            if let Some(value) = (#access).as_ref() {
                if !value.is_empty() {
                    ::proto_rs::encoding::string::encode(#tag, value, buf);
                }
            }
        },
        "bytes" => quote! {
            if let Some(value) = (#access).as_ref() {
                if !value.is_empty() {
                    ::proto_rs::encoding::bytes::encode(#tag, value, buf);
                }
            }
        },
        "bool" => quote! {
            if let Some(&value) = (#access).as_ref() {
                if value {
                    ::proto_rs::encoding::bool::encode(#tag, &value, buf);
                }
            }
        },
        "double" => encode_option_numeric(access, tag, &inner_parsed, quote! { 0f64 }),
        "float" => encode_option_numeric(access, tag, &inner_parsed, quote! { 0f32 }),
        _ if inner_parsed.is_numeric_scalar => encode_option_numeric(access, tag, &inner_parsed, quote! { 0 }),
        _ => quote! {
            let __proto_rs_value = (#access).as_ref().map(|value| {
                <<#inner_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value)
            });
            <#inner_ty as ::proto_rs::ProtoExt>::encode_option_field(#tag, __proto_rs_value, buf);
        },
    }
}

fn encode_option_numeric(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, zero: TokenStream) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! {};
    };
    let proto_ty = &parsed.proto_rust_type;
    let convert = if needs_numeric_widening(parsed) {
        quote! { (*value) as #proto_ty }
    } else {
        quote! { *value }
    };

    quote! {{
        if let Some(value) = (#access).as_ref() {
            let __proto_rs_value: #proto_ty = #convert;
            if __proto_rs_value != #zero {
                ::proto_rs::encoding::#codec::encode(#tag, &__proto_rs_value, buf);
            }
        }
    }}
}

fn decode_option(access: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let inner_ty = &parsed.elem_type;

    if let Some(box_inner) = inner_box_type(inner_ty) {
        return decode_option_box(access, tag, &box_inner);
    }

    if let Some(arc_inner) = inner_arc_type(inner_ty) {
        return decode_option_arc(access, tag, &arc_inner);
    }

    quote! {
        <#inner_ty as ::proto_rs::ProtoExt>::merge_option_field(
            wire_type,
            &mut (#access),
            buf,
            ctx.clone(),
        )?;
    }
}

fn encoded_len_option(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, cfg: &FieldConfig) -> TokenStream {
    let inner_ty = &parsed.elem_type;
    let inner_parsed = parse_field_type(inner_ty);

    if cfg.is_rust_enum || cfg.is_proto_enum || inner_parsed.is_rust_enum {
        return quote! {{
            (#access).as_ref().map_or(0usize, |value| {
                let __proto_rs_value: i32 = (*value) as i32;
                ::proto_rs::encoding::int32::encoded_len(#tag, &__proto_rs_value)
            })
        }};
    }

    match inner_parsed.proto_type.as_str() {
        "string" => quote! {{
            (#access).as_ref().map_or(0usize, |value| {
                if value.is_empty() {
                    0
                } else {
                    ::proto_rs::encoding::string::encoded_len(#tag, value)
                }
            })
        }},
        "bytes" => quote! {{
            (#access).as_ref().map_or(0usize, |value| {
                if value.is_empty() {
                    0
                } else {
                    ::proto_rs::encoding::bytes::encoded_len(#tag, value)
                }
            })
        }},
        "bool" => quote! {{
            (#access).as_ref().map_or(0usize, |&value| {
                if value {
                    ::proto_rs::encoding::bool::encoded_len(#tag, &value)
                } else {
                    0
                }
            })
        }},
        "double" => encoded_len_option_numeric(access, tag, &inner_parsed, quote! { 0f64 }),
        "float" => encoded_len_option_numeric(access, tag, &inner_parsed, quote! { 0f32 }),
        _ if inner_parsed.is_numeric_scalar => encoded_len_option_numeric(access, tag, &inner_parsed, quote! { 0 }),
        _ => quote! {{
            let __proto_rs_value = (#access).as_ref().map(|value| {
                <<#inner_ty as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value)
            });
            <#inner_ty as ::proto_rs::ProtoExt>::encoded_len_option_field(#tag, __proto_rs_value)
        }},
    }
}

fn encoded_len_option_numeric(access: &TokenStream, tag: u32, parsed: &ParsedFieldType, zero: TokenStream) -> TokenStream {
    let Some(codec) = scalar_codec(parsed) else {
        return quote! { 0 };
    };
    let proto_ty = &parsed.proto_rust_type;
    let convert = if needs_numeric_widening(parsed) {
        quote! { (*value) as #proto_ty }
    } else {
        quote! { *value }
    };

    quote! {{
        (#access).as_ref().map_or(0usize, |value| {
            let __proto_rs_value: #proto_ty = #convert;
            if __proto_rs_value != #zero {
                ::proto_rs::encoding::#codec::encoded_len(#tag, &__proto_rs_value)
            } else {
                0
            }
        })
    }}
}

fn inner_box_type(ty: &Type) -> Option<Type> {
    if let Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
        && segment.ident == "Box"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }

    None
}

fn inner_arc_type(ty: &Type) -> Option<Type> {
    if let Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
        && segment.ident == "Arc"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }

    None
}

fn decode_option_box(access: &TokenStream, _tag: u32, inner_ty: &Type) -> TokenStream {
    quote! {
        if let Some(__proto_rs_existing) = (#access).as_mut() {
            <#inner_ty as ::proto_rs::ProtoExt>::merge_singular_field(
                wire_type,
                __proto_rs_existing.as_mut(),
                buf,
                ctx.clone(),
            )?;
        } else {
            let mut __proto_rs_tmp: <::proto_rs::alloc::boxed::Box<#inner_ty> as ::proto_rs::ProtoExt>::Shadow<'_> =
                <::proto_rs::alloc::boxed::Box<#inner_ty> as ::proto_rs::ProtoExt>::proto_default();
            <::proto_rs::alloc::boxed::Box<#inner_ty> as ::proto_rs::ProtoExt>::merge_singular_field(
                wire_type,
                &mut __proto_rs_tmp,
                buf,
                ctx.clone(),
            )?;
            let __proto_rs_owned = <::proto_rs::alloc::boxed::Box<#inner_ty> as ::proto_rs::ProtoExt>::post_decode(__proto_rs_tmp)?;
            (#access) = Some(__proto_rs_owned);
        }
    }
}

fn decode_option_arc(access: &TokenStream, _tag: u32, inner_ty: &Type) -> TokenStream {
    let decode_new = quote! {
        let mut __proto_rs_tmp: <::proto_rs::alloc::sync::Arc<#inner_ty> as ::proto_rs::ProtoExt>::Shadow<'_> =
            <::proto_rs::alloc::sync::Arc<#inner_ty> as ::proto_rs::ProtoExt>::proto_default();
        <::proto_rs::alloc::sync::Arc<#inner_ty> as ::proto_rs::ProtoExt>::merge_singular_field(
            wire_type,
            &mut __proto_rs_tmp,
            buf,
            ctx.clone(),
        )?;
        let __proto_rs_owned = <::proto_rs::alloc::sync::Arc<#inner_ty> as ::proto_rs::ProtoExt>::post_decode(__proto_rs_tmp)?;
        (#access) = Some(__proto_rs_owned);
    };

    quote! {
        if let Some(__proto_rs_existing) = (#access).as_mut() {
            if let Some(__proto_rs_inner) = ::proto_rs::alloc::sync::Arc::get_mut(__proto_rs_existing) {
                <#inner_ty as ::proto_rs::ProtoExt>::merge_singular_field(
                    wire_type,
                    __proto_rs_inner,
                    buf,
                    ctx.clone(),
                )?;
            } else {
                #decode_new
            }
        } else {
            #decode_new
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
