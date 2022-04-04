use clap::{Parser, ArgEnum};

/// A tool to automatically update DNS entries on Gandi, using it as a dynamic DNS system.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, name = "gandi-live-dns")]
pub struct Opts {
    /// The path to the configuration file.
    #[clap(long)]
    pub config: Option<String>,

}
