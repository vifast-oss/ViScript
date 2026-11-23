<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300m1 — Darwin errno + quyền symlink + `vis-gate-macos` + release/targets

> **Người làm**: VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st300m1-macos` từ **`700bb66`**
> (main sau VD-284). **Không merge/push/tag**. `CARGO_BUILD_JOBS=3`,
> `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`, target riêng `target/st300m1`;
> trên `mps` dùng `bin/vis-lock` cho mọi job nặng.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Darwin errno | `rustix::io::Errno::{LOOP,XDEV,INVAL,ISDIR,EXIST,INPROGRESS,ALREADY,AGAIN}`; 0 hằng số thô còn lại |
| Symlink thoát gốc | trả **`VS0604` `[quyền]`**; `quyen_st295` ca mới đỏ-trước trên macOS |
| `quyen_st174` + `quyen_st295` trên APFS `mps` | **6/6 PASS**; rò **0/100** (nhánh mới) vs **100/100** (nhánh cũ) |
| `lat256_net` + `lat3_real` | mở `cfg(unix)`; **124 chạy / 121 pass / 3 ignored** trên mps (kqueue) |
| `bin/vis-gate-macos` | `--quick`/`--full`/`--dry-run`/`--self-test`; Python **3.9.6** trên mps; quick **PASS** |
| `bin/vis-release` | `aarch64-apple-darwin` trên mps: **PASS**, artifact chỉ `vis` (9.688.064 B), **không `test-child`** |
| `bin/vis-targets` | core **18/18**, sqlite **16 + 2 EXPECTED_RED**; thêm dòng `aarch64-apple-darwin (native)` |
| Gate Linux | **3672 passed / 0 failed / 4 skipped** (+1 ca mới so với 3671) |
| `#137` | **đóng** trong `SO-NO.md`; R5 vẫn **194 dòng** (VS3 16, đóng 90) |
| Blocker | **không** |

## 1. Errno Darwin

- `quyen_guard.rs`: `Some(40) | Some(18)` → `Errno::LOOP | Errno::XDEV` (Darwin `ELOOP = 62`);
  `Some(21)` → `Errno::ISDIR`.
- `quyen_mo.rs`: `22/21/17` → `Errno::{INVAL,ISDIR,EXIST}` qua helper `loi_os` (0 unsafe).
- `async_net_nguon.rs`: nhánh Unix `INPROGRESS | ALREADY | AGAIN`; Windows giữ
  `WSAEWOULDBLOCK/WSAEINPROGRESS` 10035/10036.
- `lat256_net.rs`: `cfg(unix)` + `Errno::INPROGRESS`; `lat3_real.rs`: `cfg(unix)`;
  `mio` dev-dependency đổi từ Linux-only sang `cfg(unix)`.
- Sửa thêm lỗi **có sẵn trên main**: `run_cmd.rs` nhánh wasm32 gọi
  `run_async_workers_with_full_hosts_caps` thiếu backend (10/11 tham số) ⇒ core targets
  **17/18**; thêm `None` (wasm không có reactor frame) ⇒ **18/18**.

## 2. Bằng chứng quyền trên APFS `mps`

`bin/vis-remote mps ... cargo test -p vis-host-native --lib quyen_st -- --nocapture`:

```
ST174 ca-A: lap=100 ro_ri=0 chan=100 chan_thay_doi=100
ST174 ca-A err: VS0604:[quyền] ... bị thay đổi trong khi mở
ST174 ca-B: lap=100 ok_trong=100 loi=0
ST295-DUA moi: ro=0/100 dem=100/100 (openat/O_NOFOLLOW)
ST295-DUA cu:  ro=100/100 dem=100/100 (nhanh kiem-roi-mo cu)
test result: ok. 6 passed; 0 failed
```

Ca mới `quyen_st295::symlink_thoat_goc_tra_vs0604_tren_moi_nen` khoá đúng lỗi Darwin:
trước vá `ELOOP=62` không khớp `40|18` ⇒ mất `VS0604`; nay xanh trên Linux và mps.

## 3. Tooling

- `bin/vis-gate-macos` (Python 3.9+): bọc `bin/vis-remote mps --require-clean`,
  **HEAD guard** (mps phải cùng commit), `bin/vis-lock --timeout`, log ngoài repo.
  - `--quick`: `cargo check --workspace` + `quyen_st*` + `async_it lat`.
  - `--full`: `vis-dev check` + `gate --fast` + `cargo test --workspace --no-fail-fast`
    + `vis-host-wasm` + `vis-wasm-suite --build --runtime node`.
  - `--dry-run` in lệnh; `--self-test` 10 kiểm (đã chạy **Python 3.9.6 trên mps**).
- `bin/vis-release --target aarch64-apple-darwin` trên mps: PASS, chỉ `vis`.
- `bin/vis-targets`: thêm mục `=== aarch64-apple-darwin (native) ===` (release + gate-macos,
  không cross-check).

## 4. Cổng / log (ngoài repo)

| Log | sha256 |
|---|---|
| `/home/ps/vislogs/st300m1-gate-final.log` (363 KB) | `94408aca171ae20f8a0fa068f8e32f478f027c3af46bbb29e61b2faf64c0cd8e` |
| `/home/ps/vislogs/st300m1-check.log` | `9f82ea982117e53e48f95005dbd6938702e2192f4910d5485abd06c210d2afdb` |
| `/home/ps/vislogs/st300m1-targets-2.log` | `5322f88546b037f99c0e37cfb17e38d9910537bc7aa18d44ca4f95ef532e3079` |
| `/home/ps/vislogs/vis-gate-macos-quick-20261123T023942Z.log` | `6cc21d9a91bbf45752b6a1fdd51a385c452c53564c2168253c607346d83ce697` |
| `/home/ps/vislogs/st300m1-mps-quyen.log` | `0227b7a62c803e1563ae85638a2849e42535a130b5832acd1b57b5979c5a706a` |
| `/home/ps/vislogs/st300m1-mps-release.log` | `376faacc225616d9a9af5a8890750ccf27cc59046243c9f491792c004ab24efa` |
| `/home/ps/vislogs/st300m1-mps-gate-selftest.log` | `7b3efac9c960fad1e818e9ca97456a50cf5e029b026fff8c101d23cd1fc10ce8` |

- Linux gate cuối: **3672/4** (72,2 s), fmt/clippy/R1..R7 PASS.
- `bin/vis-targets`: core **18/18**, sqlite **16 + 2 EXPECTED_RED**.
- `#137`: `SO-NO.md` §4 `đóng ở ST-300m1`, bỏ khỏi §5, §8 **194 = 85 mở + 90 đóng +
  16 VS3 + 2 không làm + 1 điều hướng**; R5 OK.

## 5. Kết luận

- Hai lỗi Darwin errno đã vá bằng `rustix::io::Errno`; quyền symlink trả đúng `VS0604`
  trên APFS; `#137` đóng bằng bằng chứng máy thật.
- `vis-gate-macos --quick` + targeted `quyen_*`/`lat*` PASS trên `mps`; release native
  không lọt `test-child`.
- **Không merge/push/tag**; chưa chạy `--full` (để W4 `ST-300m3` chạy 3× full + soak).
