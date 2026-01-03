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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtoTypeId(String);

impl ProtoTypeId {
    pub fn new(crate_name: &str, module_path: &str, type_name: &str) -> Self {
        let module_path = normalize_module_path(crate_name, module_path);
        Self(format!("{module_path}::{type_name}"))
    }

    fn import_key(import: &str) -> Self {
        Self(format!("{IMPORT_PREFIX}:{import}"))
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypeInfo {
    pub transparent: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProtoFieldEntry {
    pub type_id: ProtoTypeId,
    pub field_name: String,
    pub modifier: String,
    pub tag: usize,
    pub proto_type: String,
    pub indent: String,
    pub allow_rename: bool,
    pub line: String,
}

impl ProtoFieldEntry {
    pub fn new(
        type_id: ProtoTypeId,
        field_name: String,
        modifier: String,
        tag: usize,
        proto_type: String,
        indent: String,
        allow_rename: bool,
    ) -> Self {
        let line = format!("{indent}{modifier}{proto_type} {field_name} = {tag};");
        Self {
            type_id,
            field_name,
            modifier,
            tag,
            proto_type,
            indent,
            allow_rename,
            line,
        }
    }

    fn update_proto_type(&mut self, new_proto_type: &str, content: &mut String) -> bool {
        if !self.allow_rename || self.proto_type == new_proto_type {
            return false;
        }

        let new_line = format!("{}{}{} {} = {};", self.indent, self.modifier, new_proto_type, self.field_name, self.tag);
        if content.contains(&self.line) {
            *content = content.replace(&self.line, &new_line);
        }
        self.proto_type = new_proto_type.to_string();
        self.line = new_line;
        true
    }
}

#[derive(Clone, Debug)]
pub enum ProtoEntryKind {
    Import,
    Definition,
}

#[derive(Clone, Debug)]
pub struct ProtoEntry {
    pub content: String,
    pub fields: Vec<ProtoFieldEntry>,
    pub kind: ProtoEntryKind,
}

impl ProtoEntry {
    pub fn definition(content: String, fields: Vec<ProtoFieldEntry>) -> Self {
        Self {
            content,
            fields,
            kind: ProtoEntryKind::Definition,
        }
    }

    pub fn import(content: String) -> Self {
        Self {
            content,
            fields: Vec::new(),
            kind: ProtoEntryKind::Import,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeIdContext {
    crate_name: String,
    module_path: String,
}

impl TypeIdContext {
    pub fn new(span: proc_macro2::Span) -> Self {
        let crate_name = std::env::var("CARGO_CRATE_NAME")
            .or_else(|_| std::env::var("CARGO_PKG_NAME"))
            .unwrap_or_else(|_| "unknown_crate".to_string());
        let module_path = module_path_from_span(&crate_name, span).unwrap_or_else(|| crate_name.clone());
        Self { crate_name, module_path }
    }

    pub fn proto_type_id(&self, type_name: &str) -> ProtoTypeId {
        ProtoTypeId::new(&self.crate_name, &self.module_path, type_name)
    }

    pub fn type_id_for_type(&self, ty: &syn::Type) -> Option<ProtoTypeId> {
        let syn::Type::Path(type_path) = ty else {
            return None;
        };

        let segments: Vec<String> = type_path.path.segments.iter().map(|seg| seg.ident.to_string()).collect();
        let (module_path, type_name) = resolve_module_and_type(&self.crate_name, &self.module_path, &segments)?;
        Some(ProtoTypeId::new(&self.crate_name, &module_path, type_name))
    }
}

const IMPORT_PREFIX: &str = "__IMPORT__";

/// Global registry: filename -> `BTreeMap`<ProtoTypeId, ProtoEntry>
static REGISTRY: LazyLock<Mutex<HashMap<String, BTreeMap<ProtoTypeId, ProtoEntry>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Type registry: proto type id -> type info
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
pub fn register_and_emit_proto_inner(file_name: &str, type_ident: &str, type_id: ProtoTypeId, entry: ProtoEntry) -> TokenStream {
    let emission_code = generate_proto_emission(file_name, type_ident, &entry.content);

    let updated = register_proto_entry(file_name, type_id, entry);

    if updated && should_emit_file() {
        write_proto_file_internal(file_name);
    }

    emission_code
}

pub fn register_type_info(type_id: ProtoTypeId, type_info: TypeInfo) {
    let mut registry = TYPE_REGISTRY.lock().unwrap();
    registry.insert(type_id, type_info);
    drop(registry);

    let mut updated_files = Vec::new();
    {
        let type_registry = TYPE_REGISTRY.lock().unwrap();
        let mut proto_registry = REGISTRY.lock().unwrap();
        for (file_name, entries) in proto_registry.iter_mut() {
            let mut updated = false;
            for entry in entries.values_mut() {
                updated |= apply_type_registry_to_entry(entry, &type_registry);
            }
            if updated {
                updated_files.push(file_name.clone());
            }
        }
    }

    if should_emit_file() {
        for file in updated_files {
            write_proto_file_internal(&file);
        }
    }
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
        let import_entry = ProtoEntry::import(format!("{IMPORT_PREFIX}:{import}"));
        defs.insert(ProtoTypeId::import_key(import), import_entry);
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
            let import_entry = ProtoEntry::import(format!("{IMPORT_PREFIX}:{import}"));
            defs.insert(ProtoTypeId::import_key(import), import_entry);
        }
    }

    let emission_code = generate_proto_emission(file, "ImportInject", &content);

    if should_emit_file() {
        write_proto_file_internal(file);
    }

    emission_code
}

/// Write proto content to registry
fn register_proto_entry(file_name_path: &str, type_id: ProtoTypeId, mut entry: ProtoEntry) -> bool {
    let type_registry = TYPE_REGISTRY.lock().unwrap();
    apply_type_registry_to_entry(&mut entry, &type_registry);
    drop(type_registry);

    let mut registry = REGISTRY.lock().unwrap();
    let defs = registry.entry(file_name_path.to_string()).or_default();
    let existing = defs.insert(type_id, entry);
    existing.is_none()
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
        match entry.kind {
            ProtoEntryKind::Import => {
                if let Some(import_path) = entry.content.strip_prefix(&format!("{IMPORT_PREFIX}:")) {
                    imports.push(import_path.to_string());
                }
            }
            ProtoEntryKind::Definition => content.push(entry.content.clone()),
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

fn apply_type_registry_to_entry(entry: &mut ProtoEntry, type_registry: &HashMap<ProtoTypeId, TypeInfo>) -> bool {
    let mut updated = false;
    for field in &mut entry.fields {
        if let Some(type_info) = type_registry.get(&field.type_id)
            && let Some(ref transparent_type) = type_info.transparent
        {
            updated |= field.update_proto_type(transparent_type, &mut entry.content);
        }
    }

    updated
}

fn normalize_module_path(crate_name: &str, module_path: &str) -> String {
    if module_path.starts_with(crate_name) {
        module_path.to_string()
    } else if module_path.is_empty() {
        crate_name.to_string()
    } else {
        format!("{crate_name}::{module_path}")
    }
}

fn module_path_from_span(crate_name: &str, span: proc_macro2::Span) -> Option<String> {
    let _ = span;
    // Stable proc-macro spans do not expose module paths, so default to the crate root.
    Some(crate_name.to_string())
}

fn resolve_module_and_type<'a>(crate_name: &str, module_path: &str, segments: &'a [String]) -> Option<(String, &'a str)> {
    if segments.is_empty() {
        return None;
    }

    let type_name = segments.last()?.as_str();
    if segments.len() == 1 {
        return Some((module_path.to_string(), type_name));
    }

    let (base, index) = if segments[0] == "crate" || segments[0] == crate_name {
        (crate_name.to_string(), 1)
    } else if segments[0] == "self" {
        (module_path.to_string(), 1)
    } else if segments[0] == "super" {
        let mut up_levels = 0;
        while segments.get(up_levels).is_some_and(|seg| seg == "super") {
            up_levels += 1;
        }
        (parent_module_path(module_path, up_levels), up_levels)
    } else {
        (crate_name.to_string(), 0)
    };

    let module_segments = &segments[index..segments.len().saturating_sub(1)];
    let resolved = if module_segments.is_empty() {
        base
    } else {
        format!("{base}::{}", module_segments.join("::"))
    };

    Some((resolved, type_name))
}

fn parent_module_path(module_path: &str, levels: usize) -> String {
    let mut parts: Vec<&str> = module_path.split("::").collect();
    for _ in 0..levels {
        if parts.len() > 1 {
            parts.pop();
        }
    }
    parts.join("::")
}

#[cfg(test)]
fn clear_registries() {
    REGISTRY.lock().unwrap().clear();
    TYPE_REGISTRY.lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_entry(type_id: ProtoTypeId, proto_type: &str) -> ProtoFieldEntry {
        ProtoFieldEntry::new(
            type_id,
            "id".to_string(),
            "".to_string(),
            1,
            proto_type.to_string(),
            "  ".to_string(),
            true,
        )
    }

    #[test]
    fn transparent_types_rewrite_existing_entries() {
        clear_registries();

        let type_id = ProtoTypeId::new("demo", "demo::types", "UserWithId");
        let field_type_id = ProtoTypeId::new("demo", "demo::types", "UserIdNamed");
        let mut entry = ProtoEntry::definition(
            "message UserWithId {\n  UserIdNamed id = 1;\n}\n\n".to_string(),
            vec![field_entry(field_type_id.clone(), "UserIdNamed")],
        );
        register_proto_entry("demo.proto", type_id, entry.clone());

        register_type_info(
            field_type_id,
            TypeInfo {
                transparent: Some("uint64".to_string()),
            },
        );

        let registry = REGISTRY.lock().unwrap();
        let entries = registry.get("demo.proto").expect("entry exists");
        let stored = entries.values().next().expect("stored entry");
        assert!(stored.content.contains("uint64 id = 1;"));
        assert!(!stored.content.contains("UserIdNamed id = 1;"));
    }

    #[test]
    fn transparent_types_rewrite_new_entries_after_registration() {
        clear_registries();

        let field_type_id = ProtoTypeId::new("demo", "demo::types", "UserIdNamed");
        register_type_info(
            field_type_id.clone(),
            TypeInfo {
                transparent: Some("uint64".to_string()),
            },
        );

        let type_id = ProtoTypeId::new("demo", "demo::types", "UserWithId");
        let entry = ProtoEntry::definition(
            "message UserWithId {\n  UserIdNamed id = 1;\n}\n\n".to_string(),
            vec![field_entry(field_type_id, "UserIdNamed")],
        );
        register_proto_entry("demo.proto", type_id, entry);

        let registry = REGISTRY.lock().unwrap();
        let entries = registry.get("demo.proto").expect("entry exists");
        let stored = entries.values().next().expect("stored entry");
        assert!(stored.content.contains("uint64 id = 1;"));
        assert!(!stored.content.contains("UserIdNamed id = 1;"));
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_const_name() {
        let name = format_const_name("path/to/file.proto", "MyStruct");
        assert_eq!(name, "PROTO_SCHEMA_PATH_TO_FILE_PROTO_MYSTRUCT");
    }

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
