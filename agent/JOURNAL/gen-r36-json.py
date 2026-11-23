#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Sinh `agent/reports/R36-viscript.json` từ `QUEUE.md` tại head `c3b7290`.

Dùng khi dựng R36 (QA prep cho `v0.5.0`): đổi `HEAD_SHA` khi A0 chốt head cuối rồi chạy lại:
    python3 agent/JOURNAL/gen-r36-json.py
Script đọc `git show <HEAD_SHA>:agent/QUEUE.md`, phủ **mọi** hàng `ST-*`/`KT-*`,
lấy status đúng QUEUE@head (kèm `queue` khi status chuẩn hoá khác raw), và chỉ
ghi commit đã có thật. Không bịa kết quả `ST-300m2/m3`: hai task này vẫn BLOCKED.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HEAD_SHA = "c3b7290751aaa9ab131eb89ba8829d4f78078e47"  # cập nhật khi A0 chốt head R36
ROUND = 36

# Chỉ các commit đã kiểm `git cat-file -e <sha>^{commit}`; task khác để [].
# Gồm commit lịch sử R34/R35 (để tra cứu) + commit thật của GĐ 3 đang có trên các nhánh.
COMMITS = {
    "ST-290": ["3627f3bc", "a48d5b8"],
    "ST-291": ["269ff71", "8cc3588"],
    "ST-292": ["eca4e82", "e47b3ca"],
    "ST-293": ["82064e9", "6d6bfe9"],
    "ST-294": ["39cc81cc", "e35dcf6"],
    "ST-295": ["ba4023f", "cd76da5"],
    "ST-296": ["8696edc", "c2accdf"],
    "ST-296d-tools": ["b36f8a5", "4b2484f"],
    "ST-296a": ["53990c0", "e04d051"],
    "ST-296b": ["067e1a5", "1b27cb8"],
    "ST-296c": ["ab8d393", "044b289"],
    "ST-296d": ["a1730c5", "90844bc"],
    "ST-297": ["f4cb136"],
    "ST-298": ["4ec8f96", "3daff62"],
    "ST-299a": ["5cc1903", "060a744"],
    "ST-299b": ["3b8049c", "aab5ffa"],
    "ST-300": ["3581c9a", "8286caf", "b3a9742", "84e8eb6"],
    "ST-300b": ["5675a7f", "770fdf7"],
    "ST-300m1": ["24ee56e", "2cbbed0", "d6aa839", "c3b7290"],
    "ST-301": ["3e8c2cc", "2bc6be7"],
    "ST-302": ["d444c9c", "d3ea467"],
    "ST-303": ["728a5bc", "a114e38"],
    "ST-304": ["32b921d", "6e0675a"],
    "ST-305": ["b64e925", "fa1a70b"],
}

# Ghi đè trạng thái khi sổ QUEUE@head chưa lật. Giá trị gốc vẫn ghi ở trường `queue`.
OVERRIDES = {
    "ST-300m1": "DONE",    # đã merge vào head c3b7290
    "ST-302": "REVIEW",    # d3ea467 local, chưa merge; Windows/macOS quick chưa chạy
    "ST-304": "REVIEW",    # 6e0675a local, chưa merge; Windows/macOS chưa chạy
    "ST-304b": "REVIEW",   # đang chạy, chưa có commit
    "ST-305": "REVIEW",    # fa1a70b local; release v0.5.0 + 3-OS + R36 còn chờ
    "ST-306": "REVIEW",    # đang chạy, chưa có commit
}

NOTES = {
    "ST-297": "DONE f4cb136: gate 3660/4; Windows 3x full 3611/0/3; #149 đóng",
    "ST-298": "DONE: v0.4.5 tag @3daff62; soak 4h Windows 985 vòng/0 fail; Tier 1",
    "ST-299a": "DONE 5cc1903/060a744: WasiHost wasm32-wasip1; wasm 26/26",
    "ST-299b": "DONE 3b8049c/aab5ffa: vis-loader đa tệp; native 29/29, wasm 28/28",
    "ST-300": "DONE: vis-wasm-sem 388/390 eligible = 99,5%; 2 stack-overflow interp (#193)",
    "ST-300b": "DONE 770fdf7: tách vis-host-native/vis-pkg; main.rs 225 dòng",
    "ST-300m1": "DONE c3b7290 (merge): Darwin errno, #137 đóng; gate 3672/4; mps quick PASS; release arm64 PASS",
    "ST-300m2": "BLOCKED: CHƯA chạy trên mps rảnh — không có kết quả đo; chờ bin/vis-lock",
    "ST-300m3": "BLOCKED: CHƯA chạy 3x full + soak 4h trên mps — không tuyên bố Tier 1 macOS",
    "ST-301": "DONE 2bc6be7: lambda annot + T?? + #192/#155; fuzz 1h 0 vi phạm",
    "ST-302": "REVIEW d3ea467 (local, chưa merge): async impl/cmd #13 + derives Json #8; gate 3676/4; diff 100k lech=0",
    "ST-303": "DONE a114e38: vis new/doc/repl; gate 3671/4",
    "ST-304": "REVIEW 6e0675a (local, chưa merge): vish-edit; gate 3679/4; 8/8 test; warm 50 us",
    "ST-304b": "REVIEW: đang chạy; chưa có commit/kết quả trong repo",
    "ST-305": "REVIEW fa1a70b (local): LSP hover generic + completion std.* + VS Code; gate 3674/4; release v0.5.0 chờ",
    "ST-306": "REVIEW: đang chạy; chưa có commit/kết quả trong repo",
}

BLOCKED_Q = {
    "ST-300m2": "Chưa chạy trên mps rảnh (không có kết quả #147/#152); chờ bin/vis-lock",
    "ST-300m3": "Chưa chạy 3x vis-gate-macos --full + soak 4h; chưa công bố macOS Tier 1",
    "ST-302": "d3ea467 local chưa merge; Windows/macOS quick chưa chạy",
    "ST-304": "6e0675a local chưa merge; Windows/macOS chưa chạy",
    "ST-304b": "Đang chạy; chưa có commit để nghiệm thu",
    "ST-305": "fa1a70b local; còn release v0.5.0 + QA 3-OS + R36",
    "ST-306": "Đang chạy; chưa có commit để nghiệm thu",
}


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


def queue_rows() -> dict[str, str]:
    text = subprocess.check_output(
        ["git", "show", f"{HEAD_SHA}:agent/QUEUE.md"], cwd=ROOT, text=True
    )
    rows: dict[str, str] = {}
    for line in text.splitlines():
        t = line.strip()
        if t.startswith("| ST-") or t.startswith("| KT-"):
            c = split_row(t)
            if len(c) >= 6:
                rows[c[0]] = c[4].strip("`* ")
    return rows


def chuan(raw: str) -> str:
    if raw.startswith("BLOCKED"):
        return "BLOCKED"
    if raw.startswith("REVIEW"):
        return "REVIEW"
    if raw.startswith("READY"):
        return "READY"
    return "DONE"


def main() -> None:
    rows = queue_rows()
    tasks = []
    blocked = []
    for tid, raw in rows.items():
        st = OVERRIDES.get(tid, chuan(raw))
        task = {"id": tid, "status": st, "commits": COMMITS.get(tid, []), "gates": ["fast"]}
        if st != raw:
            task["queue"] = raw
        task["note"] = NOTES.get(tid, f"QUEUE@{HEAD_SHA[:7]}: {raw}")
        tasks.append(task)
        if st in {"BLOCKED", "REVIEW"}:
            blocked.append({
                "id": tid,
                "question": BLOCKED_Q.get(tid, f"QUEUE@{HEAD_SHA[:7]} đang {raw}; xem QUEUE/ACTIVE"),
            })

    data = {
        "project": "viscript",
        "round": ROUND,
        "head": HEAD_SHA,
        "build_id": f"gd3-v050-r36-{HEAD_SHA[:7]}-20261123",
        "tasks": tasks,
        "gates": {"fast": "PASS", "full": "—", "nightly": "—"},
        "sim": {
            "seeds": 0,
            "failures": 0,
            "diff_seeds": [20261115],
            "note": "ST-302 (nhánh): diff 100k seed 20261115 lech=0 (84,6 s); R36 chưa chạy lại toàn ma trận",
        },
        "coverage": {
            "tokens": 0,
            "fixtures": 525,
            "note": "test/sem 525 (475 .vis + 50 .vish); WASI kế thừa R35 388/390 eligible = 99,5%",
        },
        "mutants": {"measured": False, "killed": None, "alive": None},
        "perf": {
            "head_do": HEAD_SHA,
            "baseline": "Giai đoạn 3 v0.5.0 — R36 QA prep, chưa tag",
            "host": "ps (Linux; rustc 1.93.1)",
            "linux_gate": "PASS @c3b7290: 3672/4 (m1); nhánh: ST-302 3676/4, ST-304 3679/4, ST-305 3674/4",
            "release_linux": "chưa chạy release v0.5.0",
            "linux_soak": "chưa chạy QA R36 (xem JOURNAL/v050-qa-matrix.md)",
            "windows_full": "chưa chạy QA R36",
            "windows_soak": "chưa chạy QA R36",
            "macos": "m1 quick PASS trên mps + release aarch64-apple-darwin PASS; m2/m3 BLOCKED, chưa có kết quả",
            "wasi": "kế thừa R35: 388/390 eligible = 99,5%; chưa chạy lại R36",
            "tags": "v0.4.0 @c2accdf; v0.4.1 @90844bc; v0.4.5 @3daff62; v0.5.0 chưa tag",
            "note": "Số QA 3-OS/soak R36 chỉ điền sau khi chạy đủ ma trận JOURNAL/v050-qa-matrix.md; không placeholder trong JSON",
        },
        "blocked": blocked,
    }

    out = ROOT / "agent/reports/R36-viscript.json"
    out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    license = ROOT / "agent/reports/R36-viscript.json.license"
    license.write_text(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n"
        "# R36 — ViScript, vòng 36 (Giai đoạn 3 `v0.5.0`, QA prep) — sinh bởi gen-r36-json.py\n",
        encoding="utf-8",
    )
    print(f"R36 JSON: {len(tasks)} tasks, {len(blocked)} blocked, head={HEAD_SHA}")


if __name__ == "__main__":
    main()
