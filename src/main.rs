use serde::Deserialize;
use std::error::Error;
use std::fs;

#[derive(Deserialize, Debug)]
struct Config {
    fqdn: String,
}

fn gandi_api(fqdn: &String) {
    return format!("https://api.gandi.net/v5/livedns/domains/{}/records", fqdn);
}

fn load_config(file: &String) -> Result<Config, Box<dyn Error>> {
    let contents = fs::read_to_string(file)?.as_str();

    let config = toml::from_str(contents)?;
    return Ok(config);
}

fn main() {
    println!("Hello, world!");
}
