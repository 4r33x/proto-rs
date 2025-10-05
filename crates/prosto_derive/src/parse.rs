use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
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

/// Implementation for syn::Data (covers Struct, Enum, Union)
impl ParseFieldAttr for &syn::Data {
    fn extract_field_imports(&self, map: &mut BTreeMap<String, BTreeSet<String>>) {
        match self {
            Data::Struct(data) => {
                // Extract from struct fields
                let field_imports = extract_field_imports(&data.fields);
                for (pkg, types) in field_imports {
                    map.entry(pkg).or_default().extend(types);
                }
            }
            Data::Enum(data) => {
                // Extract from all enum variant fields
                for variant in &data.variants {
                    let variant_imports = extract_field_imports(&variant.fields);
                    for (pkg, types) in variant_imports {
                        map.entry(pkg).or_default().extend(types);
                    }
                }
            }
            Data::Union(_) => {
                // Unions not supported, do nothing
            }
        }
    }
}

/// Implementation for ItemTrait (traits have no field-level imports)
impl ParseFieldAttr for &ItemTrait {
    fn extract_field_imports(&self, _map: &mut BTreeMap<String, BTreeSet<String>>) {
        // Traits use #[proto_imports(...)] at trait level, not field level
        // Do nothing
    }
}

/// Implementation for unit type (when there are no fields to parse)
impl ParseFieldAttr for () {
    fn extract_field_imports(&self, _map: &mut BTreeMap<String, BTreeSet<String>>) {
        // No fields to parse, do nothing
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
    pub fn register_and_emit_proto(&mut self, type_ident: &str, content: &str) {
        let mat = register_and_emit_proto_inner(self.proto_path(), type_ident, content);
        let imports = &self.imports_mat;
        self.imports_mat = quote! { #imports #mat};
    }
    /// Parse configuration from attributes and extract all imports
    pub fn from_attributes(attr: TokenStream, type_ident: &str, item_attrs: &[Attribute], fields: impl ParseFieldAttr) -> Self {
        let mut config = Self::default();

        if !attr.is_empty() {
            // Parse as Meta
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

            syn::parse::Parser::parse(parser, attr).ok();
        }

        // Extract imports from item-level #[proto_imports(...)] attributes
        let mut all_imports = extract_item_imports(item_attrs);

        // Extract field-level imports directly into the map
        fields.extract_field_imports(&mut all_imports);

        for package in all_imports.keys() {
            config.file_imports.entry(config.proto_path().to_owned()).or_default().insert(package.to_owned());
        }

        config.type_imports = all_imports;
        config.imports_mat = register_imports(type_ident, &config.file_imports);
        config
    }

    /// Get the RPC package name (either explicit or derived from proto_path)
    pub fn get_rpc_package(&self) -> &str {
        self.rpc_package.as_ref().expect("You should specify RPC package name: 'rpc_package = \"name \"'")
    }
    pub fn proto_path(&self) -> &str {
        self.proto_path.as_deref().unwrap_or("protos/generated.proto")
    }
}

/// Extract proto_imports from item attributes
pub fn extract_item_imports(item_attrs: &[Attribute]) -> BTreeMap<String, BTreeSet<String>> {
    let mut imports = BTreeMap::new();

    for attr in item_attrs {
        if attr.path().is_ident("proto_imports") {
            let _ = attr.parse_nested_meta(|meta| {
                let package = meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default();

                // Parse the array value
                if let Ok(expr) = meta.value()?.parse::<syn::Expr>()
                    && let syn::Expr::Array(array) = expr
                {
                    let mut types = BTreeSet::new();
                    for elem in array.elems {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = elem {
                            types.insert(s.value());
                        }
                    }
                    imports.insert(package, types);
                }
                Ok(())
            });
        }
    }

    imports
}

/// Extract import_path from field-level #[proto(import_path = "...")] attributes
/// and add them to the imports map
pub fn extract_field_imports(fields: &syn::Fields) -> HashMap<String, Vec<String>> {
    let mut imports: HashMap<String, Vec<String>> = HashMap::new();

    for field in fields.iter() {
        let config = parse_field_config(field);
        if let Some(import_path) = config.import_path {
            // Get the type name from the field
            if let syn::Type::Path(type_path) = &field.ty {
                // Handle Option<T> and Vec<T>
                let type_name = if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Option" || segment.ident == "Vec" {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                rust_type_path_ident(inner_ty).to_string()
                            } else {
                                segment.ident.to_string()
                            }
                        } else {
                            segment.ident.to_string()
                        }
                    } else {
                        segment.ident.to_string()
                    }
                } else {
                    continue;
                };

                imports.entry(import_path.clone()).or_default().push(type_name);
            }
        }
    }

    imports
}
