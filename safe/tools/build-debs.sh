#!/usr/bin/env bash
set -euo pipefail

workspace=""
out=""

usage() {
    cat <<'EOF'
usage: build-debs.sh --workspace <workspace-copy> --out <artifact-dir>
EOF
}

while (($#)); do
    case "$1" in
        --workspace)
            workspace=${2:?missing value for --workspace}
            shift 2
            ;;
        --out)
            out=${2:?missing value for --out}
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

[[ -n "${workspace}" ]] || { usage >&2; exit 1; }
[[ -n "${out}" ]] || { usage >&2; exit 1; }
[[ -d "${workspace}" ]] || { printf 'workspace does not exist: %s\n' "${workspace}" >&2; exit 1; }

workspace="$(cd "${workspace}" && pwd)"
mkdir -p "${out}"
out="$(cd "${out}" && pwd)"

[[ -d "${workspace}/safe/debian" ]] || {
    printf 'missing safe/debian under workspace %s\n' "${workspace}" >&2
    exit 1
}

touch "${workspace}/.build-debs-write-test"
rm -f "${workspace}/.build-debs-write-test"

rm -rf "${workspace}/debian" "${workspace}/build-deb"
mkdir -p "${workspace}/debian"
cp -a "${workspace}/safe/debian/." "${workspace}/debian/"

(
    cd "${workspace}"
    dpkg-buildpackage -rfakeroot -b -us -uc
)

version="$(dpkg-parsechangelog -l"${workspace}/debian/changelog" -SVersion)"
arch="$(dpkg-architecture -qDEB_HOST_ARCH)"
parent_dir="$(dirname "${workspace}")"

artifacts=(
    "${parent_dir}/libjson-c5_${version}_${arch}.deb"
    "${parent_dir}/libjson-c-dev_${version}_${arch}.deb"
    "${parent_dir}/json-c_${version}_${arch}.buildinfo"
    "${parent_dir}/json-c_${version}_${arch}.changes"
)

for artifact in "${artifacts[@]}"; do
    [[ -f "${artifact}" ]] || {
        printf 'missing build artifact: %s\n' "${artifact}" >&2
        exit 1
    }
    cp -f "${artifact}" "${out}/"
done

printf 'wrote Debian artifacts to %s\n' "${out}"
