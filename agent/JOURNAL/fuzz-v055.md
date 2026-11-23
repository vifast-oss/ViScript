<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Fuzz `v0.5.5` — parser 2h, verifier/decoder 60M ca, differential 100k

> **Người làm**: subagent fuzz, 2026-11-24. **Nhánh** `plan/fuzz-v055` từ **`b4db374`**
> (tag `v0.5.5`). **Local only** — không ssh `mps`/`psvn`, không merge/push/tag.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`; **worktree riêng** `/home/ps/tmp/fuzz-v055`,
> `CARGO_TARGET_DIR=/home/ps/tmp/fuzz-v055-target` (không dùng chung target với cây chính).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Fuzz **parser** 2h, seed mới `202611240550` | **PASS** — 861.371.033 ca / 19.550.360.573 token / **0 vi phạm** (7.200,0 s) |
| Fuzz **verifier/decoder** `st309_fuzz_verify`, 60.000.000 ca | **PASS** — `vi_pham=0` (0 panic), 6.888.776 decode OK / 6.007.365 verify OK, 40,66 s |
| Differential `VIS_DIFF_N=100000`, seed mới `202611240550` | **lech=0** / 93.768 chương trình chạy được (41,7 s) |
| Blocker | **Không** — không có vi phạm/panic/lệch; không có artifact phải lưu |

## 1. Môi trường

```text
HEAD    b4db374ac16223ae01d184fdfd3033ba1175b435  [ST-310e] t3: timeout 30s (macOS flake sat 8s, test-only)
tag     v0.5.5
rustc   1.100.0 (0ef1bc386 2026-11-09)   # qua rustup shim ~/.cargo/bin (rustc hệ thống 1.93.1 không đủ MSRV)
host    Linux 7.0.0-43-generic x86_64, i5-1235U, 12 CPU, 14 GiB RAM
```

- Cây làm việc: worktree sạch `/home/ps/tmp/fuzz-v055` tại đúng `b4db374`; `git status` trống
  trước khi chạy.
- Build release **chỉ** các đích cần: `vis-syntax --bin vis-parse-fuzz`,
  `vis-bytecode --test st309_fuzz_verify`, `vis-vm --test diff_100k`.

## 2. Fuzz parser 2 giờ — seed mới `202611240550`

```text
vis-parse-fuzz: 861371033 ca, 19550360573 token, 119635 ca/s, 2715328 token/s, 7200.0 s, vi phạm 0
vis-parse-fuzz: PASS (không vi phạm)
```

- Lệnh: `vis-parse-fuzz --seconds 7200 --seed 202611240550` (release, sha256 binary
  `d6c67384b3a12e7eaab50d740b45aa48d008ea46157082f1d964967903478cd5`).
- Bất biến harness: không panic; CST **lossless** khi parse sạch; span hợp lệ; có lá lỗi ⇒ có
  chẩn đoán. Seed mới (các lần trước dùng `20261221`/`20261226`).
- **0 vi phạm** ⇒ không có seed/corpus lỗi cần lưu; log đầy đủ ngoài repo (mục 5).

## 3. Fuzz verifier/decoder — 60.000.000 ca

```text
ST309-FUZZ ca=60000000 decode_ok=6888776 verify_ok=6007365 vi_pham=0 giay=40.66 ca_giay=1475630
test decoder_verifier_khong_panic ... ok
```

- Lệnh: `VBC_FUZZ_CA=60000000 cargo test --release -p vis-bytecode --test st309_fuzz_verify
  -- --nocapture` (binary sha256 `945a18cac24d2c0df3e46fd090a4dbdd33a8cbb853a8f2431c73cda12edf4767`).
- Mỗi ca: đột biến 1–3 lần (bit/byte/cắt cụt/chèn/xoá) trên bytecode encode của 8 module mẫu;
  `decode` không panic; nếu decode OK thì `verify` + `need_stack` không panic. **Không chạy VM**
  (bề mặt VM đã có `fuzz_mutate`, xem ghi chú mục 6).
- `vi_pham=0`, không có artifact. 60M là **cận trên** của khoảng 30–60M yêu cầu.

## 4. Differential `VIS_DIFF_N=100000` — seed mới `202611240550`

```text
DIFF tong=100000 loi_tinh=6232 chay=93768 lech=0 giay=41.7 toc_do=2400/s seed=202611240550
```

- Lệnh: `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611240550 <diff_100k> diff_chuong_trinh_sinh_tu_dong
  --nocapture` (binary sha256 `fef3ed53570a79138e807266018f33ab809b5ed66b485779c82bfb406989d539`).
- **lech=0** trên 93.768 chương trình chạy được (interp vs VM: stdout, exit, chẩn đoán).
- `loi_tinh=6.232` là **mẫu âm cố ý** của bộ sinh hỗn hợp, bị loại ở parse/lower/check — không
  phải lỗi. Kiểm chứng bằng `VIS_DIFF_DEBUG=1`: toàn `CHECK VS0412` (`send` sai kiểu), `CHECK
  VS0401` (thiếu đối số `can`), `CHECK ... await cần Task[T]`.

## 5. Log ngoài repo + sha256

Thư mục log: `/home/ps/tmp/fuzz-v055-logs/` (ngoài repo, không commit).

| Tệp | Nội dung | sha256 |
|---|---|---|
| `parser-2h.log` | fuzz parser 2h | `86edef16e4b4ec5ac462e17650626e978a150d320e8ef5f4ded63e13bf950777` |
| `verify-60M.log` | fuzz verifier 60M ca | `b6a1a299e0f4476ce90d79fc807d1c1e74fede86a14c433f855d949ad5adf7ab` |
| `diff-100k.log` | differential 100k | `27e019b79f14b5e33190acb5ee7186f00eabb02c541e5071f7bc90d93854914a` |
| `build-diff.log` | build release `diff_100k` | `755636c1b46ad7c83cedb3ae1d7ce87f71186133ae7474b7899a6eef9361cd0a` |

## 6. Ghi chú / lệch so với yêu cầu

- **`fuzz_decode_only` không có ở `b4db374`**: harness này chỉ tồn tại trên nhánh **chưa merge**
  `plan/st309-qa` (`623f3ba`), không nằm trong tag `v0.5.5`. Bề mặt nó phủ (`decode`+`verify` thuần)
  trùng với `st309_fuzz_verify` — harness **có** ở tag và chạy **theo số ca** (phù hợp mốc 30–60M),
  nên đã dùng `st309_fuzz_verify` cho 60M ca. Không chép tệp từ nhánh khác vào cây tag.
- **Không chạy `vis-vm/tests/fuzz_mutate.rs`**: harness đó chạy cả VM và đã có phát hiện O(n²)
  `IterInit`/`loop_ends` (nợ `#195`, journal `st309-qa.md`); nó chạy theo **thời gian** và có thể
  treo >100 phút một ca, không phù hợp mốc "30–60M ca, 0 panic" trong time-box này.
- Không sửa sản phẩm, không đổi test, không merge/push/tag.

## 7. Kết luận

- Parser 2h seed mới: **0 vi phạm**. Verifier/decoder 60M ca: **0 panic**. Differential 100k seed
  mới: **0 lệch**. **Không blocker**, không artifact.
