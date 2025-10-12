// unified_field_handler.rs
//! Unified field handling for structs and enum variants (prost-compatible).

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Field;
use syn::Ident;
use syn::Index;
use syn::Type;

use crate::utils::FieldConfig;
use crate::utils::ParsedFieldType;
use crate::utils::is_bytes_array;
use crate::utils::is_bytes_vec;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;

// ————————————————————————————————————————————————————————————————————————
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

// ————————————————————————————————————————————————————————————————————————
// Public entry points used by struct/enum handlers

pub fn generate_field_encode(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    let ty = &field.ty;
    let parsed = parse_field_type(ty);
    let fa = access.to_tokens();

    // Skipped fields aren't encoded
    if cfg.skip {
        return quote! {};
    }

    // Custom "into" conversion
    if let Some(into_ty) = &cfg.into_type {
        let conv = cfg.into_fn.as_deref().map(|f| format_ident!("{}", f));
        let into_ty: Type = syn::parse_str(into_ty).expect("invalid into type");

        let val = if let Some(fun) = conv {
            quote! { #fun(&self.#fa) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from(self.#fa.clone()) }
        };
        return encode_scalar(&val, tag, &into_ty);
    }

    // Rust enum (backed i32)
    if cfg.is_rust_enum || cfg.is_proto_enum {
        let val = quote! { (self.#fa as i32) };
        return quote! {
            if #val != 0 {
                ::proto_rs::encoding::int32::encode(#tag, &#val, buf);
            }
        };
    }

    // Arrays
    if let Type::Array(arr) = ty {
        return encode_array_no_alloc(&fa, tag, arr);
    }

    // Repeated vs Optional vs Message vs Scalar
    if parsed.is_repeated {
        return encode_repeated(&fa, tag, &parsed);
    }
    if parsed.is_option {
        return encode_option(&fa, tag, &parsed);
    }
    if parsed.is_message_like || cfg.is_message {
        return quote! {
            let __inner = &self.#fa;
            if ::proto_rs::encoding::Message::encoded_len(__inner) != 0 {
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                let __len = ::proto_rs::encoding::Message::encoded_len(__inner);
                ::proto_rs::encoding::encode_varint(__len as u64, buf);
                ::proto_rs::encoding::Message::encode_raw(__inner, buf);
            }
        };
    }

    // Scalar fallback
    encode_scalar(&quote! { self.#fa }, tag, ty)
}

pub fn generate_field_decode(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    let ty = &field.ty;
    let parsed = parse_field_type(ty);
    let fa = access.to_tokens();

    if cfg.skip {
        // We ignore the field on the wire; the value is filled after full decode (struct_handler hook).
        return quote! {/* skip on wire */};
    }

    // Custom "from" conversion
    if let Some(from_ty) = &cfg.from_type {
        let conv = cfg.from_fn.as_deref().map(|f| format_ident!("{}", f));
        let from_ty: Type = syn::parse_str(from_ty).expect("invalid from type");

        // merge source typed as from_ty, then convert
        return quote! {
            {
                let mut __tmp: #from_ty = ::core::default::Default::default();
                if let Err(e) = <#from_ty as ::proto_rs::ProtoExt>::merge_field(&mut __tmp, #tag, wire_type, buf, ctx.clone()) {
                    return Err(e);
                }
                self.#fa = {
                    #( let _ = &__tmp; )*
                    #{
                        if let Some(fun) = #conv {
                            quote!{ #fun(__tmp) }
                        } else {
                            quote!{ <_ as ::core::convert::From<_>>::from(__tmp) }
                        }
                    }
                };
            }
        };
    }

    if cfg.is_rust_enum || cfg.is_proto_enum {
        return quote! {
            if #tag == tag {
                let mut __tmp: i32 = 0;
                ::proto_rs::encoding::int32::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                // Safety: user-defined enums are assumed repr(i32) prost-like
                self.#fa = unsafe { ::core::mem::transmute(__tmp) };
            }
        };
    }

    if let Type::Array(arr) = ty {
        return decode_array_no_alloc(&fa, tag, arr);
    }

    if parsed.is_repeated {
        return decode_repeated(&fa, tag, &parsed);
    }

    if parsed.is_option {
        // Option<T>: merge into an inner default then Some(...)
        return quote! {
            if #tag == tag {
                let mut __tmp = <_ as ::proto_rs::ProtoExt>::proto_default();
                <_ as ::proto_rs::ProtoExt>::merge_field(&mut __tmp, #tag, wire_type, buf, ctx.clone())?;
                self.#fa = Some(__tmp);
            }
        };
    }

    if parsed.is_message_like {
        return quote! {
            if #tag == tag {
                let mut __ctx = ctx.clone();
                if wire_type == ::proto_rs::encoding::WireType::LengthDelimited {
                    let len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                    let mut limited = buf.take(len);
                    <_ as ::proto_rs::ProtoExt>::merge(&mut self.#fa, &mut limited)?;
                    if limited.has_remaining() { return Err(::proto_rs::DecodeError::new("length-delimited message overrun")); }
                } else {
                    <_ as ::proto_rs::ProtoExt>::merge_field(&mut self.#fa, #tag, wire_type, buf, __ctx)?;
                }
            }
        };
    }

    // Scalar merge fallback
    decode_scalar_into(&quote! { self.#fa }, tag, ty)
}

pub fn generate_field_encoded_len(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg = parse_field_config(field);
    if cfg.skip {
        return quote! { 0 };
    }

    let ty = &field.ty;
    let parsed = parse_field_type(ty);
    let fa = access.to_tokens();

    if let Some(into_ty) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty).unwrap();
        let conv = cfg.into_fn.as_deref().map(|f| format_ident!("{}", f));
        let val = if let Some(fun) = conv {
            quote! { #fun(&self.#fa) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from(self.#fa.clone()) }
        };
        return encoded_len_scalar(&val, tag, &into_ty);
    }

    if cfg.is_rust_enum || cfg.is_proto_enum {
        return quote! {
            if (self.#fa as i32) != 0 {
                ::proto_rs::encoding::int32::encoded_len(#tag, &(self.#fa as i32))
            } else { 0 }
        };
    }

    if let Type::Array(arr) = ty {
        return encoded_len_array_no_alloc(&fa, tag, arr);
    }

    if parsed.is_repeated {
        return encoded_len_repeated(&fa, tag, &parsed);
    }
    if parsed.is_option {
        return quote! {
            self.#fa.as_ref().map_or(0, |v| ::proto_rs::ProtoExt::encoded_len(v).saturating_add(::proto_rs::encoding::encoded_len_key(#tag)))
        };
    }
    if parsed.is_message_like {
        return quote! {
            let l = ::proto_rs::ProtoExt::encoded_len(&self.#fa);
            if l == 0 { 0 } else { ::proto_rs::encoding::encoded_len_key(#tag) + ::proto_rs::encoding::encoded_len_varint(l as u64) + l }
        };
    }

    encoded_len_scalar(&quote! { self.#fa }, tag, ty)
}

// ————————————————————————————————————————————————————————————————————————
// Scalar helpers (no allocation, prost wire-compatible)

fn encode_scalar(val: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    use quote::quote;
    if is_bytes_vec(ty) {
        // Vec<u8> as bytes
        return quote! {
            if !(#val).is_empty() {
                ::proto_rs::encoding::bytes::encode(#tag, &#val, buf);
            }
        };
    }
    let w = scalar_codec_ident(ty);
    quote! {
        let __v = #val;
        // Zero/empty elision is handled by the codec's encode (prost-compatible)
        ::proto_rs::encoding::#w::encode(#tag, &__v, buf);
    }
}

fn decode_scalar_into(place: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    let w = scalar_codec_ident(ty);
    quote! {
        if #tag == tag {
            ::proto_rs::encoding::#w::merge(wire_type, &mut #place, buf, ctx.clone())?;
        }
    }
}

fn encoded_len_scalar(val: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    let w = scalar_codec_ident(ty);
    quote! {
        ::proto_rs::encoding::#w::encoded_len(#tag, &#val)
    }
}

fn scalar_codec_ident(ty: &Type) -> Ident {
    // Map Rust type -> encoding module ident name used in your encoding.rs (int32,uint64,string,bytes,float,double,bool)
    let i = match ty {
        Type::Path(tp) => {
            if let Some(seg) = tp.path.segments.last() {
                let id = &seg.ident;
                match id.to_string().as_str() {
                    "i32" => "int32",
                    "i64" => "int64",
                    "u32" => "uint32",
                    "u64" => "uint64",
                    "bool" => "bool",
                    "f32" => "float",
                    "f64" => "double",
                    "String" => "string",
                    "Bytes" => "bytes",
                    "Vec" => "bytes", // Vec<u8> handled higher but safe fallback
                    _ => "string",    // message-like handled earlier; fallback
                }
            } else {
                "string"
            }
        }
        _ => "string",
    };
    Ident::new(i, Span::call_site())
}

// ————————————————————————————————————————————————————————————————————————
// Arrays (no temporary allocation).
// [u8; N] -> bytes. Other [T;N] -> repeated T (unpacked, prost will accept).

fn encode_array_no_alloc(fa: &TokenStream, tag: u32, arr: &syn::TypeArray) -> TokenStream {
    let elem = &*arr.elem;
    if is_bytes_array(&Type::Array(arr.clone())) {
        return quote! {
            // bytes: single length-delimited field
            ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
            ::proto_rs::encoding::encode_varint((#fa).len() as u64, buf);
            // direct slice view of array; no allocation
            buf.put_slice(&#fa[..]);
        };
    }

    // Non-u8 arrays: emit as repeated T (unpacked)
    let enc = scalar_codec_ident(elem);
    quote! {
        for __x in (#fa).iter() {
            ::proto_rs::encoding::#enc::encode(#tag, __x, buf);
        }
    }
}

fn decode_array_no_alloc(fa: &TokenStream, tag: u32, arr: &syn::TypeArray) -> TokenStream {
    // We need to fill a fixed-size array without realloc.
    // Strategy: decode into a local index; error if too many/too few.
    let elem = &*arr.elem;
    let enc = scalar_codec_ident(elem);
    quote! {
        if #tag == tag {
            let mut __i = 0usize;
            // accept packed and unpacked for numeric arrays
            match wire_type {
                ::proto_rs::encoding::WireType::LengthDelimited => {
                    let __len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                    let mut __limited = buf.take(__len);
                    while __limited.has_remaining() {
                        if __i >= (#fa).len() { return Err(::proto_rs::DecodeError::new("too many elements for fixed array")); }
                        let mut __tmp = ::core::default::Default::default();
                        ::proto_rs::encoding::#enc::merge(::proto_rs::encoding::WireType::Varint, &mut __tmp, &mut __limited, ctx.clone())?;
                        (#fa)[__i] = __tmp;
                        __i += 1;
                    }
                }
                _ => {
                    if __i >= (#fa).len() { return Err(::proto_rs::DecodeError::new("too many elements for fixed array")); }
                    let mut __tmp = ::core::default::Default::default();
                    ::proto_rs::encoding::#enc::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                    (#fa)[__i] = __tmp;
                    __i += 1;
                }
            }
        }
    }
}

fn encoded_len_array_no_alloc(fa: &TokenStream, tag: u32, arr: &syn::TypeArray) -> TokenStream {
    let elem = &*arr.elem;
    if is_bytes_array(&Type::Array(arr.clone())) {
        return quote! {
            let l = (#fa).len();
            ::proto_rs::encoding::encoded_len_key(#tag) + ::proto_rs::encoding::encoded_len_varint(l as u64) + l
        };
    }
    let enc = scalar_codec_ident(elem);
    quote! {
        {
            let mut __sum = 0usize;
            for __x in (#fa).iter() {
                __sum += ::proto_rs::encoding::#enc::encoded_len(#tag, __x);
            }
            __sum
        }
    }
}

// ————————————————————————————————————————————————————————————————————————
// Repeated (Vec<T>) — emit packed for numeric, unpacked for messages/strings.
// Decode accepts both packed and unpacked (prost-compatible).

fn encode_repeated(fa: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_numeric_scalar {
        let enc = scalar_codec_ident(&parsed.proto_rust_type);
        return quote! {
            if !(#fa).is_empty() {
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                // packed payload len
                let mut __len = 0usize;
                for __x in (#fa).iter() {
                    __len += ::proto_rs::encoding::#enc::encoded_len_no_tag(__x);
                }
                ::proto_rs::encoding::encode_varint(__len as u64, buf);
                for __x in (#fa).iter() {
                    ::proto_rs::encoding::#enc::encode_no_tag(__x, buf);
                }
            }
        };
    }

    // strings, bytes, messages: one element = one full key+value
    let enc = scalar_codec_ident(&parsed.proto_rust_type);
    quote! {
        for __x in (#fa).iter() {
            ::proto_rs::encoding::#enc::encode(#tag, __x, buf);
        }
    }
}

fn decode_repeated(fa: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let enc = scalar_codec_ident(&parsed.proto_rust_type);
    quote! {
        if #tag == tag {
            match wire_type {
                ::proto_rs::encoding::WireType::LengthDelimited if #parsed.is_numeric_scalar => {
                    // packed
                    let __len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                    let mut __limited = buf.take(__len);
                    while __limited.has_remaining() {
                        let mut __tmp = ::core::default::Default::default();
                        ::proto_rs::encoding::#enc::merge(::proto_rs::encoding::WireType::Varint, &mut __tmp, &mut __limited, ctx.clone())?;
                        (#fa).push(__tmp);
                    }
                }
                _ => {
                    // unpacked element
                    let mut __tmp = ::core::default::Default::default();
                    ::proto_rs::encoding::#enc::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                    (#fa).push(__tmp);
                }
            }
        }
    }
}

fn encoded_len_repeated(fa: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_numeric_scalar {
        let enc = scalar_codec_ident(&parsed.proto_rust_type);
        return quote! {
            if (#fa).is_empty() { 0 } else {
                let mut __payload = 0usize;
                for __x in (#fa).iter() {
                    __payload += ::proto_rs::encoding::#enc::encoded_len_no_tag(__x);
                }
                ::proto_rs::encoding::encoded_len_key(#tag) + ::proto_rs::encoding::encoded_len_varint(__payload as u64) + __payload
            }
        };
    }
    let enc = scalar_codec_ident(&parsed.proto_rust_type);
    quote! {
        {
            let mut __sum = 0usize;
            for __x in (#fa).iter() {
                __sum += ::proto_rs::encoding::#enc::encoded_len(#tag, __x);
            }
            __sum
        }
    }
}

// ————————————————————————————————————————————————————————————————————————
// Option<T>: if Some, encode just like T; prost elides None.

fn encode_option(fa: &TokenStream, tag: u32, _parsed: &ParsedFieldType) -> TokenStream {
    quote! {
        if let Some(__v) = (&self.#fa) {
            ::proto_rs::ProtoExt::encode_raw(__v, buf); // __v is T; the T-impl will write the correct key/tag
        }
    }
}
