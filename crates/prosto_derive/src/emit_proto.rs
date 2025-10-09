use std::collections::BTreeMap;
use std::collections::BTreeSet;

use syn::DataEnum;
use syn::Fields;
use syn::Type;

use crate::utils::is_bytes_vec;
use crate::utils::is_complex_type;
use crate::utils::is_option_type;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::rust_type_path_ident;
use crate::utils::strip_proto_suffix;
use crate::utils::to_upper_snake_case;
use crate::utils::vec_inner_type;
use crate::utils::*;

pub fn generate_simple_enum_proto(name: &str, data: &syn::DataEnum) -> String {
    let variants = &data.variants;
    let mut proto_variants = String::new();
    for (i, variant) in variants.iter().enumerate() {
        let variant_name = variant.ident.to_string();
        let proto_name = to_upper_snake_case(&variant_name);
        proto_variants.push_str(&format!("  {} = {};\n", proto_name, i));
    }
    format!("enum {} {{\n{}}}\n\n", name, proto_variants)
}

pub fn generate_struct_proto(name: &str, fields: &Fields) -> String {
    match fields {
        Fields::Named(fields) => generate_named_struct_proto(name, &fields.named),
        Fields::Unnamed(fields) => generate_tuple_struct_proto(name, &fields.unnamed),
        Fields::Unit => format!("message {} {{}}\n\n", name),
    }
}
fn generate_tuple_struct_proto(name: &str, fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> String {
    let mut proto_fields = String::new();

    for (idx, field) in fields.iter().enumerate() {
        let field_num = idx + 1;
        let field_name = format!("field_{}", idx);
        let ty = &field.ty;

        if is_bytes_array(ty) {
            proto_fields.push_str(&format!("  bytes {} = {};\n", field_name, field_num));
            continue;
        }

        if is_bytes_vec(ty) {
            proto_fields.push_str(&format!("  bytes {} = {};\n", field_name, field_num));
            continue;
        }

        // Handle other arrays as repeated
        if let Type::Array(type_array) = ty {
            let elem_ty = &*type_array.elem;
            let parsed = parse_field_type(elem_ty);
            let proto_ty_str = if parsed.is_message_like {
                let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
                strip_proto_suffix(&rust_name)
            } else {
                parsed.proto_type.clone()
            };
            proto_fields.push_str(&format!("  repeated {} {} = {};\n", proto_ty_str, field_name, field_num));
            continue;
        }

        let parsed = parse_field_type(ty);
        let proto_ty_str = if parsed.is_message_like {
            let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
            strip_proto_suffix(&rust_name)
        } else {
            parsed.proto_type.clone()
        };

        proto_fields.push_str(&format!("  {} {} = {};\n", proto_ty_str, field_name, field_num));
    }

    format!("message {} {{\n{}}}\n\n", name, proto_fields)
}

pub fn generate_named_struct_proto(name: &str, fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> String {
    let mut proto_fields = String::new();
    let mut field_num = 0;

    for field in fields.iter() {
        let config = parse_field_config(field);
        if config.skip {
            continue;
        }

        field_num += 1;
        let ident = field.ident.as_ref().unwrap().to_string();

        // Get the type to use for proto generation
        let ty = if let Some(ref into_type) = config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field.ty.clone())
        } else {
            field.ty.clone()
        };

        // Special handling for Vec<u8> -> bytes
        if is_bytes_vec(&ty) || is_bytes_array(&ty) {
            proto_fields.push_str(&format!("  bytes {} = {};\n", ident, field_num));
            continue;
        }

        // Extract the actual type from wrappers (Option/Vec)
        let (is_option, is_repeated, inner_type) = if is_option_type(&ty) {
            if let Type::Path(type_path) = &ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            (true, false, inner.clone())
                        } else {
                            (true, false, ty.clone())
                        }
                    } else {
                        (true, false, ty.clone())
                    }
                } else {
                    (true, false, ty.clone())
                }
            } else {
                (true, false, ty.clone())
            }
        } else if let Some(inner) = vec_inner_type(&ty) {
            (false, true, inner)
        } else if let Type::Array(type_array) = &ty {
            // Handle arrays [T; N] as repeated
            let elem_ty = (*type_array.elem).clone();
            (false, true, elem_ty)
        } else {
            (false, false, ty.clone())
        };

        // Determine the proto type string
        let proto_ty_str = if let Some(ref import_path) = config.import_path {
            // Use the import path prefix
            let base_name = if let Type::Path(_) = &inner_type {
                rust_type_path_ident(&inner_type).to_string()
            } else {
                // For non-path types (like primitives in arrays), parse directly
                let parsed = parse_field_type(&inner_type);
                parsed.proto_type.clone()
            };
            if let Type::Path(_) = &inner_type { format!("{}.{}", import_path, base_name) } else { base_name }
        } else if config.is_rust_enum {
            // For Rust enums converted to proto, get the type name
            rust_type_path_ident(&inner_type).to_string()
        } else if config.is_proto_enum {
            // For proto-native enums, use the type name as-is
            rust_type_path_ident(&inner_type).to_string()
        } else if config.is_message {
            // For imported message types, use as-is without Proto suffix
            rust_type_path_ident(&inner_type).to_string()
        } else if is_complex_type(&inner_type) {
            // For complex types, strip Proto suffix for .proto file
            let base_name = rust_type_path_ident(&inner_type).to_string();
            strip_proto_suffix(&base_name)
        } else {
            // For primitives, use the proto type
            let parsed = parse_field_type(&inner_type);
            parsed.proto_type
        };

        // Determine modifier
        let modifier = if is_repeated {
            "repeated "
        } else if is_option {
            "optional "
        } else {
            ""
        };

        proto_fields.push_str(&format!("  {}{} {} = {};\n", modifier, proto_ty_str, ident, field_num));
    }

    format!("message {} {{\n{}}}\n\n", name, proto_fields)
}

pub fn generate_complex_enum_proto(name: &str, data: &DataEnum) -> String {
    let proto_name = format!("{}Proto", name);
    let mut proto_fields = String::new();
    let mut nested_messages = String::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = idx + 1;
        let variant_ident = &variant.ident;
        let field_name_snake = to_snake_case(&variant_ident.to_string());

        match &variant.fields {
            // === 1️⃣ Unit variant ===
            Fields::Unit => {
                let empty_msg_name = format!("{}{}", proto_name, variant_ident);
                nested_messages.push_str(&format!("message {} {{}}\n\n", empty_msg_name));
                proto_fields.push_str(&format!("    {} {} = {};\n", empty_msg_name, field_name_snake, tag));
            }

            // === 2️⃣ Tuple variant (e.g., Address(AddressProto)) ===
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    panic!("Complex enum unnamed variants must have exactly one field");
                }

                let field_ty = &fields.unnamed.first().unwrap().ty;

                if is_bytes_array(field_ty) || is_bytes_vec(field_ty) {
                    proto_fields.push_str(&format!("    bytes {} = {};\n", field_name_snake, tag));
                    continue;
                }

                let parsed = parse_field_type(field_ty);

                let proto_type_str = if parsed.is_message_like {
                    let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
                    strip_proto_suffix(&rust_name)
                } else {
                    parsed.proto_type.clone()
                };

                proto_fields.push_str(&format!("    {} {} = {};\n", proto_type_str, field_name_snake, tag));
            }

            // === 3️⃣ Named variant (e.g., Third { id: u64, address: Address }) ===
            Fields::Named(fields) => {
                let nested_msg_name = format!("{}{}", proto_name, variant_ident);
                let mut nested_proto_fields = String::new();

                let mut field_tag = 0;
                for field in fields.named.iter() {
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    let field_config = parse_field_config(field);

                    if field_config.skip {
                        continue;
                    }

                    field_tag += 1;
                    let ty_for_parsing = if let Some(ref into_type) = field_config.into_type {
                        syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field.ty.clone())
                    } else {
                        field.ty.clone()
                    };

                    if is_bytes_array(&ty_for_parsing) || is_bytes_vec(&ty_for_parsing) {
                        nested_proto_fields.push_str(&format!("  bytes {} = {};\n", field_name, field_tag));
                        continue;
                    }

                    let parsed = parse_field_type(&ty_for_parsing);

                    let proto_type_str = if let Some(ref import_path) = field_config.import_path {
                        let base_type = rust_type_path_ident(&extract_wrapper_info(&ty_for_parsing).0).to_string();
                        format!("{}.{}", import_path, base_type)
                    } else if field_config.is_rust_enum || field_config.is_proto_enum {
                        rust_type_path_ident(&extract_wrapper_info(&ty_for_parsing).0).to_string()
                    } else if field_config.is_message || parsed.is_message_like {
                        if field_config.is_message {
                            rust_type_path_ident(&extract_wrapper_info(&ty_for_parsing).0).to_string()
                        } else {
                            let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
                            strip_proto_suffix(&rust_name)
                        }
                    } else {
                        parsed.proto_type.clone()
                    };

                    let modifier = if parsed.is_repeated {
                        "repeated "
                    } else if parsed.is_option {
                        "optional "
                    } else {
                        ""
                    };

                    nested_proto_fields.push_str(&format!("  {}{} {} = {};\n", modifier, proto_type_str, field_name, field_tag));
                }

                nested_messages.push_str(&format!("message {} {{\n{}}}\n\n", nested_msg_name, nested_proto_fields));
                proto_fields.push_str(&format!("    {} {} = {};\n", nested_msg_name, field_name_snake, tag));
            }
        }
    }

    // === Combine into final proto message ===
    format!("{}message {} {{\n  oneof value {{\n{}}}\n}}\n\n", nested_messages, proto_name, proto_fields)
}

pub fn generate_service_content(trait_name: &syn::Ident, methods: &[MethodInfo], proto_imports: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut content = String::new();
    content.push_str(&format!("service {} {{\n", trait_name));

    for method in methods {
        let method_name = to_pascal_case(&method.name.to_string());
        let request_type = qualify_type_name(&method.request_type, proto_imports);

        if method.is_streaming {
            let response_type = qualify_type_name(method.inner_response_type.as_ref().unwrap(), proto_imports);
            content.push_str(&format!("  rpc {}({}) returns (stream {}) {{}}\n", method_name, request_type, response_type));
        } else {
            let response_type = qualify_type_name(&method.response_type, proto_imports);
            content.push_str(&format!("  rpc {}({}) returns ({}) {{}}\n", method_name, request_type, response_type));
        }
    }

    content.push_str("}\n");
    content
}

fn qualify_type_name(ty: &syn::Type, proto_imports: &BTreeMap<String, BTreeSet<String>>) -> String {
    let type_name = extract_type_name(ty);

    // Check if this type is in any import
    for (package, types) in proto_imports {
        if types.contains(&type_name) {
            return format!("{}.{}", package, type_name);
        }
    }

    type_name
}

/// Extract the simple type name from a Type (removes paths and generics)
fn extract_type_name(ty: &syn::Type) -> String {
    use syn::Type;
    use syn::TypePath;

    match ty {
        Type::Path(TypePath { path, .. }) => {
            if let Some(segment) = path.segments.last() {
                segment.ident.to_string()
            } else {
                "Unknown".to_string()
            }
        }
        _ => "Unknown".to_string(),
    }
}
