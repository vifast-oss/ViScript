#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# test/release/run.sh — canh đóng gói phát hành (VD-259).
#
# Mục tiêu: `cargo build --release` / `cargo install` của package vẫn build `test-child`
# (hợp đồng `CARGO_BIN_EXE_test-child` của test), nhưng **artifact phát hành** chỉ có `vis`.
# Phép kiểm này chứng minh:
#   1. metadata `release-bins` đúng một nguồn và bằng ["vis"];
#   2. `[[bin]] test-child` vẫn còn (không phá test);
#   3. `bin/vis-release --dry-run` sinh lệnh có `--bin vis`, không có `--bins`/`test-child`;
#   4. metadata chứa `test-child` bị từ chối;
#   5. staging thật (cargo giả tạo cả test-child trong target) **không** chép test-child.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REL="$ROOT/bin/vis-release"
CLI="$ROOT/crates/vis-cli/Cargo.toml"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fail=0
check() {
    if [ "$1" -eq 0 ]; then printf 'ok - %s\n' "$2"; else printf 'not ok - %s\n' "$2"; fail=$((fail + 1)); fi
}

# 1) `[package.metadata.vis] release-bins` phải đúng ["vis"].
meta="$(awk '
    /^[[:space:]]*\[/ { in_meta = ($0 ~ /^[[:space:]]*\[package\.metadata\.vis\][[:space:]]*$/); next }
    in_meta && /^[[:space:]]*release-bins[[:space:]]*=/ {
        s = $0
        sub(/^[^=]*=[[:space:]]*\[/, "", s)
        sub(/\][[:space:]]*$/, "", s)
        n = split(s, a, ",")
        for (i = 1; i <= n; i++) { gsub(/["[:space:]]/, "", a[i]); if (a[i] != "") print a[i] }
    }
' "$CLI" | paste -sd, -)"
[ "$meta" = "vis" ]; check $? "Cargo.toml: release-bins = [\"vis\"] (thật: [$meta])"

# 2) test-child vẫn khai báo làm bin của package (nếu bỏ sẽ gãy CARGO_BIN_EXE_test-child).
grep -Eq '^[[:space:]]*name[[:space:]]*=[[:space:]]*"test-child"' "$CLI"
check $? 'Cargo.toml: [[bin]] test-child vẫn còn cho test'

# Cargo giả: trả version hợp lệ; khi build thì tạo CẢ vis lẫn test-child trong target.
mkdir -p "$TMP/bin"
cat >"$TMP/bin/cargo" <<'EOF'
#!/usr/bin/env bash
if [ "${1:-}" = "build" ]; then
    td="${CARGO_TARGET_DIR:?fake cargo cần CARGO_TARGET_DIR}"
    mkdir -p "$td/release"
    printf '#!/bin/sh\nexit 0\n' >"$td/release/vis"
    chmod +x "$td/release/vis"
    printf 'stale test-child — phải bị loại khỏi artifact\n' >"$td/release/test-child"
    exit 0
fi
echo "cargo 1.100.0 (fake test/release)"
EOF
chmod +x "$TMP/bin/cargo"
FAKE_PATH="$TMP/bin:$PATH"

# 3) Lệnh build phải chỉ định `--bin vis`, không dùng `--bins` và không nhắc test-child.
out="$(PATH="$FAKE_PATH" "$REL" --dry-run 2>&1)"
rc=$?
[ "$rc" -eq 0 ]; check $? "vis-release --dry-run exit 0 (thật: $rc)"
printf '%s' "$out" | grep -q -- '--bin vis'; check $? 'dry-run: có `--bin vis`'
printf '%s' "$out" | grep -q -- '--bins'; [ $? -ne 0 ]; check $? 'dry-run: KHÔNG dùng `--bins`'
printf '%s' "$out" | grep -q -- '--bin test-child'; [ $? -ne 0 ]; check $? 'dry-run: không có `--bin test-child`'

# 4) Metadata có test-child ⇒ từ chối trước khi build.
sed 's/release-bins = \["vis"\]/release-bins = ["vis", "test-child"]/' "$CLI" >"$TMP/Cargo.toml"
out="$(VIS_RELEASE_CLI_MANIFEST="$TMP/Cargo.toml" PATH="$FAKE_PATH" "$REL" --dry-run 2>&1)"
rc=$?
[ "$rc" -ne 0 ]; check $? 'metadata có test-child ⇒ bị từ chối'
printf '%s' "$out" | grep -q 'test-child'; check $? 'thông báo từ chối nêu test-child'

# 5) Staging thật: target có test-child nhưng artifact phát hành chỉ có vis (+ giấy phép).
out="$(CARGO_TARGET_DIR="$TMP/target" PATH="$FAKE_PATH" "$REL" --out-dir "$TMP/out" --force 2>&1)"
rc=$?
[ "$rc" -eq 0 ]; check $? "staging exit 0 (thật: $rc; $out)"
[ -x "$TMP/out/vis" ]; check $? 'artifact có vis'
[ ! -e "$TMP/out/test-child" ]; check $? 'artifact KHÔNG có test-child'
[ -e "$TMP/target/release/test-child" ]; check $? 'cargo giả đã tạo test-child trong target (phép kiểm có răng)'
la="$(ls -A "$TMP/out" 2>/dev/null | sort | paste -sd, -)"
[ "$la" = "LICENSE-APACHE,LICENSE-MIT,vis" ]; check $? "artifact chỉ có vis + giấy phép (thật: $la)"

if [ "$fail" -ne 0 ]; then
    printf '# %s mục sai\n' "$fail" >&2
    exit 1
fi
printf '# release: PASS\n'
