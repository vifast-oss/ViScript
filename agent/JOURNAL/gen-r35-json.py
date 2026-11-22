#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Sinh `agent/reports/R35-viscript.json` từ `QUEUE.md` tại một head cố định.

Dùng khi chốt R35: đổi `HEAD_SHA` (commit prep cuối) rồi chạy lại:
    python3 agent/JOURNAL/gen-r35-json.py
Script đọc `git show <HEAD_SHA>:agent/QUEUE.md`, phủ **mọi** hàng `ST-*`/`KT-*`,
lấy status đúng QUEUE@head (kèm `queue` khi status chuẩn hoá khác raw), và chỉ
ghi commit đã có thật. Không placeholder `[SOAK_*]` trong JSON.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HEAD_SHA = "9af397648d0345c22b1351cdc1cfd308027093ca"  # cập nhật khi chốt
ROUND = 35

# Chỉ các commit đã kiểm `git cat-file -e <sha>^{commit}`; task khác để [].
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
    "ST-297": ["f4cb136", "8a9f93e"],
    "ST-299a": ["5cc1903", "060a744"],
    "ST-299b": ["3b8049c", "aab5ffa"],
    "ST-300": ["3581c9a", "8286caf", "b3a9742"],
}

NOTES = {
    "ST-297": "DONE f4cb136: gate 3660/4; Windows 3x full 3611/0/3; #149 đóng",
    "ST-298": "REVIEW: 3x full PASS 3611/0/3 @f4cb136; soak 4h đang chạy; Tier 1 chưa công bố",
    "ST-299a": "DONE 5cc1903/060a744: WasiHost wasm32-wasip1; wasm 26/26",
    "ST-299b": "DONE 3b8049c/aab5ffa: vis-loader đa tệp; native 29/29, wasm 28/28",
    "ST-300": "REVIEW: harness WASI 388/390 = 99,5%; release v0.4.5/R35 chờ",
    "ST-300b": "READY: tách vis-host-native/vis-pkg sau tag v0.4.5",
    "ST-301": "READY: đóng #192 + Pha 2a sau tag v0.4.5",
}

BLOCKED_Q = {
    "ST-298": "Soak 4h Windows đang chạy; chưa công bố Tier 1; cần số vòng/fail/fd-RSS thực tế",
    "ST-300": "Harness WASI xong; phần release v0.4.5 + R35 + tag đang chờ A0",
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
        st = chuan(raw)
        task = {"id": tid, "status": st, "commits": COMMITS.get(tid, []), "gates": ["fast"]}
        if st != raw:
            task["queue"] = raw
        task["note"] = NOTES.get(tid, f"QUEUE@{HEAD_SHA[:7]}: {raw}")
        tasks.append(task)
        if raw.startswith("BLOCKED") or raw.startswith("REVIEW"):
            blocked.append({
                "id": tid,
                "question": BLOCKED_Q.get(tid, f"QUEUE@{HEAD_SHA[:7]} đang {raw}; xem QUEUE/ACTIVE"),
            })

    data = {
        "project": "viscript",
        "round": ROUND,
        "head": HEAD_SHA,
        "build_id": f"gd2-v045-r35-{HEAD_SHA[:7]}-20261123",
        "tasks": tasks,
        "gates": {"fast": "PASS", "full": "—", "nightly": "PASS"},
        "sim": {
            "seeds": 0,
            "failures": 0,
            "diff_seeds": [202611230230, 20261115],
            "note": "diff 100k seed 202611230230 lech=0 (100000; 69,2 s); soak 3x seed 20261115 lech=0",
        },
        "coverage": {
            "tokens": 0,
            "fixtures": 525,
            "note": "test/sem 525 (475 .vis + 50 .vish); WASI: 388/390 eligible = 99,5%; native oracle 475/475",
        },
        "mutants": {"measured": False, "killed": None, "alive": None},
        "perf": {
            "head_do": HEAD_SHA,
            "baseline": "Giai đoạn 2 v0.4.5 — chưa tag; R35 đang chốt",
            "host": "ps (Linux; rustc 1.100.0)",
            "linux_gate": "PASS @f4cb136: 3660 passed / 4 skipped",
            "release_linux": "chưa chạy release v0.4.5; final QA chốt trước tag",
            "linux_soak": "final soak 45122b7/84e8eb6: 10/10 (759 s); 3x diff lech=0, host 54/54, cache 5/5, wasm 28/28",
            "windows_full": "3x full 3611/0/3 @f4cb136; host 54/54; targets 18/18 + 17/1; dirty 0",
            "windows_soak": "4h đang chạy; chưa công bố Tier 1",
            "wasi": "bin/vis-wasm-sem 388/390 eligible = 99,5%; VM 390/390; 2 stack-overflow interp (known)",
            "tags": "v0.4.0 @c2accdf; v0.4.1 @90844bc; v0.4.5 chưa tag",
            "note": "Không placeholder trong JSON; soak/Tier 1 chốt ở tài liệu md trước tag",
        },
        "blocked": blocked,
    }

    out = ROOT / "agent/reports/R35-viscript.json"
    out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    license = ROOT / "agent/reports/R35-viscript.json.license"
    license.write_text(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n"
        "# R35 — ViScript, vòng 35 (Giai đoạn 2 `v0.4.5`) — sinh bởi gen-r35-json.py\n",
        encoding="utf-8",
    )
    print(f"R35 JSON: {len(tasks)} tasks, {len(blocked)} blocked, head={HEAD_SHA}")


if __name__ == "__main__":
    main()
