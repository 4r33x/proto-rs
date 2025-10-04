use proc_macro::TokenStream;
use syn::LitStr;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;

use crate::utils::REGISTRY;

struct ProtoImportArgs {
    file_name: String,
    imports: Vec<String>,
}

impl Parse for ProtoImportArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let file_name: LitStr = input.parse()?;
        let mut imports = Vec::new();

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let package: LitStr = input.parse()?;
            imports.push(package.value());
        }

        Ok(ProtoImportArgs {
            file_name: file_name.value(),
            imports,
        })
    }
}

pub fn inject_proto_import_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ProtoImportArgs);

    // Add imports as special entries in the registry with a marker prefix
    let mut registry = REGISTRY.lock().unwrap();
    let entries = registry.entry(args.file_name.clone()).or_default();

    for package in &args.imports {
        let import_entry = format!("__IMPORT__:{}", package);
        entries.insert(import_entry);
    }

    // Return empty token stream - this macro doesn't generate any Rust code
    TokenStream::new()
}
