## gandi-live-dns-rust

A program that can set the IP addresses for configured DNS entries in
[Gandi](https://gandi.net)'s domain configuration. Thanks to Gandi's
[LiveDNS API](https://api.gandi.net/docs/livedns/),
this creates a dynamic DNS system.

If you want to host web services but you don't have a static IP address, this
tool will allow you to keep your domains pointed at the right IP address. This
program can update both IPv4 and IPv6 addresses for one or more domains and
subdomains. It's a one-shot tool that's meant to be managed with a systemd timer
or cron.

Inspired by [cavebeat's similar tool](https://github.com/cavebeat/gandi-live-dns),
which seems to be unmaintained at the time I'm writing this. I decided to rewrite
it in Rust as a learning project.

## Usage

> This tool doesn't rate limit itself at the moment. If you have more than 30
> entries that need to be updated, the operation may hit rate the limit of Gandi
> and fail. You can work around this using multiple config files and waiting at
> least 1 minute between runs.

### Prebuilt binaries

`gandi-live-dns-rust` provides pre-built binaries with the releases. See the
[releases page](https://github.com/SeriousBug/gandi-live-dns-rust/releases) to
get the latest version. These binaries are statically linked, and provided for
both Linux and Windows, including ARM architectures for the Linux version.

Download the latest version from the releases page, extract it from the archive, and place it somewhere in your `$PATH` to use it.

- Create a file `gandi.toml`, then copy and paste the contents of [`example.toml`](https://raw.githubusercontent.com/SeriousBug/gandi-live-dns-rust/master/example.toml)
- Follow the instructions in the example config to get your API key and put it in the config
- Follow the examples in the config to set up the entries you want to update
- Run `gandi-live-dns` inside the directory with the configration to update your DNS entries

### With docker

`gandi-live-dns-rust` has Docker images available for x86_64, arm64, armv6, and armv7 platforms.
Follow the steps below to use these images.

- Create a file `gandi.toml`, then copy and paste the contents of [`example.toml`](https://raw.githubusercontent.com/SeriousBug/gandi-live-dns-rust/master/example.toml)
- Follow the instructions in the example config to get your API key and put it in the config
- Follow the examples in the config to set up the entries you want to update
- Run `docker run --rm -it -v $(pwd)/gandi.toml:/gandi.toml:ro seriousbug/gandi-live-dns-rust:latest`

> Docker doesn't [support IPv6](https://docs.docker.com/config/daemon/ipv6/) out
> of the box. Check the linked page to enable it, or use the native option.

> If you get [errors](https://stackoverflow.com/questions/42248198/how-to-mount-a-single-file-in-a-volume) about not finding the config file, make sure your command
> has a full path to the config file (`$(pwd)/gandi.toml` part). Otherwise
> Docker will create a directory.

## Automation

The `Packaging` folder contains a Systemd service and timer, which you can use
to automatically run this tool. By default it will update the IP addresses after
every boot up, and at least once a day. You can adjust the timer to speed this
up, but avoid unnecessarily overloading Gandi's servers.

## Development

### Local builds

`cargo build` and `cargo build --release` are sufficient for development and release builds.
No special instructions are needed.

### Making a release

To make a release, first set up `cross` and `docker`. Make sure you log into
Docker with `docker login`. Then follow these steps:

- bump up the version in `Cargo.toml` according to [semver](https://semver.org/)
- run `./make-release.sh`
    > This will build binaries, then package them into archives, as well as
    > build and upload docker images.
- Create a release on Github
    - Make sure to create a tag for the release version on `master`
    - Upload the binary archives to the Github release
