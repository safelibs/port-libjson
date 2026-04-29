#!/usr/bin/env bash
# libjson: drive the port-owned safe/tools/build-debs.sh, which uses
# apt cargo + rustc (declared as Build-Depends in safe/debian/control).
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=/dev/null
. "$repo_root/scripts/lib/build-deb-common.sh"

prepare_dist_dir "$repo_root"

bash "$repo_root/safe/tools/build-debs.sh" \
  --workspace "$repo_root" \
  --out "$repo_root/dist"
