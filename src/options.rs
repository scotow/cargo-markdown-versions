use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Options {
    #[clap(long)]
    pub manifest_path: Option<PathBuf>,
    #[clap(short, long)]
    pub package: Option<String>,
    #[clap(short, long)]
    pub default_configuration: bool,
}
