#!/usr/bin/env bash
# libjson: drive the port-owned safe/tools/build-debs.sh.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist"

# shellcheck source=/dev/null
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

if [[ -d "$HOME/.cargo/bin" ]]; then
  case ":$PATH:" in
    *":$HOME/.cargo/bin:"*) ;;
    *) export PATH="$HOME/.cargo/bin:$PATH" ;;
  esac
fi

rm -rf -- "$dist_dir"
mkdir -p -- "$dist_dir"

bash "$repo_root/safe/tools/build-debs.sh" \
  --workspace "$repo_root" \
  --out "$dist_dir"
