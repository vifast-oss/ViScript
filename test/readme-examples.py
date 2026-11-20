#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Kiểm mọi khối mã ViScript trong README/CHEAT-SHEET bằng `vis check`.

Khác `test/doc-examples/doc_examples.py` (cổng QD §7.4, chỉ quét `docs/*.md`
và có số liệu do Agent 0 chốt), tập lệnh này **không đổi ngưỡng gì cả**: nó chỉ
đảm bảo các ví dụ hướng dẫn trên README vẫn biên dịch được sau mỗi lần sửa code.

Quy ước fence: ViScript chưa có grammar trong GitHub Linguist, nên README gắn
` ```python title="ViScript" ` — GitHub tô màu theo Python, còn tập lệnh này
nhận diện qua chữ `ViScript` trong info string để check bằng `vis check`.
Khối ` ```python ` thật (ví dụ FizzBuzz bằng Python) không có chữ đó ⇒ bị bỏ qua.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = (REPO / "README.md", REPO / "CHEAT-SHEET-ViScript.md")
FENCE = re.compile(r"^```(.*)$")
CLOSE = re.compile(r"^```\s*$")


def blocks():
    """Yield (tên tệp, dòng mở khối, nội dung) cho mọi khối ViScript."""
    for md in DOCS:
        if not md.is_file():
            continue
        lines = md.read_text(encoding="utf-8").splitlines()
        i = 0
        while i < len(lines):
            m = FENCE.match(lines[i])
            if not m:
                i += 1
                continue
            info = m.group(1).strip()
            i += 1
            body: list[str] = []
            while i < len(lines) and not CLOSE.match(lines[i]):
                body.append(lines[i])
                i += 1
            i += 1
            low = info.lower()
            if "viscript" in low or re.match(r"^vis\b", low):
                yield md.name, i - len(body) - 1, "\n".join(body) + "\n"


def resolve_bin(explicit: str | None) -> str:
    if explicit:
        p = Path(explicit)
        if not p.is_file():
            raise SystemExit(f"readme-examples: không thấy binary {p}")
        return str(p)
    built = REPO / "target" / "debug" / "vis"
    if built.is_file():
        return str(built)
    shim = REPO / "vis"
    if shim.is_file():
        return str(shim)
    raise SystemExit("readme-examples: chưa có target/debug/vis — chạy build trước")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--vis", help="đường dẫn tới binary vis (mặc định target/debug/vis)")
    args = ap.parse_args()

    vis = resolve_bin(args.vis)
    found = list(blocks())
    if not found:
        print("readme-examples: KHÔNG thấy khối ViScript nào — có đang đổi sai fence?", file=sys.stderr)
        return 1

    print("TAP version 13")
    print(f"1..{len(found)}")
    bad = 0
    with tempfile.TemporaryDirectory(prefix="readme-examples-") as td:
        for idx, (name, line, body) in enumerate(found, 1):
            path = Path(td) / f"b{line}.vis"
            path.write_text(body, encoding="utf-8")
            r = subprocess.run(
                [vis, "check", str(path)],
                cwd=REPO,
                capture_output=True,
                text=True,
            )
            detail = (r.stdout + r.stderr).strip().splitlines()
            if r.returncode == 0:
                print(f"ok {idx} - {name}:{line}")
            else:
                bad += 1
                msg = detail[-1] if detail else f"exit {r.returncode}"
                print(f"not ok {idx} - {name}:{line}: {msg}")
    if bad:
        print(f"# readme-examples: {bad}/{len(found)} khối đỏ", file=sys.stderr)
        return 1
    print(f"# readme-examples: {len(found)}/{len(found)} khối xanh")
    return 0


if __name__ == "__main__":
    sys.exit(main())
