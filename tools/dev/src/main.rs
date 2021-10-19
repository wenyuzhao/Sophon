#![feature(const_btree_new)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate xshell;
extern crate alloc;

mod build;
mod build_initfs;
mod clean;
mod dis;
mod run;
mod util;

use clap::{AppSettings, Clap};
use std::path::Path;

#[derive(Clap)]
#[clap(name = "Sophon Build Tool", version = "0.1", author = "Wenyu Zhao", setting = AppSettings::TrailingVarArg)]
struct Opts {
    #[clap(subcommand)]
    sub_command: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Build the kernel
    #[clap(name = "build")]
    Build(build::Build),
    /// Run with QEMU
    #[clap(name = "run")]
    Run(run::Run),
    /// Build init.fs image
    #[clap(name = "build-initfs")]
    BuildInitFS(build_initfs::BuildInitFS),
    /// Cleanup the workspace
    #[clap(name = "clean")]
    Clean(clean::Clean),
    /// Disassemble executables under ./target/_out
    #[clap(name = "dis")]
    Disassemble(dis::Disassemble),
}

fn main() {
    let _p = xshell::pushd(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap(),
    );
    let opts: Opts = Opts::parse();
    match opts.sub_command {
        SubCommand::Build(t) => t.run(),
        SubCommand::Run(t) => t.run(),
        SubCommand::BuildInitFS(t) => t.run(),
        SubCommand::Clean(t) => t.run(),
        SubCommand::Disassemble(t) => t.run(),
    }
}
