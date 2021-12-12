use structopt::StructOpt;
use std::error::Error;
mod opts;
mod config;


fn gandi_api(fqdn: &str) -> String {
    return format!("https://api.gandi.net/v5/livedns/domains/{}/records", fqdn);
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = opts::Opts::from_args();
    println!("{:#?}", opts);
    let conf_path = config::config_path(&opts);
    println!("{:#?}", conf_path);
    let conf = config::load_config(conf_path);
    println!("{:#?}", conf);
    println!("Hello, world!");

    return Ok(());
}
