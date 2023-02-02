## Gandi Live Dns Rust <!-- omit in toc -->

<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->

[![Contributors](https://img.shields.io/badge/all_contributors-1-orange.svg?label=contributors)](#contributors) <!-- ALL-CONTRIBUTORS-BADGE:END -->
[![tests](https://img.shields.io/github/actions/workflow/status/SeriousBug/gandi-live-dns-rust/test.yml?label=tests&branch=master)](https://github.com/SeriousBug/gandi-live-dns-rust/actions/workflows/test.yml)
[![Test coverage report](https://img.shields.io/codecov/c/github/SeriousBug/gandi-live-dns-rust)](https://codecov.io/gh/SeriousBug/gandi-live-dns-rust)
[![lint checks](https://img.shields.io/github/actions/workflow/status/SeriousBug/gandi-live-dns-rust/lint.yml?label=lints&branch=master)](https://github.com/SeriousBug/gandi-live-dns-rust/actions/workflows/lint.yml)
[![Releases](https://img.shields.io/github/v/release/SeriousBug/gandi-live-dns-rust?include_prereleases)](https://github.com/SeriousBug/gandi-live-dns-rust/releases)
[![Docker Image Size](https://img.shields.io/docker/image-size/seriousbug/gandi-live-dns-rust)](https://hub.docker.com/r/seriousbug/gandi-live-dns-rust)
[![MIT license](https://img.shields.io/github/license/SeriousBug/gandi-live-dns-rust)](https://github.com/SeriousBug/gandi-live-dns-rust/blob/master/LICENSE.txt)

A program that can set the IP addresses for configured DNS entries in
[Gandi](https://gandi.net)'s domain configuration. Thanks to Gandi's
[LiveDNS API](https://api.gandi.net/docs/livedns/),
this creates a dynamic DNS system.

If you want to host web services but you don't have a static IP address, this
tool will allow you to keep your domains pointed at the right IP address. This
program can update both IPv4 and IPv6 addresses for one or more domains and
subdomains. It can be used as a one-shot tool managed with a systemd timer
or cron, or a long-running process that reschedules itself.

## Table of Contents <!-- omit in toc -->

- [Usage](#usage)
  - [System packages](#system-packages)
  - [Prebuilt binaries](#prebuilt-binaries)
  - [With docker](#with-docker)
  - [From source](#from-source)
- [Automation](#automation)
  - [By running as a background process](#by-running-as-a-background-process)
    - [Skipped updates](#skipped-updates)
  - [With a Systemd timer](#with-a-systemd-timer)
- [Development](#development)
  - [Local builds](#local-builds)
  - [Making a release](#making-a-release)
- [Alternatives](#alternatives)
- [Contributors](#contributors)

## Usage

The Gandi Live DNS API is rate limited at 30 requests per minute. This program
respects this rate limit: if you have more than 30 domains to update, the
program will pause and wait for a minute, plus a random delay to ensure it
doesn't hit the rate limit.

### System packages

Packages are available for some linux distributions.

- ArchLinux: [gandi-live-dns-rust on AUR](https://aur.archlinux.org/packages/gandi-live-dns-rust/)

> Contributions to release this for other distributions are welcome!

### Prebuilt binaries

`gandi-live-dns` provides pre-built binaries with the releases. See the
[releases page](https://github.com/SeriousBug/gandi-live-dns-rust/releases) to
get the latest version. These binaries are statically linked, and provided for
both Linux and Windows, including ARM architectures for the Linux version.

Download the latest version from the releases page, extract it from the archive, and place it somewhere in your `$PATH` to use it.

- Create a file `gandi.toml`, then copy and paste the contents of [`example.toml`](https://raw.githubusercontent.com/SeriousBug/gandi-live-dns-rust/master/example.toml)
- Follow the instructions in the example config to get your API key and put it in the config
- Follow the examples in the config to set up the entries you want to update
- Run `gandi-live-dns` inside the directory with the configration to update your DNS entries

### With docker

Use the [seriousbug/gandi-live-dns-rust](https://hub.docker.com/r/seriousbug/gandi-live-dns-rust) Docker images, which are available for x86_64,
arm64, armv6, and armv7 platforms. Follow the steps below to use these images.

- Create a file `gandi.toml`, then copy and paste the contents of [`example.toml`](https://raw.githubusercontent.com/SeriousBug/gandi-live-dns-rust/master/example.toml)
- Follow the instructions in the example config to get your API key and put it in the config
- Follow the examples in the config to set up the entries you want to update
- Run `docker run --rm -it -v $(pwd)/gandi.toml:/gandi.toml:ro seriousbug/gandi-live-dns-rust:latest`

> Docker doesn't [support IPv6](https://docs.docker.com/config/daemon/ipv6/) out
> of the box. If you need to update IPv6 addresses, check the linked page to enable IPv6 or use the prebuilt binaries directly.

> If you get [errors](https://stackoverflow.com/questions/42248198/how-to-mount-a-single-file-in-a-volume) about not finding the config file, make sure your command
> has a full path to the config file (`$(pwd)/gandi.toml` part). Otherwise
> Docker will create a directory.

### From source

This package is also published on `crates.io` as
[gandi-live-dns](https://crates.io/crates/gandi-live-dns). If you would like to
build it from source and you have a working rust install, you can use `cargo install gandi-live-dns` to build and install it.

## Automation

### By running as a background process

`gandi-live-dns` can run as a daemon, a background process, periodically perform
the IP address updates. To do so, add the `--repeat=<delay-in-seconds>` command
line option. When given, this tool will not quit after updating your IP address
and instead will continue to perform periodic updates.

If you are using Docker, you can add this option when starting it:

```bash
# This will update your IP now, then repeat every 24 hours
docker run --rm -it -v $(pwd)/gandi.toml:/gandi.toml:ro seriousbug/gandi-live-dns-rust:latest --repeat=86400
```

Or with a `docker-compose.yml` file, add it in the arguments:

```yml
gandi-live-dns:
  image: seriousbug/gandi-live-dns-rust:latest
  restart: always
  volumes:
    - ./gandi.toml:/gandi.toml:ro
  # Repeat the update every day
  command: --repeat=86400
```

#### Skipped updates

In background process mode, the tool will avoid sending an update to Gandi if
your IP address has not changed since the last update. This only works so long
as the tool continues to run, it will send an update when restarted even if your
IP address has not changed. You can also override this behavior by adding
`always_update = true` to the top of your config file.

### With a Systemd timer

The `Packaging` folder contains a Systemd service and timer, which you can use
to automatically run this tool. By default it will update the IP addresses after
every boot up, and at least once a day. You can adjust the timer to speed this
up, but avoid unnecessarily overloading Gandi's servers.

- Place `gandi-live-dns.timer` and `gandi-live-dns.service` into `/etc/systemd/system`
- Put `gandi-live-dns` binary into `/usr/bin/`
  - You can also place it in `/usr/local/bin` or some other directory, just make sure to update the path in the service file
- Create the folder `/etc/gandi-live-dns`, and place your `gandi.toml` into it
- Create a user for the service: `useradd --system gandi-live-dns --home-dir /etc/gandi-live-dns`
- Make sure only the service can access the config file: `chown gandi-live-dns: /etc/gandi-live-dns/gandi.toml && chmod 600 /etc/gandi-live-dns/gandi.toml`
- Enable the timer with `systemctl enable --now gandi-live-dns.timer`

## Development

### Local builds

`cargo build` and `cargo build --release` are sufficient for development and release builds.
No special instructions are needed.

### Making a release

To make a release, first set up `cross` and `docker`. Make sure you log into
Docker with `docker login`. Then follow these steps:

- bump up the version in `Cargo.toml` according to [semver](https://semver.org/)
  - commit and push the changes
- run `./make-release.sh`
  > This will build binaries, then package them into archives, as well as
  > build and upload docker images.
- Create a release on Github
  - Make sure to create a tag for the release version on `master`
  - Upload the binary archives to the Github release
- Update the AUR version
- Run `cargo publish` to update the crates.io version

## Alternatives

- [laur89's Bash based updater](https://github.com/laur89/docker-gandi-dns-update)
- [ Adam Vigneaux's Bash based updater, with a docker image](https://github.com/AdamVig/gandi-dynamic-dns)
- [Yago Riveiro's Python based updater](https://github.com/yriveiro/giu)
- [ Maxime Le Conte des Floris' Go based updater](https://github.com/mlcdf/dyndns)

## Contributors

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center"><a href="https://github.com/jannikac"><img src="https://avatars.githubusercontent.com/u/21014142?v=4?s=100" width="100px;" alt="jannikac"/><br /><sub><b>jannikac</b></sub></a><br /><a href="https://github.com/SeriousBug/gandi-live-dns-rust/commits?author=jannikac" title="Code">💻</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->
