use directories::ProjectDirs;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use crate::opts;

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

const DEFAULT_TYPES: Vec<&str> = vec!["A"];

impl Config {
    pub fn fqdn<'c>(entry: &'c Entry, config: &'c Config) -> &'c str {
        return entry.fqdn.as_ref().unwrap_or(&config.fqdn).as_str();
    }

    pub fn types<'e>(entry: &'e Entry) -> Vec<&'e str> {
        return entry.types.as_ref().and_then(
            |ts| Some(ts.iter().map(|t| t.as_str()).collect())
        ).unwrap_or(DEFAULT_TYPES);
    }
}

pub fn load_config(file: PathBuf) -> Result<Config, Box<dyn Error>> {
    let output = fs::read_to_string(file)?;
    let contents = output.as_str();

    let config = toml::from_str(contents)?;
    return Ok(config);
}

pub fn validate_config(config: &Config) -> Result<(), String> {
    for entry in &config.entry {
        for entry_type in Config::types(&entry) {
            if entry_type != "A" && entry_type != "AAA" {
                return Err(format!("Entry {} has invalid type {}", entry.name, entry_type));
            }
        }
    }
    return Ok(());
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
