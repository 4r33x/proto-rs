//! Handler for complex enums (with associated data) with `ProtoExt` support

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;
use syn::Lit;
use syn::spanned::Spanned;

use super::unified_field_handler::field_default_expr;
use super::unified_field_handler::generate_field_decode;
use super::unified_field_handler::generate_field_encode;
use super::unified_field_handler::generate_field_encoded_len;
use crate::utils::find_marked_default_variant;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;

pub fn handle_complex_enum(input: &DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let default_variant_index = match find_marked_default_variant(data) {
        Ok(Some(idx)) => idx,
        Ok(None) => 0,
        Err(err) => return err.to_compile_error(),
    };

    let mut original_variants: Vec<_> = data
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

    if default_variant_index != 0 {
        let default_variant = original_variants.remove(default_variant_index);
        original_variants.insert(0, default_variant);
    }

    // Collect variant data for encoding/decoding
    let (encode_arms, decode_arms, encoded_len_arms) = match generate_variant_arms(name, data) {
        Ok(parts) => parts,
        Err(err) => return err.to_compile_error(),
    };
    let default_variant = &data.variants[default_variant_index];
    let default_variant_ident = &default_variant.ident;

    let default_value = match &default_variant.fields {
        Fields::Unit => quote! { Self::#default_variant_ident },
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let default_value = field_default_expr(&fields.unnamed[0]);
            quote! { Self::#default_variant_ident(#default_value) }
        }
        Fields::Named(fields) => {
            let field_defaults: Vec<_> = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let default_value = field_default_expr(f);
                    quote! { #ident: #default_value }
                })
                .collect();
            quote! { Self::#default_variant_ident { #(#field_defaults),* } }
        }
        Fields::Unnamed(_) => {
            panic!("Complex enum variants must have exactly one unnamed field or multiple named fields")
        }
    };

    quote! {
        #(#attrs)*
        #vis enum #name #generics {
            #(#original_variants),*
        }

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

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            type Shadow<'a> = Self;

            #[inline(always)]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                #default_value
            }

            #[inline(always)]
            fn encoded_len(value: &::proto_rs::ViewOf<'_, Self>) -> usize {
                let value: &Self = *value;
                match value {
                    #(#encoded_len_arms)*
                }
            }

            #[inline(always)]
            fn encode_raw(value: ::proto_rs::ViewOf<'_, Self>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                let value: &Self = value;
                match value {
                    #(#encode_arms)*
                }
            }

            #[inline(always)]
            fn merge_field(
                shadow: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                use ::proto_rs::bytes::Buf;
                match tag {
                    #(#decode_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }
        }

        impl #generics ::proto_rs::MessageField for #name #generics {}

    }
}

fn generate_variant_arms(name: &syn::Ident, data: &DataEnum) -> syn::Result<(Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>)> {
    let mut encode_arms = Vec::new();
    let mut decode_arms = Vec::new();
    let mut encoded_len_arms = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = resolve_variant_tag(variant, idx + 1)?;
        let variant_ident = &variant.ident;

        match &variant.fields {
            Fields::Unit => {
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
                        *shadow = #name::#variant_ident;
                        Ok(())
                    }
                });

                encoded_len_arms.push(quote! {
                    #name::#variant_ident => {
                        ::proto_rs::encoding::key_len(#tag) + 1
                    }
                });
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let (encode_arm, decode_arm, encoded_len_arm) = generate_tuple_variant_arms(name, variant_ident, tag, &fields.unnamed[0])?;
                encode_arms.push(encode_arm);
                decode_arms.push(decode_arm);
                encoded_len_arms.push(encoded_len_arm);
            }
            Fields::Named(fields_named) => {
                let (encode_arm, decode_arm, encoded_len_arm) = generate_named_variant_arms(name, variant_ident, tag, fields_named)?;
                encode_arms.push(encode_arm);
                decode_arms.push(decode_arm);
                encoded_len_arms.push(encoded_len_arm);
            }
            Fields::Unnamed(_) => {
                panic!("Complex enum variants must have exactly one unnamed field or multiple named fields")
            }
        }
    }

    Ok((encode_arms, decode_arms, encoded_len_arms))
}

fn generate_tuple_variant_arms(name: &syn::Ident, variant_ident: &syn::Ident, tag: u32, field: &syn::Field) -> syn::Result<(TokenStream, TokenStream, TokenStream)> {
    let binding_ident = syn::Ident::new("inner", Span::call_site());
    let access_expr = quote! { #binding_ident };
    let cfg = parse_field_config(field);
    let field_ty = &field.ty;
    let parsed = parse_field_type(field_ty);
    if let Some(custom_tag) = cfg.custom_tag {
        if custom_tag == 0 {
            return Err(syn::Error::new(field.span(), "proto field tags must be greater than or equal to 1"));
        }

        if custom_tag != 1 {
            return Err(syn::Error::new(field.span(), "tuple enum fields cannot override their protobuf tag"));
        }
    }
    let is_array = matches!(field_ty, syn::Type::Array(_));
    let has_wrapper = parsed.is_option || parsed.is_repeated || parsed.map_kind.is_some() || parsed.set_kind.is_some() || is_array;
    let has_conversion = cfg.into_type.is_some() || cfg.into_fn.is_some() || cfg.from_type.is_some() || cfg.from_fn.is_some();
    let binding_default = field_default_expr(field);

    if !cfg.skip && !has_wrapper && !has_conversion {
        let encode_value = if parsed.is_message_like {
            quote! { ::proto_rs::encoding::message::encode::<#field_ty>(#tag, #access_expr, buf); }
        } else {
            let codec_ident = syn::Ident::new(&parsed.proto_type, Span::call_site());
            quote! { ::proto_rs::encoding::#codec_ident::encode(#tag, #access_expr, buf); }
        };

        let encode_arm = quote! {
            #name::#variant_ident(#binding_ident) => {
                #encode_value
            }
        };

        let decode_arm = quote! {
            #tag => {
                let mut #binding_ident = #binding_default;
                <#field_ty as ::proto_rs::SingularField>::merge_singular_field(
                    wire_type,
                    &mut #binding_ident,
                    buf,
                    ctx.clone(),
                )?;
                *shadow = #name::#variant_ident(#binding_ident);
                Ok(())
            }
        };

        let encoded_len_value = if parsed.is_message_like {
            quote! { ::proto_rs::encoding::message::encoded_len::<#field_ty>(#tag, &(#access_expr)) }
        } else {
            let codec_ident = syn::Ident::new(&parsed.proto_type, Span::call_site());
            quote! { ::proto_rs::encoding::#codec_ident::encoded_len(#tag, #access_expr) }
        };

        let encoded_len_arm = quote! {
            #name::#variant_ident(#binding_ident) => {
                #encoded_len_value
            }
        };

        return Ok((encode_arm, decode_arm, encoded_len_arm));
    }

    let binding_pattern_encode = if cfg.skip {
        quote! { _ }
    } else {
        quote! { #binding_ident }
    };

    let field_tag = match cfg.custom_tag.unwrap_or(1) {
        0 => {
            return Err(syn::Error::new(field.span(), "proto field tags must be greater than or equal to 1"));
        }
        value => value.try_into().unwrap(),
    };

    let mut post_hooks = Vec::new();
    if cfg.skip
        && let Some(fun) = &cfg.skip_deser_fn
    {
        let fun_path: syn::Path = syn::parse_str(fun).expect("invalid skip function path");
        let skip_binding = syn::Ident::new("__value", Span::call_site());
        let computed_ident = syn::Ident::new("__skip_value", Span::call_site());
        post_hooks.push(quote! {
            let #computed_ident = #fun_path(&variant_value);
            if let #name::#variant_ident(ref mut #skip_binding) = variant_value {
                *#skip_binding = #computed_ident;
            }
        });
    }

    let encoded_len_expr = generate_field_encoded_len(field, access_expr.clone(), field_tag);
    let encoded_len_expr_for_encode = encoded_len_expr.tokens.clone();
    let encoded_len_expr_for_len = encoded_len_expr.tokens.clone();

    let mut encode_fields = Vec::new();
    let mut decode_match = Vec::new();

    if !cfg.skip {
        encode_fields.push(generate_field_encode(field, access_expr.clone(), field_tag));
        let decode_body = generate_field_decode(field, access_expr.clone(), field_tag);
        decode_match.push(quote! {
            #field_tag => {
                let wire_type = field_wire_type;
                #decode_body
                Ok(())
            }
        });
    }

    let encode_arm = {
        let encode_body = encode_fields.clone();
        let msg_len_expr = encoded_len_expr_for_encode;
        quote! {
            #name::#variant_ident(#binding_pattern_encode) => {
                let msg_len = #msg_len_expr;
                ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
                #(#encode_body)*
            }
        }
    };

    let decode_loop = if decode_match.is_empty() {
        quote! {
            while buf.remaining() > limit {
                let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, ctx.clone())?;
            }
        }
    } else {
        quote! {
            while buf.remaining() > limit {
                let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                match field_tag {
                    #(#decode_match,)*
                    _ => {
                        ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, ctx.clone())?;
                        Ok(())
                    }
                }?;
            }
        }
    };

    let decode_arm = {
        let post_decode_hooks = post_hooks.clone();
        quote! {
            #tag => {
                let len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                let remaining = buf.remaining();
                if len > remaining {
                    return Err(::proto_rs::DecodeError::new("buffer underflow"));
                }
                let limit = remaining - len;

                let mut #binding_ident = #binding_default;

                #decode_loop

                let mut variant_value = #name::#variant_ident(#binding_ident);
                #(#post_decode_hooks)*
                *shadow = variant_value;
                Ok(())
            }
        }
    };

    let encoded_len_arm = {
        let msg_len_expr = encoded_len_expr_for_len;
        let binding_pattern_len = if cfg.skip || !encoded_len_expr.uses_access {
            quote! { _ }
        } else {
            quote! { #binding_ident }
        };
        quote! {
            #name::#variant_ident(#binding_pattern_len) => {
                let msg_len = #msg_len_expr;
                ::proto_rs::encoding::key_len(#tag)
                    + ::proto_rs::encoding::encoded_len_varint(msg_len as u64)
                    + msg_len
            }
        }
    };

    Ok((encode_arm, decode_arm, encoded_len_arm))
}

fn generate_named_variant_arms(name: &syn::Ident, variant_ident: &syn::Ident, tag: u32, fields_named: &syn::FieldsNamed) -> syn::Result<(TokenStream, TokenStream, TokenStream)> {
    let mut field_bindings = Vec::new();
    let mut field_bindings_encode = Vec::new();
    let mut field_bindings_len = Vec::new();
    let mut field_defaults = Vec::new();
    let mut encode_fields = Vec::new();
    let mut decode_match = Vec::new();
    let mut encoded_len_exprs = Vec::new();
    let mut post_hooks = Vec::new();

    for (index, field) in fields_named.named.iter().enumerate() {
        let ident = field.ident.as_ref().unwrap();
        let cfg = parse_field_config(field);
        let parsed = parse_field_type(&field.ty);
        let should_copy_scalar = !parsed.is_option
            && !parsed.is_repeated
            && parsed.map_kind.is_none()
            && parsed.set_kind.is_none()
            && !matches!(field.ty, syn::Type::Array(_))
            && (parsed.is_numeric_scalar || parsed.proto_type.as_str() == "bool");
        field_bindings.push(quote! { #ident });
        let field_binding_encode = if cfg.skip {
            quote! { #ident: _ }
        } else {
            quote! { #ident }
        };
        field_bindings_encode.push(field_binding_encode);
        let encode_access_expr = if should_copy_scalar {
            quote! { *#ident }
        } else {
            quote! { #ident }
        };
        let decode_access_expr = quote! { #ident };
        let field_tag = match cfg.custom_tag.unwrap_or(index + 1) {
            0 => {
                return Err(syn::Error::new(field.span(), "proto field tags must be greater than or equal to 1"));
            }
            value => value.try_into().unwrap(),
        };
        let default_expr = field_default_expr(field);
        field_defaults.push(quote! { let mut #ident = #default_expr; });

        let encoded_len_tokens = generate_field_encoded_len(field, encode_access_expr.clone(), field_tag);
        encoded_len_exprs.push(encoded_len_tokens.tokens.clone());
        let field_binding_len = if cfg.skip || !encoded_len_tokens.uses_access {
            quote! { #ident: _ }
        } else {
            quote! { #ident }
        };
        field_bindings_len.push(field_binding_len);

        if !cfg.skip {
            encode_fields.push(generate_field_encode(field, encode_access_expr.clone(), field_tag));
            let decode_body = generate_field_decode(field, decode_access_expr.clone(), field_tag);
            decode_match.push(quote! {
                #field_tag => {
                    let wire_type = field_wire_type;
                    #decode_body
                    Ok(())
                }
            });
        }

        if cfg.skip
            && let Some(fun) = &cfg.skip_deser_fn
        {
            let fun_path: syn::Path = syn::parse_str(fun).expect("invalid skip function path");
            let binding_name = format!("__{ident}_skip");
            let binding_ident = syn::Ident::new(&binding_name, Span::call_site());
            let computed_name = format!("__{ident}_computed");
            let computed_ident = syn::Ident::new(&computed_name, Span::call_site());
            post_hooks.push(quote! {
                let #computed_ident = #fun_path(&variant_value);
                if let #name::#variant_ident { #ident: ref mut #binding_ident, .. } = variant_value {
                    *#binding_ident = #computed_ident;
                }
            });
        }
    }

    let field_bindings_for_encode = field_bindings_encode;
    let field_bindings_for_decode = field_bindings.clone();
    let field_bindings_for_len = field_bindings_len;
    let encoded_len_exprs_for_encode = encoded_len_exprs.clone();
    let encoded_len_exprs_for_len = encoded_len_exprs.clone();
    let encode_fields_body = encode_fields.clone();
    let decode_match_arms = decode_match.clone();
    let post_decode_hooks = post_hooks.clone();

    let encode_arm = quote! {
        #name::#variant_ident { #(#field_bindings_for_encode),* } => {
            let msg_len = 0 #(+ #encoded_len_exprs_for_encode)*;
            ::proto_rs::encoding::encode_key(#tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
            ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
            #(#encode_fields_body)*
        }
    };
    let decode_loop = if decode_match_arms.is_empty() {
        quote! {
            while buf.remaining() > limit {
                let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, ctx.clone())?;
            }
        }
    } else {
        quote! {
            while buf.remaining() > limit {
                let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                match field_tag {
                    #(#decode_match_arms,)*
                    _ => {
                        ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, ctx.clone())?;
                        Ok(())
                    }
                }?;
            }
        }
    };

    let decode_arm = quote! {
        #tag => {
            let len = ::proto_rs::encoding::decode_varint(buf)? as usize;
            let remaining = buf.remaining();
            if len > remaining {
                return Err(::proto_rs::DecodeError::new("buffer underflow"));
            }

            let limit = remaining - len;
            #(#field_defaults)*

            #decode_loop

            let mut variant_value = #name::#variant_ident { #(#field_bindings_for_decode),* };
            #(#post_decode_hooks)*
            *shadow = variant_value;
            Ok(())
        }
    };
    let encoded_len_arm = quote! {
        #name::#variant_ident { #(#field_bindings_for_len),* } => {
            let msg_len = 0 #(+ #encoded_len_exprs_for_len)*;
            ::proto_rs::encoding::key_len(#tag)
                + ::proto_rs::encoding::encoded_len_varint(msg_len as u64)
                + msg_len
        }
    };
    Ok((encode_arm, decode_arm, encoded_len_arm))
}

fn resolve_variant_tag(variant: &syn::Variant, default: usize) -> syn::Result<u32> {
    let mut custom_tag = None;

    for attr in &variant.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.get_ident().is_some_and(|ident| ident == "tag") {
                if custom_tag.is_some() {
                    return Err(syn::Error::new(meta.path.span(), "duplicate proto(tag) attribute for variant"));
                }

                let lit: Lit = meta.value()?.parse()?;
                let value = match lit {
                    Lit::Int(int_lit) => int_lit.base10_parse::<usize>()?,
                    Lit::Str(str_lit) => str_lit.value().parse::<usize>().map_err(|_| syn::Error::new(str_lit.span(), "proto tag must be a positive integer"))?,
                    _ => {
                        return Err(syn::Error::new(lit.span(), "proto tag must be specified as an integer"));
                    }
                };

                custom_tag = Some(value);
            }

            Ok(())
        })?;
    }

    let tag = custom_tag.unwrap_or(default);
    if tag == 0 {
        return Err(syn::Error::new(variant.ident.span(), "proto enum variant tags must be greater than or equal to 1"));
    }

    Ok(tag.try_into().unwrap())
}
