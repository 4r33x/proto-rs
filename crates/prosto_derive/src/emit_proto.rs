//! Proto file generation - refactored to eliminate duplication

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use syn::DataEnum;
use syn::Field;
use syn::Fields;
use syn::Type;
use syn::punctuated::Punctuated;
use syn::token::Comma;

use crate::utils::MethodInfo;
use crate::utils::collect_discriminants_for_variants;
use crate::utils::extract_field_wrapper_info;
use crate::utils::find_marked_default_variant;
use crate::utils::is_bytes_array;
use crate::utils::is_bytes_vec;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::proto_type_name;
use crate::utils::resolved_field_type;
use crate::utils::rust_type_path_ident;
use crate::utils::strip_proto_suffix;
use crate::utils::to_pascal_case;
use crate::utils::to_snake_case;
use crate::utils::to_upper_snake_case;

pub fn generate_simple_enum_proto(name: &str, data: &DataEnum) -> String {
    let marked_default = find_marked_default_variant(data).unwrap_or_else(|err| panic!("{}", err));

    let mut order: Vec<usize> = (0..data.variants.len()).collect();
    if let Some(idx) = marked_default
        && idx < order.len()
    {
        order.remove(idx);
        order.insert(0, idx);
    }

    let ordered_variants: Vec<&syn::Variant> = order.iter().map(|&idx| &data.variants[idx]).collect();
    let ordered_discriminants = collect_discriminants_for_variants(&ordered_variants).unwrap_or_else(|err| panic!("{}", err));

    assert!(
        !(marked_default.is_some() && ordered_discriminants.first().copied().unwrap_or_default() != 0),
        "enum #[default] variant must have discriminant 0"
    );

    assert!(ordered_discriminants.contains(&0), "proto enums must contain a variant with discriminant 0");

    let variants: Vec<String> = ordered_variants
        .into_iter()
        .zip(ordered_discriminants)
        .map(|(variant, value)| {
            let proto_name = to_upper_snake_case(&variant.ident.to_string());
            format!("  {proto_name} = {value};")
        })
        .collect();

    format!("enum {} {{\n{}\n}}\n\n", name, variants.join("\n"))
}

pub fn generate_complex_enum_proto(name: &str, data: &DataEnum, generic_params: &[syn::Ident]) -> String {
    let proto_name = name.to_string();

    let mut nested_messages = Vec::new();
    let mut oneof_fields = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = idx + 1;
        let variant_ident = &variant.ident;
        let field_name_snake = to_snake_case(&variant_ident.to_string());

        match &variant.fields {
            Fields::Unit => {
                let msg_name = format!("{proto_name}{variant_ident}");
                nested_messages.push(format!("message {msg_name} {{}}"));
                oneof_fields.push(format!("    {msg_name} {field_name_snake} = {tag};"));
            }
            Fields::Unnamed(fields) => {
                assert!((fields.unnamed.len() == 1), "Complex enum unnamed variants must have exactly one field");

                let field = &fields.unnamed[0];
                let config = parse_field_config(field);

                // If the field is marked with #[proto(skip)], treat it like a unit variant (empty message)
                if config.skip {
                    let msg_name = format!("{proto_name}{variant_ident}");
                    nested_messages.push(format!("message {msg_name} {{}}"));
                    oneof_fields.push(format!("    {msg_name} {field_name_snake} = {tag};"));
                } else {
                    let proto_type = get_field_proto_type(field, generic_params);
                    oneof_fields.push(format!("    {proto_type} {field_name_snake} = {tag};"));
                }
            }
            Fields::Named(fields) => {
                let msg_name = format!("{proto_name}{variant_ident}");
                let field_defs = generate_named_fields(&fields.named, generic_params);

                nested_messages.push(format!("message {msg_name} {{\n{field_defs}\n}}"));
                oneof_fields.push(format!("    {msg_name} {field_name_snake} = {tag};"));
            }
        }
    }

    format!(
        "{}\nmessage {} {{\n  oneof value {{\n{}\n  }}\n}}\n\n",
        nested_messages.join("\n\n"),
        proto_name,
        oneof_fields.join("\n")
    )
}

pub fn generate_struct_proto(name: &str, fields: &Fields, generic_params: &[syn::Ident]) -> String {
    match fields {
        Fields::Named(fields) => generate_named_struct_proto(name, &fields.named, generic_params),
        Fields::Unnamed(fields) => generate_tuple_struct_proto(name, &fields.unnamed, generic_params),
        Fields::Unit => format!("message {name} {{}}\n\n"),
    }
}

fn generate_named_struct_proto(name: &str, fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, generic_params: &[syn::Ident]) -> String {
    let field_defs = generate_named_fields(fields, generic_params);
    format!("message {name} {{\n{field_defs}\n}}\n\n")
}

fn generate_tuple_struct_proto(name: &str, fields: &Punctuated<Field, Comma>, generic_params: &[syn::Ident]) -> String {
    let mut proto_fields = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let config = parse_field_config(field);
        if config.skip {
            continue;
        }

        let field_name = format!("field_{idx}");
        let base_ty = resolved_field_type(field, &config);
        let ty = if let Some(ref into_type) = config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| base_ty.clone())
        } else {
            base_ty
        };

        let (mut is_option, mut is_repeated, inner_type) = extract_field_wrapper_info(&ty);
        let proto_type = resolve_proto_type(&inner_type, &config, &mut is_option, &mut is_repeated, generic_params);

        let modifier = field_modifier(is_option, is_repeated);
        let tag = config.custom_tag.unwrap_or(idx + 1);
        proto_fields.push(format!("  {modifier}{proto_type} {field_name} = {tag};"));
    }

    format!("message {} {{\n{}\n}}\n\n", name, proto_fields.join("\n"))
}

fn resolve_proto_type(inner_type: &Type, config: &crate::utils::FieldConfig, is_option: &mut bool, is_repeated: &mut bool, generic_params: &[syn::Ident]) -> String {
    if let Some(rename) = &config.rename {
        if let Some(flag) = rename.is_optional {
            *is_option = flag;
        }
        if let Some(flag) = rename.is_repeated {
            *is_repeated = flag;
        }
        return rename.proto_type.clone();
    }

    determine_proto_type(inner_type, config, generic_params)
}

fn field_modifier(is_option: bool, is_repeated: bool) -> &'static str {
    match (is_option, is_repeated) {
        (true, false) => "optional ",
        (true | false, true) => "repeated ",
        (false, false) => "",
    }
}

/// Generate proto fields for named struct/enum variant
fn generate_named_fields(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, generic_params: &[syn::Ident]) -> String {
    let mut proto_fields = Vec::new();
    let mut field_num = 0;

    for field in fields {
        let config = parse_field_config(field);
        if config.skip {
            continue;
        }

        field_num += 1;
        let field_name = field.ident.as_ref().unwrap().to_string();

        // Get effective type for proto generation
        let base_ty = resolved_field_type(field, &config);
        let ty = if let Some(ref into_type) = config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| base_ty.clone())
        } else {
            base_ty
        };

        // Extract wrapper info
        let (mut is_option, mut is_repeated, inner_type) = extract_field_wrapper_info(&ty);

        // Determine proto type string
        let proto_type = resolve_proto_type(&inner_type, &config, &mut is_option, &mut is_repeated, generic_params);

        // Add modifier
        let modifier = field_modifier(is_option, is_repeated);

        let tag = config.custom_tag.unwrap_or(field_num);

        proto_fields.push(format!("  {modifier}{proto_type} {field_name} = {tag};"));
    }

    proto_fields.join("\n")
}

/// Get proto type string for a field type
fn get_field_proto_type(field: &Field, generic_params: &[syn::Ident]) -> String {
    let config = parse_field_config(field);
    let base_ty = resolved_field_type(field, &config);
    let ty = if let Some(ref into_type) = config.into_type {
        syn::parse_str::<Type>(into_type).unwrap_or_else(|_| base_ty.clone())
    } else {
        base_ty
    };

    // Handle bytes
    if is_bytes_vec(&ty) || is_bytes_array(&ty) {
        return "bytes".to_string();
    }

    // Handle arrays as repeated
    if let Type::Array(type_array) = &ty {
        let elem_ty = &*type_array.elem;
        let parsed = parse_field_type(elem_ty);

        return if parsed.is_message_like { proto_type_name(&parsed.proto_rust_type) } else { parsed.proto_type };
    }

    let parsed = parse_field_type(&ty);

    if is_generic_param(&ty, generic_params) {
        return "bytes".to_string();
    }

    if parsed.map_kind.is_some() {
        return parsed.proto_type;
    }

    if config.is_rust_enum || config.is_proto_enum || config.is_message {
        return rust_type_path_ident(&ty).to_string();
    }

    if parsed.is_message_like { proto_type_name(&parsed.proto_rust_type) } else { parsed.proto_type }
}

/// Determine proto type string based on field config
fn determine_proto_type(inner_type: &Type, config: &crate::utils::FieldConfig, generic_params: &[syn::Ident]) -> String {
    if is_bytes_vec(inner_type) || is_bytes_array(inner_type) {
        return "bytes".to_string();
    }

    if let Some(ref import_path) = config.import_path {
        let base_name = proto_type_name(inner_type);
        return format!("{import_path}.{base_name}");
    }

    let parsed = parse_field_type(inner_type);

    if is_generic_param(inner_type, generic_params) {
        return "bytes".to_string();
    }

    if parsed.map_kind.is_some() {
        return parsed.proto_type;
    }

    if config.is_rust_enum || config.is_proto_enum {
        return rust_type_path_ident(inner_type).to_string();
    }

    if config.is_message {
        return proto_type_name(inner_type);
    }

    if parsed.is_message_like {
        return proto_type_name(&parsed.proto_rust_type);
    }

    parsed.proto_type
}

pub fn generate_service_content(trait_name: &syn::Ident, methods: &[MethodInfo], proto_imports: &BTreeMap<String, BTreeSet<String>>, import_all_from: Option<&str>) -> String {
    let mut lines = vec![format!("service {} {{", trait_name)];

    for method in methods {
        let method_name = to_pascal_case(&method.name.to_string());
        let request_type = qualify_type_name(&method.request_type, proto_imports, import_all_from);

        let rpc_def = if method.is_streaming {
            let response_type = qualify_type_name(method.inner_response_type.as_ref().unwrap(), proto_imports, import_all_from);
            format!("  rpc {method_name}({request_type}) returns (stream {response_type}) {{}}")
        } else {
            let response_type = qualify_type_name(&method.response_type, proto_imports, import_all_from);
            format!("  rpc {method_name}({request_type}) returns ({response_type}) {{}}")
        };

        lines.push(rpc_def);
    }

    lines.push("}".to_string());
    lines.join("\n")
}

fn qualify_type_name(ty: &Type, proto_imports: &BTreeMap<String, BTreeSet<String>>, import_all_from: Option<&str>) -> String {
    let type_name = extract_type_name(ty);
    let base_name = extract_base_type_name(ty);

    // Check if type is in any import
    for (package, types) in proto_imports {
        if types.contains(&type_name) || base_name.as_ref().is_some_and(|base| types.contains(base)) {
            return format!("{package}.{type_name}");
        }
    }

    if let Some(package) = import_all_from {
        return format!("{package}.{type_name}");
    }

    type_name
}

fn is_generic_param(ty: &Type, generic_params: &[syn::Ident]) -> bool {
    match ty {
        Type::Path(path) => {
            if path.qself.is_some() || path.path.segments.len() != 1 {
                return false;
            }
            let segment = &path.path.segments[0];
            if !segment.arguments.is_empty() {
                return false;
            }
            generic_params.iter().any(|param| param == &segment.ident)
        }
        Type::Reference(reference) => is_generic_param(&reference.elem, generic_params),
        Type::Group(group) => is_generic_param(&group.elem, generic_params),
        Type::Paren(paren) => is_generic_param(&paren.elem, generic_params),
        _ => false,
    }
}

fn extract_type_name(ty: &Type) -> String {
    proto_type_name(ty)
}

fn extract_base_type_name(ty: &Type) -> Option<String> {
    if let Type::Path(type_path) = ty {
        return type_path.path.segments.last().map(|segment| strip_proto_suffix(&segment.ident.to_string()));
    }
    None
}
