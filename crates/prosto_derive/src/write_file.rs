use std::fs::OpenOptions;
use std::fs::{self};
use std::io::Write;
use std::path::Path;

const IMPORT_PREFIX: &str = "__IMPORT__:";

fn derive_package_name(file_path: &str) -> String {
    file_path.trim_end_matches(".proto").replace(['/', '\\', '-', '.'], "_").to_lowercase()
}

use crate::utils::INITIALIZED_FILES;
use crate::utils::REGISTRY;

pub fn register_import(file_name: &str, import_name: &str) {
    let import_entry = format!("__IMPORT__:{}", import_name);
    write_proto_file(file_name, &import_entry);
}

pub fn write_proto_file(file_name_path: &str, content: &str) {
    let path = Path::new(".").join(file_name_path);
    let file_name_last = path.file_name().unwrap().to_str().unwrap();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Lock registry and add new content
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file_name_path.to_string()).or_default();

    // Add content to registry (deduplication happens here)
    let was_new = defs.insert(content.to_string());

    // If this is a duplicate, skip rewriting
    if !was_new {
        return;
    }

    // Collect and sort imports
    let mut imports: Vec<String> = defs
        .iter()
        .filter(|e| e.starts_with(IMPORT_PREFIX))
        .map(|e| e.strip_prefix(IMPORT_PREFIX).unwrap().to_string())
        .collect();
    imports.sort();
    imports.dedup();

    // Collect content (everything that's not an import marker)
    let content_items: Vec<String> = defs.iter().filter(|e| !e.starts_with(IMPORT_PREFIX)).cloned().collect();

    // Build complete file content
    let file_content = build_complete_proto_file(file_name_last, &imports, &content_items);

    // Write entire file atomically
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(&path).expect("Failed to open proto file for writing");

    write!(file, "{}", file_content).expect("Failed to write proto file");

    // Mark as initialized
    let mut initialized = INITIALIZED_FILES.lock().unwrap();
    initialized.insert(file_name_path.to_string());
}

/// Build the complete proto file content from scratch
fn build_complete_proto_file(file_name: &str, imports: &[String], content_items: &[String]) -> String {
    let package_name = derive_package_name(file_name);

    let mut output = String::new();

    // Header
    output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
    output.push_str("syntax = \"proto3\";\n");
    output.push_str(&format!("package {};\n", package_name));

    // Imports
    if !imports.is_empty() {
        output.push('\n');
        for import in imports {
            output.push_str(&format!("import \"{}.proto\";\n", import));
        }
    }

    // Content separator
    output.push('\n');

    // All content items
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
        write_proto_file("test_me", "sweet content");
    }
}
