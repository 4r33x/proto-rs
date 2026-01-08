#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type, maybe_uninit_array_assume_init))]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_self)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::inline_always)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate self as proto_rs;

pub use prosto_derive::inject_proto_import;
pub use prosto_derive::proto_dump;
pub use prosto_derive::proto_message;
pub use prosto_derive::proto_rpc;
pub use traits::const_test_validate_with_ext;
pub mod utils;
#[cfg(not(feature = "no-recursion-limit"))]
const RECURSION_LIMIT: u32 = 100;

mod custom_types;

#[doc(hidden)]
pub extern crate alloc;

// Re-export the bytes crate for use within derived code.
pub use bytes;

mod coders;
mod error;
mod name;
#[cfg(feature = "tonic")]
mod tonic;
mod traits;
mod types;
mod wrappers;
mod zero_copy;

#[doc(hidden)]
pub mod encoding;

pub use crate::coders::BytesMode;
pub use crate::coders::ProtoCodec;
pub use crate::coders::ProtoEncoder;
pub use crate::coders::SunByRef;
pub use crate::coders::SunByVal;
pub use crate::encoding::length_delimiter::decode_length_delimiter;
pub use crate::encoding::length_delimiter::encode_length_delimiter;
pub use crate::encoding::length_delimiter::length_delimiter_len;
pub use crate::error::DecodeError;
pub use crate::error::EncodeError;
pub use crate::error::UnknownEnumValue;
pub use crate::name::Name;
#[cfg(feature = "tonic")]
pub use crate::tonic::EncoderExt;
#[cfg(feature = "tonic")]
pub use crate::tonic::ProtoRequest;
#[cfg(feature = "tonic")]
pub use crate::tonic::ProtoResponse;
#[cfg(feature = "tonic")]
pub use crate::tonic::ToZeroCopyRequest;
#[cfg(feature = "tonic")]
pub use crate::tonic::ToZeroCopyResponse;
#[cfg(feature = "tonic")]
pub use crate::tonic::ZeroCopyRequest;
#[cfg(feature = "tonic")]
pub use crate::tonic::ZeroCopyResponse;
#[cfg(feature = "tonic")]
pub use crate::tonic::map_proto_response;
#[cfg(feature = "tonic")]
pub use crate::tonic::map_proto_stream_result;
pub use crate::traits::EncodeInputFromRef;
pub use crate::traits::OwnedSunOf;
pub use crate::traits::ProtoExt;
pub use crate::traits::ProtoKind;
pub use crate::traits::ProtoShadow;
pub use crate::traits::ProtoWire;
pub use crate::traits::Shadow;
pub use crate::traits::SunOf;
pub use crate::traits::ViewOf;
// pub use crate::traits::RepeatedCollection;
#[cfg(feature = "papaya")]
pub use crate::wrappers::conc_map::papaya_map_encode_input;
#[cfg(feature = "papaya")]
pub use crate::wrappers::conc_set::papaya_set_encode_input;
pub use crate::zero_copy::ToZeroCopy;
pub use crate::zero_copy::ZeroCopy;

/// Build-time proto schema registry
/// Only available when "build-schemas" feature is enabled
#[cfg(all(feature = "build-schemas", feature = "std"))]
pub mod schemas {
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

        for (file_name, entries) in registry.iter() {
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
                .map(ToString::to_string)
                .unwrap_or_else(|| derive_package_name(file_name_last));
            let mut output = String::new();

            output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
            output.push_str("syntax = \"proto3\";\n");
            writeln!(&mut output, "package {package_name};").unwrap();

            output.push('\n');

            let imports = collect_imports(entries, &ident_index, file_name, &package_name)?;
            if !imports.is_empty() {
                for import in &imports {
                    writeln!(&mut output, "import \"{import}.proto\";").unwrap();
                }
                output.push('\n');
            }

            let mut ordered_entries: Vec<&ProtoSchema> = entries.iter().copied().collect();
            ordered_entries.sort_by(|left, right| entry_sort_key(left).cmp(&entry_sort_key(right)));

            for entry in ordered_entries {
                if let Some(definition) = render_entry(entry, &package_name) {
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
            if schema.id.proto_file_path.is_empty() {
                continue;
            }
            if ident_index.insert(schema.id, schema).is_some() {
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
            collect_proto_ident_imports(imports, ident_index, &field.proto_ident, file_name, package_name)?;
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
            collect_proto_ident_imports(imports, ident_index, &method.request, file_name, package_name)?;
            collect_proto_ident_imports(imports, ident_index, &method.response, file_name, package_name)?;
        }
        Ok(())
    }

    fn collect_proto_ident_imports(
        imports: &mut BTreeSet<String>,
        ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
        ident: &ProtoIdent,
        file_name: &str,
        package_name: &str,
    ) -> io::Result<()> {
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
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "unresolved ProtoIdent for {} (file: {}, package: {})",
                        ident.proto_type, ident.proto_file_path, ident.proto_package_name
                    ),
                ));
            }
            imports.insert(ident.proto_file_path.to_string());
        }

        Ok(())
    }

    fn render_entry(entry: &ProtoSchema, package_name: &str) -> Option<String> {
        match entry.content {
            ProtoEntry::Struct { fields } => Some(render_struct(entry.id.proto_type, fields, package_name)),
            ProtoEntry::SimpleEnum { variants } => Some(render_simple_enum(entry.id.proto_type, variants)),
            ProtoEntry::ComplexEnum { variants } => Some(render_complex_enum(entry.id.proto_type, variants, package_name)),
            ProtoEntry::Import { .. } => None,
            ProtoEntry::Service { methods } => Some(render_service(entry.id.proto_type, methods, package_name)),
        }
    }

    fn render_struct(name: &str, fields: &[&Field], package_name: &str) -> String {
        if fields.is_empty() {
            return format!("message {name} {{}}\n");
        }

        let mut lines = Vec::new();
        for (idx, field) in fields.iter().enumerate() {
            lines.push(render_field(field, idx, package_name));
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

    fn render_complex_enum(name: &str, variants: &[&Variant], package_name: &str) -> String {
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
                let proto_type = field_type_name(variant.fields[0], package_name);
                oneof_fields.push(format!("    {proto_type} {field_name} = {tag};"));
                continue;
            }

            let msg_name = format!("{name}{variant_name}");
            let field_defs = render_named_fields(variant.fields, package_name);
            nested_messages.push(format!("message {msg_name} {{\n{field_defs}\n}}"));
            oneof_fields.push(format!("    {msg_name} {field_name} = {tag};"));
        }

        format!("{}\nmessage {} {{\n  oneof value {{\n{}\n  }}\n}}\n", nested_messages.join("\n\n"), name, oneof_fields.join("\n"))
    }

    fn render_named_fields(fields: &[&Field], package_name: &str) -> String {
        let mut lines = Vec::new();
        for (idx, field) in fields.iter().enumerate() {
            lines.push(render_field(field, idx, package_name));
        }
        lines.join("\n")
    }

    fn render_field(field: &Field, idx: usize, package_name: &str) -> String {
        let name = field.name.map_or_else(|| format!("field_{idx}"), ToString::to_string);
        let label = match field.proto_label {
            ProtoLabel::None => "",
            ProtoLabel::Optional => "optional ",
            ProtoLabel::Repeated => "repeated ",
        };
        let proto_type = field_type_name(field, package_name);
        format!("  {label}{proto_type} {name} = {};", field.tag)
    }

    fn render_service(name: &str, methods: &[&ServiceMethod], package_name: &str) -> String {
        let mut lines = Vec::new();
        lines.push(format!("service {name} {{"));

        for method in methods {
            let request_type = proto_ident_type_name(&method.request, package_name);
            let response_type = proto_ident_type_name(&method.response, package_name);
            let response_type = if method.server_streaming { format!("stream {response_type}") } else { response_type };
            lines.push(format!("  rpc {}({}) returns ({});", method.name, request_type, response_type));
        }

        lines.push("}".to_string());
        lines.join("\n")
    }

    fn field_type_name(field: &Field, package_name: &str) -> String {
        let ident = &field.proto_ident;
        if ident.proto_type.starts_with("map<") {
            return ident.proto_type.to_string();
        }
        proto_ident_type_name(ident, package_name)
    }

    fn proto_ident_type_name(ident: &ProtoIdent, package_name: &str) -> String {
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
}

// Example build.rs that users can copy:
#[cfg(all(feature = "build-schemas", feature = "std", doc))]
/// Example build.rs for consuming projects
///
/// ```no_run
/// // build.rs
/// fn main() {
///     // Only generate protos when explicitly requested
///     if std::env::var("GENERATE_PROTOS").is_ok() {
///         match proto_rs::schemas::write_all("protos") {
///             Ok(count) => println!("Generated {} proto files", count),
///             Err(e) => panic!("Failed to generate protos: {}", e),
///         }
///     }
/// }
/// ```
mod _build_example {}
