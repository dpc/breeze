use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "indexer", about = "Bitcoin Indexer")]
pub struct Opts {
    //Input files or directories.
    #[structopt(parse(from_os_str))]
    pub input: Option<PathBuf>,
}
