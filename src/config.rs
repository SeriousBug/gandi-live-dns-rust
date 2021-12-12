use directories::ProjectDirs;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use crate::opts;

#[derive(Deserialize, Debug)]
pub struct Config {
    fqdn: String,
}

pub fn load_config(file: PathBuf) -> Result<Config, Box<dyn Error>> {
    let output = fs::read_to_string(file)?;
    let contents = output.as_str();

    let config = toml::from_str(contents)?;
    return Ok(config);
}

pub fn config_path(opts: &opts::Opts) -> PathBuf {
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
