use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::runtime::{self};

#[derive(Parser)]
#[clap(
    name = "lv8",
    version = env!("CARGO_PKG_VERSION"),
    about = "lv8 is a WebAssembly runtime"
)]
pub struct Cli {
    pub wasmfile_path: PathBuf,

    /// Arguments after -- are passed to wasm module
    #[arg(trailing_var_arg = true)]
    wasm_args: Vec<String>,
}

pub fn run() -> Result<i32> {
    let args = Cli::parse();
    runtime::run(&args)
}
