//! Configuration parsing - refactored to use consolidated utilities

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::Attribute;
use syn::Data;
use syn::ItemTrait;
use syn::Lit;

use crate::utils::parse_field_config;
use crate::utils::rust_type_path_ident;
use crate::write_file::register_and_emit_proto_inner;
use crate::write_file::register_imports;

pub trait ParseFieldAttr {
    fn extract_field_imports(&self, map: &mut BTreeMap<String, BTreeSet<String>>);
}

impl ParseFieldAttr for &syn::Data {
    fn extract_field_imports(&self, map: &mut BTreeMap<String, BTreeSet<String>>) {
        match self {
            Data::Struct(data) => {
                merge_field_imports(map, extract_field_imports(&data.fields));
            }
            Data::Enum(data) => {
                for variant in &data.variants {
                    merge_field_imports(map, extract_field_imports(&variant.fields));
                }
            }
            Data::Union(_) => {
                // Unions not supported
            }
        }
    }
}

impl ParseFieldAttr for &ItemTrait {
    fn extract_field_imports(&self, _map: &mut BTreeMap<String, BTreeSet<String>>) {
        // Traits use #[proto_imports(...)] at trait level, not field level
    }
}

impl ParseFieldAttr for () {
    fn extract_field_imports(&self, _map: &mut BTreeMap<String, BTreeSet<String>>) {
        // No fields to parse
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnifiedProtoConfig {
    pub proto_path: Option<String>,
    pub rpc_server: bool,
    pub rpc_client: bool,
    rpc_package: Option<String>,
    pub type_imports: BTreeMap<String, BTreeSet<String>>,
    file_imports: BTreeMap<String, BTreeSet<String>>,
    pub imports_mat: TokenStream2,
}

impl UnifiedProtoConfig {
    /// Register and emit proto content
    pub fn register_and_emit_proto(&mut self, type_ident: &str, content: &str) {
        let mat = register_and_emit_proto_inner(self.proto_path(), type_ident, content);
        let imports = &self.imports_mat;
        self.imports_mat = quote::quote! { #imports #mat };
    }

    /// Parse configuration from attributes and extract all imports
    pub fn from_attributes(attr: TokenStream, type_ident: &str, item_attrs: &[Attribute], fields: impl ParseFieldAttr) -> Self {
        let mut config = Self::default();

        // Parse attribute parameters
        if !attr.is_empty() {
            parse_attr_params(attr, &mut config);
        }

        // Extract imports from item-level attributes
        let mut all_imports = extract_item_imports(item_attrs);

        // Extract field-level imports
        fields.extract_field_imports(&mut all_imports);

        // Register file imports
        for package in all_imports.keys() {
            config.file_imports.entry(config.proto_path().to_owned()).or_default().insert(package.to_owned());
        }

        config.type_imports = all_imports;
        config.imports_mat = register_imports(type_ident, &config.file_imports);

        config
    }

    /// Get the RPC package name
    pub fn get_rpc_package(&self) -> &str {
        self.rpc_package.as_ref().expect("RPC package name required: use rpc_package = \"name\"")
    }

    /// Get the proto file path
    pub fn proto_path(&self) -> &str {
        self.proto_path.as_deref().unwrap_or("protos/generated.proto")
    }
}

fn parse_attr_params(attr: TokenStream, config: &mut UnifiedProtoConfig) {
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("proto_path") {
            if let Ok(lit_str) = meta.value()?.parse::<syn::LitStr>() {
                config.proto_path = Some(lit_str.value());
            }
        } else if meta.path.is_ident("rpc_server") {
            if let Ok(lit_bool) = meta.value()?.parse::<syn::LitBool>() {
                config.rpc_server = lit_bool.value;
            }
        } else if meta.path.is_ident("rpc_client") {
            if let Ok(lit_bool) = meta.value()?.parse::<syn::LitBool>() {
                config.rpc_client = lit_bool.value;
            }
        } else if meta.path.is_ident("rpc_package")
            && let Ok(lit_str) = meta.value()?.parse::<syn::LitStr>()
        {
            config.rpc_package = Some(lit_str.value());
        }
        Ok(())
    });

    let _ = syn::parse::Parser::parse(parser, attr);
}

/// Extract proto_imports from item attributes
pub fn extract_item_imports(item_attrs: &[Attribute]) -> BTreeMap<String, BTreeSet<String>> {
    let mut imports = BTreeMap::new();

    for attr in item_attrs {
        if !attr.path().is_ident("proto_imports") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            let package = meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default();

            // Parse array value
            if let Ok(syn::Expr::Array(array)) = meta.value()?.parse::<syn::Expr>() {
                let types = extract_string_array(&array);
                if !types.is_empty() {
                    imports.insert(package, types);
                }
            }

            Ok(())
        });
    }

    imports
}

fn extract_string_array(array: &syn::ExprArray) -> BTreeSet<String> {
    array
        .elems
        .iter()
        .filter_map(|elem| {
            if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = elem {
                Some(s.value())
            } else {
                None
            }
        })
        .collect()
}

/// Extract import_path from field-level attributes
pub fn extract_field_imports(fields: &syn::Fields) -> HashMap<String, Vec<String>> {
    let mut imports = HashMap::new();

    for field in fields.iter() {
        let config = parse_field_config(field);

        if let Some(import_path) = config.import_path {
            let type_name = extract_field_type_name(&field.ty);
            imports.entry(import_path).or_insert_with(Vec::new).push(type_name);
        }
    }

    imports
}

fn extract_field_type_name(ty: &syn::Type) -> String {
    // Handle Option<T> and Vec<T>
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let ident = &segment.ident;

        if matches!(ident.to_string().as_str(), "Option" | "Vec")
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            return rust_type_path_ident(inner_ty).to_string();
        }

        if matches!(ident.to_string().as_str(), "HashMap" | "BTreeMap")
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        {
            let mut generics = args.args.iter().filter_map(|arg| match arg {
                syn::GenericArgument::Type(inner_ty) => Some(inner_ty.clone()),
                _ => None,
            });

            let value_ty = generics.nth(1).unwrap_or_else(|| ty.clone());
            return rust_type_path_ident(&value_ty).to_string();
        }

        if matches!(ident.to_string().as_str(), "HashSet" | "BTreeSet")
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            return rust_type_path_ident(inner_ty).to_string();
        }

        return ident.to_string();
    }

    String::from("Unknown")
}

fn merge_field_imports(dest: &mut BTreeMap<String, BTreeSet<String>>, src: HashMap<String, Vec<String>>) {
    for (package, types) in src {
        dest.entry(package).or_default().extend(types);
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_extract_string_array() {
        let array: syn::ExprArray = parse_quote! { ["Type1", "Type2", "Type3"] };
        let result = extract_string_array(&array);

        assert_eq!(result.len(), 3);
        assert!(result.contains("Type1"));
        assert!(result.contains("Type2"));
        assert!(result.contains("Type3"));
    }

    #[test]
    fn test_extract_field_type_name() {
        let ty: syn::Type = parse_quote! { MyType };
        assert_eq!(extract_field_type_name(&ty), "MyType");

        let ty: syn::Type = parse_quote! { Option<MyType> };
        assert_eq!(extract_field_type_name(&ty), "MyType");

        let ty: syn::Type = parse_quote! { Vec<MyType> };
        assert_eq!(extract_field_type_name(&ty), "MyType");
    }

    #[test]
    fn test_unified_proto_config_defaults() {
        let config = UnifiedProtoConfig::default();
        assert_eq!(config.proto_path(), "protos/generated.proto");
        assert!(!config.rpc_server);
        assert!(!config.rpc_client);
    }
}
