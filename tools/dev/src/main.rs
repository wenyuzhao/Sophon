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

use clap::{AppSettings, Parser};
use std::path::Path;
use xshell::Shell;

/// Tools for sophon development and compilation.
#[derive(Parser)]
#[clap(name = "Sophon Build Tool", version, author = "Wenyu Zhao", setting = AppSettings::TrailingVarArg)]
struct Opts {
    #[clap(subcommand)]
    sub_command: SubCommand,
}

#[derive(Parser)]
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
    let shell = Shell::new().unwrap();
    let _p = shell.push_dir(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap(),
    );
    let opts: Opts = Opts::parse();
    match opts.sub_command {
        SubCommand::Build(t) => t.run(&shell, false),
        SubCommand::Run(t) => t.run(&shell),
        SubCommand::BuildInitFS(t) => t.run(&shell),
        SubCommand::Clean(t) => t.run(&shell),
        SubCommand::Disassemble(t) => t.run(&shell),
    }
}
