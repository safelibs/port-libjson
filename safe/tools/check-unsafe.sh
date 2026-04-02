#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
audit_file="${1:-"${repo_root}/safe/abi/unsafe-audit.tsv"}"
src_root="${2:-"${repo_root}/safe/src"}"

python3 - "${repo_root}" "${audit_file}" "${src_root}" <<'PY'
from __future__ import annotations

import hashlib
import re
import sys
from collections import Counter
from pathlib import Path

repo_root = Path(sys.argv[1])
audit_path = Path(sys.argv[2])
src_root = Path(sys.argv[3])
allowed = {
    "ffi-entrypoint",
    "ffi-callback",
    "repr-c-layout",
    "ownership-transfer",
    "libc-call",
    "variadic-shim",
}

unsafe_re = re.compile(r"\bunsafe\b")


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def scan_unsafe_sites() -> dict[tuple[str, int], tuple[str, str]]:
    sites: dict[tuple[str, int], tuple[str, str]] = {}
    for path in sorted(src_root.rglob("*.rs")):
        rel = path.relative_to(repo_root).as_posix()
        for lineno, line in enumerate(path.read_text().splitlines(), start=1):
            if unsafe_re.search(line):
                sites[(rel, lineno)] = (line, digest(line))
    return sites


if not audit_path.is_file():
    raise SystemExit(f"missing unsafe audit inventory: {audit_path}")

if not src_root.is_dir():
    raise SystemExit(f"missing source root: {src_root}")

actual = scan_unsafe_sites()
documented: dict[tuple[str, int], tuple[str, str]] = {}
category_counts: Counter[str] = Counter()
errors: list[str] = []

for rowno, raw in enumerate(audit_path.read_text().splitlines(), start=1):
    if not raw or raw.startswith("#"):
        continue

    parts = raw.split("\t")
    if len(parts) != 4:
        errors.append(
            f"{audit_path}:{rowno}: expected 4 tab-separated columns "
            "(path, line, category, sha16)"
        )
        continue

    rel_path, lineno_text, category, sha16 = parts
    try:
        lineno = int(lineno_text)
    except ValueError:
        errors.append(f"{audit_path}:{rowno}: invalid line number {lineno_text!r}")
        continue

    key = (rel_path, lineno)
    if category not in allowed:
        errors.append(f"{audit_path}:{rowno}: disallowed category {category!r}")
        continue
    if key in documented:
        errors.append(
            f"{audit_path}:{rowno}: duplicate inventory entry for {rel_path}:{lineno}"
        )
        continue

    actual_site = actual.get(key)
    if actual_site is None:
        errors.append(
            f"{audit_path}:{rowno}: stale entry for {rel_path}:{lineno} "
            "(no current unsafe site at that location)"
        )
        continue

    actual_line, actual_sha = actual_site
    if actual_sha != sha16:
        errors.append(
            f"{audit_path}:{rowno}: stale hash for {rel_path}:{lineno} "
            f"(expected {actual_sha}, found {sha16})"
        )
        continue

    documented[key] = (category, actual_line)
    category_counts[category] += 1

missing = sorted(set(actual) - set(documented))
extra = sorted(set(documented) - set(actual))

for rel_path, lineno in missing:
    line, sha16 = actual[(rel_path, lineno)]
    errors.append(
        f"undocumented unsafe site: {rel_path}:{lineno}: {line.strip()} [sha16={sha16}]"
    )

for rel_path, lineno in extra:
    errors.append(f"extra inventory entry: {rel_path}:{lineno}")

if errors:
    for error in errors:
        print(error, file=sys.stderr)
    raise SystemExit(1)

print(
    "unsafe audit ok:"
    f" documented {len(documented)} sites"
    f" across {len(category_counts)} categories"
)
for category in sorted(category_counts):
    print(f"  {category}: {category_counts[category]}")
PY
