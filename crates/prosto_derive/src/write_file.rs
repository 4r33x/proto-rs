//! Proto file writing and registry management - refactored

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::fs::{self};
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;
use std::sync::Mutex;

use proc_macro2::TokenStream;
use quote::quote;

use crate::utils::derive_package_name;
use crate::utils::format_import;

const IMPORT_PREFIX: &str = "__IMPORT__";

/// Global registry: filename -> `BTreeSet`<proto definitions>
static REGISTRY: LazyLock<Mutex<HashMap<String, BTreeSet<String>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Track initialized files
static INITIALIZED_FILES: LazyLock<Mutex<BTreeSet<String>>> = LazyLock::new(|| Mutex::new(BTreeSet::new()));

/// Determine if we should emit .proto files
/// Priority: env var > feature flag > default (false)
pub fn should_emit_file() -> bool {
    match std::env::var("PROTO_EMIT_FILE").ok().as_deref() {
        Some("0" | "false" | "False" | "FALSE") => false,
        Some("1" | "true" | "True" | "TRUE") => true,
        _ => cfg!(feature = "emit-proto-files"),
    }
}

/// Register proto content and optionally write to file
pub fn register_and_emit_proto_inner(file_name: &str, content: &str, schema_tokens: TokenStream) -> TokenStream {
    let emission_code = schema_tokens;

    if should_emit_file() {
        write_proto_file(file_name, content);
    }

    emission_code
}

/// Register imports for a proto file
pub fn register_imports(type_ident: &str, imports: &BTreeMap<String, BTreeSet<String>>) -> TokenStream {
    let mut code = TokenStream::new();

    for (file, import_set) in imports {
        // Register in global registry
        register_imports_in_registry(file, import_set);

        // Write file if emission enabled
        if should_emit_file() {
            write_proto_file_internal(file);
        }

        // Generate emission code
        let import_list: Vec<String> = import_set.iter().cloned().collect();
        let emission = crate::schema::schema_tokens_for_imports(type_ident, file, &import_list);

        code = quote! { #code #emission };
    }

    code
}

fn register_imports_in_registry(file: &str, imports: &BTreeSet<String>) {
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file.to_string()).or_default();

    for import in imports {
        let import_entry = format!("{IMPORT_PREFIX}:{import}");
        defs.insert(import_entry);
    }
}

/// Register single import
pub fn register_import(file: &str, imports: &[String]) -> TokenStream {
    {
        let mut registry = REGISTRY.lock().unwrap();
        let defs = registry.entry(file.to_string()).or_default();

        for import in imports {
            let import_entry = format!("{IMPORT_PREFIX}:{import}");
            defs.insert(import_entry);
        }
    }

    let emission_code = crate::schema::schema_tokens_for_imports("ImportInject", file, imports);

    if should_emit_file() {
        write_proto_file_internal(file);
    }

    emission_code
}

/// Write proto content to registry
fn write_proto_file(file_name_path: &str, content: &str) {
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file_name_path.to_string()).or_default();
    let was_new = defs.insert(content.to_string());

    if was_new && should_emit_file() {
        drop(registry);
        write_proto_file_internal(file_name_path);
    }
}

/// Internal file writing implementation
fn write_proto_file_internal(file_name_path: &str) {
    let path = Path::new(".").join(file_name_path);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let registry = REGISTRY.lock().unwrap();
    let Some(defs) = registry.get(file_name_path) else {
        return;
    };

    // Separate imports and content
    let (imports, content_items) = separate_imports_and_content(defs);

    // Build complete file
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let file_content = build_complete_proto_file(file_name, &imports, &content_items);

    // Write atomically
    write_file_atomically(&path, &file_content);

    // Mark as initialized
    drop(registry);
    mark_file_initialized(file_name_path);
}

fn separate_imports_and_content(defs: &BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut content = Vec::new();

    for item in defs {
        if let Some(import_path) = item.strip_prefix(&format!("{IMPORT_PREFIX}:")) {
            imports.push(import_path.to_string());
        } else {
            content.push(item.clone());
        }
    }

    (imports, content)
}

fn build_complete_proto_file(file_name: &str, imports: &[String], content_items: &[String]) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Header
    output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
    output.push_str("syntax = \"proto3\";\n");
    writeln!(&mut output, "package {};", derive_package_name(file_name)).unwrap();

    // Imports
    if !imports.is_empty() {
        output.push('\n');
        for import in imports {
            output.push_str(&format_import(import));
        }
    }

    // Content
    output.push('\n');
    for item in content_items {
        output.push_str(item);
    }

    output
}

fn write_file_atomically(path: &Path, content: &str) {
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(path).expect("Failed to open proto file");

    write!(file, "{content}").expect("Failed to write proto file");
}

fn mark_file_initialized(file_name_path: &str) {
    let mut initialized = INITIALIZED_FILES.lock().unwrap();
    initialized.insert(file_name_path.to_string());
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separate_imports_and_content() {
        let mut defs = BTreeSet::new();
        defs.insert("__IMPORT__:common/types".to_string());
        defs.insert("message Foo {}".to_string());
        defs.insert("__IMPORT__:other/service".to_string());

        let (imports, content) = separate_imports_and_content(&defs);

        assert_eq!(imports.len(), 2);
        assert!(imports.contains(&"common/types".to_string()));
        assert!(imports.contains(&"other/service".to_string()));

        assert_eq!(content.len(), 1);
        assert_eq!(content[0], "message Foo {}");
    }

    #[test]
    fn test_should_emit_file() {
        // Test depends on env var and feature flag
        // Just ensure it doesn't panic
        let _ = should_emit_file();
    }
}
