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
    ttl: Option<u32>,
}

fn default_ttl() -> u32 {
    return 300;
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum IPSourceName {
    Ipify,
    Icanhazip,
}

impl Default for IPSourceName {
    fn default() -> Self {
        // Ipify was the first IP source gandi-live-dns had, before it supported
        // multiple sources. Keeping that as the default.
        Self::Ipify
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    fqdn: String,
    pub api_key: String,
    #[serde(default)]
    pub ip_source: IPSourceName,
    pub entry: Vec<Entry>,
    #[serde(default = "default_ttl")]
    pub ttl: u32,
}

const DEFAULT_TYPES: &'static [&'static str] = &["A"];

impl Config {
    pub fn fqdn<'c>(entry: &'c Entry, config: &'c Config) -> &'c str {
        entry.fqdn.as_ref().unwrap_or(&config.fqdn).as_str()
    }

    pub fn ttl(entry: &Entry, config: &Config) -> u32 {
        entry.ttl.unwrap_or(config.ttl)
    }

    pub fn types<'e>(entry: &'e Entry) -> Vec<&'e str> {
        entry
            .types
            .as_ref()
            .and_then(|ts| Some(ts.iter().map(|t| t.as_str()).collect()))
            .unwrap_or_else(|| DEFAULT_TYPES.to_vec())
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
                .and_then(|dir| Some(PathBuf::from(dir.config_dir()).join("config.toml")))
                .ok_or(anyhow::anyhow!("Can't find config directory"));
            confpath
                .and_then(|path| {
                    println!("Checking for config: {}", path.to_string_lossy());
                    load_config_from(path)
                })
                .or_else(|_| {
                    let path = PathBuf::from(".").join("gandi.toml");
                    println!("Checking for config: {}", path.to_string_lossy());
                    load_config_from(path)
                })
        }
    }
}

pub fn validate_config(config: &Config) -> anyhow::Result<()> {
    for entry in &config.entry {
        for entry_type in Config::types(&entry) {
            if entry_type != "A" && entry_type != "AAAA" {
                anyhow::bail!("Entry {} has invalid type {}", entry.name, entry_type);
            }
        }
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::load_config;
    use crate::{config::IPSourceName, opts::Opts};
    use std::{env::temp_dir, fs};

    #[test]
    fn load_config_test() {
        let mut temp = temp_dir().join("gandi-live-dns-test");
        fs::create_dir_all(&temp).expect("Failed to create test dir");
        temp.push("test-1.toml");
        fs::write(
            &temp,
            r#"
fqdn = "example.com"
api_key = "xxx"
ttl = 300

[[entry]]
name = "@"
"#,
        )
        .expect("Failed to write test config file");

        let opts = Opts {
            config: Some(temp.to_string_lossy().to_string()),
        };
        let conf = load_config(&opts).expect("Failed to load config file");

        assert_eq!(conf.fqdn, "example.com");
        assert_eq!(conf.api_key, "xxx");
        assert_eq!(conf.ttl, 300);
        assert_eq!(conf.entry.len(), 1);
        assert_eq!(conf.entry[0].name, "@");
        // default
        assert_eq!(conf.ip_source, IPSourceName::Ipify);
    }

    #[test]
    fn load_config_change_ip_source() {
        let mut temp = temp_dir().join("gandi-live-dns-test");
        fs::create_dir_all(&temp).expect("Failed to create test dir");
        temp.push("test-2.toml");
        fs::write(
            &temp,
            r#"
fqdn = "example.com"
api_key = "yyy"
ttl = 1200
ip_source = "Icanhazip"

[[entry]]
name = "www"

[[entry]]
name = "@"
"#,
        )
        .expect("Failed to write test config file");

        let opts = Opts {
            config: Some(temp.to_string_lossy().to_string()),
        };
        let conf = load_config(&opts).expect("Failed to load config file");

        assert_eq!(conf.fqdn, "example.com");
        assert_eq!(conf.api_key, "yyy");
        assert_eq!(conf.ttl, 1200);
        assert_eq!(conf.entry.len(), 2);
        assert_eq!(conf.entry[0].name, "www");
        assert_eq!(conf.entry[1].name, "@");
        assert_eq!(conf.ip_source, IPSourceName::Icanhazip);
    }
}
