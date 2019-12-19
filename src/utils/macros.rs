#[macro_export]
macro_rules! uninterruptable {
    ($e: expr) => {{
        use crate::arch::*;
        Target::Interrupt::uninterruptable(|| {
            $e
        })
    }};
}

fn test() -> i32 {
    uninterruptable! {{
        let y = 23;
        y
    }}
}