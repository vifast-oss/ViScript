<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Tag `v0.5.0` — message soạn sẵn + checklist 5 điều kiện (final2)

> **Trạng thái: SOẠN SẴN — CHƯA TAG.** Head ứng viên
> **`e9c62cb11eae2725a024a833ec9aa3de257cb248`** (version `0.5.0`; gồm security hotfix
> `5902887`, `SPEC.md` `51e937d`/`f9e7503`). Không tạo tag/push/merge trong bước này.
> Chỉ tạo tag annotated sau khi **đủ 5/5 điều kiện** **và** A0 phê duyệt head cuối.
> Máy `mps` đang chạy `ST-300m3` — **Tier 1 macOS chưa công bố**.

## 1. Tag message (nguyên văn, dùng cho `git tag -a`)

```text
ViScript v0.5.0 — 2026-11-23 (Giai đoạn 3: Pha 2 ngôn ngữ + ViShell + LSP/VS Code + Plugin)

- ST-300m2: đo độc quyền mps arm64 — soak 10 vòng 379,7 s vs ps 517,1 s = 0,73×;
  A/B 24 cặp interp 1,0385 / VM 1,0626; #147/#152 đóng.
- ST-302/302b: async fn trong impl + async cmd (#13), derives Json (#8),
  spawn obj.m(); gate 3689/4.
- ST-304/304b: vish-edit (raw mode 3 nền, CST, history mờ, Ctrl-C 130),
  completion theo kiểu + POSIX job control; 17/17.
- ST-305: LSP hover generic + completion std.*, editors/vscode/.
- ST-306/307: plugin tĩnh VIS_PLUGIN_ABI_V1 + macros; plugin động WASI/WASM
  sandbox cap, 3 lớp ABI; vá race driver WASI c4337ce.
- ST-308: vish-term Terminal trait + AnsiPosix/WinConsole/ViTermFramed.
- ST-309 (2/3): docs/SPEC.md tự chứa 1.181 dòng (51e937d/f9e7503); kiểm toán
  bảo mật vá F1/F2 symlink cap + plugin (5902887), mở #194; QA fuzz parser 1h
  286.313.039 ca/0, decode 1h 2.517.069.629 ca/0, diff 5x100k lech=0 (623f3ba).
- QA final2 @e9c62cb: Linux gate 3754/4 x3; Linux soak final 10/10
  (650 s, diff 100k lech=0 x3, host 54/54 x2 — log, chưa có .md tổng kết);
  Linux release 3753/0/5 (fresh clone c4337ce, host 54/54, vis 0.5.0);
  Windows full 3701/0/4 (f9e7503, host 54/54, targets 18/18 + 17/1).
- macOS arm64 Tier 1 theo ST-300m3 (3× vis-gate-macos --full + soak 4 h +
  vis-wasm-sem ≥ 388/390) — số đo trong R36-viscript.md; CHƯA công bố tại
  thời điểm soạn tag message.
- Nợ đã biết: #194 metadata TOCTOU; đề xuất #195 VM back-edge O(n²)/fuel
  wall-clock; #193 WASI interp stack 2/390; #156 sqlite wasm32.
- Version 0.5.0; cache PHIEN_BAN +v050-release-2026-11-23.
- Tag trước: v0.4.5 @ 3daff62.
```

### Lệnh tạo tag (chỉ chạy sau khi đủ 5/5 + A0 duyệt)

```bash
# Tách đúng khối message ở §1 (bỏ dòng ``` đầu/cuối)
sed -n '/^```text$/,/^```$/p' agent/JOURNAL/tag-v050.md | sed '1d;$d' > /tmp/tag-v050.msg
git tag -a v0.5.0 -F /tmp/tag-v050.msg --cleanup=verbatim <head-da-chot>
git tag -l -n99 v0.5.0   # kiểm message
# KHÔNG push nếu chưa được phép
```

## 2. Checklist 5 điều kiện

| # | Điều kiện | Bằng chứng | Trạng thái @`e9c62cb` |
|---|---|---|---|
| 1 | Linux `./vis-dev gate --fast` 0 failed; skipped đúng danh sách known | `JOURNAL/final-linux-gate-e9c62cb.md` §2 — **3754/4 ×3** | **PASS** |
| 2 | Linux release `3753/0/5` + `vis 0.5.0` + host 54/54 trên fresh clone | `JOURNAL/final-linux-release-c4337ce.md` §1–4 | **PASS** @`c4337ce` |
| 3 | Linux soak 10/10 + diff 100 k `lech=0` + host 54/54 | `JOURNAL/final-linux-gate-e9c62cb.md` §3 — soak **đang chạy**; diff 5×100k `lech=0` @`623f3ba` | **ĐANG CHẠY** |
| 4 | Windows full `3701/0/4` + host 54/54 + targets `18/18` & `17/1` | `JOURNAL/win-full-f9e7503.md` §0–4 — **550 s** | **PASS** @`f9e7503` |
| 5 | macOS arm64 Tier 1: 3× `bin/vis-gate-macos --full` PASS + soak 4 h (0 fail, RSS/fd phẳng) + `vis-wasm-sem` ≥ 388/390 | `mps`, `ST-300m3` | **ĐANG CHẠY — chưa có kết quả** |

**Điều kiện bổ sung trước khi tag**: `R36-viscript.json` chốt đúng head được tag;
`python3 test/report/check_report.py --all` PASS; A0 phê duyệt checklist này.

## 3. Sau khi `m3` xong

1. Ghi kết quả `m3` vào `JOURNAL/` (log ngoài repo + sha256), `R36-viscript.{md,json}`.
2. Cập nhật `HOST-BAO-DAM.md` & `ROADMAP.md` tuyên bố **macOS arm64 Tier 1** (caveat: không có
   Landlock; phạm vi `aarch64-apple-darwin`) — chỉ khi `m3` xanh.
3. Đọc kết quả soak Linux `e9c62cb` (`/home/ps/vislogs/final-linux-e9c62cb.log`); nếu 10/10 và
   có `.md` tổng kết thì cập nhật điều kiện 3 thành **PASS**.
4. A0 chốt head cuối (nếu khác `e9c62cb`, chạy lại `gen-r36-json.py` + `report-check`).
5. Tạo tag `v0.5.0` bằng lệnh ở §1; **không push** nếu chưa được phép.
