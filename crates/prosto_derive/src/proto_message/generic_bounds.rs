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

    let mut used_lifetimes = BTreeSet::new();
    let mut used_encode = BTreeSet::new();
    let mut bound_types = Vec::new();

    for info in fields {
        if !uses_proto_wire_directly(info) {
            continue;
        }
        collect_type_params(&info.parsed.rust_type, &type_params, &mut used_lifetimes);
        if info.parsed.map_kind.is_some() {
            collect_type_params(&info.parsed.proto_rust_type, &type_params, &mut used_encode);
        } else {
            collect_type_params(&info.parsed.rust_type, &type_params, &mut used_encode);
        }
        if info.parsed.is_option {
            bound_types.push(info.parsed.elem_type.clone());
        } else {
            bound_types.push(info.proto_ty.clone());
        }
    }

    if used_lifetimes.is_empty() && used_encode.is_empty() && bound_types.is_empty() {
        return generics.clone();
    }

    let mut bounded = generics.clone();
    let where_clause = bounded.make_where_clause();
    for ident in &used_lifetimes {
        where_clause.predicates.push(parse_quote!(for<'a> #ident: 'a));
    }
    for ident in used_encode {
        where_clause.predicates.push(parse_quote!(for<'a> #ident: ::proto_rs::EncodeInputFromRef<'a>));
    }
    for ty in bound_types {
        where_clause.predicates.push(parse_quote!(for<'a> #ty: ::proto_rs::EncodeInputFromRef<'a>));
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
                                GenericArgument::Type(inner_ty) => {
                                    collect_type_params(inner_ty, params, used);
                                }
                                GenericArgument::AssocType(assoc) => {
                                    collect_type_params(&assoc.ty, params, used);
                                }
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
