# VS3 W1 — fuzz/nightly dài trên `70aeddc` (2026-11-21)

- **Baseline**: `main` `70aeddc` (v0.3.2 đã tag); nhánh `vs3/w1-runtime`, merge `main`
  (merge commit `1573bef`).
- **Mục tiêu**: hardening parser + vòng tròn fmt trên cây v0.3.2; không sửa sản phẩm.
- **Lệnh**:

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w1 \
  ./vis-dev nightly --skip-gate --fuzz-minutes 30
```

`--skip-gate` theo yêu cầu; fuzz và vòng tròn fmt vẫn chạy đủ. Binary dùng:
`target/debug/vis` + `target/release/vis-parse-fuzz` (symlink tới target `vs3-w1`).

## Kết quả fuzz parser

```text
vis-parse-fuzz: 260206874 ca, 5905617724 token, 144559 ca/s,
                3280899 token/s, 1800.0 s, vi phạm 0
vis-parse-fuzz: PASS (không vi phạm)
```

- Seed: `20261121` (`vis-dev` dùng `date +%Y%m%d`).
- Số ca: **260.206.874**; token: **5.905.617.724**; vi phạm: **0** (panic/CST lossless/span/
  chẩn đoán đều sạch).
- Không có seed lỗi cần lưu; không có ca FAIL.

## Vòng tròn fmt

```text
--- vòng tròn fmt: OK (475 tệp, bất động; bỏ qua 78 ca lỗi cú pháp)
```

- **475 tệp** `test/sem/**/*.vis` (trừ 78 ca có `.err`) được copy sang temp, `vis fmt --write`
  rồi `vis fmt --check`: bất động, 0 tệp hỏng.

## Kết luận hardening

```text
--- fuzz parser: OK
--- vòng tròn fmt: OK (475 tệp, bất động; bỏ qua 78 ca lỗi cú pháp)
vis-dev nightly: PASS
```

- Không phát hiện lỗi parser/fmt; không cần sửa sản phẩm hoặc mở blocker mới.
- Không chạy GitHub Actions; không push/merge.
- `df` trước/sau: ~20G → ~19G (target `vs3-w1` vẫn còn; không tạo target thứ hai).
- Log gốc: `/tmp/opencode/w1_fuzz_30.log`
  (`sha256 c506f3161742da181e7ec5c07e519f62b692106f5348e2c6788d2296aa6ec76d`).
