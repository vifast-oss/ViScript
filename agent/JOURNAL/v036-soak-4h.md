<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Soak dài `v0.3.6` @ tag `8fa2c09` — 14 lần × 10 vòng

> **Người chạy**: W3 (Gatekeeper), 2026-11-22 17:48–20:21 JST (~2 h 33 phút thực đo; time-box 4,5 h).
> **Phạm vi**: **chỉ đo**. Không sửa code, không push/merge/tag, không GitHub Actions.
> **Tag**: **`v0.3.6` @ `8fa2c09`**; worktree `plan/w3-platform` merge `main` lên `54a2653`
> (docs sau tag); `git diff 8fa2c09..HEAD -- crates Cargo.toml Cargo.lock` **rỗng** ⇒ soak đúng mã tag.

## 0. TL;DR

| Chỉ số | Giá trị |
|---|---|
| Số lần chạy | **14** (12 lần 10/10 pass; 2 lần dừng ở gate-2) |
| Vòng hoàn tất | **120/120 vòng** (12 × 10) + 2 gate-1 OK trong 2 lần dừng sớm |
| Kết quả | **12/14 lần PASS**; 1 **T-INFRA disk**; 1 **timing flake test-only dưới tải cao** |
| `vs3_http_cancel` h1 | **không tái hiện** trong 36 gate của 12 lần pass |
| `vis --version` | **`vis 0.3.6`** mọi lần |
| Wall (đo soak) | **9.117 s = 2 h 31 m 57 s** (từ 08:48:39Z đến 11:20:35Z) |
| Đĩa | cuối **30 G** (> 5 G); target riêng xoá sau mỗi lần |
| Kết luận | **không có blocker sản phẩm**; tag giữ nguyên. Flake còn lại là **test timing** dưới tải, không phải h1 cancel |

## 1. Mốc / môi trường

| Mục | Giá trị |
|---|---|
| Tag | **`v0.3.6` @ `8fa2c09`** (`git tag --points-at`) |
| HEAD khi soak | `54a2653` (docs sau tag; mã **identical** `8fa2c09`) |
| Nền | Linux `ps` x86_64, 12 CPU |
| Toolchain | `cargo`/`rustc` **1.100.0** |
| Mỗi lần | `CARGO_TARGET_DIR=target/soak-4h-<n>` — **riêng**, `CARGO_INCREMENTAL=0`, `debug=0` |
| Tải | load 3,1–25,2 (worktree khác build song song; đỉnh ở lần 13) |
| Log | `/home/ps/v036-soak-4h/summary.tsv`, `soak-<n>.log`, `soak-<n>.out` |

## 2. Bảng từng lần

Mỗi lần pass = 3× `gate --fast`, 3× diff 100k seed `20261115`, 2× host, 2× cache.

| Lần | Bắt đầu UTC | Giây | Gate OK | Diff 0 | Host 54 | Cache 5 | Kết quả |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | 08:48:39 | 894 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 2 | 09:03:34 | 304 | 1 | 0 | 0 | 0 | **DỪNG — T-INFRA disk quota** |
| 3 | 09:09:15 | 575 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 4 | 09:18:50 | 627 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 5 | 09:29:17 | 570 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 6 | 09:38:48 | 573 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 7 | 09:48:21 | 572 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 8 | 09:58:05 | 613 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 9 | 10:08:18 | 824 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 10 | 10:22:03 | 774 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 11 | 10:34:58 | 755 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 12 | 10:47:33 | 788 | 3 | 3 | 2 | 2 | **PASS 10/10** |
| 13 | 11:01:01 | 283 | 2 | 0 | 0 | 0 | **DỪNG — timing flake `vd270_test_slowest`** |
| 14 | 11:06:50 | 825 | 3 | 3 | 2 | 2 | **PASS 10/10** (bù mức tối thiểu) |

- 12 lần pass: mỗi lần gate **3581/3**, 3 diff **`lech=0`**, host **54/54**, cache **5/5**.
- Run 13 dừng theo luật stop-on-fail; run 14 chỉ chạy **một lần bù tối thiểu** (không retry run 13)
  sau khi phân loại lỗi là timing test dưới tải, để đạt mốc **12 lần / 2,5 h**.
- Không có lần nào fail vì `vs3_http_cancel`; fix h1 Windows/trigger byte giữ xanh trên Linux.

## 3. Hai lần dừng — phân loại

### Run 2 — **T-INFRA** (disk quota `/tmp`)
- Gate-1 OK 181 s; gate-2 exit 1 sau 123 s: `Disk quota exceeded (os error 122)` (log
  `soak-2.log`). `/tmp` là **tmpfs 7,2 G** và bị các tiến trình khác dùng gần đầy (còn ~1,1 G);
  không phải test fail.
- Xử lý: chuyển log sang `/home/ps/v036-soak-4h`, đặt `TMPDIR=/home/ps/.cache/v036-soak-tmp`
  (đĩa `/home`), chạy lại từ lần 3. Không còn lỗi disk ở 12 lần sau.
- Không tính là lỗi sản phẩm; ghi **T-INFRA**.

### Run 13 — **timing flake test-only dưới tải cao**
- Gate-1 OK 138 s; gate-2 exit 1 sau 144 s: **2 test** `vd270_test_slowest::slowest_1_chi_ca_cham`
  và `slowest_2_giam_dan` fail; summary `2073 passed, 2 failed, 3 skipped`; load lúc đó
  **25,16 / 17,92 / 15,33**.
- Thông điệp: `# slowest: 499 ms - nhanh` / `# slowest: 190 ms - cham` — thứ tự thời gian bị đảo
  dưới tải; assertion `dòng đầu phải là ca chậm` (`vd270_test_slowest.rs:153`) và
  `us0 >= us1` (`:155-157`) — **test đo wall-clock**, không phải lỗi sản phẩm.
- Chạy riêng 3 lần trên cùng mã sau đó: **3/3 pass** (7/7 test, ~0,8 s mỗi lần).
- Phân loại: **flake test timing/T-INFRA tải**, không phải h1 cancel, không phải regression tag.
- Đề xuất W4: làm test chịu tải (tách biên thời gian lớn hơn, hoặc skip/`#[ignore]` khi load cao
  có ghi lý do) trước soak dài tiếp theo; **không sửa code trong phiên này**.

## 4. Tổng hợp

- **12 lần 10/10 = 120 vòng hoàn tất**, không có fail sản phẩm; 36 gate pass, 36 diff 100k
  `lech=0`, 24 host 54/54, 24 cache 5/5.
- **1 T-INFRA disk** (run 2, môi trường `/tmp` tmpfs) và **1 flake timing test-only** (run 13,
  load 25) — cả hai đã phân loại, không phải lỗi mã tag.
- `vis --version` = **0.3.6** ở cả 14 lần.
- Wall soak **2 h 31 m 57 s**; dừng vì 2 lần fail theo luật stop-on-fail, không chạy đủ 20 lần.
  Mức tối thiểu **12 lần / 2,5 h** đã đạt.

## 5. Blocker / khuyến nghị

1. **Không có blocker sản phẩm** cho tag `v0.3.6`; h1 cancel không tái hiện.
2. **Blocker vận hành soak**: `/tmp` tmpfs 7,2 G dễ đầy khi nhiều worktree build; phải dùng
   `TMPDIR` trên `/home` và log ngoài `/tmp`.
3. **Flake test-only** `vd270_test_slowest` dưới load >~20 — đề nghị W4 làm test chịu tải trước
   soak 4 h đầy đủ; không ảnh hưởng tag.
4. Không tag/push/merge trong phiên này (tag `v0.3.6` đã có @ `8fa2c09`). Không GitHub Actions.
5. Đĩa cuối **30 G** (> 5 G); target từng lần đã xoá; giữ log `/home/ps/v036-soak-4h`
   (13 MB) và `target/diag` 459 MB.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry
git merge main   # 54a2653 docs sau tag; code identical 8fa2c09
for i in $(seq 1 20); do
  export CARGO_TARGET_DIR=$PWD/target/soak-4h-$i
  export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  export TMPDIR=/home/ps/.cache/v036-soak-tmp
  ./bin/vis-soak --log /home/ps/v036-soak-4h/soak-$i.log || break
  rm -rf "$CARGO_TARGET_DIR"
done
```
