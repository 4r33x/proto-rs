use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Field;
use syn::GenericArgument;
use syn::Lit;
use syn::PathArguments;
use syn::Type;
use syn::TypePath;
use syn::parse_quote;

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
    pub rust_type: Type,         // Rust type (original)
    pub proto_type: String,      // Proto type string (for .proto file)
    pub prost_type: TokenStream, // prost attribute type
    pub is_option: bool,
    pub is_repeated: bool,

    pub is_message_like: bool, // custom message
    pub proto_rust_type: Type, // Rust type for Proto struct (i.e., QuoteLamportsProto)
}
pub fn parse_field_type(ty: &Type) -> ParsedFieldType {
    match ty {
        Type::Array(type_array) => {
            // Handle array types [T; N]
            let elem_ty = &*type_array.elem;

            // [u8; N] is handled separately as bytes
            if let Type::Path(elem_path) = elem_ty
                && let Some(segment) = elem_path.path.segments.last()
                && segment.ident == "u8"
            {
                return ParsedFieldType {
                    rust_type: ty.clone(),
                    proto_type: "bytes".to_string(),
                    prost_type: quote! { bytes },
                    is_option: false,
                    is_repeated: false,
                    is_message_like: false,
                    proto_rust_type: parse_quote! { Vec<u8> },
                };
            }

            // Other arrays are treated as repeated
            let inner_parsed = parse_field_type(elem_ty);
            ParsedFieldType {
                rust_type: ty.clone(),
                proto_type: inner_parsed.proto_type.clone(),
                prost_type: inner_parsed.prost_type.clone(),
                is_option: false,
                is_repeated: true, // Arrays are like repeated fields
                is_message_like: inner_parsed.is_message_like,
                proto_rust_type: parse_quote! { Vec<#elem_ty> },
            }
        }
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
                "u8" => ParsedFieldType::primitive(ty.clone(), "uint32", quote! { uint32 }),
                "u16" => ParsedFieldType::primitive(ty.clone(), "uint32", quote! { uint32 }),
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

pub fn rust_type_path_ident(ty: &syn::Type) -> &syn::Ident {
    match ty {
        syn::Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().unwrap();
            let ident = &last_segment.ident;

            match ident.to_string().as_str() {
                // Recursively unwrap Vec<T>, Option<T>, Box<T>, etc.
                "Vec" | "Option" | "Box" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        return rust_type_path_ident(inner_ty);
                    }
                    ident
                }
                _ => ident,
            }
        }
        syn::Type::Array(arr) => rust_type_path_ident(&arr.elem),
        syn::Type::Reference(r) => rust_type_path_ident(&r.elem),
        syn::Type::Group(g) => rust_type_path_ident(&g.elem),
        // fallback — this shouldn't happen in your macro context
        _ => panic!("Unsupported type structure in rust_type_path_ident: {:?}", ty.to_token_stream().to_string()),
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

// Helper to extract wrapper info (returns base type, is_option, is_repeated)
pub fn extract_wrapper_info(ty: &Type) -> (Type, bool, bool) {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        if segment.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                return (inner.clone(), true, false);
            }
        } else if segment.ident == "Vec"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return (inner.clone(), false, true);
        }
    }
    (ty.clone(), false, false)
}

/// Information about a method extracted from the trait
pub struct MethodInfo {
    pub name: syn::Ident,
    pub _attrs: Vec<syn::Attribute>,
    pub request_type: Box<Type>,
    pub response_type: Box<Type>,
    pub is_streaming: bool,
    pub stream_type_name: Option<syn::Ident>,
    pub inner_response_type: Option<Type>,
    pub user_method_signature: TokenStream,
}

pub fn is_bytes_array(ty: &Type) -> bool {
    if let Type::Array(type_array) = ty
        && let Type::Path(elem_type) = &*type_array.elem
        && let Some(segment) = elem_type.path.segments.first()
    {
        return segment.ident == "u8";
    }
    false
}

pub fn is_array_type(ty: &Type) -> bool {
    matches!(ty, Type::Array(_))
}

pub fn array_elem_type(ty: &Type) -> Option<Type> {
    if let Type::Array(type_array) = ty { Some((*type_array.elem).clone()) } else { None }
}
