#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# **ST-314a low-peak** — case âm + dương cho `bin/vis-dag-check`:
# - metadata giả có **3 cạnh cấm**: 2 ngược tầng (`vis-core→vis-host-native`,
#   `vis-syntax→vis-types`) + 1 cùng tầng (`vis-vm→vis-interp`) + **1 crate chưa khai tầng**;
# - metadata thiếu crate thành viên phải báo `thiếu crate ...`;
# - metadata sạch (23 crate, không cạnh) phải xanh.
# Chạy trong `vis-dev check`/`gate --fast`/`nightly`; chỉ `cargo metadata` giả, không build.
set -u
export PYTHONIOENCODING=utf-8 PYTHONUTF8=1
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PYTHON="${PYTHON:-}"
if [ -z "$PYTHON" ]; then
    for c in python3.12 python3.11 python3; do
        if command -v "$c" >/dev/null 2>&1; then PYTHON="$c"; break; fi
    done
fi
if [ -z "$PYTHON" ]; then
    echo "vis-dag-check self-test: không thấy python3/python3.12" >&2
    exit 2
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/vis-dag-self.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

"$PYTHON" - "$TMP/meta-bad.json" "$TMP/meta-ok.json" "$TMP/meta-missing.json" <<'PY'
import json, sys

names = [
    "vis-diag", "vis-syntax", "vis-fmt", "vis-plugin-abi", "vish-term",
    "vis-hir", "vis-core", "vis-http",
    "vis-types", "vis-bytecode", "vis-runtime", "vis-interp", "vis-vm",
    "vis-loader", "vis-lsp", "vis-pkg", "vis-host-native", "vis-host-linux",
    "vis-plugin-host", "vis-plugin-macros", "vish-edit",
    "vis-cli", "vis-conformance",
]
extra = {
    "vis-core": ["vis-host-native"],   # T1 -> T3
    "vis-syntax": ["vis-types"],       # T0 -> T2
    "vis-vm": ["vis-interp"],          # cùng T2 (CAM_NGANG)
}
bad = [
    {"name": n, "dependencies": [{"name": d} for d in extra.get(n, [])]}
    for n in names
]
bad.append({"name": "vis-la", "dependencies": []})  # crate thứ 24 chưa khai tầng
ok = [{"name": n, "dependencies": []} for n in names]
missing = [p for p in ok if p["name"] != "vis-conformance"]
for path, pkgs in ((sys.argv[1], bad), (sys.argv[2], ok), (sys.argv[3], missing)):
    with open(path, "w", encoding="utf-8") as f:
        json.dump({"packages": pkgs}, f)
PY

for kind in bad ok missing; do
    printf '#!/bin/sh\ncat "%s/meta-%s.json"\n' "$TMP" "$kind" >"$TMP/cargo-$kind"
    chmod +x "$TMP/cargo-$kind"
done

OUT="$TMP/out-bad"
CARGO="$TMP/cargo-bad" bash "$ROOT/bin/vis-dag-check" >"$OUT" 2>&1
rc=$?
if [ "$rc" -ne 2 ]; then
    echo "not ok - vis-dag-check phải exit 2 với metadata giả (nhận $rc)"
    cat "$OUT"
    exit 1
fi
for want in \
    'vis-core[T1] -> vis-host-native[T3]' \
    'vis-syntax[T0] -> vis-types[T2]' \
    'vis-vm -> vis-interp (cùng tầng/sai DAG)' \
    'chưa khai tầng trong TANG_CUA'
do
    if ! grep -qF "$want" "$OUT"; then
        echo "not ok - thiếu chẩn đoán: $want"
        cat "$OUT"
        exit 1
    fi
done

OUT_MISS="$TMP/out-missing"
CARGO="$TMP/cargo-missing" bash "$ROOT/bin/vis-dag-check" >"$OUT_MISS" 2>&1
rc=$?
if [ "$rc" -ne 2 ] || ! grep -qF 'thiếu crate `vis-conformance` trong workspace' "$OUT_MISS"; then
    echo "not ok - thiếu crate thành viên không bị bắt đúng (rc=$rc)"
    cat "$OUT_MISS"
    exit 1
fi

OUT_OK="$TMP/out-ok"
if ! CARGO="$TMP/cargo-ok" bash "$ROOT/bin/vis-dag-check" >"$OUT_OK" 2>&1; then
    echo "not ok - vis-dag-check báo đỏ metadata sạch"
    cat "$OUT_OK"
    exit 1
fi
if ! grep -qF 'vis-dag-check: OK — 23 crate / 5 tầng' "$OUT_OK"; then
    echo "not ok - metadata sạch thiếu dòng OK 23 crate / 5 tầng"
    cat "$OUT_OK"
    exit 1
fi

echo "ok - vis-dag-check bắt 3 cạnh cấm + crate chưa khai tầng + thiếu crate; metadata sạch xanh"
