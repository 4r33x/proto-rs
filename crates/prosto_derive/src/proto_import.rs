use proc_macro::TokenStream;
use quote::quote;
use syn::LitStr;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;

use crate::write_file::register_import;

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

    // Register imports in the registry
    // This will write to file if emission is enabled (feature/env)
    let c = register_import(&args.file_name, &args.imports);

    // Return empty token stream - this macro doesn't generate any Rust code
    // The emission is handled by register_import which respects emission mode
    quote! {#c}.into()
}
