#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# ST-144 (VD-146, VD-136) — đo lại 4 phép đo trên máy Windows `psvn` bằng **Git Bash**.
#
# Vì sao có tệp này: ST-136 đo tay từng lệnh; VD-136 đòi tái lập bằng script trong repo. Script này
# chạy đúng 4 lệnh đo của ST-144, mỗi lệnh một log riêng mang ID task (`st144-*.log`, VD-147(5)) và
# đầu mỗi log có header môi trường VD-136 (máy, OS/kernel, hash đang đo, toolchain, target đã cài).
#
# Dùng (trên `psvn`, qua Git Bash — KHÔNG dùng bash của WSL):
#   ssh psvn '& "C:\Program Files\Git\bin\bash.exe" -lc "cd /c/Users/ps/vifast-windows/repo/viscript/infra/windows && bash measure.sh"'
#
# Mã thoát: 0 = cả 4 bước xanh · 1 = có ít nhất một bước đỏ (số bước đỏ in ở stderr).
# Script **không** bỏ qua lỗi: mỗi bước ghi `[exit=N]` ở cuối log và lỗi được cộng dồn vào mã thoát;
# vẫn chạy tiếp các bước sau để có **đủ** 4 phép đo trên cùng một cây nguồn (Windows chỉ ĐO — VD-121).
#
# Không cài gì, không `rustup update`, không đổi toolchain mặc định, không reboot. Toolchain được
# GHIM (VD-140): `rust-toolchain.toml` khai `stable`, mà trên `psvn` "stable" không phải 1.99.1.
set -u

VIS="$(cd "$(dirname "$0")/../.." && pwd)"
JOURNAL="$VIS/agent/JOURNAL"
export RUSTUP_TOOLCHAIN="1.99.1-x86_64-pc-windows-msvc"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-C:/vis-target}"   # target dir riêng (VD-136)
export CARGO_TERM_COLOR=never
export RUST_BACKTRACE=1
cd "$VIS" || { echo "measure: không vào được $VIS" >&2; exit 1; }

utc() { date -u +%Y-%m-%dT%H:%M:%SZ; }
so_hong=0

# Header môi trường VD-136 (mọi mục đều có tiền tố `#` để phần kết quả của lệnh dễ đọc).
ghi_header() { # $1 = tên bước, $2 = lệnh (chuỗi hiển thị)
    local ten="$1" lenh="$2" log="$JOURNAL/st144-$1.log"
    {
        echo "# ST-144 (VD-146, VD-136) — log đo trên Windows (Git Bash); bước: $ten"
        echo "# thoi_diem_utc=$(utc)"
        echo "# may=$(hostname 2>&1)"
        echo "# os=$(uname -sr 2>&1)"
        echo "# git_head=$(git -C .. rev-parse HEAD 2>&1)"
        echo "# git_branch=$(git -C .. rev-parse --abbrev-ref HEAD 2>&1)"
        echo "# git_dirty=$(git -C .. status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
        echo "# core.autocrlf=$(git -C .. config core.autocrlf 2>&1)"
        echo "# rustc=$(rustc -V 2>&1)"
        echo "# cargo=$(cargo -V 2>&1)"
        echo "# rustup_toolchain=$RUSTUP_TOOLCHAIN"
        echo "# rustup_targets=$(rustup target list --installed 2>&1 | tr '\n' ' ')"
        echo "# cargo_target_dir=$CARGO_TARGET_DIR"
        echo "# cwd=$(pwd 2>&1)"
        echo "# lenh=$lenh"
        echo "# bat_dau_utc=$(utc)"
        echo "# ---"
    } >"$log"
    echo "measure: bắt đầu $ten → $log" >&2
}

# Chạy một bước: ghi header, chạy lệnh (stdout+stderr vào log), ghi footer `[exit=]`.
chay_buoc() { # $1 = tên bước; phần còn lại = lệnh
    local ten="$1"
    shift
    local log="$JOURNAL/st144-$ten.log"
    local t0 t1 rc
    ghi_header "$ten" "$*"
    t0=$(date +%s)
    "$@" >>"$log" 2>&1
    rc=$?
    t1=$(date +%s)
    {
        echo "# ---"
        echo "# ket_thuc_utc=$(utc)"
        echo "# thoi_gian_s=$((t1 - t0))"
        echo "[exit=$rc]"
    } >>"$log"
    echo "measure: xong $ten → $log ([exit=$rc], $((t1 - t0)) s)" >&2
    if [ "$rc" -ne 0 ]; then so_hong=$((so_hong + 1)); fi
}

echo "measure: may=$(hostname) os=$(uname -sr) head=$(git -C .. rev-parse --short HEAD) toolchain=$RUSTUP_TOOLCHAIN" >&2

# 4 phép đo của ST-144, đúng thứ tự brief. Không `|| true`: mỗi lỗi vào `[exit=]` + mã thoát cuối.
chay_buoc cargo-test cargo test --workspace --no-fail-fast
chay_buoc host-suite bash bin/vis-host-suite
chay_buoc vis-targets bash bin/vis-targets
chay_buoc gate-fast bash bin/vis-dev gate --fast

echo "measure: xong 4 bước; so_buoc_do=$so_hong" >&2
echo "measure: log: $JOURNAL/st144-{cargo-test,host-suite,vis-targets,gate-fast}.log" >&2
if [ "$so_hong" -ne 0 ]; then exit 1; fi
exit 0
