use proc_macro::TokenStream;
use quote::quote;
use syn::LitStr;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;

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

    let c = crate::schema::schema_tokens_for_imports("ImportInject", &args.file_name, &args.imports);
    quote! {#c}.into()
}
