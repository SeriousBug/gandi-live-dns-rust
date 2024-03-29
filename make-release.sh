#!/bin/bash
#
# Make sure `cross` is installed.
# You'll also need `sed`, a relatively recent version of `tar`, and `7z`.
#
DOCKER="docker"
#
shopt -s extglob
# Trap errors and interrupts
set -Eeuo pipefail
function handle_sigint() {
  echo "SIGINT, exiting..."
  exit 1
}
trap handle_sigint SIGINT
function handle_err() {
  echo "Error in run.sh!" 1>&2
  echo "$(caller): ${BASH_COMMAND}" 1>&2
  echo "Exiting..."
  exit 2
}
trap handle_err ERR

# Go to the root of the project
SCRIPT=$(realpath "${0}")
SCRIPTPATH=$(dirname "${SCRIPT}")
cd "${SCRIPTPATH}" || exit 12

declare -A TARGETS=(
  ['x86_64-unknown-linux-musl']='linux-x86_64'
  ['x86_64-pc-windows-gnu']='windows-x86_64'
  ['aarch64-unknown-linux-musl']='linux-arm64'
  ['armv7-unknown-linux-musleabihf']='linux-armv7'
  ['arm-unknown-linux-musleabihf']='linux-armv6'
)

declare -A DOCKER_TARGETS=(
  ['x86_64-unknown-linux-musl']='linux/amd64'
  ['aarch64-unknown-linux-musl']='linux/arm64'
  ['armv7-unknown-linux-musleabihf']='linux/arm/v7'
  ['arm-unknown-linux-musleabihf']='linux/arm/v6'
)

# Get the version number
VERSION=$(sed -nr 's/^version *= *"([0-9.]+)"/\1/p' Cargo.toml | head --lines=1)

# Make the builds
for target in "${!TARGETS[@]}"; do
  echo Building "${target}"
  # Keeping the cached builds seem to be breaking things when going between targets
  # This wouldn't be a problem if these were running in a matrix on the CI...
  rm -rf target/release/
  cross build -j $(($(nproc) / 2)) --release --target "${target}"
  if [[ "${target}" =~ .*"windows".* ]]; then
    zip -j "gandi-live-dns.${VERSION}.${TARGETS[${target}]}.zip" target/"${target}"/release/gandi-live-dns.exe 1>/dev/null
  else
    tar -acf "gandi-live-dns.${VERSION}.${TARGETS[${target}]}.tar.xz" -C "target/${target}/release/" "gandi-live-dns"
  fi
done

if [[ "$#" -ge 2 && "$1" = "--no-docker" ]]; then
  echo "Exiting without releasing to docker"
  exit 0
fi

# Copy files into place so Docker can get them easily
cd Docker
echo Building Docker images
mkdir -p binaries
for target in "${!DOCKER_TARGETS[@]}"; do
  mkdir -p "binaries/${DOCKER_TARGETS[${target}]}"
  cp ../target/"${target}"/release/gandi-live-dns?(|.exe) "binaries/${DOCKER_TARGETS[${target}]}/gandi-live-dns"
done

${DOCKER} buildx build . \
  --platform=linux/amd64,linux/arm64,linux/arm/v6,linux/arm/v7 \
  --file "Dockerfile" \
  --tag "seriousbug/gandi-live-dns-rust:latest" \
  --tag "seriousbug/gandi-live-dns-rust:${VERSION}" \
  --tag "ghcr.io/seriousbug/gandi-live-dns-rust:latest" \
  --tag "ghcr.io/seriousbug/gandi-live-dns-rust:${VERSION}" \
  --push
