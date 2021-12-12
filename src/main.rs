use directories::ProjectDirs;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// A tool to automatically update DNS entries on Gandi, using it as a dynamic DNS system.
#[derive(StructOpt, Debug)]
#[structopt(name = "gandi-dynamic-dns")]
struct Opts {
    /// The path to the configuration file.
    #[structopt(long)]
    config: Option<String>,

    /// If set, it will only update the DNS once then exit.
    #[structopt(long)]
    oneshot: bool,
}

#[derive(Deserialize, Debug)]
struct Config {
    fqdn: String,
}

fn gandi_api(fqdn: &str) -> String {
    return format!("https://api.gandi.net/v5/livedns/domains/{}/records", fqdn);
}

fn load_config(file: PathBuf) -> Result<Config, Box<dyn Error>> {
    let output = fs::read_to_string(file)?;
    let contents = output.as_str();

    let config = toml::from_str(contents)?;
    return Ok(config);
}

fn config_path(opts: &Opts) -> PathBuf {
    return opts
        .config
        .as_ref()
        .and_then(|conf| Some(PathBuf::from(conf)))
        .unwrap_or(
            ProjectDirs::from("me", "kaangenc", "gandi-dynamic-dns")
                .and_then(|dir| Some(PathBuf::from(dir.config_dir())))
                .unwrap_or(PathBuf::from(".")),
        );
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::from_args();
    println!("{:#?}", opts);
    let conf_path = config_path(&opts);
    println!("{:#?}", conf_path);
    let conf = load_config(conf_path);
    println!("{:#?}", conf);
    println!("Hello, world!");

    return Ok(());
}
