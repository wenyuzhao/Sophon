use super::ipc::Message;

pub trait Driver {
    fn new() -> Self;
    fn handle_message(&mut self, m: &Message);
}



#[macro_export]
macro_rules! driver_entry {
    ($driver: ty) => {
        #[no_mangle]
        pub extern fn _start(_argc: isize, _argv: *const *const u8) -> isize {
            let mut driver = <$driver as $crate::driver::Driver>::new();
            loop {
                let m = Message::receive(None);
                driver.handle_message(&m);
            }
        }
    };
}