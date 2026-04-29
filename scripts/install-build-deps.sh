#!/usr/bin/env bash
# Install apt packages and a stable rust toolchain for libjson's safe
# build. The port's own safe/tools/build-debs.sh drives cmake+cargo.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  cmake \
  curl \
  debhelper \
  devscripts \
  dpkg-dev \
  equivs \
  fakeroot \
  file \
  git \
  jq \
  pkg-config \
  python3 \
  rsync \
  xz-utils

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path

# shellcheck source=/dev/null
. "$HOME/.cargo/env"
rustup default stable
rustc --version
cargo --version

if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$HOME/.cargo/bin" >> "$GITHUB_PATH"
fi
