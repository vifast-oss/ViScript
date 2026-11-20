# SPDX-License-Identifier: MIT OR Apache-2.0
# _vis-toolchain.sh — chọn cargo đúng hợp đồng tái lập (VD-241 §3/T6, nợ #185).
#
# Dùng: source "$(dirname "$0")/_vis-toolchain.sh" trong bin/vis, bin/vis-dev, …
#   CARGO="$(vis_find_cargo "vis")" || exit 78
#
# Luật (giữ nguyên VD-241):
#   1) tôn trọng cargo đang hoạt động trên PATH (RUSTUP_TOOLCHAIN/override…) nếu đạt
#      `rust-version` của workspace; KHÔNG hardcode ưu tiên ~/.cargo/bin;
#   2) không đạt ⇒ thử rustup default ở $HOME/.cargo/bin/cargo;
#   3) không có ⇒ báo rõ hai ứng viên + yêu cầu, exit 78 (EX_CONFIG) do caller quyết.
# Không sửa PATH ngầm — caller nhận đường dẫn tuyệt đối và tự quyết.

vis_required_rust_version() {
    local root="${VIS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
    grep -m1 -oP '^rust-version\s*=\s*"\K[^"]+' "$root/Cargo.toml" 2>/dev/null || echo 1.99.1
}

vis_semver_of() {
    "$1" --version 2>/dev/null | head -1 | sed -E 's/.* ([0-9]+\.[0-9]+\.[0-9]+).*/\1/'
}

vis_rust_ge() {
    [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | head -n1)" = "$2" ]
}

# $1 = nhãn prefix cho chẩn đoán (vis / vis-dev / …)
vis_find_cargo() {
    local prefix="${1:-vis}" req active_ver
    req="$(vis_required_rust_version)"
    if command -v cargo >/dev/null 2>&1; then
        active_ver="$(vis_semver_of "$(command -v cargo)")"
        if [ -n "$active_ver" ] && vis_rust_ge "$active_ver" "$req"; then
            command -v cargo
            return 0
        fi
        echo "$prefix: cargo trên PATH quá cũ ($active_ver; yêu cầu ≥ $req)" >&2
    else
        echo "$prefix: không có cargo trên PATH" >&2
    fi
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        local fbv
        fbv="$(vis_semver_of "$HOME/.cargo/bin/cargo")"
        if [ -n "$fbv" ] && vis_rust_ge "$fbv" "$req"; then
            echo "$HOME/.cargo/bin/cargo"
            return 0
        fi
        echo "$prefix: $HOME/.cargo/bin/cargo cũng quá cũ ($fbv; yêu cầu ≥ $req)" >&2
    else
        echo "$prefix: không có $HOME/.cargo/bin/cargo" >&2
    fi
    echo "$prefix: không thấy cargo ≥ $req (yêu cầu của workspace)." >&2
    echo "         đã thử: (1) cargo trên PATH, (2) \$HOME/.cargo/bin/cargo" >&2
    echo "         cài rustup và cập nhật: https://rustup.rs — hoặc đặt RUSTUP_TOOLCHAIN=stable" >&2
    return 1
}
