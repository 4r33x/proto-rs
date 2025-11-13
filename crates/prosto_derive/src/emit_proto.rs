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

pub fn generate_complex_enum_proto(name: &str, data: &DataEnum) -> String {
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

                let field_ty = &fields.unnamed.first().unwrap().ty;
                let proto_type = get_field_proto_type(field_ty);

                oneof_fields.push(format!("    {proto_type} {field_name_snake} = {tag};"));
            }
            Fields::Named(fields) => {
                let msg_name = format!("{proto_name}{variant_ident}");
                let field_defs = generate_named_fields(&fields.named);

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

pub fn generate_struct_proto(name: &str, fields: &Fields) -> String {
    match fields {
        Fields::Named(fields) => generate_named_struct_proto(name, &fields.named),
        Fields::Unnamed(fields) => generate_tuple_struct_proto(name, &fields.unnamed),
        Fields::Unit => format!("message {name} {{}}\n\n"),
    }
}

fn generate_named_struct_proto(name: &str, fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> String {
    let field_defs = generate_named_fields(fields);
    format!("message {name} {{\n{field_defs}\n}}\n\n")
}

fn generate_tuple_struct_proto(name: &str, fields: &Punctuated<Field, Comma>) -> String {
    let mut proto_fields = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let config = parse_field_config(field);
        if config.skip {
            continue;
        }

        let field_name = format!("field_{idx}");
        let ty = if let Some(ref into_type) = config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field.ty.clone())
        } else {
            field.ty.clone()
        };

        let (mut is_option, mut is_repeated, inner_type) = extract_field_wrapper_info(&ty);
        let proto_type = resolve_proto_type(&inner_type, &config, &mut is_option, &mut is_repeated);

        let modifier = field_modifier(is_option, is_repeated);
        let tag = config.custom_tag.unwrap_or(idx + 1);
        proto_fields.push(format!("  {modifier}{proto_type} {field_name} = {tag};"));
    }

    format!("message {} {{\n{}\n}}\n\n", name, proto_fields.join("\n"))
}

fn resolve_proto_type(inner_type: &Type, config: &crate::utils::FieldConfig, is_option: &mut bool, is_repeated: &mut bool) -> String {
    if let Some(rename) = &config.rename {
        if let Some(flag) = rename.is_optional {
            *is_option = flag;
        }
        if let Some(flag) = rename.is_repeated {
            *is_repeated = flag;
        }
        return rename.proto_type.clone();
    }

    determine_proto_type(inner_type, config)
}

fn field_modifier(is_option: bool, is_repeated: bool) -> &'static str {
    match (is_option, is_repeated) {
        (true, true) => "repeated ",
        (true, false) => "optional ",
        (false, true) => "repeated ",
        (false, false) => "",
    }
}

/// Generate proto fields for named struct/enum variant
fn generate_named_fields(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> String {
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
        let ty = if let Some(ref into_type) = config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field.ty.clone())
        } else {
            field.ty.clone()
        };

        // Extract wrapper info
        let (mut is_option, mut is_repeated, inner_type) = extract_field_wrapper_info(&ty);

        // Determine proto type string
        let proto_type = resolve_proto_type(&inner_type, &config, &mut is_option, &mut is_repeated);

        // Add modifier
        let modifier = field_modifier(is_option, is_repeated);

        let tag = config.custom_tag.unwrap_or(field_num);

        proto_fields.push(format!("  {modifier}{proto_type} {field_name} = {tag};"));
    }

    proto_fields.join("\n")
}

/// Get proto type string for a field type
fn get_field_proto_type(ty: &Type) -> String {
    // Handle bytes
    if is_bytes_vec(ty) || is_bytes_array(ty) {
        return "bytes".to_string();
    }

    // Handle arrays as repeated
    if let Type::Array(type_array) = ty {
        let elem_ty = &*type_array.elem;
        let parsed = parse_field_type(elem_ty);

        return if parsed.is_message_like {
            let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
            strip_proto_suffix(&rust_name)
        } else {
            parsed.proto_type
        };
    }

    let parsed = parse_field_type(ty);

    if parsed.map_kind.is_some() {
        return parsed.proto_type;
    }

    if parsed.is_message_like {
        let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
        strip_proto_suffix(&rust_name)
    } else {
        parsed.proto_type
    }
}

/// Determine proto type string based on field config
fn determine_proto_type(inner_type: &Type, config: &crate::utils::FieldConfig) -> String {
    if is_bytes_vec(inner_type) || is_bytes_array(inner_type) {
        return "bytes".to_string();
    }

    if let Some(ref import_path) = config.import_path {
        let base_name = rust_type_path_ident(inner_type).to_string();
        return format!("{import_path}.{base_name}");
    }

    let parsed = parse_field_type(inner_type);

    if parsed.map_kind.is_some() {
        return parsed.proto_type;
    }

    if config.is_rust_enum || config.is_proto_enum || config.is_message {
        return rust_type_path_ident(inner_type).to_string();
    }

    if parsed.is_message_like {
        let base_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
        return strip_proto_suffix(&base_name);
    }

    parsed.proto_type
}

pub fn generate_service_content(trait_name: &syn::Ident, methods: &[MethodInfo], proto_imports: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut lines = vec![format!("service {} {{", trait_name)];

    for method in methods {
        let method_name = to_pascal_case(&method.name.to_string());
        let request_type = qualify_type_name(&method.request_type, proto_imports);

        let rpc_def = if method.is_streaming {
            let response_type = qualify_type_name(method.inner_response_type.as_ref().unwrap(), proto_imports);
            format!("  rpc {method_name}({request_type}) returns (stream {response_type}) {{}}")
        } else {
            let response_type = qualify_type_name(&method.response_type, proto_imports);
            format!("  rpc {method_name}({request_type}) returns ({response_type}) {{}}")
        };

        lines.push(rpc_def);
    }

    lines.push("}".to_string());
    lines.join("\n")
}

fn qualify_type_name(ty: &Type, proto_imports: &BTreeMap<String, BTreeSet<String>>) -> String {
    let type_name = extract_type_name(ty);

    // Check if type is in any import
    for (package, types) in proto_imports {
        if types.contains(&type_name) {
            return format!("{package}.{type_name}");
        }
    }

    type_name
}

fn extract_type_name(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map_or_else(|| "Unknown".to_string(), |s| s.ident.to_string())
    } else {
        "Unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    // #[test]
    // fn test_simple_enum_generation() {
    //     let data: DataEnum = parse_quote! {
    //         enum Status {
    //             Active,
    //             Inactive,
    //             Pending,
    //         }
    //     };

    //     let proto = generate_simple_enum_proto("Status", &data);
    //     assert!(proto.contains("ACTIVE = 0"));
    //     assert!(proto.contains("INACTIVE = 1"));
    //     assert!(proto.contains("PENDING = 2"));
    // }

    #[test]
    fn test_get_field_proto_type() {
        let ty: Type = parse_quote! {  [u8; B_LEN] };
        assert_eq!(get_field_proto_type(&ty), "bytes");
        let ty: Type = parse_quote! { [u8; 32] };
        assert_eq!(get_field_proto_type(&ty), "bytes");
        let ty: Type = parse_quote! { Vec<u8> };
        assert_eq!(get_field_proto_type(&ty), "bytes");

        let ty: Type = parse_quote! { u32 };
        assert_eq!(get_field_proto_type(&ty), "uint32");

        let ty: Type = parse_quote! { String };
        assert_eq!(get_field_proto_type(&ty), "string");
    }

    #[test]
    fn cache_padded_vec_is_repeated() {
        let ty: Type = parse_quote! { crossbeam_utils::CachePadded<Vec<u32>> };
        let (is_option, is_repeated, inner) = extract_field_wrapper_info(&ty);

        assert!(!is_option);
        assert!(is_repeated);
        assert_eq!(quote!(#inner).to_string(), quote!(u32).to_string());
    }

    #[test]
    fn cache_padded_option_is_optional() {
        let ty: Type = parse_quote! { crossbeam_utils::CachePadded<Option<String>> };
        let (is_option, is_repeated, inner) = extract_field_wrapper_info(&ty);

        assert!(is_option);
        assert!(!is_repeated);
        assert_eq!(quote!(#inner).to_string(), quote!(String).to_string());
    }

    #[test]
    fn arc_swap_vec_is_repeated() {
        let ty: Type = parse_quote! { arc_swap::ArcSwap<Vec<u32>> };
        let (is_option, is_repeated, inner) = extract_field_wrapper_info(&ty);

        assert!(!is_option);
        assert!(is_repeated);
        assert_eq!(quote!(#inner).to_string(), quote!(u32).to_string());
    }

    #[test]
    fn arc_swap_vec_u8_is_bytes() {
        let ty: Type = parse_quote! { arc_swap::ArcSwap<Vec<u8>> };
        let (is_option, is_repeated, inner) = extract_field_wrapper_info(&ty);

        assert!(!is_option);
        assert!(!is_repeated);
        assert_eq!(quote!(#inner).to_string(), quote!(Vec<u8>).to_string());
        assert_eq!(determine_proto_type(&inner, &crate::utils::FieldConfig::default()), "bytes");
    }

    #[test]
    fn arc_swap_option_is_optional() {
        let ty: Type = parse_quote! { arc_swap::ArcSwapOption<String> };
        let (is_option, is_repeated, inner) = extract_field_wrapper_info(&ty);

        assert!(is_option);
        assert!(!is_repeated);
        assert_eq!(quote!(#inner).to_string(), quote!(String).to_string());
    }

    #[test]
    fn rename_rust_type_to_scalar_proto() {
        let fields: syn::FieldsNamed = parse_quote!({
            #[proto(rename = u64)]
            id: Option<MyId>,
        });

        let proto = generate_named_struct_proto("User", &fields.named);
        assert!(proto.contains("optional uint64 id = 1;"));
    }

    #[test]
    fn rename_vec_bytes_to_bytes_scalar() {
        let fields: syn::FieldsNamed = parse_quote!({
            #[proto(rename = Vec<u8>)]
            payload: Blob,
        });

        let proto = generate_named_struct_proto("Packet", &fields.named);
        assert!(proto.contains("bytes payload = 1;"));
    }

    #[test]
    fn rename_vec_to_repeated_scalar() {
        let fields: syn::FieldsNamed = parse_quote!({
            #[proto(rename = Vec<u64>)]
            values: Numbers,
        });

        let proto = generate_named_struct_proto("Counter", &fields.named);
        assert!(proto.contains("repeated uint64 values = 1;"));
    }

    #[test]
    fn rename_proto_string_overrides_modifier() {
        let fields: syn::FieldsNamed = parse_quote!({
            #[proto(rename = "optional uint64")]
            id: MyId,
        });

        let proto = generate_named_struct_proto("Record", &fields.named);
        assert!(proto.contains("optional uint64 id = 1;"));
    }

    #[test]
    fn tuple_struct_field_honors_rename() {
        let fields: syn::FieldsUnnamed = parse_quote!((
            #[proto(rename = Vec<u64>)]
            Numbers,
        ));

        let proto = generate_tuple_struct_proto("Wrapper", &fields.unnamed);
        assert!(proto.contains("repeated uint64 field_0 = 1;"));
    }
}
