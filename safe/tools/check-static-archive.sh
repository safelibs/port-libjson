#!/usr/bin/env bash
set -euo pipefail

stage_root=${1:?usage: check-static-archive.sh <stage-root>}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
pkgconfig_dir="${stage_root}/lib/pkgconfig"
cmake_dir="${stage_root}/lib/cmake/json-c"
pc_file="${pkgconfig_dir}/json-c.pc"
targets_file="${cmake_dir}/json-c-targets.cmake"
archive="${stage_root}/lib/libjson-c.a"
harvested_flags_file="${repo_root}/safe/build/rust-native-static-libs.pcflags"
harvested_cmake_file="${repo_root}/safe/build/rust-native-static-libs.cmake"
baseline_archive="${repo_root}/build-check/libjson-c.a"
original_archive="${repo_root}/original/build/libjson-c.a"

[[ -f "${archive}" ]] || { echo "missing staged archive ${archive}" >&2; exit 1; }
[[ -f "${pc_file}" ]] || { echo "missing staged pkg-config file ${pc_file}" >&2; exit 1; }
[[ -f "${targets_file}" ]] || { echo "missing staged CMake targets ${targets_file}" >&2; exit 1; }

archive_has_text_symbol() {
    local archive_path=${1:?archive path is required}
    local symbol=${2:?symbol is required}
    local symbols

    symbols="$(nm --defined-only "${archive_path}" 2>/dev/null || true)"
    grep -Eq "(^|[[:space:]])T[[:space:]]+${symbol}\$" <<<"${symbols}"
}

if ! archive_has_text_symbol "${archive}" array_list_insert_idx; then
    echo "staged archive is missing array_list_insert_idx" >&2
    exit 1
fi
if ! archive_has_text_symbol "${baseline_archive}" array_list_insert_idx; then
    echo "build-check baseline archive is missing array_list_insert_idx" >&2
    exit 1
fi
if ! archive_has_text_symbol "${original_archive}" array_list_insert_idx; then
    echo "original baseline archive is missing array_list_insert_idx" >&2
    exit 1
fi

harvested_flags="$(<"${harvested_flags_file}")"
for token in ${harvested_flags}; do
    grep -F -- "${token}" "${pc_file}" >/dev/null || {
        printf 'pkg-config metadata is missing harvested flag %s\n' "${token}" >&2
        exit 1
    }
done

python3 - "${harvested_cmake_file}" "${targets_file}" <<'PY'
import pathlib
import re
import sys

cmake_vars = pathlib.Path(sys.argv[1]).read_text()
targets = pathlib.Path(sys.argv[2]).read_text()

match = re.search(r'set\(RUST_NATIVE_STATIC_LIBS "(.*)"\)', cmake_vars)
if not match:
    raise SystemExit("missing RUST_NATIVE_STATIC_LIBS in harvested cmake metadata")
names = [item for item in match.group(1).split(";") if item]
for name in names:
    if name not in targets:
        raise SystemExit(f"CMake target metadata is missing harvested library {name}")
PY

export PKG_CONFIG_PATH="${pkgconfig_dir}"
pkg_config_args=(
    --define-variable=prefix="${stage_root}"
    --define-variable=exec_prefix="${stage_root}"
    --define-variable=libdir="${stage_root}/lib"
    --define-variable=includedir="${stage_root}/include"
)

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

cat >"${tmpdir}/archive_probe.c" <<'EOF'
#include <json-c/arraylist.h>

int main(void)
{
    struct array_list *list = array_list_new(0);
    return array_list_insert_idx(list, 0, 0);
}
EOF

pkg_libs=()
for token in $(pkg-config "${pkg_config_args[@]}" --static --libs json-c); do
    if [[ "${token}" == "-ljson-c" ]]; then
        pkg_libs+=("-Wl,-Bstatic" "${token}" "-Wl,-Bdynamic")
    else
        pkg_libs+=("${token}")
    fi
done

cc $(pkg-config "${pkg_config_args[@]}" --cflags json-c) "${tmpdir}/archive_probe.c" "${pkg_libs[@]}" -o "${tmpdir}/pkg-probe"

cat >"${tmpdir}/CMakeLists.txt" <<EOF
cmake_minimum_required(VERSION 3.9)
project(json_c_static_probe LANGUAGES C)
find_package(json-c CONFIG REQUIRED PATHS "${stage_root}" NO_DEFAULT_PATH)
add_executable(cmake_probe archive_probe.c)
target_link_libraries(cmake_probe PRIVATE json-c::json-c-static)
EOF

cmake -S "${tmpdir}" -B "${tmpdir}/build"
cmake --build "${tmpdir}/build"
