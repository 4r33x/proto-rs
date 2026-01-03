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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtoTypeId {
    crate_name: String,
    module_path: String,
    type_name: String,
    proto_name: Option<String>,
}

impl ProtoTypeId {
    pub fn for_type(module_path: &str, type_name: &str) -> Self {
        Self::new(module_path, type_name, None)
    }

    pub fn for_definition(module_path: &str, type_name: &str, proto_name: &str) -> Self {
        let proto_name = (proto_name != type_name).then_some(proto_name.to_string());
        Self::new(module_path, type_name, proto_name)
    }

    fn for_import(import: &str) -> Self {
        Self::new("import", IMPORT_PREFIX, Some(import.to_string()))
    }

    fn new(module_path: &str, type_name: &str, proto_name: Option<String>) -> Self {
        let crate_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown_crate".to_string());
        Self {
            crate_name,
            module_path: module_path.to_string(),
            type_name: type_name.to_string(),
            proto_name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_name: String,
    pub transparent: Option<String>,
}

#[derive(Debug, Clone)]
struct ProtoEntry {
    raw_content: String,
    content: String,
    is_import: bool,
}

impl ProtoEntry {
    fn new_definition(content: &str, type_registry: &HashMap<ProtoTypeId, TypeInfo>) -> Self {
        let raw_content = content.to_string();
        let content = rewrite_proto_content(&raw_content, type_registry);
        Self {
            raw_content,
            content,
            is_import: false,
        }
    }

    fn new_import(import: &str) -> Self {
        Self {
            raw_content: import.to_string(),
            content: import.to_string(),
            is_import: true,
        }
    }

    fn update_raw_content(&mut self, content: &str, type_registry: &HashMap<ProtoTypeId, TypeInfo>) -> bool {
        if self.raw_content == content {
            return false;
        }
        self.raw_content = content.to_string();
        self.content = rewrite_proto_content(&self.raw_content, type_registry);
        true
    }

    fn refresh_content(&mut self, type_registry: &HashMap<ProtoTypeId, TypeInfo>) {
        if !self.is_import {
            self.content = rewrite_proto_content(&self.raw_content, type_registry);
        }
    }
}

/// Global registry: filename -> `BTreeMap`<proto definitions>
static REGISTRY: LazyLock<Mutex<HashMap<String, BTreeMap<ProtoTypeId, ProtoEntry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static TYPE_REGISTRY: LazyLock<Mutex<HashMap<ProtoTypeId, TypeInfo>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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

pub fn module_path_from_call_site() -> String {
    module_path!().to_string()
}

pub fn register_type_info(proto_type_id: ProtoTypeId, info: TypeInfo) {
    let mut type_registry = TYPE_REGISTRY.lock().unwrap();
    type_registry.insert(proto_type_id, info);

    let mut registry = REGISTRY.lock().unwrap();
    for entries in registry.values_mut() {
        for entry in entries.values_mut() {
            entry.refresh_content(&type_registry);
        }
    }
}

/// Generate proto emission code (const + inventory registration)
pub fn generate_proto_emission(file_name: &str, type_ident: &str, content: &str) -> TokenStream {
    let const_name = format_const_name(file_name, type_ident);
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

fn format_const_name(file_name: &str, type_ident: &str) -> String {
    format!("PROTO_SCHEMA_{}_{}", file_name.to_uppercase().replace(['.', '/', '-'], "_"), type_ident.to_uppercase())
}

/// Register proto content and optionally write to file
pub fn register_and_emit_proto_inner(file_name: &str, type_ident: &str, proto_type_id: ProtoTypeId, content: &str) -> TokenStream {
    let emission_code = generate_proto_emission(file_name, type_ident, content);

    if should_emit_file() {
        write_proto_file(file_name, proto_type_id, content);
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
        let imports_content = import_set.iter().map(|imp| format_import(imp)).collect::<String>();

        let emission = generate_proto_emission(file, &format!("{type_ident}_ImportInject"), &imports_content);

        code = quote! { #code #emission };
    }

    code
}

fn register_imports_in_registry(file: &str, imports: &BTreeSet<String>) {
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file.to_string()).or_default();

    for import in imports {
        defs.entry(ProtoTypeId::for_import(import)).or_insert_with(|| ProtoEntry::new_import(import));
    }
}

/// Register single import
pub fn register_import(file: &str, imports: &[String]) -> TokenStream {
    let mut content = String::new();

    {
        let mut registry = REGISTRY.lock().unwrap();
        let defs = registry.entry(file.to_string()).or_default();

        for import in imports {
            content.push_str(&format_import(import));
            defs.entry(ProtoTypeId::for_import(import)).or_insert_with(|| ProtoEntry::new_import(import));
        }
    }

    let emission_code = generate_proto_emission(file, "ImportInject", &content);

    if should_emit_file() {
        write_proto_file_internal(file);
    }

    emission_code
}

/// Write proto content to registry
fn write_proto_file(file_name_path: &str, proto_type_id: ProtoTypeId, content: &str) {
    let type_registry = TYPE_REGISTRY.lock().unwrap();
    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file_name_path.to_string()).or_default();
    let was_new = match defs.get_mut(&proto_type_id) {
        Some(entry) => entry.update_raw_content(content, &type_registry),
        None => {
            defs.insert(proto_type_id, ProtoEntry::new_definition(content, &type_registry));
            true
        }
    };

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

fn separate_imports_and_content(defs: &BTreeMap<ProtoTypeId, ProtoEntry>) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut content = Vec::new();

    for entry in defs.values() {
        if entry.is_import {
            imports.push(entry.content.clone());
        } else {
            content.push(entry.content.clone());
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

fn rewrite_proto_content(content: &str, type_registry: &HashMap<ProtoTypeId, TypeInfo>) -> String {
    let replacements: HashMap<&str, &str> = type_registry
        .values()
        .filter_map(|info| info.transparent.as_deref().map(|transparent| (info.type_name.as_str(), transparent)))
        .collect();

    if replacements.is_empty() {
        return content.to_string();
    }

    let mut output = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let (line_body, line_ending) = line.split_at(line.len().saturating_sub(line.ends_with('\n') as usize));
        let rewritten = rewrite_proto_line(line_body, &replacements);
        output.push_str(&rewritten);
        output.push_str(line_ending);
    }

    output
}

fn rewrite_proto_line(line: &str, replacements: &HashMap<&str, &str>) -> String {
    let trimmed = line.trim();
    if !trimmed.ends_with(';') || !trimmed.contains('=') {
        return line.to_string();
    }

    let Some(trimmed_no_semi) = trimmed.strip_suffix(';') else {
        return line.to_string();
    };

    let Some((lhs, rhs)) = trimmed_no_semi.split_once('=') else {
        return line.to_string();
    };

    let tokens: Vec<&str> = lhs.split_whitespace().collect();
    if tokens.len() < 2 {
        return line.to_string();
    }

    let field_name = tokens[tokens.len() - 1];
    let mut type_tokens = &tokens[..tokens.len() - 1];
    let mut modifier = "";

    if let Some(first) = type_tokens.first()
        && (*first == "optional" || *first == "repeated")
    {
        modifier = first;
        type_tokens = &type_tokens[1..];
    }

    if type_tokens.is_empty() {
        return line.to_string();
    }

    let type_value = type_tokens.join(" ");
    let updated_type = rewrite_type_value(&type_value, replacements);

    let prefix_len = line.len() - line.trim_start().len();
    let prefix = &line[..prefix_len];

    let rhs_trim = rhs.trim();
    if modifier.is_empty() {
        format!("{prefix}{updated_type} {field_name} = {rhs_trim};")
    } else {
        format!("{prefix}{modifier} {updated_type} {field_name} = {rhs_trim};")
    }
}

fn rewrite_type_value(type_value: &str, replacements: &HashMap<&str, &str>) -> String {
    if let Some(replacement) = replacements.get(type_value) {
        return (*replacement).to_string();
    }

    if let Some(inner) = type_value.strip_prefix("map<").and_then(|value| value.strip_suffix('>')) {
        let mut parts = inner.split(',').map(|part| part.trim()).collect::<Vec<_>>();
        if parts.len() == 2 {
            if let Some(replacement) = replacements.get(parts[1]) {
                parts[1] = replacement;
            }
            return format!("map<{}, {}>", parts[0], parts[1]);
        }
    }

    type_value.to_string()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_format_const_name() {
        let name = format_const_name("path/to/file.proto", "MyStruct");
        assert_eq!(name, "PROTO_SCHEMA_PATH_TO_FILE_PROTO_MYSTRUCT");
    }

    #[test]
    fn test_separate_imports_and_content() {
        let mut defs = BTreeMap::new();
        defs.insert(ProtoTypeId::for_import("common/types"), ProtoEntry::new_import("common/types"));
        defs.insert(
            ProtoTypeId::for_definition("crate", "Foo", "Foo"),
            ProtoEntry::new_definition("message Foo {}", &HashMap::new()),
        );
        defs.insert(ProtoTypeId::for_import("other/service"), ProtoEntry::new_import("other/service"));

        let (imports, content) = separate_imports_and_content(&defs);

        assert_eq!(imports.len(), 2);
        assert!(imports.contains(&"common/types".to_string()));
        assert!(imports.contains(&"other/service".to_string()));

        assert_eq!(content.len(), 1);
        assert_eq!(content[0], "message Foo {}");
    }

    #[test]
    fn rewrites_transparent_types_in_proto_fields() {
        let mut type_registry = HashMap::new();
        type_registry.insert(
            ProtoTypeId::for_type("crate::types", "UserIdNamed"),
            TypeInfo {
                type_name: "UserIdNamed".to_string(),
                transparent: Some("uint64".to_string()),
            },
        );

        let content = "\
message UserWithId {
  UserIdNamed id = 1;
  optional UserIdNamed opt = 2;
  map<uint32, UserIdNamed> ids = 3;
}
";

        let rewritten = rewrite_proto_content(content, &type_registry);

        assert!(rewritten.contains("uint64 id = 1;"));
        assert!(rewritten.contains("optional uint64 opt = 2;"));
        assert!(rewritten.contains("map<uint32, uint64> ids = 3;"));
    }

    #[test]
    fn test_should_emit_file() {
        // Test depends on env var and feature flag
        // Just ensure it doesn't panic
        let _ = should_emit_file();
    }
}
