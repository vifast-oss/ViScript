#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Nợ #185 — lệnh chuẩn chọn đúng toolchain hoặc thất bại rõ, không cần sửa PATH ngầm.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin" "$TMP/home"
cat > "$TMP/bin/cargo" <<'EOF'
#!/usr/bin/env bash
echo "cargo 1.93.1 (fake-old)"
EOF
chmod +x "$TMP/bin/cargo"

fail=0
check() {
    if [ "$1" -eq 0 ]; then printf 'ok - %s\n' "$2"; else printf 'not ok - %s\n' "$2"; fail=$((fail+1)); fi
}

# 1) PATH có cargo cũ, HOME không có fallback ⇒ lệnh chuẩn thất bại rõ (EX_CONFIG=78).
out="$(PATH="$TMP/bin:/usr/bin:/bin" HOME="$TMP/home" "$ROOT/bin/vis" --version 2>&1)"
rc=$?
[ "$rc" -eq 78 ]; check $? 'vis: cargo cũ + không có fallback ⇒ exit 78'
printf '%s' "$out" | grep -q '1.93.1'; check $? 'vis: thông báo nêu phiên bản quá cũ'
printf '%s' "$out" | grep -q 'yêu cầu ≥ 1.99.1'; check $? 'vis: thông báo nêu yêu cầu workspace'
printf '%s' "$out" | grep -q 'cargo trên PATH'; check $? 'vis: thông báo nêu ứng viên đã thử'

# 2) PATH có cargo cũ, HOME có rustup default đạt ⇒ helper chọn fallback, không sửa PATH.
out="$(PATH="$TMP/bin:/usr/bin:/bin" HOME="$HOME" bash -c \
    "source '$ROOT/bin/_vis-toolchain.sh'; vis_find_cargo vis" 2>/dev/null)"
[ "$out" = "$HOME/.cargo/bin/cargo" ]; check $? 'vis: fallback rustup default được chọn khi PATH cũ'

# 3) PATH sạch có cargo đạt ⇒ helper tôn trọng cargo đang hoạt động.
out="$(PATH="$HOME/.cargo/bin:/usr/bin:/bin" HOME="$HOME" bash -c \
    "source '$ROOT/bin/_vis-toolchain.sh'; vis_find_cargo vis" 2>/dev/null)"
[ "$out" = "$HOME/.cargo/bin/cargo" ]; check $? 'vis: tôn trọng cargo đang hoạt động trên PATH'

if [ "$fail" -ne 0 ]; then
    printf '# %s mục sai\n' "$fail" >&2
    exit 1
fi
printf '# toolchain: PASS\n'
