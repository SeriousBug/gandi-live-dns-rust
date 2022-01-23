use crate::opts;
use anyhow;
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Entry {
    pub name: String,
    types: Option<Vec<String>>,
    fqdn: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    fqdn: String,
    pub api_key: String,
    pub entry: Vec<Entry>,
}

const DEFAULT_TYPES: &'static [&'static str] = &["A"];

impl Config {
    pub fn fqdn<'c>(entry: &'c Entry, config: &'c Config) -> &'c str {
        return entry.fqdn.as_ref().unwrap_or(&config.fqdn).as_str();
    }

    pub fn types<'e>(entry: &'e Entry) -> Vec<&'e str> {
        return entry
            .types
            .as_ref()
            .and_then(|ts| Some(ts.iter().map(|t| t.as_str()).collect()))
            .unwrap_or_else(|| DEFAULT_TYPES.to_vec());
    }
}

fn load_config_from<P: std::convert::AsRef<std::path::Path>>(path: P) -> anyhow::Result<Config> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

pub fn load_config(opts: &opts::Opts) -> anyhow::Result<Config> {
    match &opts.config {
        Some(config_path) => load_config_from(&config_path),
        None => {
            let confpath = ProjectDirs::from("me", "kaangenc", "gandi-dynamic-dns")
                .and_then(|dir| Some(PathBuf::from(dir.config_dir()).join("config.toml")));
            confpath.as_ref()
                .and_then(|path| {
                    Some(load_config_from(path))
                })
                .unwrap_or_else(|| {
                    let path = PathBuf::from(".").join("gandi.toml");
                    load_config_from(path)
                })
                .or_else(|_| {
                    match &confpath {
                        Some(path) => anyhow::bail!("Can't find the config file! Please create {}, or gandi.toml in the current directory.", path.to_string_lossy()),
                        None => anyhow::bail!("Can't find the config file! Please create gandi.toml in the current directory."),
                    }
                })
        }
    }
}

pub fn validate_config(config: &Config) -> anyhow::Result<()> {
    for entry in &config.entry {
        for entry_type in Config::types(&entry) {
            if entry_type != "A" && entry_type != "AAA" {
                anyhow::bail!("Entry {} has invalid type {}", entry.name, entry_type);
            }
        }
    }
    return Ok(());
}
