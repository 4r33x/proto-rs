use proc_macro::TokenStream;
use syn::Attribute;
use syn::Expr;
use syn::ExprLit;
use syn::Lit;
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
