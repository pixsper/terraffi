use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn terraffi_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
