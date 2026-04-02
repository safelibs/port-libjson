#!/usr/bin/env bash
set -euo pipefail

manifest=${1:?usage: header-abi-audit.sh <manifest> <header-dir> <version-script> <debian-symbols>}
header_dir=${2:?usage: header-abi-audit.sh <manifest> <header-dir> <version-script> <debian-symbols>}
version_script=${3:?usage: header-abi-audit.sh <manifest> <header-dir> <version-script> <debian-symbols>}
debian_symbols=${4:?usage: header-abi-audit.sh <manifest> <header-dir> <version-script> <debian-symbols>}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
original_build="${repo_root}/original/build"
build_check="${repo_root}/build-check"

python3 - "${manifest}" "${header_dir}" "${version_script}" "${debian_symbols}" "${original_build}" "${build_check}" <<'PY'
import csv
import pathlib
import re
import sys

manifest_path = pathlib.Path(sys.argv[1])
header_dir = pathlib.Path(sys.argv[2])
version_script = pathlib.Path(sys.argv[3])
debian_symbols = pathlib.Path(sys.argv[4])
original_build = pathlib.Path(sys.argv[5])
build_check = pathlib.Path(sys.argv[6])


def fail(message: str) -> None:
    raise SystemExit(message)


def parse_version_script(path: pathlib.Path):
    mapping = {}
    current = None
    for raw in path.read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line or line.startswith("/*") or line.startswith("*"):
            continue
        match = re.match(r"^(JSONC_[A-Za-z0-9_.]+)\s*\{", line)
        if match:
            current = match.group(1)
            continue
        if line in {"global:", "local:", "*;", "};"} or line.startswith("}"):
            continue
        if current and line.endswith(";"):
            mapping[line[:-1].strip()] = current
    return mapping


def parse_debian_symbols(path: pathlib.Path):
    mapping = {}
    for raw in path.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or line.startswith("* ") or line.startswith("*\t"):
            continue
        match = re.match(r"^([A-Za-z0-9_]+)@([A-Za-z0-9_.]+)\s", line)
        if match:
            mapping[match.group(1)] = match.group(2)
    return mapping


rows = list(csv.DictReader(manifest_path.open(), delimiter="\t"))
required_columns = {"symbol", "kind", "surface", "version", "provenance", "c_type"}
if set(rows[0].keys()) != required_columns:
    fail(f"manifest columns mismatch: {rows[0].keys()}")

version_map = parse_version_script(version_script)
debian_map = parse_debian_symbols(debian_symbols)

shared_rows = {row["symbol"]: row for row in rows if row["surface"] == "shared"}
static_rows = {row["symbol"]: row for row in rows if row["surface"] == "static-only"}

if set(shared_rows) != set(version_map):
    missing = sorted(set(version_map) - set(shared_rows))
    extra = sorted(set(shared_rows) - set(version_map))
    fail(f"shared manifest mismatch: missing={missing} extra={extra}")

for symbol, row in shared_rows.items():
    if row["version"] != version_map[symbol]:
        fail(f"version-script mismatch for {symbol}: manifest={row['version']} expected={version_map[symbol]}")
    if debian_map.get(symbol) != row["version"]:
        fail(f"debian symbols mismatch for {symbol}: manifest={row['version']} debian={debian_map.get(symbol)}")

if sorted(static_rows) != ["array_list_insert_idx"]:
    fail(f"unexpected static-only rows: {sorted(static_rows)}")

for symbol in ("json_number_chars", "json_hex_chars"):
    row = shared_rows.get(symbol)
    if row is None or row["kind"] != "data" or not row["provenance"].startswith("manual:"):
        fail(f"missing manual data row for {symbol}")

for row in rows:
    provenance = row["provenance"]
    if provenance.startswith("header:"):
        rel = provenance.split(":", 1)[1]
        header_path = header_dir / rel
        if not header_path.is_file():
            fail(f"missing header provenance target {header_path}")
        text = header_path.read_text()
        if not re.search(r"\b" + re.escape(row["symbol"]) + r"\b", text):
            fail(f"{row['symbol']} not found in header provenance file {header_path}")
    elif not provenance.startswith("manual:"):
        fail(f"unsupported provenance {provenance}")

for generated in ("json.h", "json_config.h"):
    candidate = header_dir / generated
    if not candidate.is_file():
        fail(f"missing generated install header {candidate}")
    build_check_copy = build_check / generated
    original_copy = original_build / generated
    if build_check_copy.read_bytes() != candidate.read_bytes() and original_copy.read_bytes() != candidate.read_bytes():
        fail(f"{generated} does not match either prepared baseline copy")
PY
