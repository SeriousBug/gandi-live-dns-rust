#!/bin/bash
#
# Make sure `cross` is installed.
# You'll also need `sed`, and a relatively recent version of `tar`.
#
# This script runs does `sudo docker` to build and push the release to docker.
# If you have rootless docker set up, remove sudo from this variable.
DOCKER="sudo docker"
#
# Trap errors and interrupts
set -Eeuo pipefail
shopt -s extglob
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
VERSION=$(sed -nr 's/^version *= *"([0-9.]+)"/\1/p' Cargo.toml)

# Make the builds
for target in "${!TARGETS[@]}"; do
  echo Building "${target}"
  cross build -j $(($(nproc) / 2)) --release --target "${target}"
  tar -acf "gandi-live-dns.${VERSION}.${TARGETS[${target}]}.tar.xz" target/"${target}"/release/gandi-live-dns?(|.exe)
done

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
  --push