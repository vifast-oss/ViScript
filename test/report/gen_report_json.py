#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Sinh `agent/reports/R<NN>-viscript.json` từ `QUEUE.md` tại một head cố định.

Script hợp nhất `gen-r35-json.py` + `gen-r36-json.py` (ST-310f) — giữ nguyên cấu hình
từng vòng, chỉ gom về một chỗ:

    python3 test/report/gen_report_json.py --round 35
    python3 test/report/gen_report_json.py --round 36 --head <sha>

Mặc định dùng `head` đã chốt trong `ROUNDS`; `--head` để chốt lại khi A0 đổi head.
Script đọc `git show <head>:agent/QUEUE.md`, phủ **mọi** hàng `ST-*`/`KT-*`, lấy
status đúng QUEUE@head (kèm `queue` khi status chuẩn hoá/ghi đè khác raw), và chỉ
ghi commit đã có thật. Không placeholder `[SOAK_*]` trong JSON.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Chỉ các commit đã kiểm `git cat-file -e <sha>^{commit}`; task khác để [].
_COMMITS_CHUNG = {
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
    "ST-299a": ["5cc1903", "060a744"],
    "ST-299b": ["3b8049c", "aab5ffa"],
    "ST-300": ["3581c9a", "8286caf", "b3a9742"],
}

ROUNDS: dict[int, dict] = {
    35: {
        "head": "9af397648d0345c22b1351cdc1cfd308027093ca",  # cập nhật khi chốt
        "build_id": "gd2-v045-r35-{head7}-20261123",
        "commits": {
            **_COMMITS_CHUNG,
            "ST-297": ["f4cb136", "8a9f93e"],
            "ST-300": ["3581c9a", "8286caf", "b3a9742"],
        },
        "overrides": {
            # Bản JSON chốt R35 đã ghi DONE sau soak 4h; QUEUE@head còn REVIEW (legacy).
            "ST-298": "DONE",
        },
        "blocked_mode": "raw",  # R35: BLOCKED/REVIEW theo raw QUEUE
        # ST-298 được chỉnh tay sau soak 4h: `note` đứng trước `queue` trong bản chốt.
        "note_truoc_queue": {"ST-298"},
        "notes": {
            "ST-297": "DONE f4cb136: gate 3660/4; Windows 3x full 3611/0/3; #149 đóng",
            "ST-298": "DONE: 3x full 3611/0/3 @f4cb136; soak 4h 985 vong, fail 0, flake 1, free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0; Windows Tier 1 (VD-283)",
            "ST-299a": "DONE 5cc1903/060a744: WasiHost wasm32-wasip1; wasm 26/26",
            "ST-299b": "DONE 3b8049c/aab5ffa: vis-loader đa tệp; native 29/29, wasm 28/28",
            "ST-300": "REVIEW: harness WASI 388/390 = 99,5%; release v0.4.5/R35 chờ",
            "ST-300b": "READY: tách vis-host-native/vis-pkg sau tag v0.4.5",
            "ST-301": "READY: đóng #192 + Pha 2a sau tag v0.4.5",
        },
        "blocked_q": {
            "ST-298": "Đã hoàn tất soak 4h + Tier 1; QUEUE@9af3976 còn REVIEW (legacy), trạng thái DONE ghi ở field queue override",
            "ST-300": "Harness WASI xong; phần release v0.4.5 + R35 + tag đang chờ A0",
        },
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
        "perf": {
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
        "license": "# R35 — ViScript, vòng 35 (Giai đoạn 2 `v0.4.5`) — sinh bởi gen_report_json.py\n",
    },
    36: {
        "head": "c3b7290751aaa9ab131eb89ba8829d4f78078e47",  # cập nhật khi A0 chốt head R36
        "build_id": "gd3-v050-r36-{head7}-20261123",
        "commits": {
            **_COMMITS_CHUNG,
            "ST-297": ["f4cb136"],
            "ST-298": ["30890ba", "3daff62"],
            "ST-300": ["3581c9a", "8286caf", "b3a9742", "84e8eb6"],
            "ST-300b": ["5675a7f", "770fdf7"],
            "ST-300m1": ["24ee56e", "2cbbed0", "d6aa839", "c3b7290"],
            "ST-301": ["3e8c2cc", "2bc6be7"],
            "ST-302": ["d444c9c", "d3ea467"],
            "ST-303": ["728a5bc", "a114e38"],
            "ST-304": ["32b921d", "6e0675a"],
            "ST-305": ["b64e925", "fa1a70b"],
        },
        # Ghi đè trạng thái khi sổ QUEUE@head chưa lật. Giá trị gốc vẫn ghi ở trường `queue`.
        "overrides": {
            "ST-300m1": "DONE",    # đã merge vào head c3b7290
            "ST-302": "REVIEW",    # d3ea467 local, chưa merge; Windows/macOS quick chưa chạy
            "ST-304": "REVIEW",    # 6e0675a local, chưa merge; Windows/macOS chưa chạy
            "ST-304b": "REVIEW",   # đang chạy, chưa có commit
            "ST-305": "REVIEW",    # fa1a70b local; release v0.5.0 + 3-OS + R36 còn chờ
            "ST-306": "REVIEW",    # đang chạy, chưa có commit
        },
        "blocked_mode": "status",  # R36: BLOCKED/REVIEW theo status sau ghi đè
        "notes": {
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
        },
        "blocked_q": {
            "ST-300m2": "Chưa chạy trên mps rảnh (không có kết quả #147/#152); chờ bin/vis-lock",
            "ST-300m3": "Chưa chạy 3x vis-gate-macos --full + soak 4h; chưa công bố macOS Tier 1",
            "ST-302": "d3ea467 local chưa merge; Windows/macOS quick chưa chạy",
            "ST-304": "6e0675a local chưa merge; Windows/macOS chưa chạy",
            "ST-304b": "Đang chạy; chưa có commit để nghiệm thu",
            "ST-305": "fa1a70b local; còn release v0.5.0 + QA 3-OS + R36",
            "ST-306": "Đang chạy; chưa có commit để nghiệm thu",
        },
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
        "perf": {
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
        "license": "# R36 — ViScript, vòng 36 (Giai đoạn 3 `v0.5.0`, QA prep) — sinh bởi gen_report_json.py\n",
    },
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


def queue_rows(head: str) -> dict[str, str]:
    text = subprocess.check_output(
        ["git", "show", f"{head}:agent/QUEUE.md"], cwd=ROOT, text=True
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
    ap = argparse.ArgumentParser(description="Sinh JSON báo cáo vòng ViScript từ QUEUE@head")
    ap.add_argument("--round", type=int, choices=sorted(ROUNDS), required=True)
    ap.add_argument("--head", help="ghi đè head đã chốt trong ROUNDS")
    args = ap.parse_args()

    cfg = ROUNDS[args.round]
    head = args.head or cfg["head"]
    rows = queue_rows(head)
    tasks = []
    blocked = []
    for tid, raw in rows.items():
        st = cfg["overrides"].get(tid, chuan(raw))
        task = {
            "id": tid,
            "status": st,
            "commits": cfg["commits"].get(tid, []),
            "gates": ["fast"],
        }
        co_queue = st != raw
        if co_queue and tid not in cfg.get("note_truoc_queue", set()):
            task["queue"] = raw
        task["note"] = cfg["notes"].get(tid, f"QUEUE@{head[:7]}: {raw}")
        if co_queue and tid in cfg.get("note_truoc_queue", set()):
            task["queue"] = raw
        tasks.append(task)
        la_blocked = (
            raw.startswith("BLOCKED") or raw.startswith("REVIEW")
            if cfg["blocked_mode"] == "raw"
            else st in {"BLOCKED", "REVIEW"}
        )
        if la_blocked:
            blocked.append({
                "id": tid,
                "question": cfg["blocked_q"].get(
                    tid, f"QUEUE@{head[:7]} đang {raw}; xem QUEUE/ACTIVE"
                ),
            })

    data = {
        "project": "viscript",
        "round": args.round,
        "head": head,
        "build_id": cfg["build_id"].format(head7=head[:7]),
        "tasks": tasks,
        "gates": cfg["gates"],
        "sim": cfg["sim"],
        "coverage": cfg["coverage"],
        "mutants": {"measured": False, "killed": None, "alive": None},
        "perf": {"head_do": head, **cfg["perf"]},
        "blocked": blocked,
    }

    out = ROOT / f"agent/reports/R{args.round}-viscript.json"
    out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    license = ROOT / f"agent/reports/R{args.round}-viscript.json.license"
    license.write_text(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n" + cfg["license"],
        encoding="utf-8",
    )
    print(f"R{args.round} JSON: {len(tasks)} tasks, {len(blocked)} blocked, head={head}")


if __name__ == "__main__":
    main()
