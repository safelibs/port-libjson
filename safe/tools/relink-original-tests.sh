#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
original_tests_dir="${repo_root}/original/build/tests"
build_dir="${repo_root}/safe/build"
safe_lib=""
out_dir=""
tests_csv=""
positional=()

usage() {
    cat <<'EOF'
usage: relink-original-tests.sh [safe-lib [out-dir]]
       relink-original-tests.sh [--build <build-dir>] [--safe-lib <libjson-c.so.5.3.0>] [--out <out-dir>] [--tests <comma-separated-tests>]
EOF
}

while (($#)); do
    case "$1" in
        --build)
            build_dir=$2
            shift 2
            ;;
        --safe-lib)
            safe_lib=$2
            shift 2
            ;;
        --out)
            out_dir=$2
            shift 2
            ;;
        --tests)
            tests_csv=$2
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        --)
            shift
            while (($#)); do
                positional+=("$1")
                shift
            done
            ;;
        -*)
            printf 'unknown option: %s\n' "$1" >&2
            usage >&2
            exit 1
            ;;
        *)
            positional+=("$1")
            shift
            ;;
    esac
done

if ((${#positional[@]} >= 1)) && [[ -z "${safe_lib}" ]]; then
    safe_lib=${positional[0]}
fi
if ((${#positional[@]} >= 2)) && [[ -z "${out_dir}" ]]; then
    out_dir=${positional[1]}
fi

safe_lib=${safe_lib:-"${build_dir}/libjson-c.so.5.3.0"}
out_dir=${out_dir:-"${build_dir}/relinked-tests"}

if [[ "${build_dir}" != /* ]]; then
    build_dir="${repo_root}/${build_dir}"
fi
if [[ "${safe_lib}" != /* ]]; then
    safe_lib="${repo_root}/${safe_lib}"
fi
if [[ "${out_dir}" != /* ]]; then
    out_dir="${repo_root}/${out_dir}"
fi

mkdir -p "${out_dir}"
safe_lib_dir="$(cd "$(dirname "${safe_lib}")" && pwd)"

python3 - "${original_tests_dir}" "${safe_lib}" "${safe_lib_dir}" "${out_dir}" "${tests_csv}" <<'PY'
import pathlib
import subprocess
import sys

tests_dir = pathlib.Path(sys.argv[1])
safe_lib = pathlib.Path(sys.argv[2])
safe_lib_dir = pathlib.Path(sys.argv[3])
out_dir = pathlib.Path(sys.argv[4])
selected = {name for name in sys.argv[5].split(",") if name}

for link_txt in sorted(tests_dir.glob("CMakeFiles/*.dir/link.txt")):
    test_name = link_txt.parent.name[:-4]
    if selected and test_name not in selected:
        continue
    command = link_txt.read_text().strip()
    command = command.replace("../libjson-c.so.5.3.0", str(safe_lib))
    command = command.replace("-Wl,-rpath," + str(tests_dir.parent), "-Wl,-rpath," + str(safe_lib_dir))
    command = command.replace(f"-o {test_name} ", f"-o {out_dir / test_name} ")
    subprocess.run(command, shell=True, check=True, cwd=tests_dir)
PY
