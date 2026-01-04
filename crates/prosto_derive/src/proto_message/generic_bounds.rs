use std::collections::BTreeSet;

use syn::GenericArgument;
use syn::Generics;
use syn::Ident;
use syn::PathArguments;
use syn::Type;
use syn::parse_quote;

use super::unified_field_handler::FieldInfo;
use super::unified_field_handler::uses_proto_wire_directly;

pub fn add_proto_wire_bounds<'a>(generics: &Generics, fields: impl IntoIterator<Item = &'a FieldInfo<'a>>) -> Generics {
    let type_params: BTreeSet<Ident> = generics.type_params().map(|param| param.ident.clone()).collect();
    if type_params.is_empty() {
        return generics.clone();
    }

    let mut used = BTreeSet::new();
    let mut used_ref = BTreeSet::new();

    for info in fields {
        if !uses_proto_wire_directly(info) {
            continue;
        }
        collect_type_params(&info.parsed.rust_type, &type_params, &mut used);
        collect_ref_type_params(&info.parsed.rust_type, &type_params, &mut used_ref);
    }

    if used.is_empty() && used_ref.is_empty() {
        return generics.clone();
    }

    let mut bounded = generics.clone();
    let where_clause = bounded.make_where_clause();
    for ident in &used {
        where_clause.predicates.push(parse_quote!(for<'a> #ident: 'a));
    }
    for ident in used {
        where_clause.predicates.push(parse_quote!(for<'a> #ident: ::proto_rs::EncodeInputFromRef<'a>));
    }
    for ident in used_ref {
        where_clause.predicates.push(parse_quote!(for<'a> #ident: ::proto_rs::ProtoWire<EncodeInput<'a> = &'a #ident>));
    }

    bounded
}

fn collect_type_params(ty: &Type, params: &BTreeSet<Ident>, used: &mut BTreeSet<Ident>) {
    match ty {
        Type::Path(type_path) => {
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let ident = &type_path.path.segments[0].ident;
                if params.contains(ident) {
                    used.insert(ident.clone());
                }
            }
            for segment in &type_path.path.segments {
                match &segment.arguments {
                    PathArguments::None => {}
                    PathArguments::AngleBracketed(args) => {
                        for arg in &args.args {
                            match arg {
                                GenericArgument::Type(inner_ty) => collect_type_params(inner_ty, params, used),
                                GenericArgument::AssocType(assoc) => collect_type_params(&assoc.ty, params, used),
                                GenericArgument::Constraint(constraint) => {
                                    for bound in &constraint.bounds {
                                        if let syn::TypeParamBound::Trait(trait_bound) = bound {
                                            for segment in &trait_bound.path.segments {
                                                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                                                    for arg in &args.args {
                                                        if let GenericArgument::Type(inner_ty) = arg {
                                                            collect_type_params(inner_ty, params, used);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                GenericArgument::Lifetime(_) | GenericArgument::Const(_) | GenericArgument::AssocConst(_) | _ => {}
                            }
                        }
                    }
                    PathArguments::Parenthesized(args) => {
                        for input in &args.inputs {
                            collect_type_params(input, params, used);
                        }
                        if let syn::ReturnType::Type(_, output) = &args.output {
                            collect_type_params(output, params, used);
                        }
                    }
                }
            }
        }
        Type::Reference(reference) => collect_type_params(&reference.elem, params, used),
        Type::Array(array) => collect_type_params(&array.elem, params, used),
        Type::Slice(slice) => collect_type_params(&slice.elem, params, used),
        Type::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_type_params(elem, params, used);
            }
        }
        Type::Paren(paren) => collect_type_params(&paren.elem, params, used),
        Type::Group(group) => collect_type_params(&group.elem, params, used),
        Type::Ptr(ptr) => collect_type_params(&ptr.elem, params, used),
        Type::BareFn(bare_fn) => {
            for input in &bare_fn.inputs {
                collect_type_params(&input.ty, params, used);
            }
            if let syn::ReturnType::Type(_, output) = &bare_fn.output {
                collect_type_params(output, params, used);
            }
        }
        Type::ImplTrait(impl_trait) => {
            for bound in &impl_trait.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    for segment in &trait_bound.path.segments {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    collect_type_params(inner_ty, params, used);
                                }
                            }
                        }
                    }
                }
            }
        }
        Type::TraitObject(trait_object) => {
            for bound in &trait_object.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    for segment in &trait_bound.path.segments {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    collect_type_params(inner_ty, params, used);
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_ref_type_params(ty: &Type, params: &BTreeSet<Ident>, used: &mut BTreeSet<Ident>) {
    collect_ref_type_params_inner(ty, params, used, false);
}

fn collect_ref_type_params_inner(ty: &Type, params: &BTreeSet<Ident>, used: &mut BTreeSet<Ident>, ref_position: bool) {
    match ty {
        Type::Array(array) => collect_ref_type_params_inner(&array.elem, params, used, true),
        Type::Path(type_path) => {
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let ident = &type_path.path.segments[0].ident;
                if ref_position && params.contains(ident) {
                    used.insert(ident.clone());
                }
            }

            if type_path.qself.is_none() {
                if let Some(segment) = type_path.path.segments.last() {
                    match segment.ident.to_string().as_str() {
                        "Vec" | "VecDeque" | "BTreeSet" | "HashSet" => {
                            if let Some(inner_ty) = first_type_arg(&segment.arguments) {
                                collect_ref_type_params_inner(inner_ty, params, used, true);
                            }
                        }
                        "HashMap" | "BTreeMap" => {
                            if let Some(key_ty) = first_type_arg(&segment.arguments) {
                                collect_ref_type_params_inner(key_ty, params, used, true);
                            }
                        }
                        _ => {}
                    }
                }
            }

            for segment in &type_path.path.segments {
                match &segment.arguments {
                    PathArguments::None => {}
                    PathArguments::AngleBracketed(args) => {
                        for arg in &args.args {
                            match arg {
                                GenericArgument::Type(inner_ty) => collect_ref_type_params_inner(inner_ty, params, used, false),
                                GenericArgument::AssocType(assoc) => collect_ref_type_params_inner(&assoc.ty, params, used, false),
                                GenericArgument::Constraint(constraint) => {
                                    for bound in &constraint.bounds {
                                        if let syn::TypeParamBound::Trait(trait_bound) = bound {
                                            for segment in &trait_bound.path.segments {
                                                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                                                    for arg in &args.args {
                                                        if let GenericArgument::Type(inner_ty) = arg {
                                                            collect_ref_type_params_inner(inner_ty, params, used, false);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                GenericArgument::Lifetime(_) | GenericArgument::Const(_) | GenericArgument::AssocConst(_) | _ => {}
                            }
                        }
                    }
                    PathArguments::Parenthesized(args) => {
                        for input in &args.inputs {
                            collect_ref_type_params_inner(input, params, used, false);
                        }
                        if let syn::ReturnType::Type(_, output) = &args.output {
                            collect_ref_type_params_inner(output, params, used, false);
                        }
                    }
                }
            }
        }
        Type::Reference(reference) => collect_ref_type_params_inner(&reference.elem, params, used, ref_position),
        Type::Slice(slice) => collect_ref_type_params_inner(&slice.elem, params, used, ref_position),
        Type::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_ref_type_params_inner(elem, params, used, ref_position);
            }
        }
        Type::Paren(paren) => collect_ref_type_params_inner(&paren.elem, params, used, ref_position),
        Type::Group(group) => collect_ref_type_params_inner(&group.elem, params, used, ref_position),
        Type::Ptr(ptr) => collect_ref_type_params_inner(&ptr.elem, params, used, ref_position),
        Type::BareFn(bare_fn) => {
            for input in &bare_fn.inputs {
                collect_ref_type_params_inner(&input.ty, params, used, ref_position);
            }
            if let syn::ReturnType::Type(_, output) = &bare_fn.output {
                collect_ref_type_params_inner(output, params, used, ref_position);
            }
        }
        Type::ImplTrait(impl_trait) => {
            for bound in &impl_trait.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    for segment in &trait_bound.path.segments {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    collect_ref_type_params_inner(inner_ty, params, used, ref_position);
                                }
                            }
                        }
                    }
                }
            }
        }
        Type::TraitObject(trait_object) => {
            for bound in &trait_object.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    for segment in &trait_bound.path.segments {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    collect_ref_type_params_inner(inner_ty, params, used, ref_position);
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn first_type_arg(arguments: &PathArguments) -> Option<&Type> {
    match arguments {
        PathArguments::AngleBracketed(args) => args.args.iter().find_map(|arg| match arg {
            GenericArgument::Type(inner_ty) => Some(inner_ty),
            _ => None,
        }),
        _ => None,
    }
}
