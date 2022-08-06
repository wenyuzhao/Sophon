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
        pub extern "C" fn _start(service: kernel_module::KernelServiceWrapper) -> isize {
            if kernel_module::init_kernel_module(service, unsafe { &#name }).is_err() {
                return -1;
            }
            0
        }

        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            kernel_module::log!("{}", info);
            // TODO: Notify the module to release any locks.
            kernel_module::handle_panic();
        }
    };
    result.into()
}
