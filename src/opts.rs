use clap::Parser;

/// A tool to automatically update DNS entries on Gandi, using it as a dynamic DNS system.
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None, name = "gandi-live-dns")]
pub struct Opts {
    /// The path to the configuration file.
    #[clap(long)]
    pub config: Option<String>,
    /// Skip IPv4 updates.
    ///
    /// If enabled, any IPv4 (A) records in the configuration file are ignored.
    #[clap(action, long)]
    pub skip_ipv4: bool,
    /// Skip IPv4 updates.
    ///
    /// If enabled, any IPv6 (AAAA) records in the configuration file are ignored.
    #[clap(action, long)]
    pub skip_ipv6: bool,
}
