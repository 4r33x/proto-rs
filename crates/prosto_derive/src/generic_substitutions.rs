use std::collections::BTreeMap;

use syn::GenericArgument;
use syn::PathArguments;
use syn::Type;
use syn::TypeArray;
use syn::TypeGroup;
use syn::TypeParen;
use syn::TypeReference;
use syn::TypeTuple;

pub fn apply_generic_substitutions_fields(fields: &syn::Fields, substitutions: &BTreeMap<String, Type>) -> syn::Fields {
    let mut fields = fields.clone();
    match &mut fields {
        syn::Fields::Named(named) => {
            for field in &mut named.named {
                field.ty = apply_generic_substitutions_type(&field.ty, substitutions);
            }
        }
        syn::Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.unnamed {
                field.ty = apply_generic_substitutions_type(&field.ty, substitutions);
            }
        }
        syn::Fields::Unit => {}
    }
    fields
}

pub fn apply_generic_substitutions_enum(data: &syn::DataEnum, substitutions: &BTreeMap<String, Type>) -> syn::DataEnum {
    let mut data = data.clone();
    for variant in &mut data.variants {
        match &mut variant.fields {
            syn::Fields::Named(named) => {
                for field in &mut named.named {
                    field.ty = apply_generic_substitutions_type(&field.ty, substitutions);
                }
            }
            syn::Fields::Unnamed(unnamed) => {
                for field in &mut unnamed.unnamed {
                    field.ty = apply_generic_substitutions_type(&field.ty, substitutions);
                }
            }
            syn::Fields::Unit => {}
        }
    }
    data
}

fn apply_generic_substitutions_type(ty: &Type, substitutions: &BTreeMap<String, Type>) -> Type {
    if let Type::Path(path) = ty
        && path.qself.is_none()
        && path.path.segments.len() == 1
        && path.path.segments[0].arguments.is_empty()
    {
        let ident = path.path.segments[0].ident.to_string();
        if let Some(replacement) = substitutions.get(&ident) {
            return replacement.clone();
        }
    }

    let mut updated = ty.clone();
    match &mut updated {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last_mut()
                && let PathArguments::AngleBracketed(args) = &mut segment.arguments
            {
                for arg in &mut args.args {
                    if let GenericArgument::Type(ty) = arg {
                        *ty = apply_generic_substitutions_type(ty, substitutions);
                    }
                }
            }
        }

        Type::Paren(TypeParen { elem, .. })
        | Type::Group(TypeGroup { elem, .. })
        | Type::Reference(TypeReference { elem, .. })
        | Type::Array(TypeArray { elem, .. }) => {
            **elem = apply_generic_substitutions_type(elem, substitutions);
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            for elem in elems {
                *elem = apply_generic_substitutions_type(elem, substitutions);
            }
        }

        _ => {}
    }

    updated
}
