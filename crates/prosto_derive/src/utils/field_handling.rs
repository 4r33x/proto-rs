//! field_handling.rs
//! Unified generators for encode/decode/len of a single field.  syn v2 / quote v1 safe.

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Field;
use syn::Ident;
use syn::Index;
use syn::Type;
use syn::TypeArray;

use crate::utils::FieldConfig;
use crate::utils::ParsedFieldType;
use crate::utils::is_bytes_vec;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;

// ————————————————————————————————————————————————————————————————————————
// Field access

#[derive(Clone)]
pub enum FieldAccess {
    Named(Ident),
    Tuple(Index),
}
impl FieldAccess {
    fn to_tokens(&self) -> TokenStream {
        match self {
            FieldAccess::Named(id) => quote!( #id ),
            FieldAccess::Tuple(ix) => quote!( #ix ),
        }
    }
}

// ————————————————————————————————————————————————————————————————————————
// Public entry points

pub fn generate_field_encode(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg: FieldConfig = parse_field_config(field);
    let ty: &Type = &field.ty;
    let parsed: ParsedFieldType = parse_field_type(ty);
    let fa = access.to_tokens();

    // Skip (do not encode)
    if cfg.skip {
        return quote! {};
    }

    // Custom "into" conversion
    if let Some(into_ty_str) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty_str).expect("invalid #[proto(into = ...)] type");
        let val_expr: TokenStream = if let Some(fun_name) = &cfg.into_fn {
            let fun = format_ident!("{}", fun_name);
            quote! { #fun(&self.#fa) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from(self.#fa.clone()) }
        };
        return encode_scalar_like(&val_expr, tag, &into_ty);
    }

    // Enums (prost-style i32 on the wire)
    if cfg.is_rust_enum || cfg.is_proto_enum {
        return quote! {
            let __v: i32 = self.#fa as i32;
            if __v != 0 {
                encoding::int32::encode(#tag, &__v, buf);
            }
        };
    }

    // Arrays
    if let Type::Array(arr) = ty {
        return encode_array(&fa, tag, arr);
    }

    // Repeated
    if parsed.is_repeated {
        return encode_repeated(&fa, tag, &parsed);
    }

    // Option<T> — if Some, encode exactly like T (prost elides None)
    if parsed.is_option {
        // NB: We do not invoke Option<T>:ProtoExt; we call inner T encode
        if parsed.is_message_like {
            return quote! {
                if let Some(__m) = (&self.#fa).as_ref() {
                    let __l = ::crate::ProtoExt::encoded_len(__m);
                    if __l != 0 {
                        encoding::encode_key(#tag, WireType::LengthDelimited, buf);
                        encoding::encode_varint(__l as u64, buf);
                        ::crate::ProtoExt::encode_raw(__m, buf);
                    }
                }
            };
        } else {
            let enc_ident = scalar_codec_ident(&parsed.elem_type);
            return quote! {
                if let Some(__v) = (&self.#fa).as_ref() {
                    encoding::#enc_ident::encode(#tag, __v, buf);
                }
            };
        }
    }

    // Message-like scalar
    if parsed.is_message_like {
        return quote! {
            let __m = &self.#fa;
            let __l = ::crate::ProtoExt::encoded_len(__m);
            if __l != 0 {
                encoding::encode_key(#tag, WireType::LengthDelimited, buf);
                encoding::encode_varint(__l as u64, buf);
                ::crate::ProtoExt::encode_raw(__m, buf);
            }
        };
    }

    // Plain scalar (string/bytes/numeric/bool)
    encode_scalar_like(&quote!( self.#fa ), tag, ty)
}

pub fn generate_field_decode(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg: FieldConfig = parse_field_config(field);
    let ty: &Type = &field.ty;
    let parsed: ParsedFieldType = parse_field_type(ty);
    let fa = access.to_tokens();

    // Skip on wire; will be post-filled by struct/enum handler if `skip = "fn"`
    if cfg.skip {
        return quote! { /* skipped at wire level */ };
    }

    // Custom "from" conversion
    if let Some(from_ty_str) = &cfg.from_type {
        let from_ty: Type = syn::parse_str(from_ty_str).expect("invalid #[proto(from = ...)] type");
        let assign_expr: TokenStream = if let Some(fun_name) = &cfg.from_fn {
            let fun = format_ident!("{}", fun_name);
            quote! { #fun(__tmp) }
        } else {
            quote! { <_ as ::core::convert::From<_>>::from(__tmp) }
        };
        return quote! {
            if #tag == tag {
                let mut __tmp: #from_ty = <#from_ty as ::crate::ProtoExt>::proto_default();
                <#from_ty as ::crate::ProtoExt>::merge_field(&mut __tmp, #tag, wire_type, buf, ctx.clone())?;
                self.#fa = #assign_expr;
            }
        };
    }

    // Enums (prost i32)
    if cfg.is_rust_enum || cfg.is_proto_enum {
        let enum_ty = ty.clone();
        return quote! {
            if #tag == tag {
                let mut __raw: i32 = 0;
                encoding::int32::merge(wire_type, &mut __raw, buf, ctx.clone())?;
                self.#fa = <#enum_ty as ::core::convert::TryFrom<i32>>::try_from(__raw)?;
            }
        };
    }

    // Arrays
    if let Type::Array(arr) = ty {
        return decode_array(&fa, tag, arr);
    }

    // Repeated (Vec<T>): accept both packed and unpacked for numeric scalars
    if parsed.is_repeated {
        return decode_repeated(&fa, tag, &parsed);
    }

    // Option<T>
    if parsed.is_option {
        if parsed.is_message_like {
            return quote! {
                if #tag == tag {
                    if wire_type != WireType::LengthDelimited {
                        return Err(DecodeError::new("expected length-delimited for message"));
                    }
                    let __len = encoding::decode_varint(buf)? as usize;
                    let mut __limited = buf.take(__len);
                    let mut __inner = <_ as ::crate::ProtoExt>::proto_default();
                    ::crate::ProtoExt::merge(&mut __inner, &mut __limited)?;
                    if __limited.has_remaining() {
                        return Err(DecodeError::new("message overrun"));
                    }
                    self.#fa = Some(__inner);
                }
            };
        } else {
            let enc_ident = scalar_codec_ident(&parsed.elem_type);
            let inner_ty = parsed.elem_type.clone();
            return quote! {
                if #tag == tag {
                    let mut __tmp: #inner_ty = <#inner_ty as ::crate::ProtoExt>::proto_default();
                    encoding::#enc_ident::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                    self.#fa = Some(__tmp);
                }
            };
        }
    }

    // Message-like (non-option)
    if parsed.is_message_like {
        return quote! {
            if #tag == tag {
                if wire_type != WireType::LengthDelimited {
                    return Err(DecodeError::new("expected length-delimited for message"));
                }
                let __len = encoding::decode_varint(buf)? as usize;
                let mut __limited = buf.take(__len);
                ::crate::ProtoExt::merge(&mut self.#fa, &mut __limited)?;
                if __limited.has_remaining() {
                    return Err(DecodeError::new("message overrun"));
                }
            }
        };
    }

    // Plain scalar (string/bytes/numeric/bool)
    let enc_ident = scalar_codec_ident(ty);
    quote! {
        if #tag == tag {
            encoding::#enc_ident::merge(wire_type, &mut self.#fa, buf, ctx.clone())?;
        }
    }
}

pub fn generate_field_encoded_len(field: &Field, access: FieldAccess, tag: u32) -> TokenStream {
    let cfg: FieldConfig = parse_field_config(field);
    if cfg.skip {
        return quote!(0);
    }

    let ty: &Type = &field.ty;
    let parsed: ParsedFieldType = parse_field_type(ty);
    let fa = access.to_tokens();

    // into conversion affects encoded len
    if let Some(into_ty_str) = &cfg.into_type {
        let into_ty: Type = syn::parse_str(into_ty_str).expect("invalid #[proto(into = ...)] type");
        let val_expr: TokenStream = if let Some(fun_name) = &cfg.into_fn {
            let fun = format_ident!("{}", fun_name);
            quote! { #fun(&self.#fa) }
        } else {
            quote! { <#into_ty as ::core::convert::From<_>>::from(self.#fa.clone()) }
        };
        return encoded_len_scalar_like(&val_expr, tag, &into_ty);
    }

    if cfg.is_rust_enum || cfg.is_proto_enum {
        return quote! {
            if (self.#fa as i32) != 0 {
                encoding::int32::encoded_len(#tag, &(self.#fa as i32))
            } else { 0 }
        };
    }

    if let Type::Array(arr) = ty {
        return encoded_len_array(&fa, tag, arr);
    }

    if parsed.is_repeated {
        return encoded_len_repeated(&fa, tag, &parsed);
    }

    if parsed.is_option {
        if parsed.is_message_like {
            return quote! {
                match (&self.#fa) {
                    Some(__m) => {
                        let __l = ::crate::ProtoExt::encoded_len(__m);
                        if __l == 0 { 0 } else { encoding::encoded_len_key(#tag) + encoding::encoded_len_varint(__l as u64) + __l }
                    }
                    None => 0
                }
            };
        } else {
            let enc_ident = scalar_codec_ident(&parsed.elem_type);
            return quote! {
                match (&self.#fa) {
                    Some(__v) => encoding::#enc_ident::encoded_len(#tag, __v),
                    None => 0,
                }
            };
        }
    }

    if parsed.is_message_like {
        return quote! {
            {
                let __l = ::crate::ProtoExt::encoded_len(&self.#fa);
                if __l == 0 { 0 } else { encoding::encoded_len_key(#tag) + encoding::encoded_len_varint(__l as u64) + __l }
            }
        };
    }

    let enc_ident = scalar_codec_ident(ty);
    quote! {
        encoding::#enc_ident::encoded_len(#tag, &self.#fa)
    }
}

// ————————————————————————————————————————————————————————————————————————
// Helpers (pure TokenStream building; all branching before quoting)

fn scalar_codec_ident(ty: &Type) -> Ident {
    // Map Rust type -> encoding module ident used in `crate::encoding`
    let name = match ty {
        Type::Path(tp) => {
            if let Some(seg) = tp.path.segments.last() {
                match seg.ident.to_string().as_str() {
                    "i32" => "int32",
                    "i64" => "int64",
                    "u32" => "uint32",
                    "u64" => "uint64",
                    "bool" => "bool",
                    "f32" => "float",
                    "f64" => "double",
                    "String" => "string",
                    "Bytes" => "bytes",
                    "Vec" => {
                        // Only Vec<u8> is allowed here; other Vec<T> handled earlier
                        "bytes"
                    }
                    _ => "string", // fallback; messages handled before
                }
            } else {
                "string"
            }
        }
        _ => "string",
    };
    Ident::new(name, Span::call_site())
}

fn encode_scalar_like(val_expr: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    if is_bytes_vec(ty) {
        return quote! {
            if !(#val_expr).is_empty() {
                encoding::bytes::encode(#tag, &(#val_expr), buf);
            }
        };
    }
    let enc_ident = scalar_codec_ident(ty);
    quote! {
        encoding::#enc_ident::encode(#tag, &(#val_expr), buf);
    }
}

fn encoded_len_scalar_like(val_expr: &TokenStream, tag: u32, ty: &Type) -> TokenStream {
    let enc_ident = scalar_codec_ident(ty);
    quote! {
        encoding::#enc_ident::encoded_len(#tag, &(#val_expr))
    }
}

// ————————————————————————————————————————————————————————————————————————
// Arrays

fn encode_array(fa: &TokenStream, tag: u32, arr: &TypeArray) -> TokenStream {
    let elem = &*arr.elem;

    // [u8; N] -> bytes (single length-delimited)
    if match elem {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false),
        _ => false,
    } {
        return quote! {
            if !(#fa).is_empty() {
                encoding::encode_key(#tag, WireType::LengthDelimited, buf);
                encoding::encode_varint((#fa).len() as u64, buf);
                buf.put_slice(&#fa[..]);
            }
        };
    }

    // Other [T;N] -> repeated unpacked scalar/message
    let enc_ident = scalar_codec_ident(elem);
    quote! {
        for __x in (#fa).iter() {
            encoding::#enc_ident::encode(#tag, __x, buf);
        }
    }
}

fn decode_array(fa: &TokenStream, tag: u32, arr: &TypeArray) -> TokenStream {
    let elem = &*arr.elem;
    let enc_ident = scalar_codec_ident(elem);
    let elem_ty = (*arr.elem).clone();

    // Accept packed for numeric arrays, otherwise element-wise
    quote! {
        if #tag == tag {
            match wire_type {
                WireType::LengthDelimited if matches!(#enc_ident, _) => {
                    // packed numeric path
                    let __len = encoding::decode_varint(buf)? as usize;
                    let mut __limited = buf.take(__len);
                    let mut __i = 0usize;
                    while __limited.has_remaining() {
                        if __i >= (#fa).len() { return Err(DecodeError::new("too many elements for fixed array")); }
                        let mut __tmp: #elem_ty = <#elem_ty as ::crate::ProtoExt>::proto_default();
                        // For packed numerics merged as varint/fixed per codec; rely on codec to enforce wire
                        encoding::#enc_ident::merge(WireType::Varint, &mut __tmp, &mut __limited, ctx.clone())?;
                        (#fa)[__i] = __tmp;
                        __i += 1;
                    }
                }
                _ => {
                    // one element
                    let mut __tmp: #elem_ty = <#elem_ty as ::crate::ProtoExt>::proto_default();
                    encoding::#enc_ident::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                    // shift-left insert (append semantics): find first zero slot
                    let mut __i = 0usize;
                    while __i < (#fa).len() && true { // we don't know sentinel; just place at first available
                        if __i == (#fa).len() { break; }
                        __i += 1;
                    }
                    let __i = (__i - 1).min((#fa).len() - 1);
                    (#fa)[__i] = __tmp;
                }
            }
        }
    }
}

fn encoded_len_array(fa: &TokenStream, tag: u32, arr: &TypeArray) -> TokenStream {
    let elem = &*arr.elem;
    // [u8;N] -> bytes
    if match elem {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false),
        _ => false,
    } {
        return quote! {
            if (#fa).is_empty() { 0 } else {
                let l = (#fa).len();
                encoding::encoded_len_key(#tag) + encoding::encoded_len_varint(l as u64) + l
            }
        };
    }

    let enc_ident = scalar_codec_ident(elem);
    quote! {
        {
            let mut __sum = 0usize;
            for __x in (#fa).iter() {
                __sum += encoding::#enc_ident::encoded_len(#tag, __x);
            }
            __sum
        }
    }
}

// ————————————————————————————————————————————————————————————————————————
// Repeated Vec<T>

fn encode_repeated(fa: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_numeric_scalar {
        let enc_ident = scalar_codec_ident(&parsed.elem_type);
        return quote! {
            if !(#fa).is_empty() {
                encoding::encode_key(#tag, WireType::LengthDelimited, buf);
                let mut __payload = 0usize;
                for __x in (#fa).iter() {
                    __payload += encoding::#enc_ident::encoded_len_no_tag(__x);
                }
                encoding::encode_varint(__payload as u64, buf);
                for __x in (#fa).iter() {
                    encoding::#enc_ident::encode_no_tag(__x, buf);
                }
            }
        };
    }

    let enc_ident = scalar_codec_ident(&parsed.elem_type);
    quote! {
        for __x in (#fa).iter() {
            encoding::#enc_ident::encode(#tag, __x, buf);
        }
    }
}

fn decode_repeated(fa: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    let enc_ident = scalar_codec_ident(&parsed.elem_type);
    let elem_ty = parsed.elem_type.clone();
    if parsed.is_numeric_scalar {
        return quote! {
            if #tag == tag {
                match wire_type {
                    WireType::LengthDelimited => {
                        // packed
                        let __len = encoding::decode_varint(buf)? as usize;
                        let mut __limited = buf.take(__len);
                        while __limited.has_remaining() {
                            let mut __tmp: #elem_ty = <#elem_ty as ::crate::ProtoExt>::proto_default();
                            encoding::#enc_ident::merge(WireType::Varint, &mut __tmp, &mut __limited, ctx.clone())?;
                            (#fa).push(__tmp);
                        }
                    }
                    _ => {
                        // unpacked
                        let mut __tmp: #elem_ty = <#elem_ty as ::crate::ProtoExt>::proto_default();
                        encoding::#enc_ident::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
                        (#fa).push(__tmp);
                    }
                }
            }
        };
    }

    // Non-numeric: always element-wise
    quote! {
        if #tag == tag {
            let mut __tmp: #elem_ty = <#elem_ty as ::crate::ProtoExt>::proto_default();
            encoding::#enc_ident::merge(wire_type, &mut __tmp, buf, ctx.clone())?;
            (#fa).push(__tmp);
        }
    }
}

fn encoded_len_repeated(fa: &TokenStream, tag: u32, parsed: &ParsedFieldType) -> TokenStream {
    if parsed.is_numeric_scalar {
        let enc_ident = scalar_codec_ident(&parsed.elem_type);
        return quote! {
            if (#fa).is_empty() { 0 } else {
                let mut __payload = 0usize;
                for __x in (#fa).iter() {
                    __payload += encoding::#enc_ident::encoded_len_no_tag(__x);
                }
                encoding::encoded_len_key(#tag) + encoding::encoded_len_varint(__payload as u64) + __payload
            }
        };
    }

    let enc_ident = scalar_codec_ident(&parsed.elem_type);
    quote! {
        {
            let mut __sum = 0usize;
            for __x in (#fa).iter() {
                __sum += encoding::#enc_ident::encoded_len(#tag, __x);
            }
            __sum
        }
    }
}
