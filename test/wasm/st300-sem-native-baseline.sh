#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# ST-300 — baseline native interp↔VM cho test/sem/**/*.vis (không sửa sản phẩm).
# Dùng: B=/path/vis ROOT=/goc/repo OUT=/tmp/baseline.tsv ./test/wasm/st300-sem-native-baseline.sh
# Mặc định giữ nguyên đường dẫn bằng chứng ST-300; đặt biến môi trường để chạy lại ở cây khác.
set -u
B=${B:-/home/ps/ViScript/target/vs3-w1-sem/release/vis}
ROOT=${ROOT:-/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w1-runtime}
OUT=${OUT:-/home/ps/vislogs/sem-native-baseline.tsv}
EXC=${EXC:-/home/ps/vislogs/sem-native-exclusions.tsv}
T=${T:-/home/ps/tmp/semrun}
mkdir -p "$T" "$(dirname "$OUT")"
cd "$ROOT"
printf 'path\tcategory\tinterp_rc\tinterp_stdout_sha256\tvm_rc\tvm_stdout_sha256\n' > "$OUT"
printf 'path\tcategory\treason\n' > "$EXC"
n=0; ex=0
find test/sem -name '*.vis' | sort | while IFS= read -r f; do
  cat=$(printf '%s' "$f" | cut -d/ -f3)
  timeout 20 "$B" run "$f" >"$T/o1" 2>"$T/e1"; r1=$?
  timeout 20 "$B" run --vm "$f" >"$T/o2" 2>"$T/e2"; r2=$?
  if [ "$r1" -eq 124 ] || [ "$r2" -eq 124 ]; then
    printf '%s\t%s\tTIMEOUT interp_rc=%s vm_rc=%s\n' "$f" "$cat" "$r1" "$r2" >> "$EXC"; continue
  fi
  if grep -q 'không đọc được' "$T/e1" "$T/e2" 2>/dev/null; then
    printf '%s\t%s\tREAD_ERROR\n' "$f" "$cat" >> "$EXC"; continue
  fi
  if [ "$r1" -ge 128 ] || [ "$r2" -ge 128 ]; then
    printf '%s\t%s\tCRASH interp_rc=%s vm_rc=%s\n' "$f" "$cat" "$r1" "$r2" >> "$EXC"; continue
  fi
  h1=$(sha256sum "$T/o1" | cut -d' ' -f1)
  h2=$(sha256sum "$T/o2" | cut -d' ' -f1)
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$f" "$cat" "$r1" "$h1" "$r2" "$h2" >> "$OUT"
done
echo "done"
