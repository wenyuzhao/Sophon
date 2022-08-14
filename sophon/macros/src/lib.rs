use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let kind = if attr.is_empty() {
        "kernel".to_owned()
    } else {
        syn::parse_macro_input!(attr as syn::Ident).to_string()
    };
    let kind = match kind.as_str() {
        "boot" => quote!(crate::utils::testing::TestKind::Boot),
        "kernel" => quote!(crate::utils::testing::TestKind::Kernel),
        _ => panic!("Invalid test kind '{}'. Expect 'kernel' or 'boot'.", kind),
    };
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let name = &input.sig.ident;
    let result = quote! {
        #[cfg(sophon_test)]
        #[used]
        #[allow(non_upper_case_globals)]
        #[doc(hidden)]
        #[link_section = ".init_array"]
        pub static #name: extern "C" fn() = {
            #input

            extern "C" fn __test_function_wrapper() {
                crate::utils::testing::register_test(#kind, &#name);
            }

            __test_function_wrapper
        };
    };
    result.into()
}
