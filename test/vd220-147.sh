#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# test/vd220-147.sh — Script đo chuẩn nợ **#147** (VD-197), chuẩn bị bởi **VD-220 làn B (B4)**.
#
# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  KHÔNG tự chạy — Agent 1 bấm trong khung 07:30–07:50 (VD-220 §4),              ║
# ║  máy rảnh, mac + ps cùng HEAD. Xem trước các bước bằng:  bash test/vd220-147.sh --dry
# ╚══════════════════════════════════════════════════════════════════════════════╝
#
# Ràng buộc **VD-197** (DECISIONS.md) — đọc trước khi kết luận:
#   1. Ngưỡng **3×** chỉ áp khi thời gian mac của phép đó **≥ 10 s**; phép dưới 10 s
#      chỉ ghi số, **KHÔNG mở nợ** (VD-197 (4), thay VD-177 §2b).
#   2. Đo đúng cách đo gốc, trên **mac và ps**, **cùng HEAD**, **máy rảnh**,
#      **không agent nào khác chạy**: soak **10 vòng**; cổng `--fast`;
#      `cargo test` **tách thời gian biên dịch** (--no-run) và **thời gian chạy test**.
#   3. Kết luận #147 viết theo **bảng chia thành phần** (giây mac, giây ps, tỉ lệ,
#      nguyên nhân **đo được**) — không suy luận.
#   4. Máy khoá dùng chung: giữ `~/.vifast-measure.lock` (flock) nếu có; log ghi
#      load trước/sau (VD-177 §3).
#
# Script này: **idempotent** (chạy lại nhiều lần OK — tệp kết quả bị ghi đè),
# in tiêu đề "máy rảnh — log đo VD-197/VD-220" ở đầu mỗi log,
# exit != 0 nếu thiếu `cargo`/`python3`. Tệp kết quả: agent/147-bench-<date +%F>.txt
# (**không** `git add` tệp kết quả — chỉ script này được commit).
#
# Chế độ:
#   (không đối số)  chạy đo đầy đủ 6 phần
#   --dry           chỉ in các bước sẽ chạy, không chạy gì (Agent 1 xem trước ở 07:25)
#
# Nền: ps (Linux) ghi hậu tố _PS, mac (Darwin) ghi hậu tố _MAC — cùng HEAD cả hai bên.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# ---- 0. Đối số & kiểm phụ thuộc -------------------------------------------------
DRY=0
for arg in "$@"; do
  case "$arg" in
    --dry) DRY=1 ;;
    -h|--help)
      sed -n '2,40p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *)
      echo "tham số lạ: $arg (chỉ nhận --dry)" >&2
      exit 2
      ;;
  esac
done

log() { printf '%s\n' "$*"; }

if ! command -v cargo >/dev/null 2>&1; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "LỖI: không thấy cargo (đặt PATH hoặc \$HOME/.cargo/bin) — exit 2" >&2
  exit 2
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "LỖI: không thấy python3 (dùng để chấm thời gian float) — exit 2" >&2
  exit 2
fi

export PATH="$HOME/.cargo/bin:$PATH"

HEAD_SHA="$(git rev-parse HEAD)"
HEAD_SHORT="$(git rev-parse --short HEAD)"
DATE_F="$(date +%F)"
OUT_FILE="agent/147-bench-${DATE_F}.txt"

case "$(uname -s)" in
  Darwin) SFX=MAC ;;
  *)      SFX=PS  ;;
esac

NOW() { python3 -c 'import time; print(time.perf_counter())'; }
ELAPSED() { python3 -c "print(f'{float('$2') - float('$1'):.3f}')"; }

write_results() {
  # Ghi key=value — KHÔNG git add tệp này. Ghi đè nếu đã có (idempotent).
  mkdir -p agent
  {
    log "máy rảnh — log đo VD-197/VD-220"
    log "ngay=$(date -Iseconds)"
    log "may=$(hostname)"
    log "nen=$(uname -s)"
    log "head=${HEAD_SHA}"
    log "T_COMPILE_${SFX}=${T_COMPILE:-chưa-đo}"
    log "T_TEST_${SFX}=${T_TEST:-chưa-đo}"
    log "T_SOAK_OK=${T_SOAK_OK:-chưa-đo}"
    log "T_SOAK_${SFX}=${T_SOAK:-chưa-đo}"
    log "T_GATE_${SFX}=${T_GATE:-chưa-đo}"
    log "ghi_chu=ngưỡng 3x chỉ áp khi mac >= 10 s (VD-197 §4); dưới 10 s chỉ ghi số"
  } > "$OUT_FILE"
  log "đã ghi kết quả: $OUT_FILE (không git add)"
}

# ---- --dry: chỉ in các bước, không chạy gì --------------------------------------
if [ "$DRY" -eq 1 ]; then
  cat <<EOF
máy rảnh — log đo VD-197/VD-220  (chế độ --dry, KHÔNG chạy gì)
HEAD hiện tại : ${HEAD_SHA}
nền           : $(uname -s)  → hậu tố _${SFX}
tệp kết quả   : ${OUT_FILE}

Các bước sẽ chạy khi bấm thật (khung 07:30–07:50, máy rảnh, mac + ps cùng HEAD):
  Phần 1  time cargo test --workspace --no-fail-fast --no-run   → T_COMPILE_${SFX} (giây, float)
  Phần 2  time cargo test --workspace --no-fail-fast            → T_TEST_${SFX} (giây, float)
  Phần 3  soak 10 vòng: cargo test --workspace --no-fail-fast
          rồi: ./vis test test/sem   và   ./vis test --vm test/sem
          → T_SOAK_OK (0 = mọi vòng xanh; 1 = có vòng đỏ → phát hiện flake, exit 3)
          → T_SOAK_${SFX} (tổng giây)
  Phần 4  ./vis-dev gate --fast                                → T_GATE_${SFX}; FAIL → exit 4
  Phần 5  in lệnh ssh chạy trên mac + tên tệp kết quả
  Phần 6  ghi key=value vào ${OUT_FILE} (SECONDS/python3, float)

Lệnh chạy thật trên ps:  bash test/vd220-147.sh
Lệnh chạy thật trên mac: ssh mac "cd ~/ViScript && git fetch && git checkout ${HEAD_SHA} && bash test/vd220-147.sh"
EOF
  exit 0
fi

# ---- Đầu log --------------------------------------------------------------------
log "máy rảnh — log đo VD-197/VD-220"
log "ngày: $(date -Iseconds) · máy: $(hostname) · nền: $(uname -s) · HEAD: ${HEAD_SHA}"
log "load trước khi đo: $(uptime)"
log "ràng buộc VD-197: ngưỡng 3× chỉ khi mac ≥ 10 s; soak 10 vòng; cargo test tách biên dịch/chạy; kèm gate --fast"
log "LƯU Ý: Agent 1 phải chắc máy rảnh (không agent nào đang chạy) TRƯỚC khi bấm — script không tự kiểm điều đó"
log ""

T_COMPILE=""; T_TEST=""; T_SOAK=""; T_GATE=""; T_SOAK_OK=""

# ---- Phần 1 — Biên dịch (cargo test --no-run) ------------------------------------
log "== Phần 1/6 — biên dịch: time cargo test --workspace --no-fail-fast --no-run"
_t0="$(NOW)"
if time cargo test --workspace --no-fail-fast --no-run; then _rc=0; else _rc=$?; fi
_t1="$(NOW)"
T_COMPILE="$(ELAPSED "$_t0" "$_t1")"
log "T_COMPILE_${SFX}=${T_COMPILE}  (exit=${_rc})"
if [ "$_rc" -ne 0 ]; then
  log "LỖI: biên dịch test thất bại (exit ${_rc}) — dừng, ghi kết quả dở dang"
  write_results
  exit "$_rc"
fi
log ""

# ---- Phần 2 — Chạy test (cargo test) ----------------------------------------------
log "== Phần 2/6 — chạy test: time cargo test --workspace --no-fail-fast"
_t0="$(NOW)"
if time cargo test --workspace --no-fail-fast; then _rc=0; else _rc=$?; fi
_t1="$(NOW)"
T_TEST="$(ELAPSED "$_t0" "$_t1")"
log "T_TEST_${SFX}=${T_TEST}  (exit=${_rc})"
if [ "$_rc" -ne 0 ]; then
  log "LỖI: cargo test đỏ (exit ${_rc}) — dừng, ghi kết quả dở dang"
  write_results
  exit "$_rc"
fi
log ""

# ---- Phần 3 — Soak 10 vòng + vis test (interp & VM) --------------------------------
log "== Phần 3/6 — soak 10 vòng: cargo test --workspace --no-fail-fast; rồi ./vis test test/sem và ./vis test --vm test/sem"
_t0="$(NOW)"
T_SOAK_OK=0
_soak_fail=""
for _i in $(seq 1 10); do
  log "-- soak vòng ${_i}/10"
  if ! cargo test --workspace --no-fail-fast; then
    T_SOAK_OK=1
    _soak_fail="cargo test vòng ${_i}"
    break
  fi
done
if [ "$T_SOAK_OK" -eq 0 ]; then
  log "-- soak: ./vis test test/sem (interp)"
  if ! ./vis test test/sem; then
    T_SOAK_OK=1
    _soak_fail="./vis test test/sem"
  fi
fi
if [ "$T_SOAK_OK" -eq 0 ]; then
  log "-- soak: ./vis test --vm test/sem (VM)"
  if ! ./vis test --vm test/sem; then
    T_SOAK_OK=1
    _soak_fail="./vis test --vm test/sem"
  fi
fi
_t1="$(NOW)"
T_SOAK="$(ELAPSED "$_t0" "$_t1")"
log "T_SOAK_OK=${T_SOAK_OK}  T_SOAK_${SFX}=${T_SOAK}"
if [ "$T_SOAK_OK" -ne 0 ]; then
  log "LỖI: có vòng đỏ ở [${_soak_fail}] — phát hiện flake? ghi T_SOAK_OK=1, exit 3"
  write_results
  exit 3
fi
log ""

# ---- Phần 4 — Cổng ./vis-dev gate --fast -------------------------------------------
log "== Phần 4/6 — cổng: ./vis-dev gate --fast"
_t0="$(NOW)"
if ./vis-dev gate --fast; then _rc=0; else _rc=$?; fi
_t1="$(NOW)"
T_GATE="$(ELAPSED "$_t0" "$_t1")"
log "T_GATE_${SFX}=${T_GATE}  (exit=${_rc})"
if [ "$_rc" -ne 0 ]; then
  log "LỖI: gate --fast FAIL (exit ${_rc}) — ghi kết quả dở dang, exit 4"
  write_results
  exit 4
fi
log ""

# ---- Phần 5 — Hướng dẫn chạy trên mac ----------------------------------------------
log "== Phần 5/6 — chạy trên mac (cùng HEAD, máy rảnh)"
log "Lệnh ssh đầy đủ:"
log "  ssh mac \"cd ~/ViScript && git fetch && git checkout ${HEAD_SHA} && bash test/vd220-147.sh\""
log "Tệp kết quả (cả hai bên, cùng tên): agent/147-bench-${DATE_F}.txt  (ps: ${ROOT}/${OUT_FILE}; mac: ~/ViScript/${OUT_FILE})"
log "So sánh ps vs mac theo từng thành phần (T_COMPILE/T_TEST/T_SOAK/T_GATE); kết luận #147 chỉ khi mac ≥ 10 s (VD-197 §4)"
log ""

# ---- Phần 6 — Ghi kết quả -----------------------------------------------------------
log "== Phần 6/6 — ghi kết quả (key=value)"
log "load sau khi đo: $(uptime)"
write_results
log "Xong. Nhớ: KHÔNG git add ${OUT_FILE} — chỉ commit script test/vd220-147.sh"
exit 0
