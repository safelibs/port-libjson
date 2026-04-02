#!/usr/bin/env bash
set -euo pipefail

candidate=${1:?usage: check-symbols.sh <candidate-so> <baseline-so>}
baseline=${2:?usage: check-symbols.sh <candidate-so> <baseline-so>}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
original_baseline="${repo_root}/original/build/libjson-c.so.5.3.0"

soname() {
    readelf -d "$1" | sed -n 's/.*SONAME.*\[\(.*\)\]/\1/p'
}

normalize_symbols() {
    nm -D --defined-only "$1" | awk '{print $2 "\t" $3}' | sort
}

candidate_soname="$(soname "${candidate}")"
baseline_soname="$(soname "${baseline}")"
original_soname="$(soname "${original_baseline}")"

[[ "${candidate_soname}" == "${baseline_soname}" ]] || {
    printf 'SONAME mismatch: %s vs %s\n' "${candidate_soname}" "${baseline_soname}" >&2
    exit 1
}
[[ "${candidate_soname}" == "${original_soname}" ]] || {
    printf 'SONAME mismatch against original baseline: %s vs %s\n' "${candidate_soname}" "${original_soname}" >&2
    exit 1
}

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

normalize_symbols "${candidate}" >"${tmpdir}/candidate.sym"
normalize_symbols "${baseline}" >"${tmpdir}/baseline.sym"
normalize_symbols "${original_baseline}" >"${tmpdir}/original.sym"

diff -u "${tmpdir}/baseline.sym" "${tmpdir}/candidate.sym"
diff -u "${tmpdir}/original.sym" "${tmpdir}/candidate.sym"
