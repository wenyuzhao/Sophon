use xshell::Shell;

use crate::util::ShellExt;

#[derive(Parser)]
pub struct Disassemble {
    /// Binary name to disassemble.
    file: String,
}

impl Disassemble {
    pub fn run(&self, shell: &Shell) {
        shell.disassemble(
            format!("./target/_out/{}", &self.file),
            format!("./target/_out/{}.s", &self.file),
        )
    }
}
