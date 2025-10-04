use std::collections::HashMap;

use super::utils::MethodInfo;
use crate::utils::to_pascal_case;
use crate::write_file::register_import;
use crate::write_file::write_proto_file;

pub fn generate_proto_file(trait_name: &syn::Ident, methods: &[MethodInfo], output_path: &str, proto_imports: &HashMap<String, Vec<String>>) {
    // Register imports
    for package in proto_imports.keys() {
        register_import(output_path, package);
    }

    let service_content = generate_service_content(trait_name, methods, proto_imports);
    write_proto_file(output_path, &service_content);
}

/// Generate the service definition content (without header)
fn generate_service_content(trait_name: &syn::Ident, methods: &[MethodInfo], proto_imports: &HashMap<String, Vec<String>>) -> String {
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

fn qualify_type_name(ty: &syn::Type, proto_imports: &HashMap<String, Vec<String>>) -> String {
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
