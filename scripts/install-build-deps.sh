#!/usr/bin/env bash
# Install apt packages for libjson's safe build. The port's
# safe/debian/control declares apt cargo + rustc as Build-Depends, and
# safe/tools/build-debs.sh runs dpkg-buildpackage which enforces that
# — so we install them as apt packages rather than via rustup.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  cargo \
  cmake \
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
  rustc \
  xz-utils
