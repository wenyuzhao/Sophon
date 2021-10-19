#[derive(Clap)]
pub struct Clean {}

impl Clean {
    pub fn run(&self) {
        panic!("Please run `cargo clean` instead.")
    }
}
