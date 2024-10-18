use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn kernel_module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStatic);
    let name = &input.ident;
    let result = quote! {
        #input

        #[global_allocator]
        static ALLOCATOR: kernel_module::KernelModuleAllocator = kernel_module::KernelModuleAllocator;

        #[no_mangle]
        #[allow(unused)]
        #[allow(static_mut_refs)]
        pub extern "C" fn _start(service: kernel_module::KernelServiceWrapper) -> isize {
            if kernel_module::init_kernel_module(service, unsafe { &#name }).is_err() {
                return -1;
            }
            0
        }

        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            kernel_module::error!("{}", info);
            // TODO: Notify the module to release any locks.
            kernel_module::handle_panic();
        }
    };
    result.into()
}

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
                ::kernel_module::testing::register_test(&#name);
            }

            __test_function_wrapper
        };
    };
    result.into()
}
