use clap::Parser;
use std::path::PathBuf;

use crate::runtime::{self};

#[derive(Parser)]
#[clap(name = "lv8", version = env!("CARGO_PKG_VERSION"), about = "lv8 is a WebAssembly runtime")]
pub struct Cli {
    pub wasmfile_path: PathBuf,
}

pub fn run() {
    let args = Cli::parse();
    runtime::run(&args);
}
