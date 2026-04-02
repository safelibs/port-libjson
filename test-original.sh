#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBJSON_ORIGINAL_TEST_IMAGE:-libjson-original-test:ubuntu24.04}"
MODE="safe-package"
CHECKS="runtime"
PACKAGE_DIR=""
ONLY=""

usage() {
  cat <<'EOF'
usage: test-original.sh [--mode safe|safe-package|original-source] [--checks runtime|compile|all] [--only <dependent-name-or-source-package>] [--package-dir <dir>]

safe-package is the default compatibility target; safe is accepted as a
backward-compatible alias. Use original-source only for baseline comparisons
against a /usr/local install.

--checks defaults to runtime so existing callers keep their current behavior.
Use compile to build downstream dependents from source, or all to run compile
checks first and runtime checks second in the same container session.

--only filters the dependent matrix by display name or by source package name
from dependents.json. Matching a source package runs every listed dependent
that shares that source package.

--package-dir reuses prebuilt safe Debian packages instead of rebuilding them
inside the Ubuntu 24.04 testbed. This option is only valid with safe-package.
EOF
}

while (($#)); do
  case "$1" in
    --mode)
      MODE="${2:?missing value for --mode}"
      shift 2
      ;;
    --checks)
      CHECKS="${2:?missing value for --checks}"
      shift 2
      ;;
    --only)
      ONLY="${2:?missing value for --only}"
      shift 2
      ;;
    --package-dir)
      PACKAGE_DIR="${2:?missing value for --package-dir}"
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

case "$MODE" in
  safe)
    MODE="safe-package"
    ;;
esac

case "$MODE" in
  safe-package|original-source)
    ;;
  *)
    printf 'unsupported mode: %s\n' "$MODE" >&2
    usage >&2
    exit 1
    ;;
esac

case "$CHECKS" in
  runtime|compile|all)
    ;;
  *)
    printf 'unsupported checks mode: %s\n' "$CHECKS" >&2
    usage >&2
    exit 1
    ;;
esac

if [[ -n "$PACKAGE_DIR" ]]; then
  PACKAGE_DIR="$(readlink -f "$PACKAGE_DIR")"
  [[ -d "$PACKAGE_DIR" ]] || {
    printf 'package directory does not exist: %s\n' "$PACKAGE_DIR" >&2
    exit 1
  }
  [[ "$MODE" == "safe-package" ]] || {
    echo "--package-dir is only valid with --mode safe-package" >&2
    exit 1
  }
fi

command -v docker >/dev/null 2>&1 || {
  echo "docker is required to run $0" >&2
  exit 1
}

docker build -t "$IMAGE_TAG" - <<'DOCKERFILE'
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN sed 's/^Types: deb$/Types: deb-src/' /etc/apt/sources.list.d/ubuntu.sources \
      > /etc/apt/sources.list.d/ubuntu-src.sources \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      autoconf \
      automake \
      bind9 \
      bluez-meshd \
      bison \
      build-essential \
      ca-certificates \
      cargo \
      check \
      cmake \
      curl \
      daxctl \
      debhelper \
      dbus \
      dpkg-dev \
      fakeroot \
      flex \
      frr \
      gdal-bin \
      jq \
      libasound2-dev \
      libbluetooth-dev \
      libdbus-1-dev \
      libdw-dev \
      libell-dev \
      libglib2.0-dev \
      libical-dev \
      libreadline-dev \
      libtool \
      libudev-dev \
      ndctl \
      nvme-cli \
      pd-purest-json \
      pkg-config \
      puredata-core \
      python3 \
      python3-docutils \
      python3-pygments \
      python3-websockets \
      rustc \
      sway \
      syslog-ng-core \
      systemd-dev \
      tlog \
      ttyd \
      udev \
 && rm -rf /var/lib/apt/lists/*
DOCKERFILE

docker_args=(
  --rm
  -i
  --cap-add=NET_ADMIN
  --cap-add=SYS_ADMIN
  -e "LIBJSON_TEST_MODE=$MODE"
  -e "LIBJSON_TEST_CHECKS=$CHECKS"
  -e "LIBJSON_TEST_ONLY=$ONLY"
  -v "$ROOT":/work:ro
)

if [[ -n "$PACKAGE_DIR" ]]; then
  docker_args+=(
    -e LIBJSON_TEST_PACKAGE_DIR=/packages
    -v "$PACKAGE_DIR":/packages:ro
  )
fi

docker run "${docker_args[@]}" \
  "$IMAGE_TAG" \
  bash -s <<'CONTAINER_SCRIPT'
set -euo pipefail

export LANG=C.UTF-8
export LC_ALL=C.UTF-8

ROOT=/work
MODE="${LIBJSON_TEST_MODE:-safe-package}"
CHECKS="${LIBJSON_TEST_CHECKS:-runtime}"
ONLY_FILTER="${LIBJSON_TEST_ONLY:-}"
PACKAGE_DIR="${LIBJSON_TEST_PACKAGE_DIR:-}"
WORKSPACE_COPY=/tmp/libjson-safe-work
ARTIFACT_DIR=/tmp/libjson-safe-artifacts
DEPENDENT_SOURCE_ROOT=/tmp/libjson-dependent-sources
JSON_C_LIBDIR=""
JSON_C_RUNTIME_LIB=""
JSON_C_MODE_LABEL=""
JSON_C_INCLUDEDIR=""
JSON_C_HEADER_DIR=""
JSON_C_PREFIX=""
JSON_C_PKGCONFIG_DIR=""
JSON_C_CMAKE_DIR=""
JSON_C_SHARED_LINK=""
JSON_C_CFLAGS=""
CURRENT_DEPENDENT_NAME=""
CURRENT_SOURCE_PACKAGE=""
CURRENT_ARTIFACT_PATH=""
NDCTL_MATRIX_BUILD_DIR=""
APT_UPDATED=0
declare -A SOURCE_CACHE=()
declare -A BUILD_DEPS_CACHE=()

case "$MODE" in
  safe)
    MODE="safe-package"
    ;;
esac

case "$CHECKS" in
  runtime|compile|all)
    ;;
  *)
    printf 'unsupported checks mode inside container: %s\n' "$CHECKS" >&2
    exit 1
    ;;
esac

log_step() {
  printf '\n==> %s\n' "$1"
}

die() {
  echo "error: $*" >&2
  exit 1
}

assert_dependents_inventory() {
  local expected actual
  expected=$'BIND 9\tbind9\nFRRouting\tfrr\nSway\tsway\nGDAL\tgdal\nnvme-cli\tnvme-cli\nndctl\tndctl\ndaxctl\tndctl\nBlueZ Mesh Daemon\tbluez\nsyslog-ng\tsyslog-ng\nttyd\tttyd\ntlog\ttlog\nPuREST JSON for Pure Data\tpd-purest-json'
  actual="$(jq -r '.dependents[] | [.name, .source_package] | @tsv' "$ROOT/dependents.json")"
  if [[ "$actual" != "$expected" ]]; then
    echo "dependents.json does not match the expected dependent matrix" >&2
    diff -u <(printf '%s\n' "$expected") <(printf '%s\n' "$actual") >&2 || true
    exit 1
  fi
}

setup_original_json_env() {
  local ld_parts=() pkg_parts=()

  for path in /usr/local/lib /usr/local/lib/x86_64-linux-gnu; do
    if [[ -d "$path" ]]; then
      ld_parts+=("$path")
    fi
  done
  for path in /usr/local/lib/pkgconfig /usr/local/lib/x86_64-linux-gnu/pkgconfig /usr/local/share/pkgconfig; do
    if [[ -d "$path" ]]; then
      pkg_parts+=("$path")
    fi
  done

  if ((${#ld_parts[@]} == 0)); then
    die "no /usr/local library directories were created by the original json-c install"
  fi
  if ((${#pkg_parts[@]} == 0)); then
    die "no /usr/local pkg-config directories were created by the original json-c install"
  fi

  export LD_LIBRARY_PATH
  LD_LIBRARY_PATH="$(IFS=:; echo "${ld_parts[*]}")${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

  export PKG_CONFIG_PATH
  PKG_CONFIG_PATH="$(IFS=:; echo "${pkg_parts[*]}")${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

  JSON_C_LIBDIR="$(pkg-config --variable=libdir json-c)"
  [[ -d "$JSON_C_LIBDIR" ]] || die "pkg-config reported a missing libdir: ${JSON_C_LIBDIR}"
  [[ "$JSON_C_LIBDIR" == /usr/local/* ]] || die "original-source pkg-config resolved the distro json-c libdir: ${JSON_C_LIBDIR}"

  local json_c_includedir
  json_c_includedir="$(pkg-config --variable=includedir json-c)"
  [[ "$json_c_includedir" == /usr/local/* ]] || die "original-source pkg-config resolved the distro json-c includedir: ${json_c_includedir}"

  JSON_C_RUNTIME_LIB="$(find "$JSON_C_LIBDIR" -maxdepth 1 -type f -name 'libjson-c.so.5*' | sort | head -n 1)"
  [[ -n "$JSON_C_RUNTIME_LIB" ]] || die "could not locate the original-source libjson-c shared object under ${JSON_C_LIBDIR}"
  JSON_C_RUNTIME_LIB="$(readlink -f "$JSON_C_RUNTIME_LIB")"
  JSON_C_MODE_LABEL="original-source"
}

setup_packaged_json_env() {
  local packaged_lib

  JSON_C_LIBDIR="$(pkg-config --variable=libdir json-c)"
  [[ -d "$JSON_C_LIBDIR" ]] || die "pkg-config reported a missing libdir: ${JSON_C_LIBDIR}"

  packaged_lib="$(dpkg -L libjson-c5 | grep -E '/libjson-c\.so\.5(\..*)?$' | head -n 1)"
  [[ -n "$packaged_lib" ]] || die "dpkg -L libjson-c5 did not report an installed libjson-c shared object"
  [[ "$(dirname "$packaged_lib")" == "$JSON_C_LIBDIR" ]] || {
    printf 'dpkg -L libjson-c5 reported %s, but pkg-config points at %s\n' "$packaged_lib" "$JSON_C_LIBDIR" >&2
    exit 1
  }

  JSON_C_RUNTIME_LIB="$(readlink -f "$packaged_lib")"
  [[ -f "$JSON_C_RUNTIME_LIB" ]] || die "installed libjson-c shared object is missing: ${JSON_C_RUNTIME_LIB}"
  JSON_C_MODE_LABEL="safe-package"
}

setup_json_c_build_env() {
  local path

  JSON_C_INCLUDEDIR="$(pkg-config --variable=includedir json-c)"
  JSON_C_PREFIX="$(pkg-config --variable=prefix json-c)"
  JSON_C_CFLAGS="$(pkg-config --cflags json-c)"

  if [[ -f "$JSON_C_INCLUDEDIR/json-c/json.h" ]]; then
    JSON_C_HEADER_DIR="$JSON_C_INCLUDEDIR/json-c"
  elif [[ -f "$JSON_C_INCLUDEDIR/json.h" ]]; then
    JSON_C_HEADER_DIR="$JSON_C_INCLUDEDIR"
  else
    die "could not locate json-c headers under ${JSON_C_INCLUDEDIR}"
  fi

  JSON_C_PKGCONFIG_DIR=""
  for path in \
    "$JSON_C_LIBDIR/pkgconfig" \
    "$JSON_C_PREFIX/lib/pkgconfig" \
    "$JSON_C_PREFIX/lib/x86_64-linux-gnu/pkgconfig" \
    "$JSON_C_PREFIX/share/pkgconfig"
  do
    if [[ -f "$path/json-c.pc" ]]; then
      JSON_C_PKGCONFIG_DIR="$path"
      break
    fi
  done
  [[ -n "$JSON_C_PKGCONFIG_DIR" ]] || die "could not locate json-c.pc for ${JSON_C_MODE_LABEL}"

  JSON_C_CMAKE_DIR=""
  for path in \
    "$JSON_C_LIBDIR/cmake/json-c" \
    "$JSON_C_PREFIX/lib/cmake/json-c" \
    "$JSON_C_PREFIX/lib/x86_64-linux-gnu/cmake/json-c"
  do
    if [[ -d "$path" ]]; then
      JSON_C_CMAKE_DIR="$path"
      break
    fi
  done
  [[ -n "$JSON_C_CMAKE_DIR" ]] || die "could not locate json-c CMake metadata for ${JSON_C_MODE_LABEL}"

  JSON_C_SHARED_LINK="$(find "$JSON_C_LIBDIR" -maxdepth 1 \( -type f -o -type l \) -name 'libjson-c.so' | sort | head -n 1)"
  [[ -n "$JSON_C_SHARED_LINK" ]] || die "could not locate libjson-c.so in ${JSON_C_LIBDIR}"
  JSON_C_SHARED_LINK="$(readlink -f "$JSON_C_SHARED_LINK")"

  export PKG_CONFIG_PATH
  PKG_CONFIG_PATH="${JSON_C_PKGCONFIG_DIR}${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

  export CMAKE_PREFIX_PATH
  CMAKE_PREFIX_PATH="${JSON_C_PREFIX}${CMAKE_PREFIX_PATH:+:$CMAKE_PREFIX_PATH}"

  export CMAKE_LIBRARY_PATH
  CMAKE_LIBRARY_PATH="${JSON_C_LIBDIR}${CMAKE_LIBRARY_PATH:+:$CMAKE_LIBRARY_PATH}"

  export CMAKE_INCLUDE_PATH
  CMAKE_INCLUDE_PATH="${JSON_C_HEADER_DIR}:${JSON_C_INCLUDEDIR}${CMAKE_INCLUDE_PATH:+:$CMAKE_INCLUDE_PATH}"

  export CPPFLAGS
  CPPFLAGS="${JSON_C_CFLAGS}${CPPFLAGS:+ $CPPFLAGS}"

  export LDFLAGS
  LDFLAGS="-L${JSON_C_LIBDIR} -Wl,-rpath,${JSON_C_LIBDIR} -Wl,-rpath-link,${JSON_C_LIBDIR}${LDFLAGS:+ $LDFLAGS}"

  export LIBRARY_PATH
  LIBRARY_PATH="${JSON_C_LIBDIR}${LIBRARY_PATH:+:$LIBRARY_PATH}"

  export LD_LIBRARY_PATH
  LD_LIBRARY_PATH="${JSON_C_LIBDIR}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
}

normalize_filter() {
  tr '[:upper:]' '[:lower:]' <<<"$1"
}

matches_only_filter() {
  local dependent_name="$1"
  local source_package="$2"
  local normalized_only

  [[ -z "$ONLY_FILTER" ]] && return 0

  normalized_only="$(normalize_filter "$ONLY_FILTER")"
  [[ "$normalized_only" == "$(normalize_filter "$dependent_name")" ]] && return 0
  [[ "$normalized_only" == "$(normalize_filter "$source_package")" ]] && return 0
  return 1
}

assert_only_filter_matches_inventory() {
  local name source matched=0

  [[ -z "$ONLY_FILTER" ]] && return 0

  while IFS=$'\t' read -r name source; do
    if matches_only_filter "$name" "$source"; then
      matched=1
      break
    fi
  done < <(jq -r '.dependents[] | [.name, .source_package] | @tsv' "$ROOT/dependents.json")

  ((matched == 1)) || die "--only did not match any dependent name or source package: ${ONLY_FILTER}"
}

set_dependent_context() {
  CURRENT_DEPENDENT_NAME="$1"
  CURRENT_SOURCE_PACKAGE="$2"
  CURRENT_ARTIFACT_PATH="${3:-}"
}

clear_dependent_context() {
  CURRENT_DEPENDENT_NAME=""
  CURRENT_SOURCE_PACKAGE=""
  CURRENT_ARTIFACT_PATH=""
}

compile_fail() {
  echo "compile failure:" >&2
  echo "  dependent: ${CURRENT_DEPENDENT_NAME:-unknown}" >&2
  echo "  source package: ${CURRENT_SOURCE_PACKAGE:-unknown}" >&2
  echo "  json-c implementation: ${JSON_C_MODE_LABEL:-unknown} (${JSON_C_RUNTIME_LIB:-unknown})" >&2
  if [[ -n "${CURRENT_ARTIFACT_PATH:-}" ]]; then
    echo "  artifact: ${CURRENT_ARTIFACT_PATH}" >&2
  fi
  echo "  reason: $*" >&2
  exit 1
}

ensure_apt_metadata() {
  if ((APT_UPDATED == 0)); then
    log_step "Refreshing apt metadata for dependent source builds" >&2
    apt-get update >/dev/null
    APT_UPDATED=1
  fi
}

run_logged() {
  local logfile="$1"
  shift

  : >"$logfile"
  printf '+ ' >>"$logfile"
  printf '%q ' "$@" >>"$logfile"
  printf '\n' >>"$logfile"
  "$@" >>"$logfile" 2>&1
}

print_log_excerpt() {
  local logfile="$1"
  echo "=== ${logfile} (head) ===" >&2
  sed -n '1,120p' "$logfile" >&2 || true
  echo "=== ${logfile} (tail) ===" >&2
  tail -n 120 "$logfile" >&2 || true
}

run_compile_step() {
  local description="$1"
  local logfile="$2"
  shift 2

  if ! run_logged "$logfile" "$@"; then
    print_log_excerpt "$logfile"
    compile_fail "${description} failed; see ${logfile}"
  fi
}

assert_pkg_config_uses_selected_json_c() {
  local resolved_libdir resolved_includedir

  resolved_libdir="$(pkg-config --variable=libdir json-c)"
  resolved_includedir="$(pkg-config --variable=includedir json-c)"

  [[ "$resolved_libdir" == "$JSON_C_LIBDIR" ]] || compile_fail "pkg-config resolved the wrong json-c libdir: ${resolved_libdir}"
  [[ "$resolved_includedir" == "$JSON_C_INCLUDEDIR" ]] || compile_fail "pkg-config resolved the wrong json-c includedir: ${resolved_includedir}"
}

assert_log_avoids_wrong_json_c() {
  local logfile="$1"
  local offending

  [[ "$JSON_C_MODE_LABEL" == "original-source" ]] || return 0

  offending="$(
    grep -En '(^|[^[:alnum:]_])(json-c|libjson-c)([^[:alnum:]_]|$)' "$logfile" 2>/dev/null \
      | grep -E '/usr/(include|lib)(/|$)' \
      | grep -vF "$JSON_C_HEADER_DIR" \
      | grep -vF "$JSON_C_INCLUDEDIR" \
      | grep -vF "$JSON_C_LIBDIR" \
      | grep -vF "$JSON_C_CMAKE_DIR" \
      || true
  )"

  [[ -z "$offending" ]] || {
    echo "$offending" >&2
    compile_fail "build log shows resolution of the distro json-c: ${logfile}"
  }
}

resolve_target_json_c_lib() {
  local target="$1"
  local ldd_out
  local resolved_lib

  ldd_out="$(LD_LIBRARY_PATH="${LD_LIBRARY_PATH-}" ldd "$target")"
  resolved_lib="$(awk '/libjson-c\.so/{print $3; exit}' <<<"$ldd_out")"
  [[ -n "$resolved_lib" ]] || {
    echo "$ldd_out" >&2
    return 1
  }
  readlink -f "$resolved_lib"
}

resolve_built_artifact_path() {
  local artifact="$1"
  local artifact_dir artifact_base candidate

  if [[ -f "$artifact" ]] && readelf -h "$artifact" >/dev/null 2>&1; then
    printf '%s\n' "$artifact"
    return 0
  fi

  artifact_dir="$(dirname "$artifact")"
  artifact_base="$(basename "$artifact")"

  candidate="${artifact_dir}/.libs/${artifact_base}"
  if [[ -f "$candidate" ]] && readelf -h "$candidate" >/dev/null 2>&1; then
    printf '%s\n' "$candidate"
    return 0
  fi

  if [[ "$artifact_base" == *.la ]]; then
    candidate="${artifact_dir}/.libs/${artifact_base%.la}.so"
    if [[ -f "$candidate" ]] && readelf -h "$candidate" >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  fi

  printf '%s\n' "$artifact"
}

assert_compiled_artifact_uses_selected_json_c() {
  local artifact="$1"
  local resolved_artifact
  local readelf_out
  local resolved_lib

  resolved_artifact="$(resolve_built_artifact_path "$artifact")"
  CURRENT_ARTIFACT_PATH="$resolved_artifact"
  [[ -e "$resolved_artifact" ]] || compile_fail "built artifact is missing: ${resolved_artifact}"

  if ! readelf_out="$(readelf -d "$resolved_artifact" 2>/dev/null)"; then
    compile_fail "readelf could not inspect built artifact: ${resolved_artifact}"
  fi

  if ! grep -q 'libjson-c\.so\.5' <<<"$readelf_out"; then
    resolved_lib="$(resolve_target_json_c_lib "$resolved_artifact" || true)"
    [[ -n "$resolved_lib" ]] || compile_fail "built artifact does not resolve libjson-c.so.5: ${resolved_artifact}"
  fi

  resolved_lib="$(resolve_target_json_c_lib "$resolved_artifact" || true)"
  [[ -n "$resolved_lib" ]] || compile_fail "built artifact is not resolving libjson-c at runtime: ${resolved_artifact}"
  [[ "$resolved_lib" == "$JSON_C_RUNTIME_LIB" ]] || compile_fail "built artifact resolved ${resolved_lib}, expected ${JSON_C_RUNTIME_LIB}"

  printf 'compiled dependent=%s source=%s json-c=%s artifact=%s\n' \
    "$CURRENT_DEPENDENT_NAME" \
    "$CURRENT_SOURCE_PACKAGE" \
    "$JSON_C_RUNTIME_LIB" \
    "$resolved_artifact"
}

assert_cmake_cache_uses_selected_json_c() {
  local cache_file="$1"
  local build_dir
  local json_lines include_line library_line
  local found_signal=0

  [[ -f "$cache_file" ]] || compile_fail "missing CMake cache: ${cache_file}"
  build_dir="$(dirname "$cache_file")"

  json_lines="$(grep -E '^(json-c_DIR|JSONC_[A-Z_]+|JSON-C_[A-Z_]+):' "$cache_file" || true)"
  [[ -n "$json_lines" ]] || compile_fail "CMake cache does not record json-c resolution: ${cache_file}"

  if grep -q '^json-c_DIR:PATH=' "$cache_file"; then
    grep -Fxq "json-c_DIR:PATH=${JSON_C_CMAKE_DIR}" "$cache_file" || compile_fail "CMake resolved json-c_DIR away from ${JSON_C_CMAKE_DIR}"
    found_signal=1
  fi

  include_line="$(grep -E '^(JSONC_INCLUDE_DIR|JSON-C_INCLUDE_DIR):PATH=' "$cache_file" | head -n 1 || true)"
  if [[ -n "$include_line" ]]; then
    [[ "$include_line" == *"${JSON_C_HEADER_DIR}"* || "$include_line" == *"${JSON_C_INCLUDEDIR}"* ]] || compile_fail "CMake cache resolved the wrong json-c include path: ${include_line}"
    found_signal=1
  fi

  library_line="$(grep -E '^(JSONC_LIBRARY|JSON-C_LIBRARY):FILEPATH=' "$cache_file" | head -n 1 || true)"
  if [[ -n "$library_line" ]]; then
    [[ "$library_line" == *"${JSON_C_LIBDIR}/"* ]] || compile_fail "CMake cache resolved the wrong json-c library path: ${library_line}"
    found_signal=1
  fi

  if ((found_signal == 0)); then
    if grep -R -Fq "$JSON_C_CMAKE_DIR" "$build_dir" \
      || grep -R -Fq "$JSON_C_SHARED_LINK" "$build_dir" \
      || grep -R -Fq "$JSON_C_HEADER_DIR" "$build_dir" \
      || grep -R -Fq "$JSON_C_LIBDIR" "$build_dir"; then
      found_signal=1
    fi
  fi

  ((found_signal == 1)) || compile_fail "CMake cache did not record a usable json-c resolution signal: ${cache_file}"
}

assert_uses_selected_json_c() {
  local target="$1"
  local resolved_lib

  resolved_lib="$(resolve_target_json_c_lib "$target" || true)"
  [[ -n "$resolved_lib" ]] || die "$target is not resolving libjson-c at all"

  if [[ "$resolved_lib" != "$JSON_C_RUNTIME_LIB" ]]; then
    die "$target is not resolving libjson-c from ${JSON_C_MODE_LABEL} (${JSON_C_RUNTIME_LIB})"
  fi
}

collect_original_install_headers() {
  awk '
    index($0, "file(INSTALL DESTINATION \"/usr/local/include/json-c\" TYPE FILE FILES") {
      in_headers = 1
      next
    }
    in_headers && $0 ~ /^[[:space:]]*\)$/ {
      exit
    }
    in_headers {
      while (match($0, /"[^"]+"/)) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        $0 = substr($0, RSTART + RLENGTH)
      }
    }
  ' "$ROOT/original/build/cmake_install.cmake"
}

resolve_prepared_original_path() {
  local path="$1"

  if [[ -e "$path" ]]; then
    printf '%s\n' "$path"
    return
  fi

  case "$path" in
    */original/*)
      printf '%s/original/%s\n' "$ROOT" "${path#*/original/}"
      return
      ;;
  esac

  printf '%s\n' "$path"
}

assert_lists_command() {
  local label="$1"
  local binary="$2"
  local required_command="$3"
  local raw_file="$4"

  if ! "$binary" --list-cmds >"$raw_file"; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' "$raw_file" >&2 || true
    die "${label} --list-cmds failed"
  fi

  if ! grep -q "^${required_command}\$" "$raw_file"; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' "$raw_file" >&2 || true
    die "${label} --list-cmds did not advertise ${required_command}"
  fi
}

build_mock_sysfs_preload() {
  local workdir="$1"

  cat >"$workdir/mockfs.c" <<'EOF'
#define _GNU_SOURCE
#include <dlfcn.h>
#include <dirent.h>
#include <fcntl.h>
#include <limits.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const char *mock_root(void) {
  static const char *root;
  static int initialized;
  if (!initialized) {
    root = getenv("MOCK_SYSFS_ROOT");
    initialized = 1;
  }
  return root;
}

static bool map_path(const char *path, char *buffer, size_t buffer_size) {
  const char *root = mock_root();

  if (!root || !path || path[0] != '/' || strncmp(path, "/sys", 4) != 0) {
    return false;
  }

  return snprintf(buffer, buffer_size, "%s%s", root, path) < (int) buffer_size;
}

int open(const char *pathname, int flags, ...) {
  static int (*real_open)(const char *, int, ...);
  char mapped[PATH_MAX];
  const char *target = pathname;
  va_list args;
  mode_t mode = 0;

  if (!real_open) {
    real_open = dlsym(RTLD_NEXT, "open");
  }
  if (map_path(pathname, mapped, sizeof(mapped))) {
    target = mapped;
  }

  if (flags & O_CREAT) {
    va_start(args, flags);
    mode = va_arg(args, mode_t);
    va_end(args);
    return real_open(target, flags, mode);
  }
  return real_open(target, flags);
}

int open64(const char *pathname, int flags, ...) {
  static int (*real_open64)(const char *, int, ...);
  char mapped[PATH_MAX];
  const char *target = pathname;
  va_list args;
  mode_t mode = 0;

  if (!real_open64) {
    real_open64 = dlsym(RTLD_NEXT, "open64");
  }
  if (map_path(pathname, mapped, sizeof(mapped))) {
    target = mapped;
  }

  if (flags & O_CREAT) {
    va_start(args, flags);
    mode = va_arg(args, mode_t);
    va_end(args);
    return real_open64(target, flags, mode);
  }
  return real_open64(target, flags);
}

DIR *opendir(const char *name) {
  static DIR *(*real_opendir)(const char *);
  char mapped[PATH_MAX];
  const char *target = name;

  if (!real_opendir) {
    real_opendir = dlsym(RTLD_NEXT, "opendir");
  }
  if (map_path(name, mapped, sizeof(mapped))) {
    target = mapped;
  }

  return real_opendir(target);
}

int stat(const char *path, struct stat *st) {
  static int (*real_stat)(const char *, struct stat *);
  char mapped[PATH_MAX];
  const char *target = path;

  if (!real_stat) {
    real_stat = dlsym(RTLD_NEXT, "stat");
  }
  if (map_path(path, mapped, sizeof(mapped))) {
    target = mapped;
  }

  return real_stat(target, st);
}

int lstat(const char *path, struct stat *st) {
  static int (*real_lstat)(const char *, struct stat *);
  char mapped[PATH_MAX];
  const char *target = path;

  if (!real_lstat) {
    real_lstat = dlsym(RTLD_NEXT, "lstat");
  }
  if (map_path(path, mapped, sizeof(mapped))) {
    target = mapped;
  }

  return real_lstat(target, st);
}

int access(const char *path, int mode) {
  static int (*real_access)(const char *, int);
  char mapped[PATH_MAX];
  const char *target = path;

  if (!real_access) {
    real_access = dlsym(RTLD_NEXT, "access");
  }
  if (map_path(path, mapped, sizeof(mapped))) {
    target = mapped;
  }

  return real_access(target, mode);
}

char *realpath(const char *path, char *resolved_path) {
  static char *(*real_realpath)(const char *, char *);
  char mapped[PATH_MAX];
  char *result;
  const char *target = path;
  const char *root;
  size_t root_len;

  if (!real_realpath) {
    real_realpath = dlsym(RTLD_NEXT, "realpath");
  }
  if (map_path(path, mapped, sizeof(mapped))) {
    target = mapped;
  }

  result = real_realpath(target, resolved_path);
  if (!result) {
    return NULL;
  }

  root = mock_root();
  root_len = root ? strlen(root) : 0;
  if (root && strncmp(result, root, root_len) == 0) {
    memmove(result, result + root_len, strlen(result + root_len) + 1);
    if (result[0] == '\0') {
      result[0] = '/';
      result[1] = '\0';
    }
  }

  return result;
}
EOF

  if ! cc -shared -fPIC -O2 -Wall -Wextra -o "$workdir/mockfs.so" "$workdir/mockfs.c" -ldl >"$workdir/mockfs.build.out" 2>"$workdir/mockfs.build.err"; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' "$workdir/mockfs.build.out" >&2 || true
    echo '=== stderr ===' >&2
    sed -n '1,160p' "$workdir/mockfs.build.err" >&2 || true
    die "failed to build mock sysfs preload shim"
  fi
}

prepare_ndctl_mock_sysfs() {
  local mock_root="$1"

  mkdir -p \
    "$mock_root/sys/class/nd/ndctl0/device" \
    "$mock_root/sys/dev/char/1:1"

  printf '1:1\n' >"$mock_root/sys/class/nd/ndctl0/dev"
  printf 'nfit_test.0\n' >"$mock_root/sys/class/nd/ndctl0/device/provider"
  printf 'bus dimm region namespace\n' >"$mock_root/sys/class/nd/ndctl0/device/commands"
  printf '0\n' >"$mock_root/sys/class/nd/ndctl0/device/wait_probe"

  ln -s ../../../class/nd/ndctl0 "$mock_root/sys/dev/char/1:1/device"
}

prepare_daxctl_mock_sysfs() {
  local mock_root="$1"

  mkdir -p \
    "$mock_root/sys/devices/platform/mock/region0/dax_region" \
    "$mock_root/sys/devices/platform/mock/region0/dax/dax0.0" \
    "$mock_root/sys/devices/platform/mock/region0/dax0.0" \
    "$mock_root/sys/class/dax" \
    "$mock_root/sys/bus/dax/devices"

  printf '4096\n' >"$mock_root/sys/devices/platform/mock/region0/dax_region/size"
  printf '4096\n' >"$mock_root/sys/devices/platform/mock/region0/dax_region/align"

  ln -s ../../devices/platform/mock/region0/dax/dax0.0 "$mock_root/sys/class/dax/dax0.0"
  ln -s ../../../devices/platform/mock/region0/dax0.0 "$mock_root/sys/bus/dax/devices/dax0.0"
}

assert_mock_sysfs_json_list() {
  local label="$1"
  local binary="$2"
  local preload_lib="$3"
  local mock_root="$4"
  local raw_file="$5"
  local stderr_file="$6"
  shift 6

  if ! LD_PRELOAD="$preload_lib${LD_PRELOAD:+:$LD_PRELOAD}" MOCK_SYSFS_ROOT="$mock_root" "$binary" "$@" >"$raw_file" 2>"$stderr_file"; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' "$raw_file" >&2 || true
    echo '=== stderr ===' >&2
    sed -n '1,160p' "$stderr_file" >&2 || true
    die "${label} failed"
  fi

  if ! grep -q '[^[:space:]]' "$raw_file"; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' "$raw_file" >&2 || true
    echo '=== stderr ===' >&2
    sed -n '1,160p' "$stderr_file" >&2 || true
    die "${label} emitted no JSON output"
  fi
}

stage_original_json_c() {
  log_step "Staging prepared original json-c baseline into /usr/local"

  local manifest="$ROOT/original/build/cmake_install.cmake"
  local export_root="$ROOT/original/build/CMakeFiles/Export"
  local stage_root=/tmp/json-c-original-install-root
  local stage_usr_local="${stage_root}/usr/local"
  local export_dir
  local export_file
  local manifest_header
  local -a export_dirs=()
  local -a export_files=()
  local -a manifest_headers=()
  local -a install_headers=()
  local -a required_files=(
    "$ROOT/original/build/libjson-c.so"
    "$ROOT/original/build/libjson-c.so.5"
    "$ROOT/original/build/libjson-c.so.5.3.0"
    "$ROOT/original/build/libjson-c.a"
    "$ROOT/original/build/json-c.pc"
    "$ROOT/original/build/json-c-config.cmake"
    "$ROOT/original/build/json.h"
    "$ROOT/original/build/json_config.h"
  )

  [[ -f "$manifest" ]] || die "missing prepared upstream install manifest: ${manifest}"
  [[ -d "$export_root" ]] || die "missing prepared upstream export root: ${export_root}"

  mapfile -t export_dirs < <(find "$export_root" -mindepth 1 -maxdepth 1 -type d | sort)
  if ((${#export_dirs[@]} != 1)); then
    printf 'expected exactly one prepared upstream export directory under %s, found %d\n' "$export_root" "${#export_dirs[@]}" >&2
    printf '%s\n' "${export_dirs[@]}" >&2
    exit 1
  fi
  export_dir="${export_dirs[0]}"

  mapfile -t export_files < <(find "$export_dir" -mindepth 1 -maxdepth 1 -type f -name 'json-c-targets*.cmake' | sort)
  ((${#export_files[@]} >= 1)) || die "prepared upstream export directory does not contain json-c-targets*.cmake files: ${export_dir}"

  mapfile -t manifest_headers < <(collect_original_install_headers)
  ((${#manifest_headers[@]} >= 1)) || die "failed to read the prepared upstream header manifest from ${manifest}"
  for manifest_header in "${manifest_headers[@]}"; do
    install_headers+=("$(resolve_prepared_original_path "$manifest_header")")
  done

  for required_file in "${required_files[@]}"; do
    [[ -e "$required_file" ]] || die "missing prepared upstream artifact: ${required_file}"
  done
  for install_header in "${install_headers[@]}"; do
    [[ -f "$install_header" ]] || die "prepared upstream header listed in ${manifest} is missing: ${install_header}"
  done

  rm -rf "$stage_root"
  mkdir -p \
    "$stage_usr_local/lib/pkgconfig" \
    "$stage_usr_local/lib/cmake/json-c" \
    "$stage_usr_local/include/json-c"

  cp -a \
    "$ROOT/original/build/libjson-c.so" \
    "$ROOT/original/build/libjson-c.so.5" \
    "$ROOT/original/build/libjson-c.so.5.3.0" \
    "$ROOT/original/build/libjson-c.a" \
    "$stage_usr_local/lib/"
  cp -a "$ROOT/original/build/json-c.pc" "$stage_usr_local/lib/pkgconfig/"
  cp -a "$ROOT/original/build/json-c-config.cmake" "$stage_usr_local/lib/cmake/json-c/"
  cp -a "$export_dir/." "$stage_usr_local/lib/cmake/json-c/"
  cp -a "${install_headers[@]}" "$stage_usr_local/include/json-c/"

  rm -f \
    /usr/local/lib/libjson-c.so \
    /usr/local/lib/libjson-c.so.5 \
    /usr/local/lib/libjson-c.so.5.3.0 \
    /usr/local/lib/libjson-c.a \
    /usr/local/lib/pkgconfig/json-c.pc
  rm -rf /usr/local/lib/cmake/json-c /usr/local/include/json-c
  mkdir -p /usr/local/lib /usr/local/lib/pkgconfig /usr/local/lib/cmake /usr/local/include

  cp -a "$stage_usr_local/lib/." /usr/local/lib/
  cp -a "$stage_usr_local/include/." /usr/local/include/
  ldconfig

  cmp -s "$ROOT/original/build/json-c.pc" /usr/local/lib/pkgconfig/json-c.pc || die "staged pkg-config metadata does not match the prepared upstream json-c.pc"
  cmp -s "$ROOT/original/build/json-c-config.cmake" /usr/local/lib/cmake/json-c/json-c-config.cmake || die "staged CMake config does not match the prepared upstream json-c-config.cmake"
  for export_file in "${export_files[@]}"; do
    cmp -s "$export_file" "/usr/local/lib/cmake/json-c/$(basename "$export_file")" || die "staged export file does not match the prepared upstream $(basename "$export_file")"
  done

  setup_original_json_env
  setup_json_c_build_env

  printf 'Using %s json-c %s from %s\n' \
    "$JSON_C_MODE_LABEL" \
    "$(pkg-config --modversion json-c)" \
    "$JSON_C_RUNTIME_LIB"
}

build_safe_packages() {
  if [[ -n "$PACKAGE_DIR" ]]; then
    log_step "Installing prebuilt safe Debian packages"
  else
    log_step "Building safe Debian packages from a writable workspace copy"

    rm -rf "$WORKSPACE_COPY" "$ARTIFACT_DIR"
    mkdir -p "$WORKSPACE_COPY" "$ARTIFACT_DIR"
    cp -a "$ROOT/." "$WORKSPACE_COPY/"

    "$WORKSPACE_COPY/safe/tools/build-debs.sh" \
      --workspace "$WORKSPACE_COPY" \
      --out "$ARTIFACT_DIR"
  fi

  local package_root="${PACKAGE_DIR:-$ARTIFACT_DIR}"

  dpkg -i \
    "$package_root"/libjson-c5_*.deb \
    "$package_root"/libjson-c-dev_*.deb
  ldconfig

  setup_packaged_json_env
  setup_json_c_build_env

  printf 'Using %s json-c %s from %s\n' \
    "$JSON_C_MODE_LABEL" \
    "$(pkg-config --modversion json-c)" \
    "$JSON_C_RUNTIME_LIB"
}

run_safe_package_smoke_tests() {
  log_step "Running package-centric installed-artifact smoke tests"

  if [[ -n "$PACKAGE_DIR" ]]; then
    "$ROOT/safe/debian/tests/unit-test"
  else
    "$WORKSPACE_COPY/safe/debian/tests/unit-test"
  fi
}

fetch_source_package() {
  local source_package="$1"
  local source_root="${DEPENDENT_SOURCE_ROOT}/${source_package}"
  local source_dir=""
  local dsc_name

  if [[ -n "${SOURCE_CACHE[$source_package]-}" && -d "${SOURCE_CACHE[$source_package]}" ]]; then
    printf '%s\n' "${SOURCE_CACHE[$source_package]}"
    return 0
  fi

  ensure_apt_metadata
  log_step "Fetching source package ${source_package}" >&2

  rm -rf "$source_root"
  mkdir -p "$source_root"
  run_compile_step \
    "apt-get source ${source_package}" \
    "${source_root}/apt-source.log" \
    bash -lc "cd '$source_root' && apt-get source '$source_package'"

  dsc_name="$(find "$source_root" -mindepth 1 -maxdepth 1 -type f -name '*.dsc' | sort | head -n 1)"
  [[ -n "$dsc_name" ]] || compile_fail "apt-get source did not produce a .dsc for ${source_package}"

  source_dir="$(find "$source_root" -mindepth 1 -maxdepth 1 -type d | sort | head -n 1)"
  [[ -n "$source_dir" ]] || compile_fail "apt-get source did not unpack ${source_package} into ${source_root}"

  SOURCE_CACHE["$source_package"]="$source_dir"
  printf '%s\n' "$source_dir"
}

install_build_deps() {
  local source_package="$1"
  local state="${BUILD_DEPS_CACHE[$source_package]-}"
  local log_dir="/tmp/libjson-build-deps/${source_package}"

  [[ -n "$state" ]] && return 0

  ensure_apt_metadata
  mkdir -p "$log_dir"
  log_step "Installing build dependencies for ${source_package}" >&2

  run_compile_step \
    "apt-get build-dep ${source_package}" \
    "${log_dir}/build-dep.log" \
    apt-get build-dep -y "$source_package"

  assert_pkg_config_uses_selected_json_c
  BUILD_DEPS_CACHE["$source_package"]=1
}

prepare_out_of_tree_build() {
  local source_package="$1"
  local build_name="$2"
  local build_root="/tmp/libjson-dependent-build/${source_package}/${build_name}"

  rm -rf "$build_root"
  mkdir -p "$build_root"
  printf '%s\n' "$build_root"
}

require_built_artifact() {
  local candidate="$1"
  [[ -e "$candidate" ]] || compile_fail "expected built artifact is missing: ${candidate}"
}

compile_bind9() {
  local source_package="bind9"
  local srcdir log_root host_multiarch artifact

  set_dependent_context "BIND 9" "$source_package" "bin/named/named"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  host_multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
  log_root="/tmp/libjson-compile/${source_package}/named"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling BIND 9 against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "configure BIND 9" \
    "${log_root}/configure.log" \
    bash -lc "cd '$srcdir' && ./configure --libdir=/usr/lib/${host_multiarch} --sysconfdir=/etc/bind --localstatedir=/ --enable-largefile --enable-shared --disable-static --with-openssl=/usr --with-gssapi=yes --with-libidn2 --with-json-c --with-lmdb=/usr --with-maxminddb --with-readline=libedit"
  run_compile_step \
    "build BIND 9 libraries" \
    "${log_root}/build-libs.log" \
    bash -lc "cd '$srcdir' && make -C lib -j'$(nproc)' V=1 LIBS=\"\$(pkg-config --libs libedit)\" all-recursive"
  run_compile_step \
    "generate BIND 9 built sources" \
    "${log_root}/built-sources.log" \
    bash -lc "cd '$srcdir' && make -j1 V=1 bind.keys.h"
  run_compile_step \
    "build BIND 9 named" \
    "${log_root}/build.log" \
    bash -lc "cd '$srcdir' && make -C bin/named -j'$(nproc)' V=1 LIBS=\"\$(pkg-config --libs libedit)\" named"
  assert_log_avoids_wrong_json_c "${log_root}/build-libs.log"
  assert_log_avoids_wrong_json_c "${log_root}/built-sources.log"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${srcdir}/bin/named/named"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_frr() {
  local source_package="frr"
  local srcdir log_root host_multiarch artifact

  set_dependent_context "FRRouting" "$source_package" "zebra/zebra"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  host_multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
  log_root="/tmp/libjson-compile/${source_package}/zebra"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling FRRouting zebra against ${JSON_C_MODE_LABEL} json-c"
  if [[ ! -x "$srcdir/configure" ]]; then
    run_compile_step \
      "bootstrap FRRouting" \
      "${log_root}/bootstrap.log" \
      bash -lc "cd '$srcdir' && autoreconf -fi"
  fi
  run_compile_step \
    "configure FRRouting" \
    "${log_root}/configure.log" \
    bash -lc "cd '$srcdir' && LIBTOOLFLAGS='-rpath /usr/lib/${host_multiarch}/frr' ./configure --localstatedir=/var/run/frr --sbindir=/usr/lib/frr --sysconfdir=/etc/frr --with-vtysh-pager=/usr/bin/pager --libdir=/usr/lib/${host_multiarch}/frr --with-moduledir=/usr/lib/${host_multiarch}/frr/modules --disable-dependency-tracking --disable-rpki --disable-scripting --disable-pim6d --disable-doc --disable-doc-html --disable-snmp --disable-fpm --disable-protobuf --disable-zeromq --disable-ospfapi --disable-bgp-vnc --enable-multipath=64 --enable-user=frr --enable-group=frr --enable-vty-group=frrvty --enable-configfile-mask=0640 --enable-logfile-mask=0640"
  run_compile_step \
    "build FRRouting zebra" \
    "${log_root}/build.log" \
    bash -lc "cd '$srcdir' && make -j'$(nproc)' V=1 zebra/zebra"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${srcdir}/zebra/zebra"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_sway() {
  local source_package="sway"
  local srcdir builddir log_root artifact

  set_dependent_context "Sway" "$source_package" "build/sway/sway"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  builddir="$(prepare_out_of_tree_build "$source_package" sway)"
  log_root="/tmp/libjson-compile/${source_package}/sway"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling Sway against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "configure Sway" \
    "${log_root}/configure.log" \
    meson setup "$builddir" "$srcdir" --libexecdir=lib
  run_compile_step \
    "build Sway" \
    "${log_root}/build.log" \
    meson compile -C "$builddir" -v sway
  assert_log_avoids_wrong_json_c "${builddir}/meson-logs/meson-log.txt"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${builddir}/sway/sway"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_gdal() {
  local source_package="gdal"
  local srcdir builddir log_root artifact

  set_dependent_context "GDAL" "$source_package" "build/apps/ogrinfo"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  builddir="$(prepare_out_of_tree_build "$source_package" gdal)"
  log_root="/tmp/libjson-compile/${source_package}/ogr"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling GDAL GeoJSON tools against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "configure GDAL" \
    "${log_root}/configure.log" \
    cmake -S "$srcdir" -B "$builddir" \
      -DCMAKE_BUILD_TYPE=Release \
      -Djson-c_DIR="$JSON_C_CMAKE_DIR" \
      -DJSONC_ROOT="$JSON_C_PREFIX" \
      -DJSONC_INCLUDE_DIR="$JSON_C_HEADER_DIR" \
      -DJSONC_LIBRARY="$JSON_C_SHARED_LINK" \
      -DBUILD_DOCS=OFF \
      -DBUILD_PYTHON_BINDINGS=OFF \
      -DGDAL_USE_JSONC=ON
  assert_cmake_cache_uses_selected_json_c "${builddir}/CMakeCache.txt"
  run_compile_step \
    "build GDAL ogrinfo/ogr2ogr" \
    "${log_root}/build.log" \
    cmake --build "$builddir" --parallel "$(nproc)" --verbose --target ogrinfo ogr2ogr
  assert_log_avoids_wrong_json_c "${log_root}/configure.log"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${builddir}/apps/ogrinfo"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"

  CURRENT_ARTIFACT_PATH="${builddir}/apps/ogr2ogr"
  require_built_artifact "$CURRENT_ARTIFACT_PATH"
  assert_compiled_artifact_uses_selected_json_c "$CURRENT_ARTIFACT_PATH"
  clear_dependent_context
}

compile_nvme_cli() {
  local source_package="nvme-cli"
  local srcdir builddir log_root artifact

  set_dependent_context "nvme-cli" "$source_package" "build/nvme"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  builddir="$(prepare_out_of_tree_build "$source_package" nvme)"
  log_root="/tmp/libjson-compile/${source_package}/nvme"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling nvme-cli against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "configure nvme-cli" \
    "${log_root}/configure.log" \
    meson setup "$builddir" "$srcdir" -Ddocs=man -Ddocs-build=false
  run_compile_step \
    "build nvme-cli" \
    "${log_root}/build.log" \
    meson compile -C "$builddir" -v nvme
  assert_log_avoids_wrong_json_c "${builddir}/meson-logs/meson-log.txt"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${builddir}/nvme"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

ensure_ndctl_build() {
  local source_package="ndctl"
  local srcdir builddir log_root

  if [[ -n "$NDCTL_MATRIX_BUILD_DIR" && -d "$NDCTL_MATRIX_BUILD_DIR" ]]; then
    return 0
  fi

  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  builddir="$(prepare_out_of_tree_build "$source_package" matrix)"
  log_root="/tmp/libjson-compile/${source_package}/matrix"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling ndctl/daxctl against ${JSON_C_MODE_LABEL} json-c" >&2
  run_compile_step \
    "configure ndctl" \
    "${log_root}/configure.log" \
    meson setup "$builddir" "$srcdir" -Dsystemd=disabled -Dlibtracefs=disabled
  run_compile_step \
    "build ndctl/daxctl" \
    "${log_root}/build.log" \
    meson compile -C "$builddir" -v ./ndctl/ndctl:executable ./daxctl/daxctl:executable
  assert_log_avoids_wrong_json_c "${builddir}/meson-logs/meson-log.txt"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  NDCTL_MATRIX_BUILD_DIR="$builddir"
}

compile_ndctl() {
  local artifact

  set_dependent_context "ndctl" "ndctl" "build/ndctl/ndctl"
  ensure_ndctl_build
  artifact="${NDCTL_MATRIX_BUILD_DIR}/ndctl/ndctl"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_daxctl() {
  local artifact

  set_dependent_context "daxctl" "ndctl" "build/daxctl/daxctl"
  ensure_ndctl_build
  artifact="${NDCTL_MATRIX_BUILD_DIR}/daxctl/daxctl"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_bluez_mesh() {
  local source_package="bluez"
  local srcdir log_root artifact

  set_dependent_context "BlueZ Mesh Daemon" "$source_package" "mesh/bluetooth-meshd"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  log_root="/tmp/libjson-compile/${source_package}/mesh"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling BlueZ mesh tools against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "configure BlueZ" \
    "${log_root}/configure.log" \
    bash -lc "cd '$srcdir' && ./configure --enable-mesh --disable-manpages --disable-systemd --disable-monitor --disable-obex --disable-client"
  run_compile_step \
    "build BlueZ mesh targets" \
    "${log_root}/build.log" \
    bash -lc "cd '$srcdir' && make -j'$(nproc)' V=1 mesh/bluetooth-meshd tools/mesh-cfgclient"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${srcdir}/mesh/bluetooth-meshd"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"

  CURRENT_ARTIFACT_PATH="${srcdir}/tools/mesh-cfgclient"
  require_built_artifact "$CURRENT_ARTIFACT_PATH"
  assert_compiled_artifact_uses_selected_json_c "$CURRENT_ARTIFACT_PATH"
  clear_dependent_context
}

compile_syslog_ng() {
  local source_package="syslog-ng"
  local srcdir builddir log_root artifact build_gnu_type

  set_dependent_context "syslog-ng" "$source_package" "modules/json/.libs/libjson-plugin.so"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  builddir="$(prepare_out_of_tree_build "$source_package" json-plugin)"
  log_root="/tmp/libjson-compile/${source_package}/json-plugin"
  rm -rf "$log_root"
  mkdir -p "$log_root"
  build_gnu_type="$(dpkg-architecture -qDEB_BUILD_GNU_TYPE)"

  log_step "Compiling syslog-ng JSON plugin against ${JSON_C_MODE_LABEL} json-c"
  if [[ ! -x "$srcdir/configure" ]]; then
    run_compile_step \
      "bootstrap syslog-ng" \
      "${log_root}/bootstrap.log" \
      bash -lc "cd '$srcdir' && autoreconf -fi"
  fi
    run_compile_step \
      "configure syslog-ng" \
      "${log_root}/configure.log" \
    bash -lc "cd '$builddir' && SOURCE_REVISION='libjson-compile' '$srcdir/configure' --build='${build_gnu_type}' --prefix=/usr --mandir=/usr/share/man --sysconfdir=/etc/syslog-ng --localstatedir=/var/lib/syslog-ng --libdir=/usr/lib/syslog-ng --disable-silent-rules --enable-dynamic-linking --disable-ssl --disable-spoof-source --disable-tcp-wrapper --disable-sql --disable-mongodb --enable-json --disable-riemann --disable-java --disable-manpages --disable-amqp --disable-python --disable-http --disable-kafka --disable-systemd --disable-pacct --with-ivykis=system --with-jsonc=system --with-module-dir=/usr/lib/syslog-ng/4.3 --with-systemdsystemunitdir=/lib/systemd/system"
  run_compile_step \
    "build syslog-ng JSON plugin" \
    "${log_root}/build.log" \
    bash -lc "cd '$builddir' && make -j1 V=1"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${builddir}/modules/json/.libs/libjson-plugin.so"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_ttyd() {
  local source_package="ttyd"
  local srcdir builddir log_root artifact

  set_dependent_context "ttyd" "$source_package" "build/ttyd"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  builddir="$(prepare_out_of_tree_build "$source_package" ttyd)"
  log_root="/tmp/libjson-compile/${source_package}/ttyd"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling ttyd against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "configure ttyd" \
    "${log_root}/configure.log" \
    cmake -S "$srcdir" -B "$builddir" \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_PREFIX_PATH="$CMAKE_PREFIX_PATH" \
      -DCMAKE_LIBRARY_PATH="$CMAKE_LIBRARY_PATH" \
      -DCMAKE_INCLUDE_PATH="$CMAKE_INCLUDE_PATH" \
      -DJSON-C_LIBRARY="$JSON_C_SHARED_LINK" \
      -DJSON-C_INCLUDE_DIR="$JSON_C_HEADER_DIR"
  assert_cmake_cache_uses_selected_json_c "${builddir}/CMakeCache.txt"
  run_compile_step \
    "build ttyd" \
    "${log_root}/build.log" \
    cmake --build "$builddir" --parallel "$(nproc)" --verbose --target ttyd
  assert_log_avoids_wrong_json_c "${log_root}/configure.log"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${builddir}/ttyd"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_tlog() {
  local source_package="tlog"
  local srcdir log_root artifact

  set_dependent_context "tlog" "$source_package" "src/tlog/.libs/tlog-rec"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  log_root="/tmp/libjson-compile/${source_package}/tlog-rec"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling tlog-rec against ${JSON_C_MODE_LABEL} json-c"
  if [[ ! -x "$srcdir/configure" ]]; then
    run_compile_step \
      "bootstrap tlog" \
      "${log_root}/bootstrap.log" \
      bash -lc "cd '$srcdir' && autoreconf -fi"
  fi
  run_compile_step \
    "configure tlog" \
    "${log_root}/configure.log" \
    bash -lc "cd '$srcdir' && ./configure --enable-utempter --disable-journal"
  run_compile_step \
    "build tlog-rec" \
    "${log_root}/build.log" \
    bash -lc "cd '$srcdir' && make -j'$(nproc)' V=1"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="${srcdir}/src/tlog/.libs/tlog-rec"
  require_built_artifact "$artifact"
  assert_compiled_artifact_uses_selected_json_c "$artifact"
  clear_dependent_context
}

compile_pd_purest_json() {
  local source_package="pd-purest-json"
  local srcdir log_root artifact

  set_dependent_context "PuREST JSON for Pure Data" "$source_package" "json-encode.pd_linux"
  install_build_deps "$source_package"
  assert_pkg_config_uses_selected_json_c
  srcdir="$(fetch_source_package "$source_package")"
  log_root="/tmp/libjson-compile/${source_package}/pd"
  rm -rf "$log_root"
  mkdir -p "$log_root"

  log_step "Compiling PuREST JSON externals against ${JSON_C_MODE_LABEL} json-c"
  run_compile_step \
    "build pd-purest-json" \
    "${log_root}/build.log" \
    bash -lc "cd '$srcdir' && make PDLIBBUILDER_DIR=/usr/share/pd-lib-builder"
  assert_log_avoids_wrong_json_c "${log_root}/build.log"

  artifact="$(find "$srcdir" -type f -name 'json-encode.pd_linux' | sort | head -n 1)"
  [[ -n "$artifact" ]] || compile_fail "could not locate json-encode.pd_linux after the pd-purest-json build"
  assert_compiled_artifact_uses_selected_json_c "$artifact"

  CURRENT_ARTIFACT_PATH="$(find "$srcdir" -type f -name 'json-decode.pd_linux' | sort | head -n 1)"
  [[ -n "$CURRENT_ARTIFACT_PATH" ]] || compile_fail "could not locate json-decode.pd_linux after the pd-purest-json build"
  assert_compiled_artifact_uses_selected_json_c "$CURRENT_ARTIFACT_PATH"
  clear_dependent_context
}

test_bind9() {
  log_step "Testing BIND 9"
  assert_uses_selected_json_c /usr/sbin/named

  (
  rm -rf /tmp/bindtest
  mkdir -p /tmp/bindtest
  cat >/tmp/bindtest/named.conf <<'CFG'
options {
  directory "/tmp/bindtest";
  listen-on port 5300 { 127.0.0.1; };
  listen-on-v6 { none; };
  pid-file "/tmp/bindtest/named.pid";
  session-keyfile "/tmp/bindtest/session.key";
  dump-file "/tmp/bindtest/named_dump.db";
  statistics-file "/tmp/bindtest/named.stats";
  memstatistics-file "/tmp/bindtest/named.memstats";
  recursion no;
  dnssec-validation no;
  allow-query { 127.0.0.1; };
};
controls {};
statistics-channels {
  inet 127.0.0.1 port 8053 allow { 127.0.0.1; };
};
zone "." IN {
  type hint;
  file "/usr/share/dns/root.hints";
};
CFG

  named -g -c /tmp/bindtest/named.conf >/tmp/bindtest/named.log 2>&1 &
  local pid=$!
  cleanup() {
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  }
  trap cleanup EXIT

  for _ in $(seq 1 60); do
    if curl -fsS http://127.0.0.1:8053/json/v1/server >/tmp/bindtest/server.json 2>/dev/null; then
      jq -e '."boot-time" and ."config-time" and ."current-time" and .version' /tmp/bindtest/server.json >/dev/null
      exit 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      break
    fi
    sleep 0.25
  done

  sed -n '1,160p' /tmp/bindtest/named.log >&2 || true
  die "BIND 9 statistics channel test failed"
  )
}

test_frr() {
  log_step "Testing FRRouting"
  assert_uses_selected_json_c /usr/lib/frr/zebra

  (
  rm -rf /tmp/frrtest
  mkdir -p /tmp/frrtest/vty
  chown -R frr:frr /tmp/frrtest
  install -o frr -g frr -m 0644 /dev/null /tmp/frrtest/zebra.conf

  /usr/lib/frr/zebra \
    --log stdout \
    --log-level info \
    --vty_socket /tmp/frrtest/vty \
    -z /tmp/frrtest/zserv.api \
    -i /tmp/frrtest/zebra.pid \
    -f /tmp/frrtest/zebra.conf \
    -u frr -g frr \
    >/tmp/frrtest/zebra.log 2>&1 &
  local pid=$!
  cleanup() {
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  }
  trap cleanup EXIT

  for _ in $(seq 1 60); do
    if [[ -S /tmp/frrtest/vty/zebra.vty ]]; then
      sleep 0.5
      if timeout 5 vtysh --vty_socket /tmp/frrtest/vty -d zebra -c 'show interface json' >/tmp/frrtest/interfaces.json 2>/tmp/frrtest/vty.err; then
        jq -e 'type == "object" and (has("lo") or has("eth0"))' /tmp/frrtest/interfaces.json >/dev/null
        exit 0
      fi
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      break
    fi
    sleep 0.25
  done

  echo '=== zebra.log ===' >&2
  sed -n '1,200p' /tmp/frrtest/zebra.log >&2 || true
  echo '=== vty.err ===' >&2
  sed -n '1,200p' /tmp/frrtest/vty.err >&2 || true
  die "FRRouting JSON interface query failed"
  )
}

test_sway() {
  log_step "Testing Sway"
  assert_uses_selected_json_c /usr/bin/sway

  (
  rm -rf /tmp/swaytest
  mkdir -p /tmp/swaytest/runtime
  chmod 700 /tmp/swaytest/runtime

  export XDG_RUNTIME_DIR=/tmp/swaytest/runtime
  export WLR_BACKENDS=headless
  export WLR_LIBINPUT_NO_DEVICES=1

  cat >/tmp/swaytest/config <<'CFG'
output HEADLESS-1 resolution 800x600
CFG

  sway --unsupported-gpu -d -c /tmp/swaytest/config >/tmp/swaytest/sway.log 2>&1 &
  local pid=$!
  cleanup() {
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  }
  trap cleanup EXIT

  for _ in $(seq 1 80); do
    local socket
    socket="$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -type s -name 'sway-ipc.*.sock' | head -n 1)"
    if [[ -n "$socket" ]]; then
      SWAYSOCK="$socket" swaymsg -t get_outputs >/tmp/swaytest/outputs.json 2>/tmp/swaytest/swaymsg.err || true
      if [[ -s /tmp/swaytest/outputs.json ]]; then
        jq -e 'type == "array" and length >= 1 and .[0].name == "HEADLESS-1"' /tmp/swaytest/outputs.json >/dev/null
        exit 0
      fi
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      break
    fi
    sleep 0.25
  done

  sed -n '1,200p' /tmp/swaytest/sway.log >&2 || true
  echo '---' >&2
  sed -n '1,80p' /tmp/swaytest/swaymsg.err >&2 || true
  die "Sway headless IPC JSON query failed"
  )
}

test_gdal() {
  log_step "Testing GDAL"

  local gdal_lib
  gdal_lib="$(ldconfig -p | awk '/libgdal\.so/{print $NF; exit}')"
  [[ -n "$gdal_lib" ]] || die "could not locate libgdal.so"
  assert_uses_selected_json_c "$gdal_lib"

  rm -rf /tmp/gdaltest
  mkdir -p /tmp/gdaltest
  cat >/tmp/gdaltest/in.geojson <<'JSON'
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"name": "alpha", "value": 7},
      "geometry": {"type": "Point", "coordinates": [1.25, 2.5]}
    }
  ]
}
JSON

  ogrinfo -ro -al -so /tmp/gdaltest/in.geojson >/tmp/gdaltest/info.txt
  ogr2ogr -f GeoJSON /tmp/gdaltest/out.geojson /tmp/gdaltest/in.geojson
  jq -e '.features[0].properties.name == "alpha" and .features[0].geometry.type == "Point"' /tmp/gdaltest/out.geojson >/dev/null
}

test_nvme_cli() {
  log_step "Testing nvme-cli"
  assert_uses_selected_json_c /usr/sbin/nvme

  rm -rf /tmp/nvmetest
  mkdir -p /tmp/nvmetest

  nvme list -o json >/tmp/nvmetest/list.json
  jq -e 'has("Devices") and (.Devices | type == "array")' /tmp/nvmetest/list.json >/dev/null

  nvme list-subsys -o json >/tmp/nvmetest/subsys.json
  jq -e 'type == "array"' /tmp/nvmetest/subsys.json >/dev/null
}

test_ndctl() {
  log_step "Testing ndctl"
  assert_uses_selected_json_c /usr/bin/ndctl

  rm -rf /tmp/ndctltest
  mkdir -p /tmp/ndctltest/root

  assert_lists_command \
    "ndctl" \
    /usr/bin/ndctl \
    list \
    /tmp/ndctltest/list-cmds.txt

  build_mock_sysfs_preload /tmp/ndctltest
  prepare_ndctl_mock_sysfs /tmp/ndctltest/root

  assert_mock_sysfs_json_list \
    "ndctl list -B" \
    /usr/bin/ndctl \
    /tmp/ndctltest/mockfs.so \
    /tmp/ndctltest/root \
    /tmp/ndctltest/list.raw \
    /tmp/ndctltest/list.err \
    list -B

  if ! jq -e '
    type == "array"
    and (
      length == 0
      or (
        length == 1
        and .[0].provider == "nfit_test.0"
        and .[0].dev == "ndctl0"
      )
    )
  ' /tmp/ndctltest/list.raw >/dev/null; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' /tmp/ndctltest/list.raw >&2 || true
    echo '=== stderr ===' >&2
    sed -n '1,160p' /tmp/ndctltest/list.err >&2 || true
    die "ndctl list -B emitted unexpected JSON output"
  fi
}

test_daxctl() {
  log_step "Testing daxctl"
  assert_uses_selected_json_c /usr/bin/daxctl

  rm -rf /tmp/daxctltest
  mkdir -p /tmp/daxctltest/root

  assert_lists_command \
    "daxctl" \
    /usr/bin/daxctl \
    list \
    /tmp/daxctltest/list-cmds.txt

  build_mock_sysfs_preload /tmp/daxctltest
  prepare_daxctl_mock_sysfs /tmp/daxctltest/root

  assert_mock_sysfs_json_list \
    "daxctl list -R" \
    /usr/bin/daxctl \
    /tmp/daxctltest/mockfs.so \
    /tmp/daxctltest/root \
    /tmp/daxctltest/list.raw \
    /tmp/daxctltest/list.err \
    list -R

  if ! jq -e '
    type == "array"
    and (
      length == 0
      or (
        length == 1
        and .[0].id == 0
        and .[0].path == "/platform/mock/region0"
        and .[0].size == 4096
        and .[0].align == 4096
      )
    )
  ' /tmp/daxctltest/list.raw >/dev/null; then
    echo '=== stdout ===' >&2
    sed -n '1,160p' /tmp/daxctltest/list.raw >&2 || true
    echo '=== stderr ===' >&2
    sed -n '1,160p' /tmp/daxctltest/list.err >&2 || true
    die "daxctl list -R emitted unexpected JSON output"
  fi
}

test_bluez_meshd() {
  log_step "Testing BlueZ Mesh Daemon"

  local meshd
  meshd="$(dpkg -L bluez-meshd | awk '/\/bluetooth-meshd$/ { print; exit }')"
  [[ -n "$meshd" ]] || die "could not locate the installed bluetooth-meshd binary"
  assert_uses_selected_json_c "$meshd"
}

test_syslog_ng() {
  log_step "Testing syslog-ng"
  assert_uses_selected_json_c /usr/lib/syslog-ng/4.3/libjson-plugin.so

  (
  rm -rf /tmp/syslogtest
  mkdir -p /tmp/syslogtest

  cat >/tmp/syslogtest/in.log <<'LOG'
{"app":"demo","answer":42}
LOG

  cat >/tmp/syslogtest/syslog-ng.conf <<'CFG'
@version: 4.3
options {
  keep-hostname(yes);
  chain-hostnames(no);
  stats(freq(0));
  create-dirs(yes);
};
source s_in {
  file("/tmp/syslogtest/in.log" flags(no-parse) follow-freq(1) read-old-records(yes));
};
parser p_json {
  json-parser(prefix(".json."));
};
destination d_out {
  file("/tmp/syslogtest/out.json" template("$(format-json .json.* --shift-levels 1)\n"));
};
log {
  source(s_in);
  parser(p_json);
  destination(d_out);
};
CFG

  syslog-ng --no-caps -F -f /tmp/syslogtest/syslog-ng.conf -R /tmp/syslogtest/persist >/tmp/syslogtest/syslog-ng.stdout 2>/tmp/syslogtest/syslog-ng.stderr &
  local pid=$!
  cleanup() {
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  }
  trap cleanup EXIT

  for _ in $(seq 1 40); do
    if [[ -s /tmp/syslogtest/out.json ]]; then
      jq -e '.json.app == "demo" and .json.answer == 42' /tmp/syslogtest/out.json >/dev/null
      exit 0
    fi
    sleep 0.25
  done

  sed -n '1,200p' /tmp/syslogtest/syslog-ng.stderr >&2 || true
  die "syslog-ng JSON parser/formatter test failed"
  )
}

test_ttyd() {
  log_step "Testing ttyd"
  assert_uses_selected_json_c /usr/bin/ttyd

  (
  rm -rf /tmp/ttydtest
  mkdir -p /tmp/ttydtest

  cat >/tmp/ttydtest/echo.sh <<'SH'
#!/bin/sh
printf 'ready\n'
while IFS= read -r line; do
  printf 'ECHO:%s\n' "$line"
  [ "$line" = quit ] && exit 0
done
SH
  chmod +x /tmp/ttydtest/echo.sh

  ttyd -p 7681 -W /tmp/ttydtest/echo.sh >/tmp/ttydtest/ttyd.log 2>&1 &
  local pid=$!
  cleanup() {
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  }
  trap cleanup EXIT

  for _ in $(seq 1 40); do
    if curl -fsS http://127.0.0.1:7681/token >/dev/null 2>/dev/null; then
      break
    fi
    sleep 0.25
  done

  python3 - <<'PY'
import asyncio
import json
import urllib.request

import websockets

token = json.loads(urllib.request.urlopen("http://127.0.0.1:7681/token").read().decode())["token"]

async def recv_output(ws, needle):
    seen = []
    for _ in range(8):
        msg = await asyncio.wait_for(ws.recv(), timeout=5)
        assert isinstance(msg, (bytes, bytearray)), type(msg)
        cmd = msg[:1].decode("ascii", errors="replace")
        text = msg[1:].decode(errors="replace")
        seen.append((cmd, text))
        if cmd == "0" and needle in text:
            return
    raise AssertionError(seen)

async def main():
    async with websockets.connect("ws://127.0.0.1:7681/ws", subprotocols=["tty"]) as ws:
        await ws.send(json.dumps({"AuthToken": token, "columns": 80, "rows": 24}).encode())
        await recv_output(ws, "ready")
        await ws.send(b"0echo-hi\n")
        await recv_output(ws, "ECHO:echo-hi")

asyncio.run(main())
PY
  )
}

test_tlog() {
  log_step "Testing tlog"
  assert_uses_selected_json_c /usr/bin/tlog-rec

  rm -rf /tmp/tlogtest
  mkdir -p /tmp/tlogtest

  tlog-rec -w file -o /tmp/tlogtest/recording.json /bin/sh -lc 'printf "hello from tlog\n"' </dev/null
  jq -e 'type == "object" and .out_txt == "hello from tlog\n"' /tmp/tlogtest/recording.json >/dev/null
}

test_pd_purest_json() {
  log_step "Testing PuREST JSON for Pure Data"
  assert_uses_selected_json_c /usr/lib/pd/extra/purest_json/json-encode.pd_linux

  rm -rf /tmp/pdtest
  mkdir -p /tmp/pdtest

  cat >/tmp/pdtest/test.pd <<'PD'
#N canvas 0 0 700 400 10;
#X declare -path /usr/lib/pd/extra/purest_json;
#X obj 20 20 loadbang;
#X obj 20 50 t b b b b;
#X msg 20 90 add name alpha;
#X msg 140 90 add value 7;
#X msg 260 90 bang;
#X obj 260 130 json-encode;
#X obj 260 160 t a a;
#X obj 260 190 json-decode;
#X obj 420 190 print json_string;
#X obj 260 230 print decoded;
#X obj 20 120 del 500;
#X msg 20 150 \; pd quit;
#X connect 0 0 1 0;
#X connect 1 3 2 0;
#X connect 1 2 3 0;
#X connect 1 1 4 0;
#X connect 1 0 10 0;
#X connect 2 0 5 0;
#X connect 3 0 5 0;
#X connect 4 0 5 0;
#X connect 5 0 6 0;
#X connect 6 1 7 0;
#X connect 6 0 8 0;
#X connect 7 1 9 0;
#X connect 10 0 11 0;
PD

  timeout 5 pd -nogui -stderr -open /tmp/pdtest/test.pd >/tmp/pdtest/stdout 2>/tmp/pdtest/stderr
  grep -F 'decoded: list name alpha' /tmp/pdtest/stderr >/dev/null
  grep -F 'decoded: list value 7' /tmp/pdtest/stderr >/dev/null
  grep -F 'json_string: symbol {"name":"alpha"' /tmp/pdtest/stderr >/dev/null
}

assert_dependents_inventory
assert_only_filter_matches_inventory
case "$MODE" in
  safe-package)
    build_safe_packages
    ;;
  original-source)
    stage_original_json_c
    ;;
  *)
    die "unsupported mode inside container: $MODE"
    ;;
esac

run_compile_checks() {
  if matches_only_filter "BIND 9" "bind9"; then
    compile_bind9
  fi
  if matches_only_filter "FRRouting" "frr"; then
    compile_frr
  fi
  if matches_only_filter "Sway" "sway"; then
    compile_sway
  fi
  if matches_only_filter "GDAL" "gdal"; then
    compile_gdal
  fi
  if matches_only_filter "nvme-cli" "nvme-cli"; then
    compile_nvme_cli
  fi
  if matches_only_filter "ndctl" "ndctl"; then
    compile_ndctl
  fi
  if matches_only_filter "daxctl" "ndctl"; then
    compile_daxctl
  fi
  if matches_only_filter "BlueZ Mesh Daemon" "bluez"; then
    compile_bluez_mesh
  fi
  if matches_only_filter "syslog-ng" "syslog-ng"; then
    compile_syslog_ng
  fi
  if matches_only_filter "ttyd" "ttyd"; then
    compile_ttyd
  fi
  if matches_only_filter "tlog" "tlog"; then
    compile_tlog
  fi
  if matches_only_filter "PuREST JSON for Pure Data" "pd-purest-json"; then
    compile_pd_purest_json
  fi
}

run_runtime_checks() {
  if [[ "$MODE" == "safe-package" ]]; then
    run_safe_package_smoke_tests
  fi

  if matches_only_filter "BIND 9" "bind9"; then
    test_bind9
  fi
  if matches_only_filter "FRRouting" "frr"; then
    test_frr
  fi
  if matches_only_filter "Sway" "sway"; then
    test_sway
  fi
  if matches_only_filter "GDAL" "gdal"; then
    test_gdal
  fi
  if matches_only_filter "nvme-cli" "nvme-cli"; then
    test_nvme_cli
  fi
  if matches_only_filter "ndctl" "ndctl"; then
    test_ndctl
  fi
  if matches_only_filter "daxctl" "ndctl"; then
    test_daxctl
  fi
  if matches_only_filter "BlueZ Mesh Daemon" "bluez"; then
    test_bluez_meshd
  fi
  if matches_only_filter "syslog-ng" "syslog-ng"; then
    test_syslog_ng
  fi
  if matches_only_filter "ttyd" "ttyd"; then
    test_ttyd
  fi
  if matches_only_filter "tlog" "tlog"; then
    test_tlog
  fi
  if matches_only_filter "PuREST JSON for Pure Data" "pd-purest-json"; then
    test_pd_purest_json
  fi
}

case "$CHECKS" in
  runtime)
    run_runtime_checks
    ;;
  compile)
    run_compile_checks
    ;;
  all)
    run_compile_checks
    run_runtime_checks
    ;;
esac

log_step "All ${JSON_C_MODE_LABEL} ${CHECKS} compatibility checks passed"
CONTAINER_SCRIPT
