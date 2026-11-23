#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Sinh `agent/reports/R36-viscript.json` từ `QUEUE.md` tại head `e9c62cb`.

Chạy lại khi A0 đổi head báo cáo:
    python3 agent/JOURNAL/gen-r36-json.py
Script đọc `git show <HEAD_SHA>:agent/QUEUE.md`, phủ **mọi** hàng `ST-*`/`KT-*`,
lấy status thật theo bằng chứng (ghi `queue` khi sổ QUEUE@head chưa lật), chỉ ghi
commit đã có thật (tự kiểm `git cat-file` trước khi ghi), và đưa **mọi** hàng QUEUE
đang BLOCKED/REVIEW vào `blocked[]` kể cả khi status thật đã DONE.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HEAD_SHA = "e9c62cb11eae2725a024a833ec9aa3de257cb248"  # cập nhật khi A0 chốt head R36
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
    "ST-298": ["30890ba", "3daff62"],
    "ST-299a": ["5cc1903", "060a744"],
    "ST-299b": ["3b8049c", "aab5ffa"],
    "ST-300": ["3581c9a", "8286caf", "b3a9742", "84e8eb6"],
    "ST-300b": ["5675a7f", "770fdf7"],
    "ST-300m1": ["24ee56e", "2cbbed0", "d6aa839", "c3b7290"],
    "ST-300m2": ["101a4ad", "5741d23"],
    "ST-300m3": ["c1c125e", "c4337ce"],
    "ST-301": ["3e8c2cc", "2bc6be7"],
    "ST-302": ["d3ea467", "b79ae3a", "204af84", "6bc4ac3"],
    "ST-303": ["728a5bc", "a114e38"],
    "ST-304": ["6e0675a", "8e73e81"],
    "ST-304b": ["426d636", "1eb2768"],
    "ST-305": ["fa1a70b", "374b4e8", "60d64e2", "75e5a53", "7b79769"],
    "ST-306": ["90b2964", "6093aef"],
    "ST-307": ["3d247c4", "ab6beb7", "b5aa40b"],
    "ST-308": ["79a6d52", "a9eef01"],
    "ST-309": ["5902887", "51e937d", "f9e7503"],
    "ST-310a": [],
    "ST-310b": [],
    "ST-310c": [],
    "ST-310d": [],
    "ST-310e": [],
    "ST-310f": [],
}

# Ghi đè trạng thái khi sổ QUEUE@head chưa lật. Giá trị gốc vẫn ghi ở trường `queue`.
OVERRIDES = {
    "ST-300m3": "BLOCKED",  # QUEUE READY; macOS m3 đang chạy — chặn tag v0.5.0
    "ST-305": "REVIEW",     # QUEUE READY; code/R36 final2 xong, tag v0.5.0 chờ m3 + A0
}

NOTES = {
    "ST-297": "DONE f4cb136: gate 3660/4; Windows 3x full 3611/0/3; #149 đóng",
    "ST-298": "DONE: v0.4.5 tag @3daff62; soak 4h Windows 985 vòng/0 fail; Tier 1",
    "ST-299a": "DONE 5cc1903/060a744: WasiHost wasm32-wasip1; wasm 26/26",
    "ST-299b": "DONE 3b8049c/aab5ffa: vis-loader đa tệp; native 29/29, wasm 28/28",
    "ST-300": "DONE: vis-wasm-sem 388/390 eligible = 99,5%; 2 stack-overflow interp (#193)",
    "ST-300b": "DONE 770fdf7: tách vis-host-native/vis-pkg; main.rs 225 dòng",
    "ST-300m1": "DONE c3b7290 (merge): Darwin errno, #137 đóng; gate 3672/4; mps quick PASS; release arm64 PASS",
    "ST-300m2": "DONE 101a4ad/5741d23: mps soak 10 vòng 379,7 s vs ps 517,1 s = 0,73x; A/B 24 cặp interp 1,0385 / VM 1,0626; #147/#152 đóng",
    "ST-300m3": "ĐANG CHẠY trên mps: 3x vis-gate-macos --full + soak 4h + vis-wasm-sem; Tier 1 macOS CHƯA công bố — chặn tag v0.5.0",
    "ST-301": "DONE 2bc6be7: lambda annot + T?? + #192/#155; fuzz 1h 0 vi phạm",
    "ST-302": "DONE d3ea467 + merge b79ae3a/6bc4ac3: async impl/cmd #13 + derives Json #8 + spawn method; gate 3689/4",
    "ST-303": "DONE a114e38: vis new/doc/repl; gate 3671/4",
    "ST-304": "DONE 6e0675a + merge 8e73e81: vish-edit raw mode da nền, CST, history; gate 3679/4",
    "ST-304b": "DONE 426d636 + merge 1eb2768: completion theo kiểu + POSIX job control; 17/17, gate 3688/4",
    "ST-305": "REVIEW: code merged 374b4e8 + v0.5.0 prep 75e5a53; R36 final2 @e9c62cb: Linux gate 3754/4, release 3753/0/5 @c4337ce, Windows full 3701/0/4 @f9e7503; tag v0.5.0 chờ macOS m3",
    "ST-306": "DONE 90b2964 + merge 6093aef: VIS_PLUGIN_ABI_V1 + macros; parity interp-VM",
    "ST-307": "DONE 3d247c4 + merge ab6beb7/b5aa40b: plugin động WASI, 3 lớp ABI, sandbox cap; Windows test hardening",
    "ST-308": "DONE 79a6d52 + merge a9eef01: vish-term Terminal trait + AnsiPosix/WinConsole/ViTermFramed",
    "ST-309": "BLOCKED 2/3 DONE: SPEC 51e937d/f9e7503 + security F1/F2 5902887; QA dài 623f3ba mới một phần (parser/decode/diff PASS, fuzz_mutate chưa xong — #195; nhánh plan/st309-qa, chưa merge); 1/3 cuối chờ ST-310f/v0.5.5; nợ #194",
    "ST-310a": "BLOCKED: chờ tag v0.5.0; trích crates/vis-core, gỡ coupling vis-interp, đóng #112",
    "ST-310b": "BLOCKED: chờ ST-310a; đấu nối ViShell vish-term ← vish-edit ← vis-cli",
    "ST-310c": "BLOCKED: chờ ST-310a; tách vis-plugin-abi vs vis-plugin-host, thống nhất openat2, rà #194",
    "ST-310d": "BLOCKED: chờ ST-310a/310c; làm mỏng Cargo.toml vis-cli (7 dep thừa)",
    "ST-310e": "BLOCKED: chờ ST-310d; gom vis-cli/tests 27 → ≤ 12 binary + cổng chống phình",
    "ST-310f": "BLOCKED: chờ ST-310b/310e; tổ chức thư mục, vệ sinh agent/CHANGELOG, tag v0.5.5",
}

BLOCKED_Q = {
    "ST-300m3": "ĐANG CHẠY trên mps: 3x vis-gate-macos --full + soak 4h + vis-wasm-sem; Tier 1 macOS chưa công bố — chặn duy nhất của tag v0.5.0",
    "ST-305": "Code merged 374b4e8/75e5a53 + R36 final2 xong; tag v0.5.0 chờ macOS m3 xanh và A0 phê duyệt (chưa tag)",
    "ST-309": "2/3 DONE (SPEC + security + QA fuzz/diff); 1/3 cuối chờ ST-310f/v0.5.5: fresh-clone 3-OS Tier 1 + WASI + diff 100k ×5 + soak 24h + fuzz 8h + R37",
    "ST-310a": "Chờ tag v0.5.0; trích crates/vis-core, gỡ coupling vis-interp, đóng #112",
    "ST-310b": "Chờ ST-310a; đấu nối ViShell vish-term ← vish-edit ← vis-cli",
    "ST-310c": "Chờ ST-310a; tách vis-plugin-abi vs vis-plugin-host, thống nhất openat2, rà #194",
    "ST-310d": "Chờ ST-310a/310c; làm mỏng Cargo.toml vis-cli (7 dep thừa)",
    "ST-310e": "Chờ ST-310d; gom vis-cli/tests 27 → ≤ 12 binary + cổng chống phình",
    "ST-310f": "Chờ ST-310b/310e; tổ chức thư mục, vệ sinh agent/CHANGELOG, tag v0.5.5",
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
        # check_report bắt buộc: mọi hàng QUEUE@head BLOCKED/REVIEW phải có mặt trong blocked[],
        # kể cả khi bằng chứng đã cho phép ghi status DONE (sổ QUEUE chưa lật).
        if st in {"BLOCKED", "REVIEW"} or chuan(raw) in {"BLOCKED", "REVIEW"}:
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
            "seeds": 5,
            "failures": 0,
            "diff_seeds": [20261221, 20261222, 20261223, 20261224, 20261225],
            "note": "QA @c4337ce (plan/st309-qa 623f3ba): diff 100k x5 seed 20261221..25 lech=0 (500.000 chương trình); soak Linux @e9c62cb 10/10, 650 s, diff 100k x3 lech=0 (chưa có .md tổng kết)",
        },
        "coverage": {
            "tokens": 0,
            "fixtures": 525,
            "note": "test/sem 525 (475 .vis + 50 .vish); WASI local Node @c4337ce: vis-wasm-sem 388/390 eligible = 99,5%, vis-wasm-suite 28/28; 2 stack-overflow interp known gap (#193)",
        },
        "mutants": {"measured": False, "killed": None, "alive": None},
        "perf": {
            "head_do": HEAD_SHA,
            "baseline": "Giai đoạn 3 v0.5.0 — QA cuối trên main e9c62cb; chưa tag",
            "host": "ps (Linux x86_64, 12 CPU); rustc/cargo 1.100.0",
            "linux_gate": "PASS @e9c62cb: 3754 passed / 4 skipped (nextest) x3 (100,969 / 70,450 / 68,886 s); check fmt+clippy+R1..R7 PASS",
            "release_linux": "PASS @c4337ce (fresh clone): workspace release 3753/0/5 (126 binary); host 54/54; vis 0.5.0",
            "linux_soak": "PASS theo log @e9c62cb: 10/10, 650 s, exit 0 — gate 3754/4 x3, diff 100k lech=0 x3, host 54/54 x2, cache 5/5 x2; CHƯA có /home/ps/vislogs/final-linux-e9c62cb.md",
            "windows_full": "PASS @f9e7503 (psvn worktree riêng): 3701/0/4 (128 block), host 54/54, targets 18/18 + 17/1, 550 s",
            "windows_soak": "Kế thừa ST-298: 4 h / 985 vòng / 0 fail @3daff62 (v0.4.5); R36 chưa soak lại Windows",
            "macos": "m1/m2 DONE; m3 ĐANG CHẠY trên mps (3x vis-gate-macos --full + soak 4h + vis-wasm-sem) — Tier 1 macOS CHƯA công bố",
            "wasi": "Local Node @c4337ce: 388/390 eligible = 99,5% (2 stack-overflow interp known gap #193); suite 28/28; m3 sẽ chạy lại trên mps",
            "tags": "v0.4.0 @c2accdf; v0.4.1 @90844bc; v0.4.5 @3daff62; v0.5.0 CHƯA tag (chờ macOS m3 + A0)",
            "note": "Security F1/F2 vá @5902887; SPEC @51e937d/f9e7503; QA parser 1h 286.313.039 ca/0, decode 1h 2.517.069.629 ca/0, diff 5x100k=0 @623f3ba (pre-security/pre-SPEC, mốc c4337ce); nợ #194 (metadata TOCTOU), đề xuất #195 (VM back-edge O(n^2)/fuel wall-clock). Log ngoài repo + sha256 trong JOURNAL; tag message v0.5.0 + checklist 5 điều kiện tại JOURNAL/tag-v050.md, chưa tag",
        },
        "blocked": blocked,
    }

    # Không ghi commit không tồn tại: kiểm một lượt `git cat-file --batch-check`.
    shas = sorted({HEAD_SHA} | {c for t in tasks for c in t.get("commits") or []})
    inp = "".join(f"{s}^{{commit}}\n" for s in shas)
    out = subprocess.run(
        ["git", "cat-file", "--batch-check"], cwd=ROOT,
        input=inp, capture_output=True, text=True, check=False,
    )
    hong = [
        s for s, line in zip(shas, out.stdout.splitlines())
        if len(line.split()) < 2 or line.split()[1] != "commit"
    ]
    if hong:
        raise SystemExit(f"R36 JSON: commit/head không tồn tại: {hong}")

    out_path = ROOT / "agent/reports/R36-viscript.json"
    out_path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    license_path = ROOT / "agent/reports/R36-viscript.json.license"
    license_path.write_text(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n"
        "# R36 — ViScript, vòng 36 (Giai đoạn 3 `v0.5.0`, QA cuối @e9c62cb) — sinh bởi gen-r36-json.py\n",
        encoding="utf-8",
    )
    print(f"R36 JSON: {len(tasks)} tasks, {len(blocked)} blocked, head={HEAD_SHA}")


if __name__ == "__main__":
    main()
