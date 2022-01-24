## gandi-live-dns-rust

A program that can set the IP addresses for configured DNS entries in [Gandi](https://gandi.net)'s domain configuration.
Thanks to Gandi's [LiveDNS API](https://api.gandi.net/docs/livedns/), this creates a dynamic DNS system.

Inspired by [cavebeat's similar tool](https://github.com/cavebeat/gandi-live-dns),
which seems to be unmaintained at the time I'm writing this. I decided to rewrite it in Rust as a learning project.

This tool can update both IPv4 and IPv6 addresses for one or more domains and subdomains.
It's a "one-shot" tool that's then orchestrated with a systemd timer or cron.

## Usage

- Copy `example.toml` to `gandi.toml`
- Follow the instructions in the example config to get your API key and put it in the config
- Follow the examples in the config to set up which entries you want to update
- Use `cargo run` to build and run the program

> Warning!
> 
> This tool does not rate limit itself, or otherwise do anything that limits how often it sends changes to Gandi's servers.
> It's up to you to use the tool properly and avoid abusing Gandi's servers. The tool is one-shot, so all you have to do is
> to avoid running it too often.