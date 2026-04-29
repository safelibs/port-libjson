#!/usr/bin/env bash
# libjson: drive the port-owned safe/tools/build-debs.sh.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist"

rm -rf -- "$dist_dir"
mkdir -p -- "$dist_dir"

bash "$repo_root/safe/tools/build-debs.sh" \
  --workspace "$repo_root" \
  --out "$dist_dir"
