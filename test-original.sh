#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBJSON_ORIGINAL_TEST_IMAGE:-libjson-original-test:ubuntu24.04}"
MODE="safe-package"

usage() {
  cat <<'EOF'
usage: test-original.sh [--mode safe-package|original-source]

safe-package is the default compatibility target. Use original-source only for
baseline comparisons against a /usr/local install.
EOF
}

while (($#)); do
  case "$1" in
    --mode)
      MODE="${2:?missing value for --mode}"
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
  safe-package|original-source)
    ;;
  *)
    printf 'unsupported mode: %s\n' "$MODE" >&2
    usage >&2
    exit 1
    ;;
esac

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

docker run --rm -i \
  --cap-add=NET_ADMIN \
  --cap-add=SYS_ADMIN \
  -e LIBJSON_TEST_MODE="$MODE" \
  -v "$ROOT":/work:ro \
  "$IMAGE_TAG" \
  bash -s <<'CONTAINER_SCRIPT'
set -euo pipefail

export LANG=C.UTF-8
export LC_ALL=C.UTF-8

ROOT=/work
MODE="${LIBJSON_TEST_MODE:-safe-package}"
WORKSPACE_COPY=/tmp/libjson-safe-work
ARTIFACT_DIR=/tmp/libjson-safe-artifacts
JSON_C_LIBDIR=""
JSON_C_RUNTIME_LIB=""
JSON_C_MODE_LABEL=""

log_step() {
  printf '\n==> %s\n' "$1"
}

die() {
  echo "error: $*" >&2
  exit 1
}

assert_dependents_inventory() {
  local expected actual
  expected=$'BIND 9\nFRRouting\nSway\nGDAL\nnvme-cli\nBlueZ Mesh Daemon\nsyslog-ng\nttyd\ntlog\nPuREST JSON for Pure Data'
  actual="$(jq -r '.dependents[].name' "$ROOT/dependents.json")"
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

assert_uses_selected_json_c() {
  local target="$1"
  local ldd_out
  local resolved_lib

  ldd_out="$(LD_LIBRARY_PATH="${LD_LIBRARY_PATH-}" ldd "$target")"
  resolved_lib="$(awk '/libjson-c\.so/{print $3; exit}' <<<"$ldd_out")"
  [[ -n "$resolved_lib" ]] || {
    echo "$ldd_out" >&2
    die "$target is not resolving libjson-c at all"
  }
  resolved_lib="$(readlink -f "$resolved_lib")"

  if [[ "$resolved_lib" != "$JSON_C_RUNTIME_LIB" ]]; then
    echo "$ldd_out" >&2
    die "$target is not resolving libjson-c from ${JSON_C_MODE_LABEL} (${JSON_C_RUNTIME_LIB})"
  fi
}

build_original_json_c() {
  log_step "Building original json-c baseline into /usr/local"

  cmake -S "$ROOT/original" -B /tmp/json-c-build -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=/usr/local
  cmake --build /tmp/json-c-build -j"$(nproc)"
  cmake --install /tmp/json-c-build
  ldconfig

  setup_original_json_env

  printf 'Using %s json-c %s from %s\n' \
    "$JSON_C_MODE_LABEL" \
    "$(pkg-config --modversion json-c)" \
    "$JSON_C_RUNTIME_LIB"
}

build_safe_packages() {
  log_step "Building safe Debian packages from a writable workspace copy"

  rm -rf "$WORKSPACE_COPY" "$ARTIFACT_DIR"
  mkdir -p "$WORKSPACE_COPY" "$ARTIFACT_DIR"
  cp -a "$ROOT/." "$WORKSPACE_COPY/"

  "$WORKSPACE_COPY/safe/tools/build-debs.sh" \
    --workspace "$WORKSPACE_COPY" \
    --out "$ARTIFACT_DIR"

  dpkg -i \
    "$ARTIFACT_DIR"/libjson-c5_*.deb \
    "$ARTIFACT_DIR"/libjson-c-dev_*.deb
  ldconfig

  log_step "Running package-centric installed-artifact smoke tests"
  "$WORKSPACE_COPY/safe/debian/tests/unit-test"

  setup_packaged_json_env

  printf 'Using %s json-c %s from %s\n' \
    "$JSON_C_MODE_LABEL" \
    "$(pkg-config --modversion json-c)" \
    "$JSON_C_RUNTIME_LIB"
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

test_bluez_mesh_build() {
  log_step "Building BlueZ mesh targets"

  (
  apt-get update >/dev/null
  rm -rf /tmp/bluez-src
  mkdir -p /tmp/bluez-src
  cd /tmp/bluez-src
  apt-get source bluez >/dev/null

  local srcdir
  srcdir="$(find /tmp/bluez-src -mindepth 1 -maxdepth 1 -type d -name 'bluez-[0-9]*' | head -n 1)"
  [[ -n "$srcdir" ]] || die "failed to unpack the BlueZ source package"

  cd "$srcdir"
  ./configure \
    --enable-mesh \
    --disable-manpages \
    --disable-systemd \
    --disable-monitor \
    --disable-obex \
    --disable-client
  make -j"$(nproc)" mesh/bluetooth-meshd tools/mesh-cfgclient

  assert_uses_selected_json_c "$srcdir/mesh/bluetooth-meshd"
  assert_uses_selected_json_c "$srcdir/tools/mesh-cfgclient"
  )
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
case "$MODE" in
  safe-package)
    build_safe_packages
    ;;
  original-source)
    build_original_json_c
    ;;
  *)
    die "unsupported mode inside container: $MODE"
    ;;
esac
test_bind9
test_frr
test_sway
test_gdal
test_nvme_cli
test_bluez_mesh_build
test_syslog_ng
test_ttyd
test_tlog
test_pd_purest_json

log_step "All ${JSON_C_MODE_LABEL} compatibility checks passed"
CONTAINER_SCRIPT
