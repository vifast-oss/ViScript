#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# ST-134 (VD-136) — tái lập **thành phần toolchain** cho runner Linux (`ps`) bằng script trong repo.
#
# Vì sao cần: `bin/vis-targets` (ma trận `cargo check --target`) cần std của các target phụ để CHECK
# chéo. Trên runner `ps` thiếu `wasm32-unknown-unknown` (R23 §4.4 lệch 5) và `x86_64-pc-windows-msvc`
# (ô Windows của ma trận 18 ô). Cả hai là **thành phần toolchain** (rustup target), KHÔNG phải phần
# mềm hệ thống ⇒ được cài bằng rustup **trong script này** (VD-136), không cài tay.
#
# Dùng:
#   bash viscript/infra/linux/setup.sh                  # kiểm + cài target còn thiếu (idempotent)
#   VIS_SETUP_CHECK=1 bash viscript/infra/linux/setup.sh # CHỈ kiểm; thiếu ⇒ exit 1, không cài
#   VIS_SETUP_LOG=<tệp> bash ...                        # ghi thêm log (mặc định $HOME/.cache/vis-setup.log)
#
# Mã thoát: 0 xanh · 1 thiếu target (chế độ CHECK) · 3 BLOCKED-ENV (không có rustup).
# Không `rustup update`, không đổi toolchain mặc định, không cài gói OS, không `sudo`.
set -u

TARGETS=(wasm32-unknown-unknown x86_64-pc-windows-msvc)
LOG="${VIS_SETUP_LOG:-$HOME/.cache/vis-setup.log}"
CHECK="${VIS_SETUP_CHECK:-0}"
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

sha256_stdin() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then shasum -a 256 | cut -d' ' -f1
    else echo "khong-co-sha256"; fi
}

# Ghi cả ra stdout (để `bin/vis-linux setup` bắt) và nối vào tệp log trên máy.
say() { printf '%s\n' "$*"; }
log_env() { # $1 = chuỗi nội dung đa dòng
    mkdir -p "$(dirname "$LOG")" 2>/dev/null || true
    printf '%s\n' "$1" >>"$LOG" 2>/dev/null || say "setup: KHÔNG ghi được log $LOG (vẫn tiếp tục)"
}

if ! command -v rustup >/dev/null 2>&1; then
    say "setup: KHÔNG có rustup trong PATH=$PATH (BLOCKED-ENV)"
    exit 3
fi
if ! command -v rustc >/dev/null 2>&1; then
    say "setup: KHÔNG có rustc trong PATH=$PATH (BLOCKED-ENV)"
    exit 3
fi

# ----- 1. Gom môi trường (VD-136: tên máy, OS, kernel, rustc -Vv, target, hash commit) -----
host="$(hostname 2>/dev/null || echo '?')"
os="$( ( . /etc/os-release 2>/dev/null && printf '%s' "${PRETTY_NAME:-}" ) || uname -s )"
kernel="$(uname -srm)"
rustc_vv="$(rustc -Vv)"
cargo_v="$(cargo -V 2>/dev/null || echo 'cargo: none')"
rustup_v="$(rustup --version 2>/dev/null | head -1 || echo 'rustup: none')"
active="$(rustup show active-toolchain 2>/dev/null | head -1 || echo '?')"
truoc="$(rustup target list --installed 2>/dev/null)"
head_hash="$(git rev-parse HEAD 2>/dev/null || echo 'khong-phai-git')"
script_sha="$(sha256_stdin <"$0")"
digest="$( {
    printf '%s\n' "$rustc_vv" "$cargo_v" "$rustup_v" "$active" "$truoc" "git=$head_hash"
} | sha256_stdin )"

env_log="$(cat <<EOS
=== setup toolchain (VD-136) $(date -u +%Y-%m-%dT%H:%M:%SZ) ===
hostname: $host
os: $os
kernel: $kernel
rustc: $(printf '%s' "$rustc_vv" | head -1)
host-target: $(printf '%s' "$rustc_vv" | sed -n 's/^host: //p')
$rustc_vv
$cargo_v
$rustup_v
active-toolchain: $active
git HEAD: $head_hash
setup.sh sha256: $script_sha
digest-toolchain: $digest
target cần: ${TARGETS[*]}
target đang có (trước): $(printf '%s' "$truoc" | tr '\n' ' ')
EOS
)" # dấu " ở đây chỉ để tránh `set -u` coi $(...) là lỗi khi rỗng
log_env "$env_log"
say "$env_log"
say "---"

# ----- 2. Kiểm/cài từng target (chỉ rustup target add) -----
thieu=0
for t in "${TARGETS[@]}"; do
    if printf '%s\n' "$truoc" | grep -qx "$t"; then
        say "target $t: đã cài (không cài lại)"
        continue
    fi
    if [ "$CHECK" = 1 ]; then
        say "target $t: THIẾU (VIS_SETUP_CHECK=1 ⇒ không cài)"
        thieu=$((thieu + 1))
        continue
    fi
    say "target $t: đang cài bằng \`rustup target add $t\` (thành phần toolchain, không phải gói OS)"
    if ! rustup target add "$t"; then
        say "target $t: CÀI THẤT BẠI (BLOCKED-ENV: mạng/rustup)"
        exit 3
    fi
done

# ----- 3. Xác nhận sau cài + digest mới -----
sau="$(rustup target list --installed 2>/dev/null)"
say "target đang có (sau): $(printf '%s' "$sau" | tr '\n' ' ')"
thieu_sau=0
for t in "${TARGETS[@]}"; do
    printf '%s\n' "$sau" | grep -qx "$t" || { say "target $t: THIẾU sau khi cài"; thieu_sau=$((thieu_sau + 1)); }
done
digest_moi="$( {
    printf '%s\n' "$rustc_vv" "$cargo_v" "$rustup_v" "$active" "$sau" "git=$head_hash"
} | sha256_stdin )"
say "digest-toolchain (sau): $digest_moi"
say "setup.sh sha256: $script_sha"

ket_thuc="=== setup kết thúc: HEAD=$head_hash digest=$digest_moi ==="
log_env "$ket_thuc"
if [ "$thieu_sau" -ne 0 ] || [ "$thieu" -ne 0 ]; then
    say "setup: THIẾU $((thieu + thieu_sau)) target (BLOCKED-ENV) — chạy lại không có VIS_SETUP_CHECK=1"
    exit 1
fi
say "setup: OK (${#TARGETS[@]}/${#TARGETS[@]} target; ${rustc_vv%%$'\n'*})"
exit 0
