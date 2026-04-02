#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
safe_dir="${repo_root}/safe"
build_dir=${1:-"${safe_dir}/build"}
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

compile_and_run_probe() {
    local label=${1:?label is required}
    local header_dir=${2:?header dir is required}
    local config_dir=${3:?config dir is required}

    cc -I"${header_dir}" -I"${config_dir}" "${safe_dir}/tools/probe-layout.c" -o "${tmpdir}/${label}-probe"
    "${tmpdir}/${label}-probe" >"${tmpdir}/${label}.layout"
}

cc -I"${safe_dir}/include/json-c" -I"${build_dir}" \
   "${safe_dir}/tests/foundation/abi_layout.c" -o "${tmpdir}/abi-layout"
"${tmpdir}/abi-layout" >/dev/null

compile_and_run_probe candidate "${safe_dir}/include/json-c" "${build_dir}"
compile_and_run_probe buildcheck "${safe_dir}/include/json-c" "${repo_root}/build-check"
compile_and_run_probe original "${repo_root}/original" "${repo_root}/original/build"

diff -u "${tmpdir}/buildcheck.layout" "${tmpdir}/candidate.layout"
diff -u "${tmpdir}/original.layout" "${tmpdir}/candidate.layout"
