use clap::{ArgEnum, Parser};
use std::{fmt::Display, str::FromStr};
use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum SilenceLevel {
    All,
    Domains,
}

impl Display for SilenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SilenceLevel::All => "all",
            SilenceLevel::Domains => "domains",
        })
    }
}

#[derive(Debug, Clone, Error)]
pub struct SilenceLevelError {
    pub message: String,
}

impl FromStr for SilenceLevel {
    type Err = SilenceLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "all" => Ok(SilenceLevel::All),
            "domains" => Ok(SilenceLevel::Domains),
            option => Err(SilenceLevelError {
                message: format!("Bad option {}, should be `{}` or `{}`", option, SilenceLevel::All, SilenceLevel::Domains),
            }),
        }
    }
}

impl Display for SilenceLevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

/// A tool to automatically update DNS entries on Gandi, using it as a dynamic DNS system.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, name = "gandi-live-dns")]
pub struct Opts {
    /// The path to the configuration file.
    #[clap(long)]
    pub config: Option<String>,

    /// Limit how much information gets printed out. Set to `all` to disable all
    /// output (other than errors), or `domains` to disable printing the domain
    /// names that were updated.
    #[clap(long)]
    pub silent: Option<SilenceLevel>,
}
