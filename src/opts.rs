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
    /// Repeat after specified delay, in seconds.
    ///
    /// If enabled, this will continue to run and perform the updates
    /// periodically. The first update will happen immediately, and later
    /// updates will be delayed by this many seconds.
    ///
    /// This process will not fork, so you may need to use something like
    /// `nohup` to keep it running in the background.
    #[clap(long)]
    pub repeat: Option<u64>,
}
