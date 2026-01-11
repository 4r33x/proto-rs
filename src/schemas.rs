use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::LazyLock;

mod proto_output;
mod rust_client;
mod utils;

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
    pub imports: &'a [&'a str],
    pub client_attrs: BTreeMap<ProtoIdent, Vec<UserAttr>>,
}

impl<'a> RustClientCtx<'a> {
    pub fn disabled() -> Self {
        Self {
            output_path: None,
            imports: &[],
            client_attrs: BTreeMap::new(),
        }
    }

    pub fn enabled(output_path: &'a str) -> Self {
        Self {
            output_path: Some(output_path),
            imports: &[],
            client_attrs: BTreeMap::new(),
        }
    }
    #[must_use]
    pub fn with_imports(mut self, imports: &'a [&'a str]) -> Self {
        self.imports = imports;
        self
    }

    #[must_use]
    pub fn add_client_attrs(mut self, ident: ProtoIdent, attr: UserAttr) -> Self {
        self.client_attrs.entry(ident).or_default().push(attr);
        self
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAttr {
    pub level: AttrLevel,
    pub attr: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttrLevel {
    Top,
    Field { field_name: String, r#type: ProtoIdent },
    Method { method_name: String },
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
    pub rust_proto_ident: ProtoIdent,
    pub generic_args: &'static [&'static ProtoIdent],
    pub proto_label: ProtoLabel,
    pub tag: u32,
    pub attributes: &'static [Attribute],
    pub array_len: Option<&'static str>,
    pub array_is_bytes: bool,
    pub array_elem: Option<ProtoIdent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceMethod {
    pub name: &'static str,
    pub request: ProtoIdent,
    pub request_generic_args: &'static [&'static ProtoIdent],
    pub response: ProtoIdent,
    pub response_generic_args: &'static [&'static ProtoIdent],
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
///         let count = proto_rs::schemas::write_all("protos", &proto_rs::schemas::RustClientCtx::disabled())
///             .expect("Failed to write proto files");
///         println!("Generated {} proto files", count);
///     }
/// }
/// ```
/// Write all registered proto schemas to a directory
/// # Errors
///
/// Will return `Err` if fs throws error
pub fn write_all(output_dir: &str, rust_client_output: &RustClientCtx<'_>) -> io::Result<usize> {
    match fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    fs::create_dir_all(output_dir)?;
    let mut count = 0;
    let (registry, ident_index) = build_registry();
    let all_entries: Vec<&ProtoSchema> = registry.values().flat_map(|entries| entries.iter().copied()).collect();
    let specializations = proto_output::collect_generic_specializations(&all_entries, &ident_index);

    for (file_name, entries) in &registry {
        let output_path = format!("{output_dir}/{file_name}");

        if let Some(parent) = Path::new(&output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let path = Path::new(file_name.as_str());
        let file_name_last = path.file_name().unwrap().to_str().unwrap();
        let package_name = entries
            .first()
            .map(|schema| schema.id.proto_package_name)
            .filter(|name| !name.is_empty())
            .map_or(utils::derive_package_name(file_name_last), ToString::to_string);
        let mut output = String::new();

        output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
        output.push_str("syntax = \"proto3\";\n");
        writeln!(output, "package {package_name};").unwrap();

        output.push('\n');

        let imports = proto_output::collect_imports(entries.as_slice(), &ident_index, file_name, &package_name)?;
        if !imports.is_empty() {
            let mut import_stems = BTreeSet::new();
            for import in &imports {
                let import_path = Path::new(import);
                let import_file = import_path.file_name().and_then(|name| name.to_str()).unwrap_or(import);
                let import_stem = import_file.strip_suffix(".proto").unwrap_or(import_file);
                import_stems.insert(import_stem.to_string());
            }
            for import_stem in import_stems {
                writeln!(output, "import \"{import_stem}.proto\";").unwrap();
            }
            output.push('\n');
        }

        let definitions = proto_output::render_entries(entries, &package_name, &ident_index, &specializations);
        for definition in definitions {
            output.push_str(&definition);
            output.push('\n');
        }

        fs::write(&output_path, output)?;
        count += 1;
    }

    if let Some(output_path) = rust_client_output.output_path {
        rust_client::write_rust_client_module(
            output_path,
            rust_client_output.imports,
            &rust_client_output.client_attrs,
            &registry,
            &ident_index,
        )?;
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

fn build_registry() -> (
    BTreeMap<String, Vec<&'static ProtoSchema>>,
    BTreeMap<ProtoIdent, &'static ProtoSchema>,
) {
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
