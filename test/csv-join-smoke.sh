#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# csv-join-smoke.sh — smoke test cho vi-du/csv-join.vis trên cả hai engine (interp + VM).
#
#   test/csv-join-smoke.sh           # dùng target/debug/vis (build nếu chưa có)
#   VIS=/đường/dẫn/vis test/csv-join-smoke.sh
#
# Kiểm: mã thoát + nội dung tệp kết quả (so với vi-du/*.out) + một số dòng báo cáo.
# Không cần mạng; fixtures nằm trong vi-du/.
set -u
cd "$(dirname "${BASH_SOURCE[0]}")/.."

VIS="${VIS:-}"
if [ -z "$VIS" ]; then
    if [ -x target/debug/vis ]; then
        VIS=target/debug/vis
    elif command -v vis >/dev/null 2>&1; then
        VIS=vis
    else
        echo "vis chưa build — chạy: cargo build --bin vis" >&2
        cargo build --bin vis || exit 2
        VIS=target/debug/vis
    fi
fi
echo "vis: $VIS"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

PASS=0
FAIL=0
TOTAL=0
ok_case()   { printf 'ok %s\n' "$1"; PASS=$((PASS + 1)); TOTAL=$((TOTAL + 1)); }
bad_case()  { printf 'FAIL %s\n' "$1"; FAIL=$((FAIL + 1)); TOTAL=$((TOTAL + 1)); }

run_engine() {   # <engine> <args...> — luôn kèm `--` trước tham số chương trình
    local engine="$1"; shift
    if [ "$engine" = vm ]; then
        "$VIS" run --vm vi-du/csv-join.vis -- "$@"
    else
        "$VIS" run vi-du/csv-join.vis -- "$@"
    fi
}

# expect <tên> <engine> <exit-kỳ-vọng> <expected.out|-> <grep-stdout|-> <args...>
# Nếu expected != "-" thì tham số $out (đường dẫn kết quả) được thêm vào cuối args.
expect() {
    local name="$1" engine="$2" want_exit="$3" expected="$4" grep1="$5"; shift 5
    local out=""
    if [ "$expected" != "-" ]; then
        out="$TMP/${name//[^A-Za-z0-9_.-]/_}.csv"
        run_engine "$engine" "$@" "$out" >"$TMP/stdout" 2>"$TMP/stderr"
    else
        run_engine "$engine" "$@" >"$TMP/stdout" 2>"$TMP/stderr"
    fi
    local code=$?
    if [ "$code" -ne "$want_exit" ]; then
        bad_case "$name#$engine (exit $code, kỳ vọng $want_exit)"
        sed 's/^/    stdout: /' "$TMP/stdout"; sed 's/^/    stderr: /' "$TMP/stderr"
        return
    fi
    if [ "$expected" != "-" ] && ! cmp -s "$expected" "$out"; then
        bad_case "$name#$engine (output khác kỳ vọng $expected)"
        diff "$expected" "$out" | head -20
        return
    fi
    if [ "$grep1" != "-" ] && ! grep -qF "$grep1" "$TMP/stdout"; then
        bad_case "$name#$engine (stdout thiếu: $grep1)"
        sed 's/^/    stdout: /' "$TMP/stdout"
        return
    fi
    ok_case "$name#$engine"
}

for engine in interp vm; do
    expect csv-ok    "$engine" 0 vi-du/csv-ok.out   '# dòng kết quả: 3' vi-du/csv-ok-a.csv vi-du/csv-ok-b.csv
    expect csv-join  "$engine" 1 vi-du/csv-join.out 'thiếu ID: 0015'   vi-du/csv-a.csv vi-du/csv-b.csv
    expect csv-dup   "$engine" 1 vi-du/csv-dup.out  'ID trùng: 0021'   vi-du/csv-dup-a.csv vi-du/csv-dup-b.csv
    expect csv-miss  "$engine" 1 vi-du/csv-miss.out 'thiếu ID: 0032'   vi-du/csv-miss-a.csv vi-du/csv-miss-b.csv
    expect csv-crlf  "$engine" 0 vi-du/csv-crlf.out '-'                vi-du/csv-crlf-a.csv vi-du/csv-crlf-b.csv
    expect csv-no-id "$engine" 1 -                  'cột id'           vi-du/csv-no-id.csv vi-du/csv-ok-b.csv "$TMP/no-id.$engine.csv"
    expect csv-overwrite "$engine" 2 -              'trùng tệp đầu vào' vi-du/csv-ok-a.csv vi-du/csv-ok-b.csv vi-du/csv-ok-a.csv
    expect csv-argc-2 "$engine" 2 -                 'cần đúng 3 tham số' vi-du/csv-ok-a.csv
    expect csv-argc-4 "$engine" 2 -                 'cần đúng 3 tham số' vi-du/csv-ok-a.csv vi-du/csv-ok-b.csv vi-du/csv-ok-a.csv vi-du/csv-ok-b.csv

    # --output <p> (tham số cờ, không phải vị trí)
    out="$TMP/csv-output-flag.$engine.csv"
    run_engine "$engine" --output "$out" vi-du/csv-ok-a.csv vi-du/csv-ok-b.csv >"$TMP/stdout" 2>"$TMP/stderr"
    code=$?
    if [ "$code" -eq 0 ] && cmp -s vi-du/csv-ok.out "$out"; then
        ok_case "csv-output-flag#$engine"
    else
        bad_case "csv-output-flag#$engine (exit $code)"
    fi

    # không tham số nào ⇒ usage + exit 2
    if [ "$engine" = vm ]; then
        "$VIS" run --vm vi-du/csv-join.vis >"$TMP/stdout" 2>"$TMP/stderr"
    else
        "$VIS" run vi-du/csv-join.vis >"$TMP/stdout" 2>"$TMP/stderr"
    fi
    code=$?
    if [ "$code" -eq 2 ] && grep -qF 'dùng: vis run vi-du/csv-join.vis' "$TMP/stdout"; then
        ok_case "csv-no-args#$engine"
    else
        bad_case "csv-no-args#$engine (exit $code)"
    fi
done

echo
echo "tổng: $TOTAL · đạt: $PASS · hỏng: $FAIL"
if [ "$FAIL" -gt 0 ]; then
    printf '✗ %d case hỏng\n' "$FAIL"
    exit 1
fi
printf '✓ tất cả %d case đạt (interp + VM)\n' "$PASS"
