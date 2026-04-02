#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
build_dir=${1:-"${repo_root}/safe/build"}
test_regex=${2:-}

ctest_args=(--test-dir "${build_dir}" --output-on-failure)
if [[ -n "${test_regex}" ]]; then
    ctest_args+=(-R "${test_regex}")
fi

ctest "${ctest_args[@]}"
