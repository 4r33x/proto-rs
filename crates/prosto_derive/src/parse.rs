//! Configuration parsing - refactored to use consolidated utilities

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::Attribute;
use syn::Data;
use syn::Expr;
use syn::ItemTrait;
use syn::Lit;
use syn::LitStr;
use syn::Type;
use syn::parse::Parse;

use crate::utils::parse_field_config;
use crate::utils::rust_type_path_ident;
use crate::utils::type_name_with_generics_for_path;
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
    pub import_all_from: Option<String>,
    pub type_imports: BTreeMap<String, BTreeSet<String>>,
    file_imports: BTreeMap<String, BTreeSet<String>>,
    pub imports_mat: TokenStream2,
    pub suns: Vec<SunConfig>,
    pub transparent: bool,
    pub validator: Option<String>,
    pub validator_with_ext: Option<String>,
    pub generic_types: Vec<GenericTypeEntry>,
    pub item_generics: syn::Generics,
    pub item_attrs: Vec<Attribute>,
}

#[derive(Clone)]
pub struct SunConfig {
    pub ty: Type,
    pub message_ident: String,
    pub by_ref: bool,
}

#[derive(Clone)]
pub struct GenericTypeEntry {
    pub param: syn::Ident,
    pub types: Vec<Type>,
}

#[derive(Clone)]
pub struct GenericTypeVariant {
    pub suffix: String,
    pub substitutions: BTreeMap<String, Type>,
}

impl UnifiedProtoConfig {
    /// Register and emit proto content (only if `proto_path` is specified)
    pub fn register_and_emit_proto(&mut self, content: &str) {
        if let Some(proto_path) = self.proto_path() {
            register_and_emit_proto_inner(proto_path, content);
            let imports = &self.imports_mat;
            self.imports_mat = quote::quote! { #imports };
        } else if self.transparent {
            let imports = &self.imports_mat;
            self.imports_mat = quote::quote! { #imports };
        }
    }

    /// Parse configuration from attributes and extract all imports
    pub fn from_attributes(attr: TokenStream, type_ident: &str, item_attrs: &[Attribute], fields: impl ParseFieldAttr, generics: syn::Generics) -> Self {
        let mut config = Self::default();

        // Parse attribute parameters
        if !attr.is_empty() {
            parse_attr_params(attr, &mut config);
        }

        config.item_generics = generics;
        config.item_attrs = item_attrs.to_vec();
        // Extract validators from item-level #[proto(...)] attributes
        let item_validators = extract_item_validators(item_attrs);
        config.validator = item_validators.validator;
        config.validator_with_ext = item_validators.validator_with_ext;
        config.generic_types = extract_item_generic_types(item_attrs);

        // Extract imports from item-level attributes
        let mut all_imports = extract_item_imports(item_attrs);
        if config.import_all_from.is_none() {
            config.import_all_from = extract_import_all_from(item_attrs);
        }

        // Extract field-level imports
        fields.extract_field_imports(&mut all_imports);

        // Register file imports (only if proto_path is specified)
        if let Some(proto_path_str) = config.proto_path.as_deref() {
            let proto_path = proto_path_str.to_owned();
            for package in all_imports.keys() {
                config.file_imports.entry(proto_path.clone()).or_default().insert(package.to_owned());
            }

            if let Some(import_all_from) = &config.import_all_from {
                config.file_imports.entry(proto_path).or_default().insert(import_all_from.to_owned());
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
        } else if meta.path.is_ident("proto_import_all_from") {
            if meta.input.peek(syn::token::Paren) {
                let mut import_path = None;
                meta.parse_nested_meta(|nested| {
                    if let Some(ident) = nested.path.get_ident() {
                        import_path = Some(ident.to_string());
                        return Ok(());
                    }

                    import_path = Some(path_to_proto_package(&nested.path));
                    Ok(())
                })?;

                if let Some(import_path) = import_path {
                    config.import_all_from = Some(import_path);
                }
            } else if meta.input.peek(syn::Token![=]) {
                let value = meta.value()?;
                if let Ok(lit_str) = value.parse::<syn::LitStr>() {
                    config.import_all_from = Some(lit_str.value());
                } else if let Ok(path) = value.parse::<syn::Path>() {
                    config.import_all_from = Some(path_to_proto_package(&path));
                } else {
                    return Err(meta.error("proto_import_all_from expects a string literal or path"));
                }
            } else if let Ok(path) = meta.input.parse::<syn::Path>() {
                config.import_all_from = Some(path_to_proto_package(&path));
            }
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

    pub fn generic_type_variants(&self, generics: &syn::Generics) -> Result<Vec<GenericTypeVariant>, syn::Error> {
        if self.generic_types.is_empty() {
            return Ok(vec![GenericTypeVariant {
                suffix: String::new(),
                substitutions: BTreeMap::new(),
            }]);
        }

        let type_params: Vec<_> = generics.type_params().map(|param| param.ident.clone()).collect();
        if type_params.is_empty() {
            return Err(syn::Error::new_spanned(generics, "generic_types specified for non-generic type"));
        }

        let mut generic_map = BTreeMap::new();
        for entry in &self.generic_types {
            generic_map.insert(entry.param.to_string(), entry.types.clone());
        }

        let mut variants = vec![GenericTypeVariant {
            suffix: String::new(),
            substitutions: BTreeMap::new(),
        }];

        // Keep the base variant for Rust client code generation
        let base_variant = GenericTypeVariant {
            suffix: String::new(),
            substitutions: BTreeMap::new(),
        };

        for param in type_params {
            let Some(types) = generic_map.get(&param.to_string()) else {
                return Err(syn::Error::new_spanned(&param, format!("missing generic_types entry for `{param}`")));
            };
            if types.is_empty() {
                return Err(syn::Error::new_spanned(&param, format!("generic_types entry for `{param}` is empty")));
            }

            let mut next_variants = Vec::new();
            for existing in &variants {
                for ty in types {
                    let mut substitutions = existing.substitutions.clone();
                    substitutions.insert(param.to_string(), ty.clone());
                    let mut suffix = existing.suffix.clone();
                    suffix.push_str(&type_name_with_generics_for_path(ty));
                    next_variants.push(GenericTypeVariant { suffix, substitutions });
                }
            }
            variants = next_variants;
        }

        // Add the base generic variant at the beginning for Rust client generation
        // This ensures we have a schema for Envelope<T>, not just concrete variants
        variants.insert(0, base_variant);

        Ok(variants)
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

pub struct ItemValidators {
    pub validator: Option<String>,
    pub validator_with_ext: Option<String>,
}

/// Extract validators from item-level #[proto(...)] attributes
pub fn extract_item_validators(item_attrs: &[Attribute]) -> ItemValidators {
    let mut validators = ItemValidators {
        validator: None,
        validator_with_ext: None,
    };

    for attr in item_attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("validator") {
                let value_parser = meta.value()?;

                // Try parsing as Expr which can be either a Lit or a Path
                if let Ok(expr) = value_parser.parse::<syn::Expr>() {
                    match expr {
                        // Handle string literals: validator = "validate_fn"
                        syn::Expr::Lit(expr_lit) => {
                            if let syn::Lit::Str(s) = expr_lit.lit {
                                validators.validator = Some(s.value());
                            }
                        }
                        // Handle paths: validator = validate_fn
                        syn::Expr::Path(expr_path) => {
                            let path_str = expr_path.path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<_>>().join("::");
                            validators.validator = Some(path_str);
                        }
                        _ => {}
                    }
                }
                return Ok(());
            }

            if meta.path.is_ident("validator_with_ext") {
                let value_parser = meta.value()?;

                if let Ok(expr) = value_parser.parse::<syn::Expr>() {
                    match expr {
                        syn::Expr::Lit(expr_lit) => {
                            if let syn::Lit::Str(s) = expr_lit.lit {
                                validators.validator_with_ext = Some(s.value());
                            }
                        }
                        syn::Expr::Path(expr_path) => {
                            let path_str = expr_path.path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<_>>().join("::");
                            validators.validator_with_ext = Some(path_str);
                        }
                        _ => {}
                    }
                }
                return Ok(());
            }

            if meta.path.is_ident("generic_types") {
                let value_parser = meta.value()?;
                let _: Expr = value_parser.parse()?;
                return Ok(());
            }

            Err(meta.error("unknown #[proto(...)] attribute"))
        })
        .expect("failed to parse #[proto(...)] attributes");
    }

    validators
}

pub fn extract_item_generic_types(item_attrs: &[Attribute]) -> Vec<GenericTypeEntry> {
    let mut entries = Vec::new();

    for attr in item_attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        let result = attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("generic_types") {
                if meta.input.peek(syn::Token![=]) {
                    let value = meta.value()?;
                    let _: Expr = value.parse()?;
                }
                return Ok(());
            }

            let expr: Expr = meta.value()?.parse()?;
            let Expr::Array(array) = expr else {
                return Err(meta.error("generic_types expects an array"));
            };

            for elem in array.elems {
                let Expr::Assign(assign) = elem else {
                    return Err(meta.error("generic_types entries must be assignments"));
                };
                let Expr::Path(param_path) = *assign.left else {
                    return Err(meta.error("generic_types entry must start with a type parameter"));
                };
                let Some(param_ident) = param_path.path.get_ident() else {
                    return Err(meta.error("generic_types entry must use a single identifier"));
                };
                let Expr::Array(values) = *assign.right else {
                    return Err(meta.error("generic_types entry must assign an array of types"));
                };

                let mut types = Vec::new();
                for value in values.elems {
                    let ty: Type = syn::parse2(value.to_token_stream()).map_err(|_| meta.error("generic_types values must be types"))?;
                    types.push(ty);
                }

                entries.push(GenericTypeEntry {
                    param: param_ident.clone(),
                    types,
                });
            }

            Ok(())
        });

        if let Err(err) = result {
            panic!("failed to parse generic_types: {err}");
        }
    }

    entries
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

pub fn extract_import_all_from(item_attrs: &[Attribute]) -> Option<String> {
    for attr in item_attrs {
        if !attr.path().is_ident("proto_import_all_from") {
            continue;
        }

        if let Ok(path) = attr.parse_args::<LitStr>() {
            return Some(path.value());
        }

        if let Ok(path) = attr.parse_args::<syn::Path>() {
            return Some(path_to_proto_package(&path));
        }
    }

    None
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

fn path_to_proto_package(path: &syn::Path) -> String {
    path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>().join(".")
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
            let _ = extract_item_validators(&[attr]);
        });

        assert!(result.is_err());
    }

    #[test]
    fn generic_type_variants_builds_combinations() {
        let mut config = UnifiedProtoConfig::default();

        let input: syn::ItemStruct = parse_quote! {
            struct GenericMap<K, V, S, const CAP: usize> {
                kv: std::collections::HashMap<K, V, S>,
            }
        };
        config.generic_types = vec![
            GenericTypeEntry {
                param: parse_quote!(K),
                types: vec![parse_quote!(u64), parse_quote!(u32)],
            },
            GenericTypeEntry {
                param: parse_quote!(V),
                types: vec![parse_quote!(String), parse_quote!(u16)],
            },
            GenericTypeEntry {
                param: parse_quote!(S),
                types: vec![parse_quote!(std::hash::RandomState)],
            },
        ];

        let variants = config.generic_type_variants(&input.generics).expect("variants");
        let suffixes: Vec<_> = variants.into_iter().map(|variant| variant.suffix).collect();

        assert_eq!(
            suffixes,
            vec![
                "U64StringStdHashRandomState",
                "U64U16StdHashRandomState",
                "U32StringStdHashRandomState",
                "U32U16StdHashRandomState",
            ]
        );
    }

    #[test]
    fn parses_generic_types_attribute_values() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote! {
            #[proto(generic_types = [T = [u64, u32], U = [String]])]
        }];

        let entries = extract_item_generic_types(&attrs);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].param.to_string(), "T");
        assert_eq!(entries[0].types.len(), 2);
        assert_eq!(entries[1].param.to_string(), "U");
        assert_eq!(entries[1].types.len(), 1);
    }
}
