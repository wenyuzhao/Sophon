use crate::util;

#[derive(Clap)]
pub struct Disassemble {
    /// Binary name to disassemble.
    file: String,
}

impl Disassemble {
    fn disassemble(&self, filename: &str) {
        util::disassemble(
            format!("./target/_out/{}", filename),
            format!("./target/_out/{}.s", filename),
        )
    }

    pub fn run(&self) {
        self.disassemble(&self.file);
    }
}
