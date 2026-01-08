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

pub struct RustClientCtx<'a> {
    pub output_path: Option<&'a str>,
}

impl<'a> RustClientCtx<'a> {
    pub fn disabled() -> Self {
        Self { output_path: None }
    }

    pub fn enabled(output_path: &'a str) -> Self {
        Self { output_path: Some(output_path) }
    }
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
    SimpleEnum {
        variants: &'static [&'static Variant],
    },
    Struct {
        fields: &'static [&'static Field],
    },
    ComplexEnum {
        variants: &'static [&'static Variant],
    },
    Import {
        paths: &'static [&'static str],
    },
    Service {
        methods: &'static [&'static ServiceMethod],
        rpc_package_name: &'static str,
    },
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
/// * `rust_client_output` - Controls whether a Rust client module is generated
///
/// # Returns
/// The number of proto files written
///
/// # Example
/// ```no_run
/// // In main.rs or build.rs (all protos should be declared in other_crates)
/// fn your_main() {
///     if std::env::var("GENERATE_PROTOS").is_ok() {
///         let count = proto_rs::schemas::write_all("protos", proto_rs::schemas::RustClientCtx::disabled())
///             .expect("Failed to write proto files");
///         println!("Generated {} proto files", count);
///     }
/// }
/// ```
/// Write all registered proto schemas to a directory
/// # Errors
///
/// Will return `Err` if fs throws error
pub fn write_all(output_dir: &str, rust_client_output: RustClientCtx<'_>) -> io::Result<usize> {
    use std::fmt::Write;
    match fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    fs::create_dir_all(output_dir)?;
    let mut count = 0;
    let (registry, ident_index) = build_registry();

    for (file_name, entries) in &registry {
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

        let imports = collect_imports(entries.as_slice(), &ident_index, file_name, &package_name)?;
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

        let mut ordered_entries: Vec<&ProtoSchema> = entries.to_vec();

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

    if let Some(output_path) = rust_client_output.output_path {
        write_rust_client_module(output_path, &registry, &ident_index)?;
    }

    Ok(count)
}

fn write_rust_client_module(output_path: &str, registry: &BTreeMap<String, Vec<&'static ProtoSchema>>, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> io::Result<()> {
    let mut package_by_ident = BTreeMap::new();
    let mut root = ModuleNode::default();
    let proto_type_index = build_proto_type_index(registry);

    for (file_name, entries) in registry {
        let package_name = package_name_for_entries(file_name, entries);
        let module_segments = module_path_segments(&package_name);
        for entry in entries {
            package_by_ident.insert(entry.id, package_name.clone());
            if matches!(entry.content, ProtoEntry::Import { .. }) {
                continue;
            }
            insert_module_entry(&mut root, &module_segments, &package_name, entry);
        }
    }

    let mut output = String::new();
    output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");

    if !root.entries.is_empty() {
        output.push_str("#[allow(unused_imports)]\n");
        output.push_str("use proto_rs::{proto_message, proto_rpc};\n\n");
        render_entries(
            &mut output,
            &root.entries,
            root.package_name.as_deref().unwrap_or(""),
            ident_index,
            &package_by_ident,
            &proto_type_index,
            0,
        );
        output.push('\n');
    }

    for (name, child) in &root.children {
        render_named_module(&mut output, name, child, 0, ident_index, &package_by_ident, &proto_type_index);
    }

    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, output)?;
    Ok(())
}

#[derive(Default)]
struct ModuleNode {
    package_name: Option<String>,
    entries: Vec<&'static ProtoSchema>,
    children: BTreeMap<String, ModuleNode>,
}

fn insert_module_entry(node: &mut ModuleNode, segments: &[String], package_name: &str, entry: &'static ProtoSchema) {
    if segments.is_empty() {
        node.package_name = Some(package_name.to_string());
        node.entries.push(entry);
        return;
    }
    let child = node.children.entry(segments[0].clone()).or_default();
    insert_module_entry(child, &segments[1..], package_name, entry);
}

fn render_named_module(
    output: &mut String,
    name: &str,
    node: &ModuleNode,
    indent: usize,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
) {
    indent_line(output, indent);
    output.push_str("pub mod ");
    output.push_str(name);
    output.push_str(" {\n");

    let inner_indent = indent + 4;
    if !node.entries.is_empty() {
        indent_line(output, inner_indent);
        output.push_str("#[allow(unused_imports)]\n");
        indent_line(output, inner_indent);
        output.push_str("use proto_rs::{proto_message, proto_rpc};\n\n");
    }

    render_entries(
        output,
        &node.entries,
        node.package_name.as_deref().unwrap_or(""),
        ident_index,
        package_by_ident,
        proto_type_index,
        inner_indent,
    );

    for (child_name, child) in &node.children {
        render_named_module(output, child_name, child, inner_indent, ident_index, package_by_ident, proto_type_index);
    }

    indent_line(output, indent);
    output.push_str("}\n");
}

fn render_entries(
    output: &mut String,
    entries: &[&'static ProtoSchema],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    indent: usize,
) {
    if entries.is_empty() {
        return;
    }
    let mut ordered_entries = entries.to_vec();
    ordered_entries.sort_by(|left, right| entry_sort_key(left).cmp(&entry_sort_key(right)));
    for entry in ordered_entries {
        if let Some(definition) = render_rust_entry(entry, package_name, ident_index, package_by_ident, proto_type_index, indent) {
            output.push_str(&definition);
            output.push('\n');
        }
    }
}

fn render_rust_entry(
    entry: &ProtoSchema,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    indent: usize,
) -> Option<String> {
    match entry.content {
        ProtoEntry::Struct { fields } => Some(render_rust_struct(entry, fields, package_name, ident_index, package_by_ident, proto_type_index, indent)),
        ProtoEntry::SimpleEnum { variants } => Some(render_rust_simple_enum(entry, variants, indent)),
        ProtoEntry::ComplexEnum { variants } => Some(render_rust_complex_enum(entry, variants, package_name, ident_index, package_by_ident, proto_type_index, indent)),
        ProtoEntry::Import { .. } => None,
        ProtoEntry::Service { methods, rpc_package_name } => Some(render_rust_service(
            entry,
            methods,
            rpc_package_name,
            package_name,
            ident_index,
            package_by_ident,
            proto_type_index,
            indent,
        )),
    }
}

fn render_rust_struct(
    entry: &ProtoSchema,
    fields: &[&Field],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    indent: usize,
) -> String {
    let mut output = String::new();
    let type_name = entry.id.name;
    let generics = render_generics(entry);
    let is_tuple = fields.iter().all(|field| field.name.is_none());

    render_top_level_attributes(&mut output, entry, indent);

    indent_line(&mut output, indent);
    if fields.is_empty() {
        output.push_str(&format!("pub struct {type_name}{generics};\n"));
        return output;
    }

    if is_tuple {
        output.push_str(&format!("pub struct {type_name}{generics}(\n"));
        for field in fields {
            render_field_attributes(&mut output, field, indent + 4);
            indent_line(&mut output, indent + 4);
            output.push_str("pub ");
            output.push_str(&render_field_type(field, package_name, ident_index, package_by_ident, proto_type_index));
            output.push_str(",\n");
        }
        indent_line(&mut output, indent);
        output.push_str(");\n");
        return output;
    }

    output.push_str(&format!("pub struct {type_name}{generics} {{\n"));
    for field in fields {
        render_field_attributes(&mut output, field, indent + 4);
        indent_line(&mut output, indent + 4);
        let name = field.name.unwrap_or("field");
        output.push_str("pub ");
        output.push_str(name);
        output.push_str(": ");
        output.push_str(&render_field_type(field, package_name, ident_index, package_by_ident, proto_type_index));
        output.push_str(",\n");
    }
    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

fn render_rust_simple_enum(entry: &ProtoSchema, variants: &[&Variant], indent: usize) -> String {
    let mut output = String::new();
    let type_name = entry.id.name;
    let generics = render_generics(entry);

    render_top_level_attributes(&mut output, entry, indent);
    indent_line(&mut output, indent);
    output.push_str(&format!("pub enum {type_name}{generics} {{\n"));
    for variant in variants {
        indent_line(&mut output, indent + 4);
        output.push_str(variant.name);
        if let Some(discriminant) = variant.discriminant {
            output.push_str(&format!(" = {discriminant}"));
        }
        output.push_str(",\n");
    }
    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

fn render_rust_complex_enum(
    entry: &ProtoSchema,
    variants: &[&Variant],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    indent: usize,
) -> String {
    let mut output = String::new();
    let type_name = entry.id.name;
    let generics = render_generics(entry);

    render_top_level_attributes(&mut output, entry, indent);
    indent_line(&mut output, indent);
    output.push_str(&format!("pub enum {type_name}{generics} {{\n"));
    for variant in variants {
        indent_line(&mut output, indent + 4);
        output.push_str(variant.name);
        if variant.fields.is_empty() {
            output.push_str(",\n");
            continue;
        }

        let has_named = variant.fields.iter().any(|field| field.name.is_some());
        if has_named {
            output.push_str(" {\n");
            for field in variant.fields {
                render_field_attributes(&mut output, field, indent + 8);
                indent_line(&mut output, indent + 8);
                let name = field.name.unwrap_or("field");
                output.push_str(name);
                output.push_str(": ");
                output.push_str(&render_field_type(field, package_name, ident_index, package_by_ident, proto_type_index));
                output.push_str(",\n");
            }
            indent_line(&mut output, indent + 4);
            output.push_str("},\n");
        } else {
            output.push_str("(\n");
            for field in variant.fields {
                render_field_attributes(&mut output, field, indent + 8);
                indent_line(&mut output, indent + 8);
                output.push_str(&render_field_type(field, package_name, ident_index, package_by_ident, proto_type_index));
                output.push_str(",\n");
            }
            indent_line(&mut output, indent + 4);
            output.push_str("),\n");
        }
    }
    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

fn render_rust_service(
    entry: &ProtoSchema,
    methods: &[&ServiceMethod],
    rpc_package_name: &str,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    indent: usize,
) -> String {
    let mut output = String::new();
    let trait_name = entry.id.name;
    let generics = render_generics(entry);

    indent_line(&mut output, indent);
    output.push_str("#[proto_rpc(rpc_package = \"");
    output.push_str(rpc_package_name);
    output.push_str("\", rpc_server = false, rpc_client = true)]\n");
    indent_line(&mut output, indent);
    output.push_str(&format!("pub trait {trait_name}{generics} {{\n"));

    let mut stream_types = Vec::new();
    for method in methods {
        if method.server_streaming {
            let stream_name = format!("{}Stream", method.name);
            let response_ident = resolve_transparent_ident(method.response, ident_index);
            let item_type = render_proto_type(response_ident, package_name, package_by_ident, proto_type_index);
            stream_types.push(stream_name.clone());
            indent_line(&mut output, indent + 4);
            output.push_str(&format!(
                "type {stream_name}: ::tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<{item_type}, ::tonic::Status>> + ::core::marker::Send + 'static;\n"
            ));
        }
    }

    if !stream_types.is_empty() {
        output.push('\n');
    }

    for method in methods {
        let request_ident = resolve_transparent_ident(method.request, ident_index);
        let request_type = render_proto_type(request_ident, package_name, package_by_ident, proto_type_index);
        let response_type = if method.server_streaming {
            format!("Self::{}Stream", method.name)
        } else {
            let response_ident = resolve_transparent_ident(method.response, ident_index);
            render_proto_type(response_ident, package_name, package_by_ident, proto_type_index)
        };

        indent_line(&mut output, indent + 4);
        output.push_str(&format!("async fn {}(\n", to_snake_case(method.name)));
        indent_line(&mut output, indent + 8);
        output.push_str(&format!("&self,\n"));
        indent_line(&mut output, indent + 8);
        output.push_str(&format!("request: ::tonic::Request<{request_type}>,\n"));
        indent_line(&mut output, indent + 4);
        output.push_str(") -> ::core::result::Result<::tonic::Response<");
        output.push_str(&response_type);
        output.push_str(">, ::tonic::Status>\n");
        indent_line(&mut output, indent + 4);
        output.push_str("where\n");
        indent_line(&mut output, indent + 8);
        output.push_str("Self: ::core::marker::Send + ::core::marker::Sync;\n\n");
    }

    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

fn render_top_level_attributes(output: &mut String, entry: &ProtoSchema, indent: usize) {
    let mut has_proto_message = false;
    for attr in entry.top_level_attributes {
        if attr.path == "proto_message" {
            has_proto_message = true;
            indent_line(output, indent);
            output.push_str(attr.tokens);
            output.push('\n');
        }
    }
    if !has_proto_message {
        indent_line(output, indent);
        output.push_str("#[proto_message]\n");
    }
}

fn render_field_attributes(output: &mut String, field: &Field, indent: usize) {
    let mut has_proto_attr = false;
    for attr in field.attributes {
        if attr.path == "proto" {
            has_proto_attr = true;
            indent_line(output, indent);
            output.push_str(attr.tokens);
            output.push('\n');
        }
    }
    if !has_proto_attr && field.tag > 0 {
        indent_line(output, indent);
        output.push_str(&format!("#[proto(tag = {})]\n", field.tag));
    }
}

fn render_field_type(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
) -> String {
    let ident = resolve_transparent_ident(field.proto_ident, ident_index);
    let base = render_proto_type(ident, package_name, package_by_ident, proto_type_index);
    match field.proto_label {
        ProtoLabel::None => base,
        ProtoLabel::Optional => format!("::core::option::Option<{base}>"),
        ProtoLabel::Repeated => format!("::proto_rs::alloc::vec::Vec<{base}>"),
    }
}

fn render_proto_type(ident: ProtoIdent, current_package: &str, package_by_ident: &BTreeMap<ProtoIdent, String>, proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>) -> String {
    if ident.proto_type.starts_with("map<") {
        return render_map_type(&ident.proto_type, current_package, package_by_ident, proto_type_index);
    }
    if ident.module_path.is_empty() && ident.proto_file_path.is_empty() && ident.proto_package_name.is_empty() {
        if let Some(scalar) = proto_scalar_type(&ident.proto_type) {
            return scalar.to_string();
        }
    }

    let type_name = ident.name;
    let package = package_by_ident
        .get(&ident)
        .map(String::as_str)
        .or_else(|| if ident.proto_package_name.is_empty() { None } else { Some(ident.proto_package_name) });

    match package {
        Some(package) if package == current_package => type_name.to_string(),
        Some(package) if !package.is_empty() => format!("crate::{}::{}", module_path_for_package(package), type_name),
        _ => type_name.to_string(),
    }
}

fn render_map_type(proto_type: &str, current_package: &str, package_by_ident: &BTreeMap<ProtoIdent, String>, proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>) -> String {
    let Some((key, value)) = parse_map_types(proto_type) else {
        return "::proto_rs::alloc::collections::BTreeMap<::core::primitive::u32, ::core::primitive::u32>".to_string();
    };
    let key_type = proto_name_to_rust_type(key, current_package, package_by_ident, proto_type_index);
    let value_type = proto_name_to_rust_type(value, current_package, package_by_ident, proto_type_index);
    format!("::proto_rs::alloc::collections::BTreeMap<{key_type}, {value_type}>")
}

fn proto_name_to_rust_type(proto_name: &str, current_package: &str, package_by_ident: &BTreeMap<ProtoIdent, String>, proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>) -> String {
    if let Some(scalar) = proto_scalar_type(proto_name) {
        return scalar.to_string();
    }
    if proto_name.starts_with("map<") {
        return render_map_type(proto_name, current_package, package_by_ident, proto_type_index);
    }

    if let Some(candidates) = proto_type_index.get(proto_name) {
        if let Some(candidate) = candidates.iter().find(|ident| package_by_ident.get(*ident).is_some_and(|pkg| pkg == current_package)) {
            return render_proto_type(*candidate, current_package, package_by_ident, proto_type_index);
        }
        if let Some(candidate) = candidates.first() {
            return render_proto_type(*candidate, current_package, package_by_ident, proto_type_index);
        }
    }

    proto_name.to_string()
}

fn proto_scalar_type(proto_type: &str) -> Option<&'static str> {
    match proto_type {
        "double" => Some("f64"),
        "float" => Some("f32"),
        "int32" | "sint32" | "sfixed32" => Some("i32"),
        "int64" | "sint64" | "sfixed64" => Some("i64"),
        "uint32" | "fixed32" => Some("u32"),
        "uint64" | "fixed64" => Some("u64"),
        "bool" => Some("bool"),
        "string" => Some("::proto_rs::alloc::string::String"),
        "bytes" => Some("::proto_rs::alloc::vec::Vec<u8>"),
        _ => None,
    }
}

fn parse_map_types(proto_type: &str) -> Option<(&str, &str)> {
    let inner = proto_type.strip_prefix("map<")?.strip_suffix('>')?;
    let mut parts = inner.splitn(2, ',');
    let key = parts.next()?.trim();
    let value = parts.next()?.trim();
    Some((key, value))
}

fn render_generics(entry: &ProtoSchema) -> String {
    if entry.generics.is_empty() && entry.lifetimes.is_empty() {
        return String::new();
    }

    let mut params = Vec::new();

    for lifetime in entry.lifetimes {
        let mut lifetime_param = format!("'{}", lifetime.name);
        if !lifetime.bounds.is_empty() {
            lifetime_param.push_str(": ");
            lifetime_param.push_str(&lifetime.bounds.join(" + "));
        }
        params.push(lifetime_param);
    }

    for generic in entry.generics {
        match generic.kind {
            GenericKind::Type => {
                let mut param = generic.name.to_string();
                if !generic.constraints.is_empty() {
                    param.push_str(": ");
                    param.push_str(&generic.constraints.join(" + "));
                }
                params.push(param);
            }
            GenericKind::Const => {
                let const_type = generic.const_type.unwrap_or("usize");
                params.push(format!("const {}: {const_type}", generic.name));
            }
        }
    }

    format!("<{}>", params.join(", "))
}

fn build_proto_type_index(registry: &BTreeMap<String, Vec<&'static ProtoSchema>>) -> BTreeMap<String, Vec<ProtoIdent>> {
    let mut index = BTreeMap::new();
    for entries in registry.values() {
        for entry in entries {
            index.entry(entry.id.proto_type.to_string()).or_insert_with(Vec::new).push(entry.id);
        }
    }
    index
}

fn package_name_for_entries(file_name: &str, entries: &[&ProtoSchema]) -> String {
    let path = Path::new(file_name);
    let file_name_last = path.file_name().and_then(|name| name.to_str()).unwrap_or(file_name);
    entries
        .first()
        .map(|schema| schema.id.proto_package_name)
        .filter(|name| !name.is_empty())
        .map_or_else(|| derive_package_name(file_name_last), ToString::to_string)
}

fn module_path_segments(package_name: &str) -> Vec<String> {
    package_name.split('.').filter(|segment| !segment.is_empty()).map(sanitize_module_segment).collect()
}

fn module_path_for_package(package_name: &str) -> String {
    module_path_segments(package_name).join("::")
}

fn sanitize_module_segment(segment: &str) -> String {
    let mut out = String::new();
    for ch in segment.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() { "_".to_string() } else { out }
}

fn indent_line(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
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
        ProtoEntry::Service { methods, .. } => Some(render_service(entry.id.proto_type, methods, package_name, ident_index)),
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
