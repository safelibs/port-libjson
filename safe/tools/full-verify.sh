#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
safe_dir="${repo_root}/safe"
build_dir="${safe_dir}/build"
stage_root="/tmp/libjson-safe-stage"
deb_out="${safe_dir}/out/debs"
deb_workspace=""

usage() {
    cat <<'EOF'
usage: full-verify.sh [--build-dir <safe/build>] [--stage-root </tmp/libjson-safe-stage>] [--deb-out <safe/out/debs>]
EOF
}

log_step() {
    printf '\n==> %s\n' "$1"
}

cleanup() {
    if [[ -n "${deb_workspace}" && -d "${deb_workspace}" ]]; then
        rm -rf "${deb_workspace}"
    fi
}

run_perf_smoke() {
    local candidate="${build_dir}/apps/json_parse"
    local baseline="${repo_root}/original/build/apps/json_parse"
    local input="${build_dir}/perf-smoke.json"

    if [[ ! -x "${candidate}" || ! -x "${baseline}" ]]; then
        printf 'skipping performance smoke: missing %s or %s\n' "${candidate}" "${baseline}"
        return 0
    fi

    log_step "Running performance smoke checks"
    python3 - "${input}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = {
    "items": [
        {
            "id": idx,
            "name": f"item-{idx}",
            "flags": [idx % 3, idx % 5, idx % 7],
            "meta": {"group": idx % 17, "enabled": idx % 2 == 0},
        }
        for idx in range(4000)
    ],
    "lookup": {f"key-{idx:04d}": idx for idx in range(4000)},
}
path.write_text(json.dumps(payload, separators=(",", ":")))
PY

    python3 - "${candidate}" "${baseline}" "${input}" <<'PY'
import statistics
import subprocess
import sys
import time

candidate, baseline, input_path = sys.argv[1:]

def measure(binary: str):
    samples = []
    for _ in range(3):
        start = time.perf_counter()
        subprocess.run(
            [binary, "-n", input_path],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        samples.append(time.perf_counter() - start)
    return samples

baseline_samples = measure(baseline)
candidate_samples = measure(candidate)
baseline_avg = statistics.fmean(baseline_samples)
candidate_avg = statistics.fmean(candidate_samples)
ratio = candidate_avg / baseline_avg if baseline_avg else float("inf")

print(
    "performance smoke:"
    f" baseline={baseline_avg:.4f}s"
    f" candidate={candidate_avg:.4f}s"
    f" ratio={ratio:.2f}x"
)

if ratio > 20.0:
    raise SystemExit(
        "candidate json_parse is more than 20x slower than the prepared original baseline"
    )
PY
}

trap cleanup EXIT

while (($#)); do
    case "$1" in
        --build-dir)
            build_dir="${2:?missing value for --build-dir}"
            shift 2
            ;;
        --stage-root)
            stage_root="${2:?missing value for --stage-root}"
            shift 2
            ;;
        --deb-out)
            deb_out="${2:?missing value for --deb-out}"
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            printf 'unknown option: %s\n' "$1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ "${build_dir}" != /* ]]; then
    build_dir="${repo_root}/${build_dir}"
fi
if [[ "${stage_root}" != /* ]]; then
    stage_root="${repo_root}/${stage_root}"
fi
if [[ "${deb_out}" != /* ]]; then
    deb_out="${repo_root}/${deb_out}"
fi

log_step "Configuring ${build_dir}"
rm -rf "${stage_root}"
cmake -S "${safe_dir}" -B "${build_dir}" \
    -DBUILD_TESTING=ON \
    -DCMAKE_BUILD_TYPE=Debug \
    -DCMAKE_INSTALL_PREFIX="${stage_root}" \
    -DCMAKE_INSTALL_LIBDIR=lib

log_step "Building ${build_dir}"
cmake --build "${build_dir}" -j"$(nproc)"

log_step "Installing into ${stage_root}"
cmake --install "${build_dir}"

log_step "Checking ABI layouts"
"${safe_dir}/tools/check-layout.sh" "${build_dir}"

log_step "Running CTest suite"
ctest --test-dir "${build_dir}" --output-on-failure -E '^verify_07_'

log_step "Auditing header ABI manifest coverage"
"${safe_dir}/tools/header-abi-audit.sh" \
    "${safe_dir}/abi/public-api-manifest.tsv" \
    "${stage_root}/include/json-c" \
    "${repo_root}/original/json-c.sym" \
    "${safe_dir}/debian/libjson-c5.symbols"

log_step "Checking shared-library symbols"
"${safe_dir}/tools/check-symbols.sh" \
    "${stage_root}/lib/libjson-c.so.5.3.0" \
    "${repo_root}/build-check/libjson-c.so.5.3.0"

log_step "Checking staged static archive metadata"
"${safe_dir}/tools/check-static-archive.sh" "${stage_root}"

log_step "Relinking all prepared original tests against the safe shared object"
"${safe_dir}/tools/relink-original-tests.sh" --build "${build_dir}" --all

log_step "Building Debian packages from a writable workspace copy"
deb_workspace="$(mktemp -d /tmp/libjson-safe-workspace.XXXXXX)"
rm -rf "${deb_out}"
mkdir -p "${deb_out}"
cp -a "${repo_root}/." "${deb_workspace}/"
"${safe_dir}/tools/build-debs.sh" \
    --workspace "${deb_workspace}" \
    --out "${deb_out}"

log_step "Installing the built packages in the Ubuntu 24.04 testbed"
"${repo_root}/test-original.sh" --mode safe --package-dir "${deb_out}"

log_step "Re-running the dedicated adversarial hash-collision test"
ctest --test-dir "${build_dir}" --output-on-failure -R '^hash_collision$'

run_perf_smoke

log_step "Full verification completed"
