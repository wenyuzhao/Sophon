use xshell::Shell;

#[derive(Parser)]
pub struct Clean {}

impl Clean {
    pub fn run(&self, _shell: &Shell) {
        panic!("Please run `cargo clean` instead.")
    }
}
