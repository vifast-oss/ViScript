<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Ma trận QA cuối `v0.5.0` (R36) — Linux / Windows / macOS / WASI

> **Trạng thái: KHUNG CHUẨN BỊ — chưa chạy.** Tệp này chốt **lệnh, tiêu chí PASS và nơi lưu
> bằng chứng** cho vòng nghiệm thu `v0.5.0`. Chỉ được điền số thật sau khi chạy; mọi số trong
> `R36-viscript.json` phải truy được về log ngoài repo + sha256. Không merge/push/tag trong lúc
> chạy; không dùng placeholder.
>
> **Head dự kiến**: sau khi A0 merge `ST-302` (`d3ea467`), `ST-304` (`6e0675a`), `ST-305`
> (`fa1a70b`) và các lát `ST-304b`/`ST-306`; head QA **phải là một commit đã chốt**, giống nhau
> trên `ps`, `psvn`, `mps` (cùng `git rev-parse HEAD`).

## 0. Điều kiện trước khi chạy

1. A0 chốt head QA `H`; ghi `H` vào đầu mọi log.
2. Cây sạch trên cả 3 máy: `git status --porcelain` rỗng; `psvn`/`mps` cùng `H`.
3. Không job nặng khác; `mps` khoá bằng `bin/vis-lock`; `psvn` dùng worktree riêng
   (`/c/Users/ps/viscript-r36-<sha>`), không dùng repo chung.
4. Log **ngoài repo**, thư mục gốc:
   - Linux `ps`: `/home/ps/vislogs/r36/`
   - Windows `psvn`: `/c/Users/ps/vislogs/r36/`
   - macOS `mps`: `/Users/mps/vislogs/r36/`
   Tên gợi ý `r36-<nền>-<lệnh>-<lần>-<sha7>.log`; mỗi log ghi `sha256` vào báo cáo/JOURNAL.
   Repo chỉ giữ tóm tắt `.md` (R7).
5. **Chặn đã biết**: `bin/vis-wasm-sem` đang hardcode oracle
   `/home/ps/vislogs/sem-native-baseline.tsv`. Trên `mps` phải kiểm tệp này trước; nếu thiếu,
   chép oracle vào đúng đường dẫn hoặc mở task sửa hằng số **trước** m3 — không sửa trong lúc QA.

## 1. Linux (`ps`) — gate + release + soak

Chạy trên bản clone sạch ngoài repo (không dùng cây soạn tài liệu):

```bash
H=<head-qa>
git clone --no-hardlinks /home/ps/ViScript /home/ps/tmp/r36-linux-$H
cd /home/ps/tmp/r36-linux-$H && git checkout --detach $H
export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp
./vis-dev check
./vis-dev gate --fast
./bin/vis-host-suite
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff
./bin/vis-wasm-sem
./bin/vis-release --out-dir /home/ps/tmp/r36-release-$H
/home/ps/tmp/r36-release-$H/vis --version
./bin/vis-soak --log /home/ps/vislogs/r36/linux-soak-$H.log
```

- `vis-release --out-dir` phải **trống** (hoặc `--force` có chủ đích); artifact **chỉ** `vis`,
  **không** `test-child`; `vis --version` in `0.5.0`.
- Gate chạy **trước** soak; soak 10 vòng là bắt buộc cho Linux (R35 dùng 10/10).
- Log: `gate.log`, `host.log`, `diff-100k.log`, `wasm-sem.log`, `release.log`, `soak.log`.

**PASS Linux**

| Mục | Tiêu chí |
|---|---|
| `vis-dev check` | exit 0: fmt, clippy, R1..R7, self-test, build, doc-examples |
| `gate --fast` | **0 failed**; skipped đúng danh sách known; `report-check` PASS |
| host | **54/54** |
| diff 100 k | **lech=0** |
| WASI | **≥ 388/390 eligible** (99,5%); 2 stack-overflow interp đã ghi nợ `#193` |
| release | chỉ `vis`; chạy `--version` = `0.5.0`; smoke `vis run` một ca `test/sem` |
| soak | **10/10 vòng**, 0 fail sản phẩm, RSS/fd phẳng |

## 2. Windows (`psvn`) — 3× full + soak 4 h

```bash
H=<head-qa>
# worktree riêng, sạch, cùng H; Git Bash
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-r36-$H \
VIS_GATE_WINDOWS_LOG=/home/ps/vislogs/r36/win-full-1.log \
  ./bin/vis-gate-windows --full
# lặp lần 2, 3 với win-full-2.log / win-full-3.log
```

Soak 4 h trên worktree cách ly (không chạy trong repo chung), mỗi vòng lặp `bin/vis-soak`
10 vòng và ghi tiếp cùng một log; script 4 h đặt **ngoài repo** (mẫu `st298-soak.sh`):

```bash
# trên psvn, Git Bash, trong worktree r36
timeout 4h bash -c 'while :; do
  ./bin/vis-soak --log /c/Users/ps/vislogs/r36/win-soak.log --append || exit 1
done'
```

- Mỗi 10 vòng ghi thêm `http2_it`, `cancel_http_` (default + serial) như R35.
- Log summary ngoài repo; ghi số vòng, fail sản phẩm, free RAM, proc/threads, dirty cuối.

**PASS Windows**

| Mục | Tiêu chí |
|---|---|
| 3× `--full` | **3/3 PASS**, mỗi lần 0 failed; host **54/54**; targets **18/18 + 17/1**; dirty **0** |
| Soak 4 h | **0 fail sản phẩm**; free RAM phẳng (không giảm đơn điệu); proc/threads 0 sau mỗi vòng |
| Handle | nếu `Get-Process` không còn tiến trình ⇒ ghi “không đo được sau soak”, dùng RSS/proc/threads |
| Kết thúc | HEAD `H`, dirty **0**, đủ dung lượng đĩa |

## 3. macOS arm64 (`mps`) — m2 trước, m3 sau

### 3a. `ST-300m2` — đo chuẩn độc quyền (chạy trước)

```bash
H=<head-qa>
bin/vis-remote mps --require-clean --log /home/ps/vislogs/r36/m2-gate.log -- \
  bash -lc 'cd /Users/mps/dvchd/ViScript && ./bin/vis-lock --timeout 7200 -- \
    ./vis-dev gate --fast'
# soak 10 vòng, tách biên dịch/chạy theo VD-182/VD-197
bin/vis-remote mps --require-clean --log /home/ps/vislogs/r36/m2-soak.log -- \
  bash -lc 'cd /Users/mps/dvchd/ViScript && ./bin/vis-lock --timeout 7200 -- \
    ./bin/vis-soak --log /Users/mps/vislogs/r36/m2-soak-10.log'
# 24 cặp A/B xen kẽ opt-bench.vis: 7c256df vs H, release
```

- Ghi bảng thành phần + thời gian biên dịch/chạy riêng vào `JOURNAL/st300m2-mac-bench.md`;
  cập nhật `SO-NO.md` (giữ R5). **Chưa có kết quả tại thời điểm dựng R36.**

### 3b. `ST-300m3` — chứng nhận Tier 1

```bash
H=<head-qa>
for i in 1 2 3; do
  VIS_GATE_MACOS_LOG=/home/ps/vislogs/r36/mac-full-$i.log \
    ./bin/vis-gate-macos --full || exit 1
done
# soak 4h trên mps (log ngoài repo, ghi tiếp)
bin/vis-remote mps --require-clean --log /home/ps/vislogs/r36/mac-soak.log -- \
  bash -lc 'cd /Users/mps/dvchd/ViScript && timeout 4h bash -c "while :; do
    ./bin/vis-lock --timeout 7200 -- ./bin/vis-soak \
      --log /Users/mps/vislogs/r36/mac-soak-4h.log --append || exit 1
  done"'
# WASI trên mps — kiểm oracle trước (xem §0.5)
bin/vis-remote mps --require-clean --log /home/ps/vislogs/r36/mac-wasm-sem.log -- \
  bash -lc 'cd /Users/mps/dvchd/ViScript && ./bin/vis-lock --timeout 7200 -- ./bin/vis-wasm-sem'
```

**PASS macOS**

| Mục | Tiêu chí |
|---|---|
| 3× `--full` | **3/3 PASS**: debug + release + host **54/54** + WASI suite **28/28** + release smoke; dirty **0** |
| Soak 4 h | **0 fail sản phẩm**; RSS/fd phẳng; cùng `H` |
| `vis-wasm-sem` | **≥ 388/390 eligible**; ghi known-gap stack-overflow interp |
| Tuyên bố | chỉ sau đủ 3 mục trên; ghi caveat **không Landlock**, phạm vi `aarch64-apple-darwin`; cập nhật `HOST-BAO-DAM.md`/`ROADMAP.md` |

## 4. WASI — dùng chung

`bin/vis-wasm-sem` so `rc + sha256(stdout)` với oracle native 475/475. Chạy trên Linux và macOS;
log `wasm-sem.log`. PASS: **≥ 388/390 eligible = 99,5%**, 0 sai khác mới ngoài 2 ca
`de-quy-1000`/`de-quy-vo-han` (stack overflow interp, VM pass, nợ `#193`).

## 5. Thứ tự chạy và điều kiện dừng

1. Merge đủ lát + chốt `H`; kiểm 3 máy cùng `H`, cây sạch.
2. Linux `check` + `gate --fast` → release → host → diff → WASI → soak.
3. Windows 3× full → soak 4 h.
4. macOS `m2` → `m3` (3× full → soak 4 h → `vis-wasm-sem`).
5. Điền số thật vào `R36-viscript.json`/`.md`, chạy `python3 test/report/check_report.py --all`,
   rồi A0 tag `v0.5.0`.

**Dừng ngay** (không tag) nếu: bất kỳ full/gate nào có fail sản phẩm; diff 100 k lệch; WASI dưới
ngưỡng; soak có fail lặp lại; cây bẩn/HEAD lệch. Flake môi trường được rerun **một lần**, phải
ghi rõ lý do + log cả hai lần; không retry mù.

## 6. Bảng ghi bằng chứng (điền khi chạy)

| Nền | Lệnh | Kết quả | Log ngoài repo | sha256 |
|---|---|---|---|---|
| Linux | `vis-dev gate --fast` | — | `/home/ps/vislogs/r36/` | — |
| Linux | `vis-release` | — | `/home/ps/vislogs/r36/` | — |
| Linux | soak 10 vòng | — | `/home/ps/vislogs/r36/` | — |
| Windows | 3× `--full` | — | `/c/Users/ps/vislogs/r36/` | — |
| Windows | soak 4 h | — | `/c/Users/ps/vislogs/r36/` | — |
| macOS | `m2` gate/soak/A-B | — | `/Users/mps/vislogs/r36/` | — |
| macOS | 3× `--full` | — | `/Users/mps/vislogs/r36/` | — |
| macOS | soak 4 h | — | `/Users/mps/vislogs/r36/` | — |
| macOS | `vis-wasm-sem` | — | `/Users/mps/vislogs/r36/` | — |

Hiện trạng R36: **mọi dòng còn `—`**; `ST-300m2`/`ST-300m3` vẫn `BLOCKED`, chưa có số đo.
