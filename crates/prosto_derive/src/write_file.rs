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

const IMPORT_PREFIX: &str = "__IMPORT__";

/// Global registry: filename -> BTreeSet<proto definitions> (stable ordering)
pub static REGISTRY: LazyLock<Mutex<HashMap<String, BTreeSet<String>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Track which files have been initialized (cleared) this compilation
pub static INITIALIZED_FILES: LazyLock<Mutex<BTreeSet<String>>> = LazyLock::new(|| Mutex::new(BTreeSet::new()));

/// Determine if we should emit .proto files immediately
///
/// Priority:
/// 1. Env var explicit false → don't emit (override feature)
/// 2. Env var explicit true → emit (override feature)
/// 3. Feature flag enabled → emit
/// 4. Default → don't emit
pub fn should_emit_file() -> bool {
    match std::env::var("PROTO_EMIT_FILE").ok().as_deref() {
        Some("0") | Some("false") | Some("False") | Some("FALSE") => {
            // Explicitly disabled via env var (overrides feature)
            false
        }
        Some("1") | Some("true") | Some("True") | Some("TRUE") => {
            // Explicitly enabled via env var (overrides feature)
            true
        }
        _ => {
            // Not set or invalid value - check feature flag
            #[cfg(feature = "emit-proto-files")]
            {
                true
            }

            #[cfg(not(feature = "emit-proto-files"))]
            {
                false
            }
        }
    }
}
/// Generate proto emission code (consts + inventory registration)
/// This is ALWAYS generated when called, but wrapped in #[cfg(feature = "build-schemas")]
pub fn generate_proto_emission(file_name: &str, type_ident: &str, content: &str) -> TokenStream {
    let const_name = format!(
        "PROTO_SCHEMA_{}_{}",
        file_name.to_uppercase().replace(".", "_").replace("/", "_").replace("-", "_"),
        type_ident.to_uppercase()
    );
    let const_ident = syn::Ident::new(&const_name, proc_macro2::Span::call_site());

    quote! {
        #[cfg(feature = "build-schemas")]
        const #const_ident: &str = #content;

        #[cfg(feature = "build-schemas")]
        inventory::submit! {
            proto_rs::schemas::ProtoSchema {
                name: #file_name,
                content: #const_ident,
            }
        }
    }
}

/// Register proto content and optionally write to file
pub fn register_and_emit_proto_inner(file_name: &str, type_ident: &str, content: &str) -> TokenStream {
    // Always generate const emission code (feature-gated)
    let emission_code = generate_proto_emission(file_name, type_ident, content);

    // Conditionally write to file based on env var / feature
    if should_emit_file() {
        crate::write_file::write_proto_file(file_name, content);
    }

    emission_code
}
fn derive_package_name(file_path: &str) -> String {
    file_path.trim_end_matches(".proto").replace(['/', '\\', '-', '.'], "_").to_lowercase()
}

/// Register import for a proto file
pub fn register_imports(type_ident: &str, imports: &BTreeMap<String, BTreeSet<String>>) -> TokenStream {
    let mut code = TokenStream::new();
    for (file, imports) in imports.iter() {
        let mut registry = REGISTRY.lock().unwrap();
        let defs = registry.entry(file.to_string()).or_default();
        for import in imports {
            let import_entry = format!("{}:{}", IMPORT_PREFIX, import);
            defs.insert(import_entry);
        }
        drop(registry);
        if should_emit_file() {
            write_proto_file_internal(file);
        }
        let imports = imports.iter().map(|x| format_import(x)).collect::<Vec<_>>().join("");
        let add_code = generate_proto_emission(file, &format!("{type_ident}ImportInject"), &imports);
        code = quote! {#code #add_code };
    }
    code
}

pub fn register_import(file: &str, imports: &[String]) -> TokenStream {
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file.to_string()).or_default();
    let mut content = String::new();
    for import in imports {
        content += &format_import(import);

        let import_entry = format!("{}:{}", IMPORT_PREFIX, import);
        defs.insert(import_entry);
    }
    drop(registry);
    let emission_code = generate_proto_emission(file, "ImportInject", &content);
    if should_emit_file() {
        write_proto_file_internal(file);
    }
    emission_code
}

/// Write proto content to registry (does NOT write file unless emission enabled)
fn write_proto_file(file_name_path: &str, content: &str) {
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file_name_path.to_string()).or_default();

    let was_new = defs.insert(content.to_string());

    if was_new && should_emit_file() {
        drop(registry); // Release lock before writing
        write_proto_file_internal(file_name_path);
    }
}

/// Internal function to actually write the .proto file
fn write_proto_file_internal(file_name_path: &str) {
    let path = Path::new(".").join(file_name_path);
    let file_name_last = path.file_name().unwrap().to_str().unwrap();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let registry = REGISTRY.lock().unwrap();
    let Some(defs) = registry.get(file_name_path) else {
        return;
    };

    // Collect imports (already sorted by BTreeSet)
    let imports: Vec<String> = defs
        .iter()
        .filter(|e| e.starts_with(IMPORT_PREFIX))
        .map(|e| e.strip_prefix(&format!("{}:", IMPORT_PREFIX)).unwrap().to_string())
        .collect();

    // Collect content (already sorted by BTreeSet)
    let content_items: Vec<String> = defs.iter().filter(|e| !e.starts_with(IMPORT_PREFIX)).cloned().collect();

    // Build complete file content
    let file_content = build_complete_proto_file(file_name_last, &imports, &content_items);

    // Write entire file atomically
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(&path).expect("Failed to open proto file for writing");

    write!(file, "{}", file_content).expect("Failed to write proto file");

    // Mark as initialized
    drop(registry);
    let mut initialized = INITIALIZED_FILES.lock().unwrap();
    initialized.insert(file_name_path.to_string());
}

fn format_import(import: &str) -> String {
    format!("import \"{}.proto\";\n", import)
}
/// Build the complete proto file content from scratch
fn build_complete_proto_file(file_name: &str, imports: &[String], content_items: &[String]) -> String {
    let package_name = derive_package_name(file_name);

    let mut output = String::new();

    // Header
    output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
    output.push_str("syntax = \"proto3\";\n");
    output.push_str(&format!("package {};\n", package_name));

    // Imports (already sorted)
    if !imports.is_empty() {
        output.push('\n');
        for import in imports {
            output.push_str(&format_import(import));
        }
    }

    // Content separator
    output.push('\n');

    // All content items (already sorted by BTreeSet)
    for item in content_items {
        output.push_str(item);
    }

    output
}

#[cfg(test)]
pub mod test {
    use super::write_proto_file;

    #[test]
    fn test_write_to_file() {
        write_proto_file("test_me.proto", "sweet content");
    }
}
