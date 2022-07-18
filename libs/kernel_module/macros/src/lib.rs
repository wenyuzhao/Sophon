use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn kernel_module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStatic);
    let name = &input.ident;
    let result = quote! {
        #input
        kernel_module::declare_kernel_module!(#name);
    };
    result.into()
}
