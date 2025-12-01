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
use syn::Type;
use syn::parse::Parse;

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

#[derive(Clone, Default)]
pub struct UnifiedProtoConfig {
    pub proto_path: Option<String>,
    pub rpc_server: bool,
    pub rpc_client: bool,
    rpc_package: Option<String>,
    pub type_imports: BTreeMap<String, BTreeSet<String>>,
    file_imports: BTreeMap<String, BTreeSet<String>>,
    pub imports_mat: TokenStream2,
    pub suns: Vec<SunConfig>,
    pub transparent: bool,
    pub validator: Option<String>,
}

#[derive(Clone)]
pub struct SunConfig {
    pub ty: Type,
    pub message_ident: String,
    pub by_ref: bool,
}

impl UnifiedProtoConfig {
    /// Register and emit proto content (only if `proto_path` is specified)
    pub fn register_and_emit_proto(&mut self, type_ident: &str, content: &str) {
        if let Some(proto_path) = self.proto_path() {
            let mat = register_and_emit_proto_inner(proto_path, type_ident, content);
            let imports = &self.imports_mat;
            self.imports_mat = quote::quote! { #imports #mat };
        }
    }

    /// Parse configuration from attributes and extract all imports
    pub fn from_attributes(attr: TokenStream, type_ident: &str, item_attrs: &[Attribute], fields: impl ParseFieldAttr) -> Self {
        let mut config = Self::default();

        // Parse attribute parameters
        if !attr.is_empty() {
            parse_attr_params(attr, &mut config);
        }

        // Extract validator from item-level #[proto(validator = ...)] attributes
        config.validator = extract_item_validator(item_attrs);

        // Extract imports from item-level attributes
        let mut all_imports = extract_item_imports(item_attrs);

        // Extract field-level imports
        fields.extract_field_imports(&mut all_imports);

        // Register file imports (only if proto_path is specified)
        if let Some(proto_path_str) = config.proto_path.as_deref() {
            let proto_path = proto_path_str.to_owned();
            for package in all_imports.keys() {
                config.file_imports.entry(proto_path.clone()).or_default().insert(package.to_owned());
            }
        }

        config.type_imports = all_imports;
        config.imports_mat = register_imports(type_ident, &config.file_imports);

        config
    }

    /// Get the RPC package name
    pub fn get_rpc_package(&self) -> &str {
        self.rpc_package.as_ref().expect("RPC package name required: use rpc_package = \"name\"")
    }

    /// Get the proto file path (returns None if not specified)
    pub fn proto_path(&self) -> Option<&str> {
        self.proto_path.as_deref()
    }
}

fn parse_attr_params(attr: TokenStream, config: &mut UnifiedProtoConfig) {
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("transparent") {
            config.transparent = true;
            return Ok(());
        } else if meta.path.is_ident("proto_path") {
            if let Ok(lit_str) = meta.value()?.parse::<syn::LitStr>() {
                config.proto_path = Some(lit_str.value());
            }
        } else if meta.path.is_ident("sun") {
            // Parse as Type instead of Expr to handle generics like DateTime<Utc>
            let value = meta.value()?;
            let lookahead = value.lookahead1();
            if lookahead.peek(syn::token::Bracket) {
                // Handle array syntax: sun = [Type1, Type2]
                let content;
                syn::bracketed!(content in value);
                let types: syn::punctuated::Punctuated<Type, syn::Token![,]> = content.parse_terminated(Type::parse, syn::Token![,])?;
                for ty in types {
                    config.push_sun(ty);
                }
            } else {
                // Handle single type: sun = Type
                let ty: Type = value.parse()?;
                config.push_sun(ty);
            }
            return Ok(());
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
        } else {
            return Err(meta.error("unknown #[proto(...)] attribute"));
        }
        Ok(())
    });

    syn::parse::Parser::parse(parser, attr).expect("failed to parse proto_message attributes");
}

fn extract_type_ident(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
        Type::Reference(reference) => extract_type_ident(&reference.elem),
        Type::Group(group) => extract_type_ident(&group.elem),
        Type::Paren(paren) => extract_type_ident(&paren.elem),
        _ => None,
    }
}

impl UnifiedProtoConfig {
    pub fn has_suns(&self) -> bool {
        !self.suns.is_empty()
    }

    pub fn proto_message_names(&self, fallback: &str) -> Vec<String> {
        if self.suns.is_empty() {
            vec![fallback.to_string()]
        } else {
            self.suns.iter().map(|sun| sun.message_ident.clone()).collect()
        }
    }

    fn push_sun(&mut self, ty: Type) {
        let by_ref = is_reference_sun(&ty);
        let ty = normalize_sun_type(ty);
        let message_ident = extract_type_ident(&ty).expect("sun attribute expects a type path");
        self.suns.push(SunConfig { ty, message_ident, by_ref });
    }
}

fn normalize_sun_type(ty: Type) -> Type {
    match ty {
        Type::Reference(reference) => *reference.elem,
        Type::Group(group) => normalize_sun_type(*group.elem),
        Type::Paren(paren) => normalize_sun_type(*paren.elem),
        other => other,
    }
}

fn is_reference_sun(ty: &Type) -> bool {
    match ty {
        Type::Reference(_) => true,
        Type::Group(group) => is_reference_sun(&group.elem),
        Type::Paren(paren) => is_reference_sun(&paren.elem),
        _ => false,
    }
}

/// Extract `validator` from item-level #[proto(validator = ...)] attributes
pub fn extract_item_validator(item_attrs: &[Attribute]) -> Option<String> {
    for attr in item_attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        let mut validator = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("validator") {
                let value_parser = meta.value()?;

                // Try parsing as Expr which can be either a Lit or a Path
                if let Ok(expr) = value_parser.parse::<syn::Expr>() {
                    match expr {
                        // Handle string literals: validator = "validate_fn"
                        syn::Expr::Lit(expr_lit) => {
                            if let syn::Lit::Str(s) = expr_lit.lit {
                                validator = Some(s.value());
                            }
                        }
                        // Handle paths: validator = validate_fn
                        syn::Expr::Path(expr_path) => {
                            let path_str = expr_path.path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<_>>().join("::");
                            validator = Some(path_str);
                        }
                        _ => {}
                    }
                }
                return Ok(());
            }

            Err(meta.error("unknown #[proto(...)] attribute"))
        })
        .expect("failed to parse #[proto(...)] attributes");

        if validator.is_some() {
            return validator;
        }
    }

    None
}

/// Extract `proto_imports` from item attributes
pub fn extract_item_imports(item_attrs: &[Attribute]) -> BTreeMap<String, BTreeSet<String>> {
    let mut imports = BTreeMap::new();

    for attr in item_attrs {
        if !attr.path().is_ident("proto_imports") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            let package = meta.path.get_ident().map(ToString::to_string).unwrap_or_default();

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

/// Extract `import_path` from field-level attributes
pub fn extract_field_imports(fields: &syn::Fields) -> HashMap<String, Vec<String>> {
    let mut imports = HashMap::new();

    for field in fields {
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

        if (ident == "Option" || ident == "Vec" || ident == "ArcSwap" || ident == "ArcSwapOption")
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
    use std::panic;

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

        let ty: syn::Type = parse_quote! { arc_swap::ArcSwap<MyType> };
        assert_eq!(extract_field_type_name(&ty), "MyType");

        let ty: syn::Type = parse_quote! { arc_swap::ArcSwapOption<MyType> };
        assert_eq!(extract_field_type_name(&ty), "MyType");
    }

    #[test]
    fn test_unified_proto_config_defaults() {
        let config = UnifiedProtoConfig::default();
        assert_eq!(config.proto_path(), None);
        assert!(!config.rpc_server);
        assert!(!config.rpc_client);
        assert!(!config.transparent);
    }

    #[test]
    fn parses_owned_sun_type() {
        let ty: Type = parse_quote!(OwnedType);
        let normalized = normalize_sun_type(ty);

        assert_eq!(extract_type_ident(&normalized), Some("OwnedType".to_string()));
        assert!(matches!(normalized, Type::Path(_)));
    }

    #[test]
    fn parses_borrowed_sun_type() {
        let ty: Type = parse_quote!(&BorrowedType);
        let normalized = normalize_sun_type(ty);

        assert_eq!(extract_type_ident(&normalized), Some("BorrowedType".to_string()));
        assert!(matches!(normalized, Type::Path(_)));
    }

    #[test]
    fn panics_on_unknown_validator_attribute() {
        let attr: syn::Attribute = parse_quote!(#[proto(foo = "bar")]);

        let result = panic::catch_unwind(|| {
            let _ = extract_item_validator(&[attr]);
        });

        assert!(result.is_err());
    }
}
