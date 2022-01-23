## gandi-live-dns-rust

A program that can set the IP addresses for configured DNS entries in [Gandi](https://gandi.net)'s domain configuration.
Thanks to Gandi's [LiveDNS API](https://api.gandi.net/docs/livedns/), this creates a dynamic DNS system.

Inspired by [cavebeat's similar tool](https://github.com/cavebeat/gandi-live-dns),
which seems to be unmaintained at the time I'm writing this. I decided to rewrite it in Rust as a learning project.

## Usage

- Copy `example.toml` to 