#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# **ST-314a low-peak** — case âm + dương cho `bin/vis-dag-check`:
# - metadata giả có **3 cạnh cấm**: 2 ngược tầng (`vis-core→vis-host-native`,
#   `vis-syntax→vis-types`) + 1 cùng tầng (`vis-vm→vis-interp`) + **1 crate chưa khai tầng**;
# - metadata sạch (23 crate, không cạnh) phải xanh.
# Chạy trong `vis-dev check`/`gate --fast`; không cần build, chỉ `cargo metadata` giả.
set -u
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

"$PYTHON" - "$TMP/meta-bad.json" "$TMP/meta-ok.json" <<'PY'
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
with open(sys.argv[1], "w", encoding="utf-8") as f:
    json.dump({"packages": bad}, f)
with open(sys.argv[2], "w", encoding="utf-8") as f:
    json.dump({"packages": ok}, f)
PY

for kind in bad ok; do
    cat >"$TMP/cargo-$kind" <<'SH'
#!/bin/sh
cat "$(dirname "$0")/meta-$(basename "$0" | sed 's/^cargo-//').json"
SH
    chmod +x "$TMP/cargo-$kind"
done

OUT="$TMP/out-bad"
if CARGO="$TMP/cargo-bad" "$ROOT/bin/vis-dag-check" >"$OUT" 2>&1; then
    echo "not ok - vis-dag-check KHÔNG bắt metadata giả"
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

OUT_OK="$TMP/out-ok"
if ! CARGO="$TMP/cargo-ok" "$ROOT/bin/vis-dag-check" >"$OUT_OK" 2>&1; then
    echo "not ok - vis-dag-check báo đỏ metadata sạch"
    cat "$OUT_OK"
    exit 1
fi

echo "ok - vis-dag-check bắt 3 cạnh cấm + crate chưa khai tầng; metadata sạch xanh"
