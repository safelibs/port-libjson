#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
original_tests_dir="${repo_root}/original/build/tests"
build_dir="${repo_root}/safe/build"
safe_lib=""
out_dir=""
tests_csv=""
relink_all=0
positional=()

usage() {
    cat <<'EOF'
usage: relink-original-tests.sh [safe-lib [out-dir]]
       relink-original-tests.sh [--build <build-dir>] [--safe-lib <libjson-c.so.5.3.0>] [--out <out-dir>] [--all] [--tests <comma-separated-tests>]
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
        --all)
            relink_all=1
            shift
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

python3 - "${original_tests_dir}" "${safe_lib}" "${safe_lib_dir}" "${out_dir}" "${tests_csv}" "${relink_all}" <<'PY'
import pathlib
import shlex
import subprocess
import sys

tests_dir = pathlib.Path(sys.argv[1])
safe_lib = pathlib.Path(sys.argv[2])
safe_lib_dir = pathlib.Path(sys.argv[3])
out_dir = pathlib.Path(sys.argv[4])
requested = {name for name in sys.argv[5].split(",") if name}
relink_all = sys.argv[6] == "1"

companions = {
    "test1": {"test1", "test1Formatted"},
    "test2": {"test2", "test2Formatted"},
}

selected = set()
for name in requested:
    selected.update(companions.get(name, {name}))

if not relink_all and not selected:
    relink_all = True

for link_txt in sorted(tests_dir.glob("CMakeFiles/*.dir/link.txt")):
    test_name = link_txt.parent.name[:-4]
    if not relink_all and test_name not in selected:
        continue
    command = shlex.split(link_txt.read_text().strip())

    rewritten = []
    idx = 0
    while idx < len(command):
        arg = command[idx]
        if arg == "../libjson-c.so.5.3.0":
            rewritten.append(str(safe_lib))
        elif arg.startswith("-Wl,-rpath,"):
            rewritten.append("-Wl,-rpath," + str(safe_lib_dir))
        elif arg == "-o" and idx + 1 < len(command):
            rewritten.extend(["-o", str(out_dir / test_name)])
            idx += 1
        else:
            rewritten.append(arg)
        idx += 1

    subprocess.run(rewritten, check=True, cwd=tests_dir)
PY
