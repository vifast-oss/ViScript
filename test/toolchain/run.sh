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

# **VD-258**: kỳ vọng lấy từ chính `Cargo.toml` (một nguồn), không hardcode — pin cũ 1.99.1
# từng lệch workspace 1.100.0 mà test vẫn xanh. `_vis-toolchain.sh` phải parse được bằng `sed`
# POSIX (macOS BSD không có `grep -P`).
REQ="$(VIS_ROOT="$ROOT" bash -c 'source "$VIS_ROOT/bin/_vis-toolchain.sh"; vis_required_rust_version')"
CHANNEL="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT/rust-toolchain.toml" | head -n1)"
[ -n "$REQ" ]; check $? 'toolchain: parse được rust-version từ Cargo.toml'
[ "$CHANNEL" = "$REQ" ]; check $? "toolchain: rust-toolchain.toml ($CHANNEL) khớp rust-version ($REQ)"

# 1) PATH có cargo cũ, HOME không có fallback ⇒ lệnh chuẩn thất bại rõ (EX_CONFIG=78).
out="$(PATH="$TMP/bin:/usr/bin:/bin" HOME="$TMP/home" "$ROOT/bin/vis" --version 2>&1)"
rc=$?
[ "$rc" -eq 78 ]; check $? 'vis: cargo cũ + không có fallback ⇒ exit 78'
printf '%s' "$out" | grep -q '1.93.1'; check $? 'vis: thông báo nêu phiên bản quá cũ'
printf '%s' "$out" | grep -q "yêu cầu ≥ $REQ"; check $? 'vis: thông báo nêu yêu cầu workspace'
printf '%s' "$out" | grep -q 'cargo trên PATH'; check $? 'vis: thông báo nêu ứng viên đã thử'

# 2/3) Chỉ kiểm fallback `$HOME/.cargo/bin/cargo` khi máy **có** nó (rustup chuẩn). Máy chỉ có
# cargo hệ thống vẫn phải xanh ở các mục còn lại — không biến một bố cục máy thành lỗi pin.
if [ -x "$HOME/.cargo/bin/cargo" ]; then
    # 2) PATH có cargo cũ, HOME có rustup default đạt ⇒ helper chọn fallback, không sửa PATH.
    out="$(PATH="$TMP/bin:/usr/bin:/bin" HOME="$HOME" bash -c \
        "source '$ROOT/bin/_vis-toolchain.sh'; vis_find_cargo vis" 2>/dev/null)"
    [ "$out" = "$HOME/.cargo/bin/cargo" ]; check $? 'vis: fallback rustup default được chọn khi PATH cũ'

    # 3) PATH sạch có cargo đạt ⇒ helper tôn trọng cargo đang hoạt động.
    out="$(PATH="$HOME/.cargo/bin:/usr/bin:/bin" HOME="$HOME" bash -c \
        "source '$ROOT/bin/_vis-toolchain.sh'; vis_find_cargo vis" 2>/dev/null)"
    [ "$out" = "$HOME/.cargo/bin/cargo" ]; check $? 'vis: tôn trọng cargo đang hoạt động trên PATH'
else
    printf 'ok - vis: bỏ qua fallback rustup (không có %s)\n' "$HOME/.cargo/bin/cargo"
fi

# 4) **VD-258** — pin hạ tầng phải theo cùng `rust-version`; lệch là đỏ (chống trôi im lặng).
grep -q "FROM rust:$REQ-bookworm" "$ROOT/infra/linux/Dockerfile"
check $? "infra/linux/Dockerfile: FROM rust:$REQ-bookworm"
grep -Eq "RUSTUP_TOOLCHAIN=\"?$REQ-x86_64-pc-windows-msvc\"?" "$ROOT/infra/windows/measure.sh"
check $? "infra/windows/measure.sh: ghim $REQ-x86_64-pc-windows-msvc"
grep -q "\$Toolchain = \"$REQ-x86_64-pc-windows-msvc\"" "$ROOT/infra/windows/setup.ps1"
check $? "infra/windows/setup.ps1: \$Toolchain = $REQ-x86_64-pc-windows-msvc"
grep -q "$REQ-x86_64-pc-windows-msvc" "$ROOT/infra/windows/setup.md"
check $? "infra/windows/setup.md: nêu $REQ-x86_64-pc-windows-msvc"

if [ "$fail" -ne 0 ]; then
    printf '# %s mục sai\n' "$fail" >&2
    exit 1
fi
printf '# toolchain: PASS\n'
