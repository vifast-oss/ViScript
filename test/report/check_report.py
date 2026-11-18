#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Máy kiểm báo cáo vòng của dòng ViScript (Khối E, `viscript/agent/QUEUE.md`).

Mọi sự thật trong báo cáo phải đối chiếu được với máy: `head` tồn tại + là tổ tiên HEAD;
mọi `commits[]` tồn tại; `status` khớp QUEUE **hoặc** có trường `queue` ghi đúng giá trị QUEUE
(ghi cả hai khi sổ chưa lật); phủ đủ mọi hàng QUEUE; `blocked[]` phủ mọi hàng BLOCKED/REVIEW;
YC phải có thật trong `viscript/agent/BLOCKED.md`; `project == "viscript"`.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REPORTS = ROOT / "agent/reports"
QUEUE = ROOT / "agent/QUEUE.md"
# **SỬA KÈM LÝ DO (ST-121)**: bản cũ trỏ `v2/agent/BLOCKED.md` — tệp của **dòng OS**.
# Hệ quả: cổng ViScript không chạy được trên một cây chỉ có `viscript/` (đo trên Linux:
# `FileNotFoundError: /root/vifast-linux/v2/agent/BLOCKED.md`). Dòng ViScript có tệp chặn
# riêng; không báo cáo ViScript nào dùng id `YC-` nên đổi đường dẫn không đổi kết luận.
BLOCKED = ROOT / "agent/BLOCKED.md"

KEYS = {"project", "round", "head", "build_id", "tasks", "gates",
        "sim", "coverage", "mutants", "perf", "blocked"}
STATUSES = {"READY", "BLOCKED", "REVIEW", "DONE"}
fails: list[str] = []


def check(cond, msg):
    if not cond:
        fails.append(msg)


def split_row(line: str) -> list[str]:
    out, cur, i = [], "", 0
    line = line.strip()
    body = line[1:] if line.startswith("|") else line
    body = body[:-1] if body.endswith("|") else body
    while i < len(body):
        if body[i] == "\\" and i + 1 < len(body) and body[i + 1] == "|":
            cur += "|"; i += 2
        elif body[i] == "|":
            out.append(cur.strip()); cur = ""; i += 1
        else:
            cur += body[i]; i += 1
    out.append(cur.strip())
    return out


def parse_queue(text: str) -> dict[str, str]:
    rows = {}
    for line in text.splitlines():
        t = line.strip()
        if t.startswith("| ST-") or t.startswith("| KT-"):
            c = split_row(t)
            if len(c) >= 6:
                rows[c[0]] = c[4].strip("`* ")
    return rows


class Git:
    """Gom **mọi** truy vấn git vào vài tiến trình (VD-182, nợ #147).

    **SỬA KÈM LÝ DO (VD-182, nợ #147)**: bản cũ gọi `git cat-file -e` cho **từng** commit
    (3 107 lần trên cây này) + `git show` + `git merge-base` cho từng báo cáo ⇒ **3 223** tiến
    trình. macOS phạt tạo tiến trình 9,6× nên `report-check` chậm 22,8× so với Linux. Nay:
    **một** `cat-file --batch-check` cho mọi SHA, **một** `rev-list HEAD` cho quan hệ tổ tiên,
    **một** `cat-file --batch` cho mọi `head:agent/QUEUE.md`. Phép kiểm, câu chữ, thứ tự báo
    lỗi giữ **nguyên văn**; chỉ giảm số tiến trình.
    """

    def __init__(self, root: Path):
        self.root = root
        self._ok: dict[str, bool] = {}
        self._oid: dict[str, str] = {}
        self._ancestors = None
        self._queue: dict[str, dict[str, str]] = {}

    @staticmethod
    def _la_sha(sha) -> bool:
        return isinstance(sha, str) and re.fullmatch(r"[0-9a-f]{7,40}", sha or "") is not None

    def prefetch(self, shas, heads) -> None:
        """Một lượt `cat-file --batch-check` cho mọi SHA + một lượt `cat-file --batch` cho QUEUE."""
        can = sorted({s for s in shas if self._la_sha(s)})
        if can:
            inp = "".join(f"{s}^{{commit}}\n" for s in can)
            out = subprocess.run(["git", "cat-file", "--batch-check"], cwd=self.root,
                                 input=inp, capture_output=True, text=True, check=False)
            for sha, line in zip(can, out.stdout.splitlines()):
                parts = line.split()
                if len(parts) >= 2 and parts[1] == "commit":
                    self._ok[sha] = True
                    self._oid[sha] = parts[0]
                else:
                    self._ok[sha] = False
        can_head = sorted({str(h) for h in heads if h})
        if can_head:
            inp = "".join(f"{h}:agent/QUEUE.md\n" for h in can_head).encode()
            out = subprocess.run(["git", "cat-file", "--batch"], cwd=self.root,
                                 input=inp, capture_output=True, check=False)
            data = out.stdout
            pos = 0
            for h in can_head:
                nl = data.find(b"\n", pos)
                if nl < 0:
                    self._queue[h] = {}
                    continue
                parts = data[pos:nl].decode("utf-8", "replace").split()
                pos = nl + 1
                if len(parts) < 3 or parts[1] != "blob":
                    self._queue[h] = {}
                    continue
                size = int(parts[2])
                self._queue[h] = parse_queue(data[pos:pos + size].decode("utf-8", "replace"))
                pos += size + 1

    def ok(self, sha) -> bool:
        if not self._la_sha(sha):
            return False
        if sha in self._ok:
            return self._ok[sha]
        # Không nằm trong lô (hiếm): hỏi thẳng, giữ nguyên hành vi cũ.
        r = subprocess.run(["git", "cat-file", "-e", f"{sha}^{{commit}}"],
                           cwd=self.root, capture_output=True, check=False)
        self._ok[sha] = r.returncode == 0
        return self._ok[sha]

    def is_ancestor(self, sha) -> bool:
        oid = self._oid.get(sha)
        if oid is None:
            r = subprocess.run(["git", "merge-base", "--is-ancestor", sha, "HEAD"],
                               cwd=self.root, check=False)
            return r.returncode == 0
        if self._ancestors is None:
            out = subprocess.run(["git", "rev-list", "HEAD"], cwd=self.root,
                                 capture_output=True, text=True, check=False)
            self._ancestors = set(out.stdout.split())
        return oid in self._ancestors

    def queue_at(self, head) -> dict[str, str]:
        return self._queue.get(str(head), {})


GIT = Git(ROOT)
_btext = None


def blocked_text() -> str:
    """BLOCKED.md + QUEUE.md — đọc **một lần** cho cả lượt (bản cũ đọc lại mỗi báo cáo)."""
    global _btext
    if _btext is None:
        _btext = BLOCKED.read_text() + QUEUE.read_text()
    return _btext


def git_ok(sha: str) -> bool:
    return GIT.ok(sha)


def queue_at(head: str) -> dict[str, str]:
    return GIT.queue_at(head)


def queue_statuses() -> dict[str, str]:
    return parse_queue(QUEUE.read_text())


def check_md(path: Path) -> None:
    check(path.is_file(), f"{path.name}: thiếu tệp MD")
    if not path.is_file():
        return
    text = path.read_text()
    for i in range(1, 7):
        check(f"## {i}." in text, f"{path.name}: thiếu mục '## {i}.'")
    check(len(text) > 800, f"{path.name}: quá ngắn ({len(text)} ký tự)")


def check_report(path: Path, round_no: int, queue: dict[str, str]) -> None:
    tag = path.name
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        fails.append(f"{tag}: JSON hỏng ({exc})")
        return
    check(set(data) == KEYS, f"{tag}: phải ĐÚNG 11 khoá (D2-39), thừa/thiếu: {sorted(set(data) ^ KEYS)}")
    check(data.get("project") == "viscript", f"{tag}: project phải 'viscript'")
    check(data.get("round") == round_no, f"{tag}: round phải {round_no}")

    head = str(data.get("head", ""))
    check(git_ok(head), f"{tag}: head {head!r} không tồn tại trong git")
    if git_ok(head):
        check(GIT.is_ancestor(head), f"{tag}: head {head} không phải tổ tiên HEAD")

    gates = data.get("gates") or {}
    for k in ("fast", "full", "nightly"):
        check(gates.get(k) in {"PASS", "FAIL", "—"}, f"{tag}: gates.{k} lạ")

    sim = data.get("sim") or {}
    check(isinstance(sim.get("seeds"), int), f"{tag}: sim.seeds phải số nguyên")
    check(isinstance(sim.get("failures"), int), f"{tag}: sim.failures phải số nguyên")

    cov = data.get("coverage") or {}
    check("tokens" in cov and "fixtures" in cov, f"{tag}: coverage phải có tokens+fixtures")

    mut = data.get("mutants") or {}
    check(isinstance(mut.get("measured"), bool), f"{tag}: mutants.measured phải bool")
    for k in ("killed", "alive"):
        v = mut.get(k, "missing")
        check(v is None or isinstance(v, int), f"{tag}: mutants.{k} phải số hoặc null")

    check(isinstance(data.get("perf"), dict), f"{tag}: perf phải object")

    tasks = data.get("tasks") or []
    seen: dict[str, str] = {}
    for t in tasks:
        tid = t.get("id")
        check(tid in queue, f"{tag}: task {tid!r} không có trong QUEUE")
        st = t.get("status")
        check(st in STATUSES, f"{tag}: {tid} status ngoài 4 trạng thái")
        if tid in queue:
            if st == queue[tid]:
                pass
            else:
                # Sổ chưa lật: được phép lệch CHỈ KHI ghi đúng giá trị QUEUE vào trường `queue`.
                check(t.get("queue") == queue[tid],
                      f"{tag}: {tid} status {st!r} ≠ QUEUE {queue[tid]!r} (thiếu/sai trường 'queue')")
        for c in t.get("commits") or []:
            check(git_ok(c), f"{tag}: {tid} commit {c!r} không tồn tại trong git")
        check(tid not in seen, f"{tag}: task {tid} trùng")
        seen[tid] = st

    missing = sorted(set(queue) - set(seen))
    check(not missing, f"{tag}: thiếu task trong báo cáo: {missing}")
    check(not sorted(set(seen) - set(queue)), f"{tag}: task lạ ngoài QUEUE")

    blocked = data.get("blocked") or []
    btext = blocked_text()
    for b in blocked:
        check(bool(b.get("id")), f"{tag}: blocked có mục thiếu id")
        check(bool(b.get("question")), f"{tag}: blocked {b.get('id')} thiếu question")
        if str(b.get("id", "")).startswith("YC-"):
            check(b["id"] in btext, f"{tag}: {b['id']} không có trong BLOCKED.md")
    ids = {b.get("id") for b in blocked}
    for tid, st in queue.items():
        if st in {"BLOCKED", "REVIEW"}:
            check(tid in ids, f"{tag}: {tid} ({st}) không xuất hiện trong blocked[]")


def rounds() -> list[int]:
    out = []
    for p in REPORTS.glob("R*-viscript.json"):
        m = re.fullmatch(r"R(\d+)-viscript\.json", p.name)
        if m:
            out.append(int(m.group(1)))
    return sorted(out)


def main() -> int:
    args = sys.argv[1:]
    queue = queue_statuses()
    check(queue, "QUEUE: không đọc được hàng nào")
    targets = []
    if "--file" in args:
        p = Path(args[args.index("--file") + 1])
        targets.append((p, int(json.loads(p.read_text()).get("round", 1))))
    elif "--all" in args:
        for n in rounds():
            targets.append((REPORTS / f"R{n}-viscript.json", n))
    else:
        n = int(args[args.index("--n") + 1]) if "--n" in args else 1
        targets.append((REPORTS / f"R{n}-viscript.json", n))
    json_only = "--json-only" in args
    check(targets, "không có báo cáo nào để kiểm")
    # Gom mọi SHA + mọi `head` của **toàn bộ** lượt rồi hỏi git một lần (VD-182, nợ #147).
    shas: list = []
    heads: list = []
    for path, _n in targets:
        data = json.loads(path.read_text())
        head = str(data.get("head", ""))
        heads.append(head)
        shas.append(head)
        for t in data.get("tasks") or []:
            if isinstance(t, dict):
                for c in t.get("commits") or []:
                    shas.append(c)
    GIT.prefetch(shas, heads)
    for path, n in targets:
        if not json_only:
            check_md(path.with_suffix(".md"))
        head = json.loads(path.read_text()).get("head", "")
        check_report(path, n, queue_at(head) or queue)
    if fails:
        for f in fails:
            print("not ok -", f)
        print(f"# {len(fails)} mục sai")
        return 1
    print(f"report-check: PASS ({len(targets)} báo cáo, {len(queue)} hàng QUEUE phủ đủ)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
