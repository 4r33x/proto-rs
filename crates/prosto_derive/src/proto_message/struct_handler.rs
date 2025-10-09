use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Fields;
use syn::Index;
use syn::Type;

use crate::utils::field_handling::FieldHandler;
use crate::utils::field_handling::FromProtoConversion;
use crate::utils::parse_field_type;

pub fn handle_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    match &data.fields {
        Fields::Named(_) => handle_named_struct(input, data),
        Fields::Unnamed(_) => handle_tuple_struct(input, data),
        Fields::Unit => handle_unit_struct(input),
    }
}

fn handle_unit_struct(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    quote! {
        #(#attrs)*
        #vis struct #name #generics;

        #[derive(Debug)]
        #vis enum #error_name {
            MissingField { field: String },
            FieldConversion { field: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                    Self::FieldConversion { field, source } =>
                        write!(f, "Error converting field {}: {}", field, source),
                }
            }
        }
        impl std::error::Error for #error_name {}

        #[derive(::prost::Message, Clone, PartialEq)]
        #vis struct #proto_name #generics {}

        impl HasProto for #name #generics {
            type Proto = #proto_name #generics;

            fn to_proto(&self) -> Self::Proto {
                #proto_name {}
            }

            fn from_proto(_proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(Self)
            }
        }

        impl From<#name #generics> for #proto_name #generics {
            fn from(value: #name #generics) -> Self {
                value.to_proto()
            }
        }

        impl TryFrom<#proto_name #generics> for #name #generics {
            type Error = #error_name;

            fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                Self::from_proto(proto).map_err(|e| #error_name::FieldConversion {
                    field: "unknown".to_string(),
                    source: e,
                })
            }
        }
    }
}

fn handle_tuple_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let Fields::Unnamed(fields) = &data.fields else {
        panic!("Expected unnamed fields");
    };

    let mut proto_fields = Vec::new();
    let mut to_proto_conversions = Vec::new();
    let mut from_proto_conversions = Vec::new();

    // Use FieldHandler for each field
    for (idx, field) in fields.unnamed.iter().enumerate() {
        let field_num = (idx + 1) as usize;
        let field_name = syn::Ident::new(&format!("field_{}", idx), name.span());
        let tuple_idx = Index::from(idx);

        // Use FieldHandler to generate proper conversions
        let handler = FieldHandler::new(field, &field_name, field_num, &error_name, format!("field_{}", idx));

        let result = handler.generate();

        // Add proto field
        if let Some(prost_field) = result.prost_field {
            proto_fields.push(prost_field);
        }

        // Extract to_proto value expression
        if let Some(to_proto) = result.to_proto {
            // The to_proto is like: "field_name: expression"
            // We need to replace self.field_name with self.N
            let to_proto_value = extract_field_value(&to_proto, &field_name, &tuple_idx);
            to_proto_conversions.push(quote! {
                #field_name: #to_proto_value
            });
        }

        // Extract from_proto value expression
        match result.from_proto {
            FromProtoConversion::Normal(from_proto) => {
                // Extract just the value part (RHS of field: value)
                let from_proto_value = extract_conversion_value(&from_proto);
                from_proto_conversions.push(from_proto_value);
            }
            _ => panic!("Tuple structs don't support skip attributes"),
        }
    }

    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;
    let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

    quote! {
        #(#attrs)*
        #vis struct #name #generics(#(pub #field_types),*);

        #[derive(Debug)]
        #vis enum #error_name {
            MissingField { field: String },
            FieldConversion { field: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                    Self::FieldConversion { field, source } =>
                        write!(f, "Error converting field {}: {}", field, source),
                }
            }
        }
        impl std::error::Error for #error_name {}

        #[derive(::prost::Message, Clone, PartialEq)]
        #vis struct #proto_name #generics {
            #(#proto_fields,)*
        }

        impl HasProto for #name #generics {
            type Proto = #proto_name #generics;

            fn to_proto(&self) -> Self::Proto {
                #proto_name {
                    #(#to_proto_conversions),*
                }
            }

            fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(Self(
                    #(#from_proto_conversions),*
                ))
            }
        }

        impl From<#name #generics> for #proto_name #generics {
            fn from(value: #name #generics) -> Self {
                value.to_proto()
            }
        }

        impl TryFrom<#proto_name #generics> for #name #generics {
            type Error = #error_name;

            fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                Self::from_proto(proto).map_err(|e| #error_name::FieldConversion {
                    field: "unknown".to_string(),
                    source: e,
                })
            }
        }
    }
}

/// Extract the value expression from a to_proto field assignment
/// Converts "field_name: self.field_name.clone()" to "self.N.clone()"
fn extract_field_value(to_proto: &TokenStream, field_name: &syn::Ident, tuple_idx: &Index) -> TokenStream {
    // Parse the TokenStream to find the value expression
    let tokens_str = to_proto.to_string();

    // Find the colon and extract everything after it
    if let Some(colon_pos) = tokens_str.find(':') {
        let value_part = tokens_str[colon_pos + 1..].trim();

        // Replace field_name references with tuple index
        let field_name_str = field_name.to_string();
        let tuple_idx_str = tuple_idx.index.to_string(); // Extract the u32 from Index
        let replaced = value_part
            .replace(&format!("self . {}", field_name_str), &format!("self . {}", tuple_idx_str))
            .replace(&format!("self.{}", field_name_str), &format!("self.{}", tuple_idx_str));

        replaced.parse().unwrap()
    } else {
        to_proto.clone()
    }
}

/// Extract conversion value from from_proto assignment
/// Converts "field_name: proto.field_name" to "proto.field_name"
fn extract_conversion_value(from_proto: &TokenStream) -> TokenStream {
    let tokens_str = from_proto.to_string();

    // Find the colon and extract everything after it
    if let Some(colon_pos) = tokens_str.find(':') {
        let value_part = tokens_str[colon_pos + 1..].trim();
        value_part.parse().unwrap()
    } else {
        from_proto.clone()
    }
}

pub fn handle_named_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let mut proto_fields = Vec::new();
    let mut to_proto_fields = Vec::new();
    let mut from_proto_fields = Vec::new();
    let mut skip_computations = Vec::new();

    let mut field_num = 0;
    if let Fields::Named(fields) = &data.fields {
        for field in &fields.named {
            let ident = field.ident.as_ref().unwrap();

            let handler = FieldHandler::new(field, ident, field_num + 1, &error_name, ident.to_string());

            let result = handler.generate();

            if result.prost_field.is_some() {
                field_num += 1;
            }

            if let Some(prost_field) = result.prost_field {
                proto_fields.push(prost_field);
            }

            if let Some(to_proto) = result.to_proto {
                to_proto_fields.push(to_proto);
            }

            match result.from_proto {
                FromProtoConversion::Normal(from_proto) => {
                    from_proto_fields.push(from_proto);
                }
                FromProtoConversion::SkipDefault(_field_name) => {
                    from_proto_fields.push(quote! { #ident: Default::default() });
                }
                FromProtoConversion::SkipWithFn { computation, field_name: _field_name } => {
                    skip_computations.push(computation);
                    from_proto_fields.push(quote! { #ident });
                }
            }
        }
    }

    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let mut fields_named_idents = Vec::new();
    let mut fields_named_attrs = Vec::new();
    let mut fields_named_types = Vec::new();

    if let Fields::Named(fields) = &data.fields {
        for field in &fields.named {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            let field_attrs: Vec<_> = field.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();

            fields_named_idents.push(ident);
            fields_named_attrs.push(field_attrs);
            fields_named_types.push(ty);
        }
    }

    quote! {
    #(#attrs)*
    #vis struct #name #generics {
        #(
            #(#fields_named_attrs)*
            pub #fields_named_idents: #fields_named_types,
        )*
    }

        #[derive(Debug)]
        #vis enum #error_name {
            MissingField { field: String },
            FieldConversion { field: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                    Self::FieldConversion { field, source } =>
                        write!(f, "Error converting field {}: {}", field, source),
                }
            }
        }
        impl std::error::Error for #error_name {}

        #[derive(::prost::Message, Clone, PartialEq)]
        #vis struct #proto_name #generics {
            #(#proto_fields,)*
        }

        impl HasProto for #name #generics {
            type Proto = #proto_name #generics;

            fn to_proto(&self) -> Self::Proto {
                #proto_name {
                    #(#to_proto_fields),*
                }
            }

            fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                #(#skip_computations)*

                Ok(Self {
                    #(#from_proto_fields),*
                })
            }
        }

        impl From<#name #generics> for #proto_name #generics {
            fn from(value: #name #generics) -> Self {
                value.to_proto()
            }
        }

        impl TryFrom<#proto_name #generics> for #name #generics {
            type Error = #error_name;

            fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                Self::from_proto(proto).map_err(|e| #error_name::FieldConversion {
                    field: "unknown".to_string(),
                    source: e,
                })
            }
        }
    }
}
