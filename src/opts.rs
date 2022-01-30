use structopt::StructOpt;

/// A tool to automatically update DNS entries on Gandi, using it as a dynamic DNS system.
#[derive(StructOpt, Debug)]
#[structopt(name = "gandi-live-dns")]
pub struct Opts {
    /// The path to the configuration file.
    #[structopt(long)]
    pub config: Option<String>,

    /// If set, it will only update the DNS once then exit.
    #[structopt(long)]
    pub oneshot: bool,
}
