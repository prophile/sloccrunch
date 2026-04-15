use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sloccrunch", about = "Count source lines of code in a directory tree")]
pub(crate) struct Cli {
    #[arg(default_value = ".")]
    pub(crate) path: PathBuf,
}
