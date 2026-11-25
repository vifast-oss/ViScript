#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# **ST-314a low-peak** — case âm cho `bin/vis-dag-check`: metadata giả có **2 cạnh cấm**
# (`vis-core→vis-host-native`, `vis-syntax→vis-types`) + **1 crate chưa khai tầng**.
# Chạy trong `vis-dev check`/`gate --fast`; không cần build, chỉ `cargo metadata` giả.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/vis-dag-self.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

python3 - "$TMP/meta.json" <<'PY'
import json, sys

names = [
    "vis-diag", "vis-syntax", "vis-fmt", "vis-plugin-abi", "vish-term",
    "vis-hir", "vis-core", "vis-http",
    "vis-types", "vis-bytecode", "vis-runtime", "vis-interp", "vis-vm",
    "vis-loader", "vis-lsp", "vis-pkg", "vis-host-native", "vis-host-linux",
    "vis-plugin-host", "vis-plugin-macros", "vish-edit",
    "vis-cli", "vis-conformance",
]
extra = {"vis-core": ["vis-host-native"], "vis-syntax": ["vis-types"]}
pkgs = [
    {"name": n, "dependencies": [{"name": d} for d in extra.get(n, [])]}
    for n in names
]
pkgs.append({"name": "vis-la", "dependencies": []})  # crate thứ 24 chưa khai tầng
with open(sys.argv[1], "w", encoding="utf-8") as f:
    json.dump({"packages": pkgs}, f)
PY

cat >"$TMP/cargo" <<'SH'
#!/bin/sh
cat "$(dirname "$0")/meta.json"
SH
chmod +x "$TMP/cargo"

OUT="$TMP/out"
if CARGO="$TMP/cargo" "$ROOT/bin/vis-dag-check" >"$OUT" 2>&1; then
    echo "not ok - vis-dag-check KHÔNG bắt metadata giả"
    cat "$OUT"
    exit 1
fi
for want in \
    'vis-core[T1] -> vis-host-native[T3]' \
    'vis-syntax[T0] -> vis-types[T2]' \
    'chưa khai tầng trong TANG_CUA'
do
    if ! grep -qF "$want" "$OUT"; then
        echo "not ok - thiếu chẩn đoán: $want"
        cat "$OUT"
        exit 1
    fi
done
echo "ok - vis-dag-check bắt 2 cạnh cấm + crate chưa khai tầng"
