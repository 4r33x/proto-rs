use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::Mutex;

use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;
use syn::Field;
use syn::Fields;
use syn::GenericArgument;
use syn::Lit;
use syn::PathArguments;
use syn::Type;
use syn::TypePath;
use syn::parse_quote;

/// Global registry: filename -> HashSet<proto definitions>
pub static REGISTRY: LazyLock<Mutex<HashMap<String, HashSet<String>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Track which files have been initialized (cleared) this compilation
pub static INITIALIZED_FILES: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Debug, Clone)]
pub struct FieldConfig {
    pub into_type: Option<String>,
    pub from_type: Option<String>,
    pub into_fn: Option<String>,
    pub from_fn: Option<String>,
    pub skip: bool,
    pub skip_deser_fn: Option<String>,
    pub is_rust_enum: bool,
    pub import_path: Option<String>,
    pub is_message: bool,
    pub is_proto_enum: bool,
}

pub fn parse_field_config(field: &Field) -> FieldConfig {
    let mut config = FieldConfig {
        into_type: None,
        from_type: None,
        into_fn: None,
        from_fn: None,
        skip: false,
        is_rust_enum: false,
        import_path: None,
        is_message: false,
        is_proto_enum: false,
        skip_deser_fn: None,
    };

    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
            // Parse each nested meta in the attribute
            let _ = attr.parse_nested_meta(|meta| {
                match meta.path.get_ident().map(|i| i.to_string()).as_deref() {
                    Some("skip") => {
                        // Check if there's a value (function name)
                        if meta.input.peek(syn::Token![=]) {
                            if let Ok(s) = meta.value()?.parse::<Lit>()
                                && let Lit::Str(s) = s
                            {
                                config.skip = true;
                                config.skip_deser_fn = Some(s.value());
                            }
                        } else {
                            // Plain skip without value
                            config.skip = true;
                        }
                    }
                    Some("rust_enum") => config.is_rust_enum = true,
                    Some("enum") => config.is_proto_enum = true,
                    Some("message") => config.is_message = true,
                    Some("into") => {
                        if let Ok(s) = meta.value()?.parse::<Lit>()
                            && let Lit::Str(s) = s
                        {
                            config.into_type = Some(s.value());
                        }
                    }
                    Some("import_path") => {
                        if let Ok(s) = meta.value()?.parse::<Lit>()
                            && let Lit::Str(s) = s
                        {
                            config.import_path = Some(s.value());
                        }
                    }
                    Some("from") => {
                        if let Ok(s) = meta.value()?.parse::<Lit>()
                            && let Lit::Str(s) = s
                        {
                            config.from_type = Some(s.value());
                        }
                    }
                    Some("into_fn") => {
                        if let Ok(s) = meta.value()?.parse::<Lit>()
                            && let Lit::Str(s) = s
                        {
                            config.into_fn = Some(s.value());
                        }
                    }
                    Some("from_fn") => {
                        if let Ok(s) = meta.value()?.parse::<Lit>()
                            && let Lit::Str(s) = s
                        {
                            config.from_fn = Some(s.value());
                        }
                    }
                    _ => {}
                }
                Ok(())
            });
        }
    }

    config
}

pub fn is_complex_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if let Some(segment) = path.segments.last() {
                let type_name = segment.ident.to_string();
                if (type_name == "Option" || type_name == "Vec")
                    && let PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
                {
                    return is_complex_type(inner_ty);
                }
                !matches!(
                    type_name.as_str(),
                    "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" | "bool" | "String"
                )
            } else {
                true
            }
        }
        _ => true,
    }
}

pub fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(segment) = path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

pub fn convert_field_type_to_proto(ty: &Type) -> Type {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();

                // Check if this type actually has a shadow
                if is_complex_type(ty) && !matches!(type_name.as_str(), "Option" | "Vec") {
                    let proto_ident = syn::Ident::new(&format!("{}Proto", type_name), segment.ident.span());
                    return syn::parse_quote! { #proto_ident };
                }

                if (type_name == "Option" || type_name == "Vec")
                    && let PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
                {
                    let inner_proto = convert_field_type_to_proto(inner_ty);
                    let container = syn::Ident::new(&type_name, segment.ident.span());
                    return syn::parse_quote! { #container<#inner_proto> };
                }
            }
            ty.clone()
        }
        _ => ty.clone(),
    }
}

#[derive(Clone)]
pub struct ParsedFieldType {
    pub rust_type: Type,          // Rust type (original)
    pub proto_type: String,       // Proto type string (for .proto file)
    pub prost_type: TokenStream2, // prost attribute type
    pub is_option: bool,
    pub is_repeated: bool,

    pub is_message_like: bool, // custom message
    pub proto_rust_type: Type, // Rust type for Proto struct (i.e., QuoteLamportsProto)
}
pub fn parse_field_type(ty: &Type) -> ParsedFieldType {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        let mut parsed = parse_field_type(inner_ty);
                        parsed.is_option = true;
                        parsed.proto_rust_type = ty.clone();
                        return parsed;
                    }
                    panic!("Invalid Option type");
                }

                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        // Vec<u8> -> bytes
                        if let Type::Path(inner_path) = inner_ty
                            && inner_path.path.segments.last().unwrap().ident == "u8"
                        {
                            return ParsedFieldType {
                                rust_type: parse_quote! { Vec<u8> },
                                proto_type: "bytes".to_string(),
                                prost_type: quote! { bytes },
                                is_option: false,
                                is_repeated: false,
                                is_message_like: false,
                                proto_rust_type: parse_quote! { Vec<u8> },
                            };
                        }
                        let mut inner_parsed = parse_field_type(inner_ty);
                        inner_parsed.is_repeated = true;
                        return inner_parsed;
                    }
                    panic!("Invalid Vec type");
                }

                "u64" => ParsedFieldType::primitive(ty.clone(), "uint64", quote! { uint64 }),
                "u32" => ParsedFieldType::primitive(ty.clone(), "uint32", quote! { uint32 }),
                "i64" => ParsedFieldType::primitive(ty.clone(), "int64", quote! { int64 }),
                "i32" => ParsedFieldType::primitive(ty.clone(), "int32", quote! { int32 }),
                "f32" => ParsedFieldType::primitive(ty.clone(), "float", quote! { float }),
                "f64" => ParsedFieldType::primitive(ty.clone(), "double", quote! { double }),
                "String" => ParsedFieldType::primitive(ty.clone(), "string", quote! { string }),
                "bool" => ParsedFieldType::primitive(ty.clone(), "bool", quote! { bool }),

                custom => {
                    // Custom type -> message
                    let proto_type = "message".to_string();
                    let proto_rust_type = if custom.ends_with("Proto") {
                        syn::parse_str::<Type>(custom).unwrap()
                    } else {
                        syn::parse_str::<Type>(&format!("{}Proto", custom)).unwrap()
                    };

                    ParsedFieldType {
                        rust_type: ty.clone(),
                        proto_type,
                        prost_type: quote! { message },
                        is_option: false,
                        is_repeated: false,
                        is_message_like: true,
                        proto_rust_type,
                    }
                }
            }
        }
        _ => panic!("Unsupported type {:?}", ty.to_token_stream()),
    }
}

impl ParsedFieldType {
    fn primitive(rust_type: Type, proto_type: &str, prost_type: proc_macro2::TokenStream) -> Self {
        Self {
            rust_type,
            proto_type: proto_type.to_string(),
            prost_type,
            is_option: false,
            is_repeated: false,
            is_message_like: false,
            proto_rust_type: parse_quote! { rust_type },
        }
    }
}

pub fn to_upper_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 && prev_is_lower {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
        prev_is_lower = c.is_lowercase();
    }
    result
}

pub fn generate_struct_proto(name: &str, fields: &Fields) -> String {
    let mut proto_fields = String::new();
    let mut field_num = 0;

    for field in fields.iter() {
        let config = parse_field_config(field);
        if config.skip {
            continue;
        }

        field_num += 1;
        let ident = field.ident.as_ref().unwrap().to_string();

        // Get the type to use for proto generation
        let ty = if let Some(ref into_type) = config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field.ty.clone())
        } else {
            field.ty.clone()
        };

        // Special handling for Vec<u8> -> bytes
        if is_bytes_vec(&ty) {
            proto_fields.push_str(&format!("  bytes {} = {};\n", ident, field_num));
            continue;
        }

        // Extract the actual type from wrappers (Option/Vec)
        let (is_option, is_repeated, inner_type) = if is_option_type(&ty) {
            if let Type::Path(type_path) = &ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            (true, false, inner.clone())
                        } else {
                            (true, false, ty.clone())
                        }
                    } else {
                        (true, false, ty.clone())
                    }
                } else {
                    (true, false, ty.clone())
                }
            } else {
                (true, false, ty.clone())
            }
        } else if let Some(inner) = vec_inner_type(&ty) {
            (false, true, inner)
        } else {
            (false, false, ty.clone())
        };

        // Determine the proto type string
        let proto_ty_str = if let Some(ref import_path) = config.import_path {
            // Use the import path prefix
            let base_name = rust_type_path_ident(&inner_type).to_string();
            format!("{}.{}", import_path, base_name)
        } else if config.is_rust_enum {
            // For Rust enums converted to proto, get the type name
            rust_type_path_ident(&inner_type).to_string()
        } else if config.is_proto_enum {
            // For proto-native enums, use the type name as-is
            rust_type_path_ident(&inner_type).to_string()
        } else if config.is_message {
            // For imported message types, use as-is without Proto suffix
            rust_type_path_ident(&inner_type).to_string()
        } else if is_complex_type(&inner_type) {
            // For complex types, strip Proto suffix for .proto file
            let base_name = rust_type_path_ident(&inner_type).to_string();
            strip_proto_suffix(&base_name)
        } else {
            // For primitives, use the proto type
            let parsed = parse_field_type(&inner_type);
            parsed.proto_type
        };

        // Determine modifier
        let modifier = if is_repeated {
            "repeated "
        } else if is_option {
            "optional "
        } else {
            ""
        };

        proto_fields.push_str(&format!("  {}{} {} = {};\n", modifier, proto_ty_str, ident, field_num));
    }

    format!("message {} {{\n{}}}\n\n", name, proto_fields)
}

pub fn generate_enum_proto(name: &str, variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>) -> String {
    let mut proto_variants = String::new();
    for (i, variant) in variants.iter().enumerate() {
        let variant_name = variant.ident.to_string();
        let proto_name = to_upper_snake_case(&variant_name);
        proto_variants.push_str(&format!("  {} = {};\n", proto_name, i));
    }
    format!("enum {} {{\n{}}}\n\n", name, proto_variants)
}

pub fn rust_type_path_ident(ty: &Type) -> &syn::Ident {
    if let Type::Path(type_path) = ty {
        &type_path.path.segments.last().unwrap().ident
    } else {
        panic!("Expected Type::Path, got {:?}", ty.to_token_stream());
    }
}

/// Extract inner type of Vec<T>
pub fn vec_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
        && seg.ident == "Vec"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return Some(inner_ty.clone());
    }
    None
}

pub fn is_bytes_vec(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(segment) = path.segments.last()
        && segment.ident == "Vec"
        && let PathArguments::AngleBracketed(ref args) = segment.arguments
        && args.args.len() == 1
        && let GenericArgument::Type(Type::Path(TypePath { path: inner_path, .. })) = &args.args[0]
        && let Some(inner_seg) = inner_path.segments.last()
    {
        inner_seg.ident == "u8"
    } else {
        false
    }
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 && prev_is_lower {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
        prev_is_lower = c.is_lowercase();
    }
    result
}

pub fn strip_proto_suffix(type_name: &str) -> String {
    type_name.strip_suffix("Proto").unwrap_or(type_name).to_string()
}

use proc_macro::TokenStream;
use syn::Attribute;
use syn::Expr;
use syn::ExprLit;
use syn::MetaNameValue;

#[derive(Debug, Clone)]
pub struct ProtoConfig {
    pub file_name: String,
}

impl Default for ProtoConfig {
    fn default() -> Self {
        Self {
            file_name: "generated.proto".to_string(),
        }
    }
}
impl ProtoConfig {
    /// Parse proto configuration from both macro attributes and item attributes
    ///
    /// # Arguments
    /// * `attr` - The macro attribute TokenStream (e.g., from `#[proto_message(file = "test.proto")]`)
    /// * `item_attrs` - The attributes on the item itself
    ///
    /// # Returns
    /// A ProtoConfig with the file name extracted from the macro attribute, or default if not provided
    pub fn parse_proto_config(attr: TokenStream, _item_attrs: &[Attribute]) -> Self {
        let mut config = Self::default();

        // Parse file name from macro attribute if present
        if !attr.is_empty() {
            if let Ok(meta) = syn::parse::<MetaNameValue>(attr) {
                if meta.path.is_ident("file") {
                    if let Expr::Lit(ExprLit { lit: Lit::Str(litstr), .. }) = &meta.value {
                        config.file_name = litstr.value();
                    } else {
                        panic!("Expected string literal for file attribute");
                    }
                } else {
                    panic!("Expected file = \"...\" attribute");
                }
            } else {
                panic!("Invalid macro attribute format. Expected: #[proto_message(file = \"filename.proto\")]");
            }
        }

        config
    }
}
