#![feature(impl_trait_in_assoc_type)]
#![allow(incomplete_features)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_self)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]

extern crate self as proto_rs;

pub use prosto_derive::inject_proto_import;
pub use prosto_derive::proto_dump;
pub use prosto_derive::proto_message;
pub use prosto_derive::proto_rpc;

#[cfg(not(feature = "no-recursion-limit"))]
const RECURSION_LIMIT: u32 = 100;

mod arrays;
mod custom_types;

#[doc(hidden)]
pub extern crate alloc;

// Re-export the bytes crate for use within derived code.
pub use bytes;

mod error;
mod name;
mod tonic;
mod traits;
mod types;
mod wrappers;

#[doc(hidden)]
pub mod encoding;

pub use crate::encoding::length_delimiter::decode_length_delimiter;
pub use crate::encoding::length_delimiter::encode_length_delimiter;
pub use crate::encoding::length_delimiter::length_delimiter_len;
pub use crate::error::DecodeError;
pub use crate::error::EncodeError;
pub use crate::error::UnknownEnumValue;
pub use crate::name::Name;
pub use crate::tonic::BytesMode;
pub use crate::tonic::EncoderExt;
pub use crate::tonic::ProtoCodec;
pub use crate::tonic::ProtoEncoder;
pub use crate::tonic::ProtoRequest;
pub use crate::tonic::SunByRef;
pub use crate::tonic::SunByVal;
pub use crate::tonic::ToZeroCopy;
pub use crate::tonic::ZeroCopyRequest;
pub use crate::traits::MessageField;
pub use crate::traits::OwnedSunOf;
pub use crate::traits::ProtoEnum;
pub use crate::traits::ProtoExt;
pub use crate::traits::ProtoShadow;
pub use crate::traits::RepeatedField;
pub use crate::traits::Shadow;
pub use crate::traits::SingularField;
pub use crate::traits::SunOf;
pub use crate::traits::ViewOf;

/// Build-time proto schema registry
/// Only available when "build-schemas" feature is enabled
#[cfg(feature = "build-schemas")]
pub mod schemas {
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::fs;
    use std::io;
    use std::path::Path;
    use std::sync::LazyLock;

    /// Represents a proto schema collected at compile time
    pub struct ProtoSchema {
        pub name: &'static str,
        pub content: &'static str,
    }

    // Auto-collect all schemas via inventory
    inventory::collect!(ProtoSchema);

    fn derive_package_name(file_path: &str) -> String {
        file_path.trim_end_matches(".proto").replace(['/', '\\', '-', '.'], "_").to_lowercase()
    }
    static REGISTRY: LazyLock<BTreeMap<String, BTreeSet<String>>> = LazyLock::new(|| {
        let mut registry = BTreeMap::new();

        for schema in inventory::iter::<ProtoSchema>() {
            registry.entry(schema.name.to_string()).or_insert_with(BTreeSet::new).insert(schema.content.to_string());
        }

        registry
    });

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
        fs::remove_dir_all(output_dir)?;
        fs::create_dir_all(output_dir)?;
        let mut count = 0;

        for (file_name, items) in REGISTRY.iter() {
            let output_path = format!("{output_dir}/{file_name}");

            if let Some(parent) = std::path::Path::new(&output_path).parent() {
                fs::create_dir_all(parent)?;
            }

            // Separate imports from other definitions
            let mut imports = Vec::new();
            let mut definitions = Vec::new();

            for item in items {
                if item.contains("import") {
                    imports.push(item.as_str());
                } else {
                    definitions.push(item.as_str());
                }
            }

            // Build file content with header
            let path = Path::new(file_name.as_str());
            let file_name_last = path.file_name().unwrap().to_str().unwrap();
            let package_name = derive_package_name(file_name_last);
            let mut output = String::new();

            output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
            output.push_str("syntax = \"proto3\";\n");
            writeln!(&mut output, "package {package_name};").unwrap();

            output.push('\n');

            // Add imports first
            for import in &imports {
                output.push_str(import);
            }

            if !imports.is_empty() {
                output.push('\n');
            }

            // Add definitions
            for definition in definitions {
                output.push_str(definition);
                output.push('\n');
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
}

// Example build.rs that users can copy:
#[cfg(all(feature = "build-schemas", doc))]
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
