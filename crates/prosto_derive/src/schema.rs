use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;
use syn::DataEnum;
use syn::Field;
use syn::Fields;
use syn::GenericParam;
use syn::Type;

use crate::parse::UnifiedProtoConfig;
use crate::utils::MethodInfo;
use crate::utils::ParsedFieldType;
use crate::utils::collect_discriminants_for_variants;
use crate::utils::derive_package_name;
use crate::utils::extract_field_wrapper_info;
use crate::utils::find_marked_default_variant;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::proto_type_name;
use crate::utils::resolved_field_type;
use crate::utils::to_pascal_case;
use crate::utils::to_upper_snake_case;

pub fn assoc_proto_ident_const(
    config: &UnifiedProtoConfig,
    type_ident: &syn::Ident,
    generics: &syn::Generics,
    proto_names: &[String],
    generic_variants: &[crate::parse::GenericTypeVariant],
) -> TokenStream2 {
    let proto_name_base = proto_names.first().map_or_else(|| type_ident.to_string(), ToString::to_string);
    let (proto_package, proto_file_path) = config.proto_path().map_or_else(
        || (String::new(), String::new()),
        |path| {
            let file_name = std::path::Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path);
            (derive_package_name(file_name), path.to_string())
        },
    );

    let proto_package = proto_package.clone();
    let proto_file_path = proto_file_path.clone();
    let type_name_literal = type_ident.to_string();
    let proto_ident_literal = |proto_name_literal: &String| {
        quote! {
            ::proto_rs::schemas::ProtoIdent {
                module_path: ::core::module_path!(),
                name: #type_name_literal,
                proto_package_name: #proto_package,
                proto_file_path: #proto_file_path,
                proto_type: #proto_name_literal,
            }
        }
    };
    let trait_impl = |impl_generics: &TokenStream2, type_tokens: &TokenStream2, where_clause: &TokenStream2, proto_name_literal: &String| {
        let proto_ident = proto_ident_literal(proto_name_literal);
        quote! {
            #[cfg(feature = "build-schemas")]
            impl #impl_generics ::proto_rs::schemas::ProtoIdentifiable for #type_tokens #where_clause {
                const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = #proto_ident;
            }
        }
    };
    let inherent_impl = |impl_generics: &TokenStream2, type_tokens: &TokenStream2, where_clause: &TokenStream2, proto_name_literal: &String| {
        let proto_ident = proto_ident_literal(proto_name_literal);
        quote! {
            #[cfg(feature = "build-schemas")]
            impl #impl_generics #type_tokens #where_clause {
                pub const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = #proto_ident;
            }
        }
    };

    if config.generic_types.is_empty() {
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let impl_generics_tokens = quote! { #impl_generics };
        let where_clause_tokens = where_clause.map_or_else(TokenStream2::new, |clause| quote! { #clause });
        let proto_name_literal = proto_name_base.clone();
        let type_tokens = quote! { #type_ident #ty_generics };
        let proto_traits = trait_impl(&impl_generics_tokens, &type_tokens, &where_clause_tokens, &proto_name_literal);
        let proto_inherent = inherent_impl(&impl_generics_tokens, &type_tokens, &where_clause_tokens, &proto_name_literal);
        let sun_trait_impls = build_sun_trait_impls(config, &impl_generics_tokens, &where_clause_tokens, &proto_name_literal, &proto_ident_literal);
        return quote! {
            #proto_inherent
            #proto_traits
            #sun_trait_impls
        };
    }

    let impl_params: Vec<_> = generics.params.iter().filter(|param| !matches!(param, GenericParam::Type(_))).collect();
    let impl_generics = if impl_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#impl_params),*> }
    };

    let mut variant_tokens = Vec::new();
    for variant in generic_variants {
        let proto_name_literal = if variant.suffix.is_empty() {
            proto_name_base.clone()
        } else {
            format!("{proto_name_base}{}", variant.suffix)
        };

        let mut type_args = Vec::new();
        for param in &generics.params {
            match param {
                GenericParam::Type(type_param) => {
                    let ty = variant.substitutions.get(&type_param.ident.to_string()).expect("missing generic type substitution");
                    type_args.push(quote! { #ty });
                }
                GenericParam::Lifetime(lifetime_def) => {
                    let lifetime = &lifetime_def.lifetime;
                    type_args.push(quote! { #lifetime });
                }
                GenericParam::Const(const_param) => {
                    let ident = &const_param.ident;
                    type_args.push(quote! { #ident });
                }
            }
        }

        let type_tokens = if type_args.is_empty() {
            quote! { #type_ident }
        } else {
            quote! { #type_ident <#(#type_args),*> }
        };

        let empty_where_clause = TokenStream2::new();
        let proto_inherent = inherent_impl(&impl_generics, &type_tokens, &empty_where_clause, &proto_name_literal);
        let proto_traits = trait_impl(&impl_generics, &type_tokens, &empty_where_clause, &proto_name_literal);
        variant_tokens.push(quote! {
            #proto_inherent
            #proto_traits
        });
    }

    let empty_where_clause = TokenStream2::new();
    let sun_trait_impls = build_sun_trait_impls(config, &impl_generics, &empty_where_clause, &proto_name_base, &proto_ident_literal);
    quote! {
        #(#variant_tokens)*
        #sun_trait_impls
    }
}

fn build_sun_trait_impls(
    config: &UnifiedProtoConfig,
    impl_generics: &TokenStream2,
    where_clause: &TokenStream2,
    proto_name_literal: &String,
    proto_ident_literal: &impl Fn(&String) -> TokenStream2,
) -> TokenStream2 {
    if !config.has_suns() {
        return quote! {};
    }

    let proto_ident = proto_ident_literal(proto_name_literal);
    let sun_impls: Vec<_> = config
        .suns
        .iter()
        .map(|sun| {
            let sun_ty = &sun.ty;
            quote! {
                #[cfg(feature = "build-schemas")]
                impl #impl_generics ::proto_rs::schemas::ProtoIdentifiable for #sun_ty #where_clause {
                    const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = #proto_ident;
                }
            }
        })
        .collect();

    quote! { #(#sun_impls)* }
}

pub fn schema_tokens_for_struct(type_ident: &syn::Ident, message_name: &str, fields: &Fields, config: &UnifiedProtoConfig, const_suffix: &str) -> TokenStream2 {
    let fields_tokens = build_fields_tokens(type_ident, const_suffix, fields, config);
    let field_consts = fields_tokens.consts;
    let field_refs = fields_tokens.refs;
    let entry_tokens = quote! {
        ::proto_rs::schemas::ProtoEntry::Struct {
            fields: #field_refs,
        }
    };

    build_schema_tokens(type_ident, message_name, config, const_suffix, entry_tokens, field_consts)
}

pub fn schema_tokens_for_simple_enum(type_ident: &syn::Ident, message_name: &str, data: &DataEnum, config: &UnifiedProtoConfig, const_suffix: &str) -> TokenStream2 {
    let marked_default = find_marked_default_variant(data).unwrap_or_else(|err| panic!("{err}"));
    let mut order: Vec<usize> = (0..data.variants.len()).collect();
    if let Some(idx) = marked_default
        && idx < order.len()
    {
        order.remove(idx);
        order.insert(0, idx);
    }
    let ordered_variants: Vec<&syn::Variant> = order.iter().map(|&idx| &data.variants[idx]).collect();
    let ordered_discriminants = collect_discriminants_for_variants(&ordered_variants).unwrap_or_else(|err| panic!("{err}"));

    let mut variant_consts = Vec::new();
    let mut variant_refs = Vec::new();

    for (idx, (variant, value)) in ordered_variants.iter().zip(ordered_discriminants.iter()).enumerate() {
        let variant_const = variant_const_ident(type_ident, const_suffix, idx);
        let name = to_upper_snake_case(&variant.ident.to_string());
        let value = *value;
        variant_consts.push(quote! {
            #[cfg(feature = "build-schemas")]
            const #variant_const: ::proto_rs::schemas::Variant = ::proto_rs::schemas::Variant {
                name: #name,
                fields: &[],
                discriminant: Some(#value),
            };
        });
        variant_refs.push(quote! { &#variant_const });
    }

    let variant_refs = quote! { &[#(#variant_refs),*] };

    let variant_consts = quote! { #(#variant_consts)* };
    let entry_tokens = quote! {
        ::proto_rs::schemas::ProtoEntry::SimpleEnum {
            variants: #variant_refs,
        }
    };

    build_schema_tokens(type_ident, message_name, config, const_suffix, entry_tokens, variant_consts)
}

pub fn schema_tokens_for_complex_enum(type_ident: &syn::Ident, message_name: &str, data: &DataEnum, config: &UnifiedProtoConfig, const_suffix: &str) -> TokenStream2 {
    let mut variant_consts = Vec::new();
    let mut variant_refs = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let variant_const = variant_const_ident(type_ident, const_suffix, idx);
        let variant_name = variant.ident.to_string();
        let fields_tokens = build_variant_fields_tokens(type_ident, const_suffix, idx, &variant.fields, config);
        let field_consts = fields_tokens.consts;
        let field_refs = fields_tokens.refs;

        variant_consts.push(quote! {
            #[cfg(feature = "build-schemas")]
            const #variant_const: ::proto_rs::schemas::Variant = ::proto_rs::schemas::Variant {
                name: #variant_name,
                fields: #field_refs,
                discriminant: None,
            };
            #field_consts
        });
        variant_refs.push(quote! { &#variant_const });
    }

    let variant_consts = quote! { #(#variant_consts)* };
    let variant_refs = quote! { &[#(#variant_refs),*] };
    let entry_tokens = quote! {
        ::proto_rs::schemas::ProtoEntry::ComplexEnum {
            variants: #variant_refs,
        }
    };

    build_schema_tokens(type_ident, message_name, config, const_suffix, entry_tokens, variant_consts)
}

pub fn schema_tokens_for_service(type_ident: &syn::Ident, service_name: &str, methods: &[MethodInfo], config: &UnifiedProtoConfig, const_suffix: &str) -> TokenStream2 {
    let methods_tokens = build_service_method_tokens(type_ident, const_suffix, methods);
    let method_consts = methods_tokens.consts;
    let method_refs = methods_tokens.refs;
    let entry_tokens = quote! {
        ::proto_rs::schemas::ProtoEntry::Service {
            methods: #method_refs,
        }
    };

    build_schema_tokens(type_ident, service_name, config, const_suffix, entry_tokens, method_consts)
}

pub fn schema_tokens_for_imports(type_ident: &str, file_name: &str, imports: &[String]) -> TokenStream2 {
    let suffix = format!("{}_{}", sanitize_ident(type_ident), sanitize_ident(file_name));
    let schema_ident = format_ident(&format!("PROTO_SCHEMA_IMPORTS_{suffix}"));
    let const_name = format_ident(&format!("PROTO_SCHEMA_IMPORT_PATHS_{suffix}"));
    let import_literals: Vec<_> = imports.iter().map(|imp| quote! { #imp }).collect();
    let file_name_literal = file_name;
    let package_name = std::path::Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .map_or(derive_package_name(file_name), derive_package_name);

    quote! {
        #[cfg(feature = "build-schemas")]
        const #const_name: &[&str] = &[#(#import_literals),*];

        #[cfg(feature = "build-schemas")]
        pub const #schema_ident: ::proto_rs::schemas::ProtoSchema = ::proto_rs::schemas::ProtoSchema {
            id: ::proto_rs::schemas::ProtoIdent {
                module_path: ::core::module_path!(),
                name: #type_ident,
                proto_package_name: #package_name,
                proto_file_path: #file_name_literal,
                proto_type: "Import",
            },
            generics: &[],
            lifetimes: &[],
            top_level_attributes: &[],
            content: ::proto_rs::schemas::ProtoEntry::Import {
                paths: #const_name,
            },
        };

        #[cfg(feature = "build-schemas")]
        inventory::submit! {
            #schema_ident
        }
    }
}

struct FieldTokens {
    consts: TokenStream2,
    refs: TokenStream2,
}

struct ServiceMethodTokens {
    consts: TokenStream2,
    refs: TokenStream2,
}

struct GenericTokens {
    consts: TokenStream2,
    refs: TokenStream2,
}

struct LifetimeTokens {
    consts: TokenStream2,
    refs: TokenStream2,
}

struct AttributeTokens {
    consts: TokenStream2,
    refs: TokenStream2,
}

struct FieldConstTokens {
    consts: TokenStream2,
    refs: TokenStream2,
}

fn build_schema_tokens(type_ident: &syn::Ident, proto_type: &str, config: &UnifiedProtoConfig, const_suffix: &str, entry_tokens: TokenStream2, extra_consts: TokenStream2) -> TokenStream2 {
    let (proto_package, proto_file_path) = proto_path_info(config);
    let schema_ident = schema_ident(type_ident, const_suffix);
    let generics_tokens = build_generics_tokens(type_ident, const_suffix, config);
    let lifetimes_tokens = build_lifetime_tokens(type_ident, const_suffix, config);
    let attrs_tokens = build_attribute_tokens(type_ident, const_suffix, &config.item_attrs);

    let generics_consts = generics_tokens.consts;
    let generics_refs = generics_tokens.refs;
    let lifetime_consts = lifetimes_tokens.consts;
    let lifetime_refs = lifetimes_tokens.refs;
    let attrs_consts = attrs_tokens.consts;
    let attrs_refs = attrs_tokens.refs;

    quote! {
        #[cfg(feature = "build-schemas")]
        pub const #schema_ident: ::proto_rs::schemas::ProtoSchema = ::proto_rs::schemas::ProtoSchema {
            id: ::proto_rs::schemas::ProtoIdent {
                module_path: ::core::module_path!(),
                name: stringify!(#type_ident),
                proto_package_name: #proto_package,
                proto_file_path: #proto_file_path,
                proto_type: #proto_type,
            },
            generics: #generics_refs,
            lifetimes: #lifetime_refs,
            top_level_attributes: #attrs_refs,
            content: #entry_tokens,
        };

        #[cfg(feature = "build-schemas")]
        inventory::submit! {
            #schema_ident
        }

        #generics_consts
        #lifetime_consts
        #attrs_consts
        #extra_consts
    }
}
fn build_fields_tokens(type_ident: &syn::Ident, suffix: &str, fields: &Fields, config: &UnifiedProtoConfig) -> FieldTokens {
    match fields {
        Fields::Named(named) => build_named_fields_tokens(type_ident, suffix, &named.named, config),
        Fields::Unnamed(unnamed) => build_unnamed_fields_tokens(type_ident, suffix, &unnamed.unnamed, config),
        Fields::Unit => FieldTokens {
            consts: quote! {},
            refs: quote! { &[] },
        },
    }
}

fn build_service_method_tokens(type_ident: &syn::Ident, suffix: &str, methods: &[MethodInfo]) -> ServiceMethodTokens {
    let mut method_consts = Vec::new();
    let mut method_refs = Vec::new();

    for (idx, method) in methods.iter().enumerate() {
        let method_ident = service_method_const_ident(type_ident, suffix, idx);
        let method_name = to_pascal_case(&method.name.to_string());
        let request_ident = proto_ident_tokens_from_type(&method.request_type);
        let response_type = method.inner_response_type.as_ref().unwrap_or(&method.response_type);
        let response_ident = proto_ident_tokens_from_type(response_type);
        let server_streaming = method.is_streaming;

        method_consts.push(quote! {
            #[cfg(feature = "build-schemas")]
            const #method_ident: ::proto_rs::schemas::ServiceMethod = ::proto_rs::schemas::ServiceMethod {
                name: #method_name,
                request: #request_ident,
                response: #response_ident,
                client_streaming: false,
                server_streaming: #server_streaming,
            };
        });
        method_refs.push(quote! { &#method_ident });
    }

    ServiceMethodTokens {
        consts: quote! { #(#method_consts)* },
        refs: quote! { &[#(#method_refs),*] },
    }
}

fn build_generics_tokens(type_ident: &syn::Ident, suffix: &str, config: &UnifiedProtoConfig) -> GenericTokens {
    let mut generic_consts = Vec::new();
    let mut generic_refs = Vec::new();

    for (idx, param) in config.item_generics.params.iter().enumerate() {
        let generic_ident = generic_const_ident(type_ident, suffix, idx);
        match param {
            syn::GenericParam::Type(type_param) => {
                let name = type_param.ident.to_string();
                let bounds = bounds_to_literals(&type_param.bounds);
                let bounds_ident = generic_bound_const_ident(type_ident, suffix, idx);
                generic_consts.push(quote! {
                    #[cfg(feature = "build-schemas")]
                    const #bounds_ident: &[&str] = &[#(#bounds),*];
                    #[cfg(feature = "build-schemas")]
                    const #generic_ident: ::proto_rs::schemas::Generic = ::proto_rs::schemas::Generic {
                        name: #name,
                        kind: ::proto_rs::schemas::GenericKind::Type,
                        constraints: #bounds_ident,
                        const_type: ::core::option::Option::None,
                    };
                });
                generic_refs.push(quote! { #generic_ident });
            }
            syn::GenericParam::Const(const_param) => {
                let name = const_param.ident.to_string();
                let const_ty = quote! { #const_param.ty };
                generic_consts.push(quote! {
                    #[cfg(feature = "build-schemas")]
                    const #generic_ident: ::proto_rs::schemas::Generic = ::proto_rs::schemas::Generic {
                        name: #name,
                        kind: ::proto_rs::schemas::GenericKind::Const,
                        constraints: &[],
                        const_type: ::core::option::Option::Some(stringify!(#const_ty)),
                    };
                });
                generic_refs.push(quote! { #generic_ident });
            }
            syn::GenericParam::Lifetime(_) => {}
        }
    }

    GenericTokens {
        consts: quote! { #(#generic_consts)* },
        refs: quote! { &[#(#generic_refs),*] },
    }
}

fn build_lifetime_tokens(type_ident: &syn::Ident, suffix: &str, config: &UnifiedProtoConfig) -> LifetimeTokens {
    let mut lifetime_consts = Vec::new();
    let mut lifetime_refs = Vec::new();

    for (idx, param) in config.item_generics.params.iter().enumerate() {
        if let syn::GenericParam::Lifetime(lifetime_param) = param {
            let name = lifetime_param.lifetime.ident.to_string();
            let bounds = lifetime_bounds_to_literals(&lifetime_param.bounds);
            let bounds_ident = lifetime_bound_const_ident(type_ident, suffix, idx);
            let lifetime_ident = lifetime_const_ident(type_ident, suffix, idx);
            lifetime_consts.push(quote! {
                #[cfg(feature = "build-schemas")]
                const #bounds_ident: &[&str] = &[#(#bounds),*];
                #[cfg(feature = "build-schemas")]
                const #lifetime_ident: ::proto_rs::schemas::Lifetime = ::proto_rs::schemas::Lifetime {
                    name: #name,
                    bounds: #bounds_ident,
                };
            });
            lifetime_refs.push(quote! { #lifetime_ident });
        }
    }

    LifetimeTokens {
        consts: quote! { #(#lifetime_consts)* },
        refs: quote! { &[#(#lifetime_refs),*] },
    }
}

fn build_attribute_tokens(type_ident: &syn::Ident, suffix: &str, attrs: &[syn::Attribute]) -> AttributeTokens {
    let mut attr_consts = Vec::new();
    let mut attr_refs = Vec::new();

    for (idx, attr) in attrs.iter().enumerate() {
        let attr_ident = attribute_const_ident(type_ident, suffix, idx);
        let path = attr.path().to_token_stream();
        let tokens = attr.to_token_stream();
        attr_consts.push(quote! {
            #[cfg(feature = "build-schemas")]
            const #attr_ident: ::proto_rs::schemas::Attribute = ::proto_rs::schemas::Attribute {
                path: stringify!(#path),
                tokens: stringify!(#tokens),
            };
        });
        attr_refs.push(quote! { #attr_ident });
    }

    AttributeTokens {
        consts: quote! { #(#attr_consts)* },
        refs: quote! { &[#(#attr_refs),*] },
    }
}

fn build_named_fields_tokens(type_ident: &syn::Ident, suffix: &str, fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>, config: &UnifiedProtoConfig) -> FieldTokens {
    let mut field_consts = Vec::new();
    let mut field_refs = Vec::new();
    let mut field_num = 0;

    for (idx, field) in fields.iter().enumerate() {
        let field_config = parse_field_config(field);
        if field_config.skip {
            continue;
        }
        field_num += 1;
        let name = field.ident.as_ref().unwrap().to_string();
        let tag: u32 = field_config.custom_tag.unwrap_or(field_num).try_into().unwrap();
        let FieldConstTokens { consts, refs } = build_field_const_tokens(type_ident, suffix, idx, field, &field_config, tag, FieldName::Named(name), config);
        field_consts.push(consts);
        field_refs.push(refs);
    }

    FieldTokens {
        consts: quote! { #(#field_consts)* },
        refs: quote! { &[#(#field_refs),*] },
    }
}

fn build_unnamed_fields_tokens(type_ident: &syn::Ident, suffix: &str, fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>, config: &UnifiedProtoConfig) -> FieldTokens {
    let mut field_consts = Vec::new();
    let mut field_refs = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let field_config = parse_field_config(field);
        if field_config.skip {
            continue;
        }
        let tag: u32 = field_config.custom_tag.unwrap_or(idx + 1).try_into().unwrap();
        let FieldConstTokens { consts, refs } = build_field_const_tokens(type_ident, suffix, idx, field, &field_config, tag, FieldName::Unnamed, config);
        field_consts.push(consts);
        field_refs.push(refs);
    }

    FieldTokens {
        consts: quote! { #(#field_consts)* },
        refs: quote! { &[#(#field_refs),*] },
    }
}

fn build_variant_fields_tokens(type_ident: &syn::Ident, suffix: &str, variant_idx: usize, fields: &Fields, config: &UnifiedProtoConfig) -> FieldTokens {
    match fields {
        Fields::Named(named) => build_named_fields_tokens(type_ident, &format!("{suffix}_VARIANT_{variant_idx}"), &named.named, config),
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() == 1 {
                let field = &unnamed.unnamed[0];
                let field_config = parse_field_config(field);
                if field_config.skip {
                    return FieldTokens {
                        consts: quote! {},
                        refs: quote! { &[] },
                    };
                }

                let FieldConstTokens { consts, refs } = build_field_const_tokens(type_ident, &format!("{suffix}_VARIANT_{variant_idx}"), 0, field, &field_config, 0, FieldName::Unnamed, config);
                return FieldTokens { consts, refs: quote! { &[#refs] } };
            }
            FieldTokens {
                consts: quote! {},
                refs: quote! { &[] },
            }
        }
        Fields::Unit => FieldTokens {
            consts: quote! {},
            refs: quote! { &[] },
        },
    }
}

fn field_proto_ident_and_label(field: &Field, config: &crate::utils::FieldConfig, item_generics: &syn::Generics) -> (TokenStream2, TokenStream2) {
    let base_ty = resolved_field_type(field, config);
    let ty = if let Some(ref into_type) = config.into_type {
        syn::parse_str::<Type>(into_type).unwrap_or_else(|_| base_ty.clone())
    } else {
        base_ty
    };
    let (mut is_option, mut is_repeated, inner_type) = extract_field_wrapper_info(&ty);

    if let Some(rename) = &config.rename {
        if let Some(flag) = rename.is_optional {
            is_option = flag;
        }
        if let Some(flag) = rename.is_repeated {
            is_repeated = flag;
        }
    }

    let label = if is_repeated {
        quote! { ::proto_rs::schemas::ProtoLabel::Repeated }
    } else if is_option {
        quote! { ::proto_rs::schemas::ProtoLabel::Optional }
    } else {
        quote! { ::proto_rs::schemas::ProtoLabel::None }
    };

    let parsed = parse_field_type(&inner_type);
    let ident_tokens = proto_ident_tokens(&inner_type, config, &parsed, item_generics);

    (ident_tokens, label)
}

fn proto_ident_tokens(inner_type: &Type, config: &crate::utils::FieldConfig, parsed: &ParsedFieldType, item_generics: &syn::Generics) -> TokenStream2 {
    if let Some(ref import_path) = config.import_path {
        let base_name = proto_type_name(inner_type);
        return proto_ident_literal(&base_name, import_path, import_path);
    }

    if let Some(rename) = &config.rename {
        return proto_ident_literal(&rename.proto_type, "", "");
    }

    if parsed.map_kind.is_some() {
        return proto_ident_literal(&parsed.proto_type, "", "");
    }

    if is_generic_param(inner_type, item_generics) {
        return proto_ident_literal("bytes", "", "");
    }

    if config.is_rust_enum || config.is_proto_enum || config.is_message || parsed.is_message_like {
        return quote! { <#inner_type as ::proto_rs::schemas::ProtoIdentifiable>::PROTO_IDENT };
    }

    proto_ident_literal(&parsed.proto_type, "", "")
}

enum FieldName {
    Named(String),
    Unnamed,
}

#[allow(clippy::too_many_arguments)]
fn build_field_const_tokens(
    type_ident: &syn::Ident,
    suffix: &str,
    idx: usize,
    field: &Field,
    config: &crate::utils::FieldConfig,
    tag: u32,
    name: FieldName,
    item_config: &UnifiedProtoConfig,
) -> FieldConstTokens {
    let field_ident = field_const_ident(type_ident, suffix, idx);
    let attrs_tokens = build_attribute_tokens(type_ident, &format!("{suffix}_FIELD_{idx}"), &field.attrs);
    let attr_consts = attrs_tokens.consts;
    let attr_refs = attrs_tokens.refs;
    let (proto_ident, label) = field_proto_ident_and_label(field, config, &item_config.item_generics);
    let name_tokens = match name {
        FieldName::Named(name) => quote! { ::core::option::Option::Some(#name) },
        FieldName::Unnamed => quote! { ::core::option::Option::None },
    };

    FieldConstTokens {
        consts: quote! {
            #[cfg(feature = "build-schemas")]
            const #field_ident: ::proto_rs::schemas::Field = ::proto_rs::schemas::Field {
                name: #name_tokens,
                proto_ident: #proto_ident,
                proto_label: #label,
                tag: #tag,
                attributes: #attr_refs,
            };
            #attr_consts
        },
        refs: quote! { &#field_ident },
    }
}

fn proto_ident_tokens_from_type(ty: &Type) -> TokenStream2 {
    let parsed = parse_field_type(ty);
    if parsed.is_message_like {
        quote! { <#ty as ::proto_rs::schemas::ProtoIdentifiable>::PROTO_IDENT }
    } else {
        proto_ident_literal(&parsed.proto_type, "", "")
    }
}

fn is_generic_param(ty: &Type, generics: &syn::Generics) -> bool {
    match ty {
        Type::Path(path) => {
            if path.qself.is_some() || path.path.segments.len() != 1 {
                return false;
            }
            let segment = &path.path.segments[0];
            if !segment.arguments.is_empty() {
                return false;
            }
            generics.type_params().any(|param| param.ident == segment.ident)
        }
        Type::Reference(reference) => is_generic_param(&reference.elem, generics),
        Type::Group(group) => is_generic_param(&group.elem, generics),
        Type::Paren(paren) => is_generic_param(&paren.elem, generics),
        _ => false,
    }
}

fn proto_ident_literal(proto_type: &str, package: &str, file_path: &str) -> TokenStream2 {
    let proto_type = proto_type.to_string();
    let package = package.to_string();
    let file_path = file_path.to_string();

    quote! {
        ::proto_rs::schemas::ProtoIdent {
            module_path: "",
            name: #proto_type,
            proto_package_name: #package,
            proto_file_path: #file_path,
            proto_type: #proto_type,
        }
    }
}

fn proto_path_info(config: &UnifiedProtoConfig) -> (String, String) {
    config.proto_path().map_or_else(
        || (String::new(), String::new()),
        |path| {
            let file_name = std::path::Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path);
            (derive_package_name(file_name), path.to_string())
        },
    )
}

fn schema_ident(type_ident: &syn::Ident, suffix: &str) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix));
    syn::Ident::new(&name, Span::call_site())
}

fn variant_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_VARIANT_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn field_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_FIELD_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn service_method_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_SERVICE_METHOD_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn generic_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_GENERIC_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn generic_bound_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_GENERIC_BOUNDS_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn lifetime_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_LIFETIME_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn lifetime_bound_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_LIFETIME_BOUNDS_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn attribute_const_ident(type_ident: &syn::Ident, suffix: &str, idx: usize) -> syn::Ident {
    let name = format!("PROTO_SCHEMA_ATTR_{}_{}_{}", sanitize_ident(&type_ident.to_string()), sanitize_ident(suffix), idx);
    syn::Ident::new(&name, Span::call_site())
}

fn format_ident(name: &str) -> syn::Ident {
    syn::Ident::new(name, Span::call_site())
}

fn sanitize_ident(value: &str) -> String {
    value.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_uppercase() } else { '_' }).collect()
}

fn bounds_to_literals(bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>) -> Vec<TokenStream2> {
    bounds.iter().map(|bound| quote! { stringify!(#bound) }).collect()
}

fn lifetime_bounds_to_literals(bounds: &syn::punctuated::Punctuated<syn::Lifetime, syn::token::Plus>) -> Vec<TokenStream2> {
    bounds.iter().map(|bound| quote! { stringify!(#bound) }).collect()
}
