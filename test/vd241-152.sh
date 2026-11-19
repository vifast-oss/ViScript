#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# test/vd241-152.sh — đo A/B xen kẽ cho nợ #152 (enum-hoá Option/Result).
#
# ╔═══════════════════════════════════════════════════════════════════════╗
# ║  MÁY RẢNH BẮT BUỘC (load < 0,3, VD-217). Không agent nào khác chạy.      ║
# ║  Kết quả ghi ra stdout + /tmp (KHÔNG git add).                            ║
# ╚═══════════════════════════════════════════════════════════════════════╝
#
# A = 7c256df (trước enum-hoá Option/Result) · B = HEAD của cây hiện tại.
# Phép: 24 cặp A,B,A,B,… xen kẽ, subprocess mới mỗi lần, `vis run --no-cache`.
# Hai đường: interp (mặc định) và `--vm`. Release build từng cây.
# Verdict #152 chỉ khi **khoả cáp/khoảng nằm hẳn một phía 1,10** cho cả VM và interp.
# Nếu chưa rõ ⇒ giữ "chờ đo chuẩn" (VD-217/VD-241); KHÔNG tối ưu ctor-enum.
#
# Dùng:  bash test/vd241-152.sh
# Nhớ:   cần free > 2,5 Gi ở /tmp và ~/.cargo
set -euo pipefail
if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
A_SHA="${1:-7c256df}"
B_SHA="$(git rev-parse HEAD)"
A_WT="/tmp/opencode/ab-A-$$"
B_WT="/tmp/opencode/ab-B-$$"
RES="/tmp/opencode/ab-result-vd241-$$.json"
rm -rf "$A_WT" "$B_WT"
echo "máy rảnh — đo #152 (VD-241 §4 việc 3)"; uptime
cargo build --release -p vis-cli --bin vis 2>/dev/null | tail -1 >/dev/null # no-op ensure toolchain
git worktree add --detach "$A_WT" "$A_SHA" >/dev/null
git worktree add --detach "$B_WT" "$B_SHA" >/dev/null
for wt in "$A_WT" "$B_WT"; do
  (cd "$wt" && export CARGO_TARGET_DIR="$wt/target" && cargo build --release -p vis-cli --bin vis >/dev/null 2>&1)
  test -x "$wt/target/release/vis" || { echo "BUILD LỖI $wt" >&2; exit 3; }
done
python3 - "$A_WT/target/release/vis" "$B_WT/target/release/vis" "$ROOT/test/bench/opt-bench.vis" "$RES" "$A_SHA" "$B_SHA" <<'PYEOF'
import json, subprocess, sys, time, statistics

a_vis, b_vis, prog, res_path, a_sha, b_sha = sys.argv[1:7]

def run(vis, engine):
    cmd = [vis, "run", "--no-cache"] + (["--vm"] if engine == "vm" else []) + [prog]
    t0 = time.perf_counter()
    p = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    t1 = time.perf_counter()
    assert p.returncode == 0, f"exit {p.returncode} ({engine})"
    return (t1 - t0) * 1000.0

results = {"interp": {"A": [], "B": []}, "vm": {"A": [], "B": []}}
for i in range(24):
    for engine in ("interp", "vm"):
        for label, vis in (("A", a_vis), ("B", b_vis)):
            results[engine][label].append(run(vis, engine))

def stat(xs):
    return {"n": len(xs), "median": round(statistics.median(xs), 3),
            "min": round(min(xs), 3), "max": round(max(xs), 3)}

out = {"A": a_sha, "B": b_sha, "script": "test/vd241-152.sh"}
for eng in ("interp", "vm"):
    a, b = results[eng]["A"], results[eng]["B"]
    pairs = [b / a for a, b in zip(a, b)]
    out[eng] = {"A_ms": stat(a), "B_ms": stat(b),
                "B/A": round(statistics.median(b) / statistics.median(a), 4),
                "pair_median": round(statistics.median(pairs), 4),
                "pair_min": round(min(pairs), 4), "pair_max": round(max(pairs), 4)}
with open(res_path, "w") as f:
    json.dump(out, f, indent=2, ensure_ascii=False)
print(json.dumps(out, indent=2, ensure_ascii=False))
PYEOF
python3 - "$RES" "$ROOT" <<'PYEOF'
import json, sys, subprocess
data = json.load(open(sys.argv[1]))
for eng in ("vm", "interp"):
    d = data[eng]
    print(f"{eng}: A med={d['A_ms']['median']} ms · B med={d['B_ms']['median']} ms · B/A={d['B/A']} · cặp: min={d['pair_min']} med={d['pair_median']} max={d['pair_max']}")
print("Ngưỡng đóng (VD-241 §4 việc 3): chỉ khi khoảng hẳn một phía 1,10 cho CẢ VM và interp.")
PYEOF
git worktree remove --force "$A_WT"
git worktree remove --force "$B_WT"
echo "xong — json thô: $RES (không git add)"
