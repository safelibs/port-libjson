#!/usr/bin/env bash
set -euo pipefail

src_dir=${1:?usage: collect-native-static-libs.sh <src-dir> <build-dir> <target-dir> <profile>}
build_dir=${2:?usage: collect-native-static-libs.sh <src-dir> <build-dir> <target-dir> <profile>}
target_dir=${3:?usage: collect-native-static-libs.sh <src-dir> <build-dir> <target-dir> <profile>}
profile=${4:?usage: collect-native-static-libs.sh <src-dir> <build-dir> <target-dir> <profile>}
cargo_lock_backup=""

cleanup() {
    if [[ -n "${cargo_lock_backup}" && -f "${cargo_lock_backup}" ]]; then
        mv -f "${cargo_lock_backup}" "${src_dir}/Cargo.lock"
    fi
}

trap cleanup EXIT

resolve_rust_tool() {
    local env_name=$1
    local system_path=$2
    local tool_name=$3
    local candidate=""

    if [[ -n "${!env_name:-}" ]]; then
        candidate="${!env_name}"
        if "${candidate}" --version >/dev/null 2>&1; then
            printf '%s\n' "${candidate}"
            return 0
        fi
    fi

    if [[ -x "${system_path}" ]] && "${system_path}" --version >/dev/null 2>&1; then
        printf '%s\n' "${system_path}"
        return 0
    fi

    candidate="$(command -v "${tool_name}" || true)"
    if [[ -n "${candidate}" ]] && "${candidate}" --version >/dev/null 2>&1; then
        printf '%s\n' "${candidate}"
        return 0
    fi

    printf 'unable to find a working %s executable\n' "${tool_name}" >&2
    return 1
}

mkdir -p "${build_dir}" "${target_dir}"

cargo_bin="$(resolve_rust_tool CARGO /usr/bin/cargo cargo)"
rustc_bin="$(resolve_rust_tool RUSTC /usr/bin/rustc rustc)"

cargo_cmd=("${cargo_bin}" rustc --manifest-path "${src_dir}/Cargo.toml" --lib --target-dir "${target_dir}")
profile_dir=${profile}
case "${profile}" in
    debug)
        ;;
    release)
        cargo_cmd+=(--release)
        ;;
    *)
        cargo_cmd+=(--profile "${profile}")
        ;;
esac
cargo_cmd+=(-- -C relocation-model=pic --print native-static-libs)

# This crate has no external dependencies. A stray ignored Cargo.lock produced
# by a newer toolchain can still break older cargo versions, so build without it.
if [[ -f "${src_dir}/Cargo.lock" ]]; then
    cargo_lock_backup="${src_dir}/Cargo.lock.collect-native-static-libs-backup.$$"
    while [[ -e "${cargo_lock_backup}" ]]; do
        cargo_lock_backup="${cargo_lock_backup}x"
    done
    mv "${src_dir}/Cargo.lock" "${cargo_lock_backup}"
fi

set +e
output="$(RUSTC="${rustc_bin}" "${cargo_cmd[@]}" 2>&1)"
cargo_status=$?
set -e
printf '%s\n' "${output}" >"${build_dir}/rust-native-static-libs.txt"
if [[ ${cargo_status} -ne 0 ]]; then
    printf '%s\n' "${output}" >&2
    exit "${cargo_status}"
fi

native_flags="$(printf '%s\n' "${output}" | sed -n 's/^note: native-static-libs: //p' | tail -n 1)"

archive_dir="${target_dir}/${profile_dir}"
if [[ -n "${CARGO_BUILD_TARGET:-}" ]]; then
    archive_dir="${target_dir}/${CARGO_BUILD_TARGET}/${profile_dir}"
fi
archive="${archive_dir}/libjson_c.a"
if [[ ! -f "${archive}" ]]; then
    printf 'expected Rust static library missing: %s\n' "${archive}" >&2
    exit 1
fi

native_lib_names=()
for token in ${native_flags}; do
    case "${token}" in
        -l*)
            native_lib_names+=("${token#-l}")
            ;;
        -pthread)
            native_lib_names+=("pthread")
            ;;
    esac
done

cmake_list=""
if ((${#native_lib_names[@]})); then
    cmake_list="$(IFS=';'; echo "${native_lib_names[*]}")"
fi

printf '%s\n' "${native_flags}" >"${build_dir}/rust-native-static-libs.pcflags"
printf '%s\n' "${cmake_list}" >"${build_dir}/rust-native-static-libs.names"

cat >"${build_dir}/rust-native-static-libs.cmake" <<EOF
set(RUST_STATICLIB "${archive}")
set(RUST_NATIVE_STATIC_PCFLAGS "${native_flags}")
set(RUST_NATIVE_STATIC_LIBS "${cmake_list}")
EOF
