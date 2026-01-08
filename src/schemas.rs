use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::LazyLock;

/// Represents a proto schema collected at compile time
#[derive(Clone, Copy)]
pub struct ProtoSchema {
    pub id: ProtoIdent,
    pub generics: &'static [Generic],
    pub lifetimes: &'static [Lifetime],
    pub top_level_attributes: &'static [Attribute],
    pub content: ProtoEntry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProtoIdent {
    pub module_path: &'static str,
    pub name: &'static str,
    pub proto_package_name: &'static str,
    pub proto_file_path: &'static str,
    pub proto_type: &'static str,
}

pub trait ProtoIdentifiable {
    const PROTO_IDENT: ProtoIdent;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Attribute {
    pub path: &'static str,
    pub tokens: &'static str,
}

#[derive(Clone, Copy)]
pub struct Generic {
    pub name: &'static str,
    pub kind: GenericKind,
    pub constraints: &'static [&'static str],
    pub const_type: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub enum GenericKind {
    Type,
    Const,
}

pub struct Lifetime {
    pub name: &'static str,
    pub bounds: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub enum ProtoEntry {
    SimpleEnum { variants: &'static [&'static Variant] },
    Struct { fields: &'static [&'static Field] },
    ComplexEnum { variants: &'static [&'static Variant] },
    Import { paths: &'static [&'static str] },
    Service { methods: &'static [&'static ServiceMethod] },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variant {
    pub name: &'static str,
    pub fields: &'static [&'static Field],
    pub discriminant: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: Option<&'static str>,
    pub proto_ident: ProtoIdent,
    pub proto_label: ProtoLabel,
    pub tag: u32,
    pub attributes: &'static [Attribute],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceMethod {
    pub name: &'static str,
    pub request: ProtoIdent,
    pub response: ProtoIdent,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtoLabel {
    None,
    Optional,
    Repeated,
}

// Auto-collect all schemas via inventory
inventory::collect!(ProtoSchema);

fn derive_package_name(file_path: &str) -> String {
    file_path.trim_end_matches(".proto").replace(['/', '\\', '-', '.'], "_").to_lowercase()
}
static REGISTRY: LazyLock<BTreeMap<String, Vec<&'static ProtoSchema>>> = LazyLock::new(|| build_registry().0);

/// Get an iterator over all registered proto schemas
///
/// Schemas are automatically collected from all crates that use
/// proto_dump, proto_message, or proto_rpc macros when compiled
/// with the "build-schemas" feature.
pub fn all() -> impl Iterator<Item = &'static ProtoSchema> {
    inventory::iter::<ProtoSchema>.into_iter()
}

/// Write all registered proto schemas to a directory
///
/// # Arguments
/// * `output_dir` - The directory to write .proto files to
///
/// # Returns
/// The number of proto files written
///
/// # Example
/// ```no_run
/// // In main.rs or build.rs (all protos should be declared in other_crates)
/// fn your_main() {
///     if std::env::var("GENERATE_PROTOS").is_ok() {
///         let count = proto_rs::schemas::write_all("protos")
///             .expect("Failed to write proto files");
///         println!("Generated {} proto files", count);
///     }
/// }
/// ```
/// Write all registered proto schemas to a directory
/// # Errors
///
/// Will return `Err` if fs throws error
pub fn write_all(output_dir: &str) -> io::Result<usize> {
    use std::fmt::Write;
    match fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    fs::create_dir_all(output_dir)?;
    let mut count = 0;
    let (registry, ident_index) = build_registry();

    for (file_name, entries) in registry {
        let output_path = format!("{output_dir}/{file_name}");

        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let path = Path::new(file_name.as_str());
        let file_name_last = path.file_name().unwrap().to_str().unwrap();
        let package_name = entries
            .first()
            .map(|schema| schema.id.proto_package_name)
            .filter(|name| !name.is_empty())
            .map_or(derive_package_name(file_name_last), ToString::to_string);
        let mut output = String::new();

        output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
        output.push_str("syntax = \"proto3\";\n");
        writeln!(&mut output, "package {package_name};").unwrap();

        output.push('\n');

        let imports = collect_imports(entries.as_slice(), &ident_index, &file_name, &package_name)?;
        if !imports.is_empty() {
            let mut import_stems = BTreeSet::new();
            for import in &imports {
                let import_path = Path::new(import);
                let import_file = import_path.file_name().and_then(|name| name.to_str()).unwrap_or(import);
                let import_stem = import_file.strip_suffix(".proto").unwrap_or(import_file);
                import_stems.insert(import_stem.to_string());
            }
            for import_stem in import_stems {
                writeln!(&mut output, "import \"{import_stem}.proto\";").unwrap();
            }
            output.push('\n');
        }

        let mut ordered_entries: Vec<&ProtoSchema> = entries.clone();

        ordered_entries.sort_by(|left, right| entry_sort_key(left).cmp(&entry_sort_key(right)));

        for entry in ordered_entries {
            if let Some(definition) = render_entry(entry, &package_name, &ident_index) {
                output.push_str(&definition);
                output.push('\n');
            }
        }

        fs::write(&output_path, output)?;
        count += 1;
    }

    Ok(count)
}

/// Get the total number of registered files
pub fn count() -> usize {
    REGISTRY.len()
}

/// Get all filenames in the registry
pub fn file_names() -> Vec<String> {
    REGISTRY.keys().cloned().collect()
}

fn build_registry() -> (BTreeMap<String, Vec<&'static ProtoSchema>>, BTreeMap<ProtoIdent, &'static ProtoSchema>) {
    let mut registry = BTreeMap::new();
    let mut ident_index = BTreeMap::new();

    for schema in inventory::iter::<ProtoSchema>() {
        if ident_index.insert(schema.id, schema).is_some() {
            continue;
        }
        if schema.id.proto_file_path.is_empty() {
            continue;
        }
        registry.entry(schema.id.proto_file_path.to_string()).or_insert_with(Vec::new).push(schema);
    }

    (registry, ident_index)
}

fn collect_imports(entries: &[&ProtoSchema], ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>, file_name: &str, package_name: &str) -> io::Result<BTreeSet<String>> {
    let mut imports = BTreeSet::new();

    for entry in entries {
        match entry.content {
            ProtoEntry::Import { paths } => {
                for path in paths {
                    imports.insert(path.to_string());
                }
            }
            ProtoEntry::Struct { fields } => {
                collect_field_imports(&mut imports, ident_index, fields, file_name, package_name)?;
            }
            ProtoEntry::SimpleEnum { .. } => {}
            ProtoEntry::ComplexEnum { variants } => {
                for variant in variants {
                    collect_field_imports(&mut imports, ident_index, variant.fields, file_name, package_name)?;
                }
            }
            ProtoEntry::Service { methods } => {
                collect_service_imports(&mut imports, ident_index, methods, file_name, package_name)?;
            }
        }
    }

    Ok(imports)
}

fn collect_field_imports(imports: &mut BTreeSet<String>, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>, fields: &[&Field], file_name: &str, package_name: &str) -> io::Result<()> {
    for field in fields {
        let ident = resolve_transparent_ident(field.proto_ident, ident_index);
        collect_proto_ident_imports(imports, ident_index, &ident, file_name, package_name)?;
    }
    Ok(())
}

fn collect_service_imports(
    imports: &mut BTreeSet<String>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    methods: &[&ServiceMethod],
    file_name: &str,
    package_name: &str,
) -> io::Result<()> {
    for method in methods {
        let request = resolve_transparent_ident(method.request, ident_index);
        let response = resolve_transparent_ident(method.response, ident_index);
        collect_proto_ident_imports(imports, ident_index, &request, file_name, package_name)?;
        collect_proto_ident_imports(imports, ident_index, &response, file_name, package_name)?;
    }
    Ok(())
}

fn collect_proto_ident_imports(imports: &mut BTreeSet<String>, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>, ident: &ProtoIdent, file_name: &str, package_name: &str) -> io::Result<()> {
    if ident.proto_file_path.is_empty() {
        return Ok(());
    }

    if ident.proto_file_path == file_name {
        return Ok(());
    }

    if ident.proto_package_name.is_empty() && ident.proto_file_path.is_empty() {
        return Ok(());
    }

    if ident.proto_package_name != package_name || ident.proto_file_path != file_name {
        if !ident.module_path.is_empty() && !ident_index.contains_key(ident) {
            return Err(io::Error::other(format!(
                "unresolved ProtoIdent for {} (file: {}, package: {})",
                ident.proto_type, ident.proto_file_path, ident.proto_package_name
            )));
        }
        imports.insert(ident.proto_file_path.to_string());
    }

    Ok(())
}

fn render_entry(entry: &ProtoSchema, package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> Option<String> {
    match entry.content {
        ProtoEntry::Struct { fields } => Some(render_struct(entry.id.proto_type, fields, package_name, ident_index)),
        ProtoEntry::SimpleEnum { variants } => Some(render_simple_enum(entry.id.proto_type, variants)),
        ProtoEntry::ComplexEnum { variants } => Some(render_complex_enum(entry.id.proto_type, variants, package_name, ident_index)),
        ProtoEntry::Import { .. } => None,
        ProtoEntry::Service { methods } => Some(render_service(entry.id.proto_type, methods, package_name, ident_index)),
    }
}

fn render_struct(name: &str, fields: &[&Field], package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    if fields.is_empty() {
        return format!("message {name} {{}}\n");
    }

    let mut lines = Vec::new();
    for (idx, field) in fields.iter().enumerate() {
        lines.push(render_field(field, idx, package_name, ident_index));
    }

    format!("message {name} {{\n{}\n}}\n", lines.join("\n"))
}

fn render_simple_enum(name: &str, variants: &[&Variant]) -> String {
    let mut lines = Vec::new();
    for variant in variants {
        let value = variant.discriminant.unwrap_or_default();
        lines.push(format!("  {} = {};", variant.name, value));
    }
    format!("enum {name} {{\n{}\n}}\n", lines.join("\n"))
}

fn render_complex_enum(name: &str, variants: &[&Variant], package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let mut nested_messages = Vec::new();
    let mut oneof_fields = Vec::new();

    for (idx, variant) in variants.iter().enumerate() {
        let tag = idx + 1;
        let variant_name = variant.name;
        let field_name = to_snake_case(variant_name);

        if variant.fields.is_empty() {
            let msg_name = format!("{name}{variant_name}");
            nested_messages.push(format!("message {msg_name} {{}}"));
            oneof_fields.push(format!("    {msg_name} {field_name} = {tag};"));
            continue;
        }

        if variant.fields.len() == 1 && variant.fields[0].name.is_none() {
            let proto_type = field_type_name(variant.fields[0], package_name, ident_index);
            oneof_fields.push(format!("    {proto_type} {field_name} = {tag};"));
            continue;
        }

        let msg_name = format!("{name}{variant_name}");
        let field_defs = render_named_fields(variant.fields, package_name, ident_index);
        nested_messages.push(format!("message {msg_name} {{\n{field_defs}\n}}"));
        oneof_fields.push(format!("    {msg_name} {field_name} = {tag};"));
    }

    format!("{}\nmessage {} {{\n  oneof value {{\n{}\n  }}\n}}\n", nested_messages.join("\n\n"), name, oneof_fields.join("\n"))
}

fn render_named_fields(fields: &[&Field], package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let mut lines = Vec::new();
    for (idx, field) in fields.iter().enumerate() {
        lines.push(render_field(field, idx, package_name, ident_index));
    }
    lines.join("\n")
}

fn render_field(field: &Field, idx: usize, package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let name = field.name.map_or_else(|| format!("field_{idx}"), ToString::to_string);
    let label = match field.proto_label {
        ProtoLabel::None => "",
        ProtoLabel::Optional => "optional ",
        ProtoLabel::Repeated => "repeated ",
    };
    let proto_type = field_type_name(field, package_name, ident_index);
    format!("  {label}{proto_type} {name} = {};", field.tag)
}

fn render_service(name: &str, methods: &[&ServiceMethod], package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let mut lines = Vec::new();
    lines.push(format!("service {name} {{"));

    for method in methods {
        let request_type = proto_ident_type_name(method.request, package_name, ident_index);
        let response_type = proto_ident_type_name(method.response, package_name, ident_index);
        let response_type = if method.server_streaming { format!("stream {response_type}") } else { response_type };
        lines.push(format!("  rpc {}({}) returns ({});", method.name, request_type, response_type));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

fn field_type_name(field: &Field, package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let ident = resolve_transparent_ident(field.proto_ident, ident_index);
    if ident.proto_type.starts_with("map<") {
        return ident.proto_type.to_string();
    }
    proto_ident_type_name(ident, package_name, ident_index)
}

fn proto_ident_type_name(ident: ProtoIdent, package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let ident = resolve_transparent_ident(ident, ident_index);
    if ident.proto_package_name.is_empty() || ident.proto_package_name == package_name {
        ident.proto_type.to_string()
    } else {
        format!("{}.{}", ident.proto_package_name, ident.proto_type)
    }
}

fn entry_sort_key(entry: &ProtoSchema) -> (u8, &'static str) {
    let kind = match entry.content {
        ProtoEntry::Import { .. } => 0,
        ProtoEntry::SimpleEnum { .. } => 1,
        ProtoEntry::Struct { .. } => 2,
        ProtoEntry::ComplexEnum { .. } => 3,
        ProtoEntry::Service { .. } => 4,
    };
    (kind, entry.id.proto_type)
}

fn resolve_transparent_ident(ident: ProtoIdent, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> ProtoIdent {
    transparent_inner_ident(&ident, ident_index).unwrap_or(ident)
}

fn transparent_inner_ident(ident: &ProtoIdent, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> Option<ProtoIdent> {
    let schema = ident_index.get(ident)?;
    if !is_transparent_schema(schema) {
        return None;
    }

    match schema.content {
        ProtoEntry::Struct { fields } if fields.len() == 1 => Some(fields[0].proto_ident),
        _ => None,
    }
}

fn is_transparent_schema(schema: &ProtoSchema) -> bool {
    schema.top_level_attributes.iter().any(|attr| attr.path == "proto_message" && attr.tokens.contains("transparent"))
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut prev_is_lower = false;
    let mut prev_is_upper = false;

    while let Some(c) = chars.next() {
        let next_is_upper = chars.peek().is_some_and(|ch| ch.is_uppercase());
        let next_is_lower = chars.peek().is_some_and(|ch| ch.is_lowercase());

        if c.is_uppercase() && !result.is_empty() && (prev_is_lower || prev_is_upper && (next_is_upper || next_is_lower)) {
            result.push('_');
        }

        result.push(c.to_ascii_lowercase());
        prev_is_lower = c.is_lowercase();
        prev_is_upper = c.is_uppercase();
    }

    result
}
