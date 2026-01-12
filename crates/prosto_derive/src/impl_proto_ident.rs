use proc_macro::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::AngleBracketedGenericArguments;
use syn::GenericArgument;
use syn::Ident;
use syn::PathArguments;
use syn::Token;
use syn::Type;
use syn::parse::Parse;
use syn::parse::ParseStream;

/// Input like: `BuildHasherDefault<T>` ;
/// Accepts either with or without trailing semicolon.
struct ImplProtoIdentInput {
    ty: Type,
}

impl Parse for ImplProtoIdentInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ty: Type = input.parse()?;
        // Allow optional trailing ';'
        let _ = input.parse::<Token![;]>();
        Ok(Self { ty })
    }
}

pub fn impl_proto_ident(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as ImplProtoIdentInput);

    let ty = input.ty;

    // Collect generic type idents used as angle-bracket args on any path segment.
    // Example: BuildHasherDefault<T> => [T]
    // Example: Foo<K, V> => [K, V]
    // Only collects GenericArgument::Type(Type::Path(..)) where last segment is an Ident with no args.
    let mut params: Vec<Ident> = Vec::new();

    // Walk the type tree (best-effort). We only really handle Type::Path for now.
    collect_type_params(&ty, &mut params);

    // Dedup while preserving order.
    let mut dedup: Vec<Ident> = Vec::new();
    for p in params {
        if !dedup.iter().any(|x| x == &p) {
            dedup.push(p);
        }
    }

    let ty_str = ty.to_token_stream().to_string();

    let expanded = if dedup.is_empty() {
        quote! {
            #[cfg(feature = "build-schemas")]
            impl ::proto_rs::schemas::ProtoIdentifiable for #ty {
                const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = ::proto_rs::schemas::ProtoIdent {
                    module_path: module_path!(),
                    name: stringify!(#ty),
                    proto_package_name: "",
                    proto_file_path: "",
                    proto_type: ::proto_rs::schemas::ProtoType::Message(stringify!(#ty)),
                    generics: &[],
                };
                const PROTO_TYPE: ::proto_rs::schemas::ProtoType = ::proto_rs::schemas::ProtoType::Message(stringify!(#ty));
            }
        }
    } else {
        quote! {
            #[cfg(feature = "build-schemas")]
            impl <#(#dedup),*> ::proto_rs::schemas::ProtoIdentifiable for #ty {
                const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = ::proto_rs::schemas::ProtoIdent {
                    module_path: module_path!(),
                    name: stringify!(#ty),
                    proto_package_name: "",
                    proto_file_path: "",
                    proto_type: ::proto_rs::schemas::ProtoType::Message(stringify!(#ty)),
                    generics: &[],
                };
                const PROTO_TYPE: ::proto_rs::schemas::ProtoType = ::proto_rs::schemas::ProtoType::Message(stringify!(#ty));
            }
        }
    };

    // Optional: if you want better error messages, you can validate that stringify!(#ty)
    // matches tokens; for now we keep it simple.
    let _ = ty_str;
    expanded.into()
}

fn collect_type_params(ty: &Type, out: &mut Vec<Ident>) {
    match ty {
        Type::Path(tp) => {
            for seg in &tp.path.segments {
                if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                    collect_angle_params(ab, out);
                }
            }
        }
        Type::Reference(tr) => collect_type_params(&tr.elem, out),
        Type::Paren(tp) => collect_type_params(&tp.elem, out),
        Type::Group(tg) => collect_type_params(&tg.elem, out),
        // Add more variants as you need (Tuple, ImplTrait, etc.)
        _ => {}
    }
}

fn collect_angle_params(ab: &AngleBracketedGenericArguments, out: &mut Vec<Ident>) {
    for arg in &ab.args {
        if let GenericArgument::Type(Type::Path(tp)) = arg {
            // Only accept "T" style args (single ident, no nested args), as generic parameters.
            if let Some(last) = tp.path.segments.last() {
                if last.arguments.is_empty() && tp.path.segments.len() == 1 {
                    out.push(last.ident.clone());
                } else {
                    // Recurse into nested generic types: Foo<Bar<T>> should still find T.
                    for seg in &tp.path.segments {
                        if let PathArguments::AngleBracketed(inner) = &seg.arguments {
                            collect_angle_params(inner, out);
                        }
                    }
                }
            }
        }
    }
}
