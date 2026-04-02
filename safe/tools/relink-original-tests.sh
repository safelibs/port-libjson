#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
safe_lib=${1:-"${repo_root}/safe/build/libjson-c.so.5.3.0"}
out_dir=${2:-"${repo_root}/safe/build/relinked-tests"}
original_tests_dir="${repo_root}/original/build/tests"

mkdir -p "${out_dir}"
safe_lib_dir="$(cd "$(dirname "${safe_lib}")" && pwd)"

python3 - "${original_tests_dir}" "${safe_lib}" "${safe_lib_dir}" "${out_dir}" <<'PY'
import pathlib
import subprocess
import sys

tests_dir = pathlib.Path(sys.argv[1])
safe_lib = pathlib.Path(sys.argv[2])
safe_lib_dir = pathlib.Path(sys.argv[3])
out_dir = pathlib.Path(sys.argv[4])

for link_txt in sorted(tests_dir.glob("CMakeFiles/*.dir/link.txt")):
    test_name = link_txt.parent.name[:-4]
    command = link_txt.read_text().strip()
    command = command.replace("../libjson-c.so.5.3.0", str(safe_lib))
    command = command.replace("-Wl,-rpath," + str(tests_dir.parent), "-Wl,-rpath," + str(safe_lib_dir))
    command = command.replace(f"-o {test_name} ", f"-o {out_dir / test_name} ")
    subprocess.run(command, shell=True, check=True, cwd=tests_dir)
PY
