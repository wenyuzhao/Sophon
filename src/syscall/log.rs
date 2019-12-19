
pub fn log(x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize {
    let string_pointer = x1 as *const &str;
    print!("{}", unsafe { *string_pointer });
    0
}
