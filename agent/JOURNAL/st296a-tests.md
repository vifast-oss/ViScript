<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296a — gom test binary theo miền (nhánh riêng, không merge)

> **Người làm**: W3, 2026-11-23 · **nhánh** `plan/w4-st296a-tests` @ **`8cc3588`**
> (`ST-291` đã merge; `ST-296d` do W4 làm song song).
> **Phạm vi**: chỉ `crates/vis-cli/tests/**` + `[[test]]` trong `crates/vis-cli/Cargo.toml`.
> **Không sửa code sản phẩm**, không sửa `vis-interp`/`vis-vm`/`vis-types`, không push/merge/tag,
> không GitHub Actions. `vs3_worker*` giữ nguyên 5 binary để W1 rebase.

## 1. Kết quả

| Chỉ số | Trước | Sau |
|---|---:|---:|
| Test binary của `vis-cli` (integration, gồm worker) | **280** | **22** |
| Test binary toàn workspace trong `gate --fast` | **350** | **90** |
| Số ca `nextest` (vis-cli) | 2.749 | **2.749** |
| Số ca `gate --fast` toàn workspace | 3.638 | **3.638** (4 skipped) |
| `gate --fast` wall | **304,10 s** | **218,25 s** (−28,2%) |
| `target/` cùng profile | **1,5 G** | **1,7 G** (+0,2 G) |

- **272 tệp ca** được `git mv` vào 12 thư mục miền; **12 binary gộp** + **3 binary đứng riêng**
  (`cache_khong_che`, `host_conformance`, `host_posix_real`) + **5 `vs3_worker*`** = **20**
  test binary không tính `bin/vis`/`bin/test-child`; `nextest` liệt kê **22** (gồm 2 bin).
- Giữ **100% test/assertion**: không copy/xoá test; chỉ thêm `#[path]` + sửa 5 tệp có
  `include_bytes!/include_str!/include!/#[path]` tương đối; `include` đổi 1 cấp `../`.
- `tests/common/mod.rs` dùng chung (`goc_repo`, `vis_bin`, `test_child_bin`), mỗi binary gộp
  `mod common;`; test gốc vẫn giữ helper riêng trong module của mình.

## 2. 12 binary miền

| Binary | Tệp | Test | Nội dung |
|---|---:|---:|---|
| `sqlite_it` | 20 | 106 | `sqlite`, `vd266_db`, `vd268_dbcache`, `vd270_db_batch`, `vd270_db_explain`, `vd270_batch_trigger`, `vd270_sqlite_*` — `required-features = ["sqlite"]` |
| `tls_it` | 2 | 27 | `vs3_tls`, `vs3_registry_https` — `required-features = ["tls"]` |
| `http2_it` | 1 | 34 | `vs3_http2` — `required-features = ["http2"]` |
| `sig_it` | 1 | 10 | `vs3_registry_sig` — `required-features = ["pkg-sig"]` |
| `lsp_it` | 15 | 152 | toàn bộ `lsp*.rs` |
| `lang_it` | 55 | 517 | `vd26x_*`, `vd_2*`, HIR/lower/tyck, generic/trait, record/pattern, `sem`, `vd258_checker`, … |
| `stdlib_it` | 33 | 287 | `stdlib`, `std.*`, `vd270_str/list/map/parse/split/...` |
| `async_it` | 35 | 339 | `async_*`, `vs3_budget/cancel/chan/select`, `kenh*`, `lat1..258`, `vd259*`, frame/recv |
| `net_pkg_it` | 15 | 164 | `net`, `http`, `vs3_default/keepalive/cookie/http_cancel`, `pkg*`, `vs3_registry`, `vs3_pkg_gc*`, `siet_caps_pkg` |
| `cli_it` | 60 | 523 | `check/fmt/run/test/explain/json/doctor/cache/vish*`, diag (`diag_cmp`, `diff_loi`, `span_tep`, …), misc |
| `host_it` | 24 | 253 | `std_fs_json`, `vd266_fs`, `vd268_fswalk/glob`, `vd269_fscopy`, `vd270_fs/path/with_*`, `vd270_windows` |
| `security_it` | 11 | 77 | `cap*`, `quyen*`, `strict_caps`, `grant_proc_canon`, `sandbox`, `st295_sandbox_proc`, `st83_no_nho`, `gan_hai_duong` |

**Đứng riêng có chủ đích** (giữ hợp đồng script, không đổi `--test`):
- `cache_khong_che` (5 ca) — `bin/vis-soak` gọi `--test cache_khong_che` (cache-1/2).
- `host_conformance` (1) + `host_posix_real` (1) — `bin/vis-host-suite` gọi đúng tên.
- 5 `vs3_worker*` (47 ca) — theo lệnh, để W1 rebase sau.

Tổng integration không-worker: 2496 ca; worker 47; `bin/vis` 199 + lib 7 ⇒ **2.749 ca**
trong package `vis-cli` (đúng trước gom).

## 3. Thay đổi cơ học

- `crates/vis-cli/Cargo.toml`: thay 8 `[[test]]` cũ bằng 4 entry gộp (`sqlite_it`, `tls_it`,
  `http2_it`, `sig_it`) giữ nguyên `required-features`; 8 binary miền còn lại auto-discover.
- `#[path = "<miền>/<tệp>.rs"] mod <tệp>;` trong 12 aggregator; không copy test.
- Sửa đường dẫn include khi đổi 1 cấp thư mục:
  - `tls_it/vs3_tls.rs`, `tls_it/vs3_registry_https.rs`, `http2_it/vs3_http2.rs`,
    `net_pkg_it/vs3_http_cancel.rs`: `fixtures/…` → `../fixtures/…`.
  - `http2_it/vs3_http2.rs`: `#[path = "support/…"]` → `../support/…`.
  - `async_it/lat257_a_coverage.rs`: `../../../test/…` → `../../../../test/…`.
  - `cli_it/check.rs`: `include!("../../../test/doc_excluded.rs")` → `../../../../test/…`.
- `mod extern;` trong `cli_it.rs` đổi `mod r#extern;` (từ khoá Rust).
- Không đổi nội dung ca/assertion nào khác.

## 4. Cổng đo

| Lệnh | Kết quả |
|---|---|
| `cargo check -p vis-cli --no-default-features --tests` | **PASS** (feature gate giữ đúng) |
| `./vis-dev gate --fast` | **PASS — 3638 passed / 4 skipped**, 90 binary, wall **218,25 s** |
| `nextest` `vis-cli` | **2.749** ca (2496 integration + 47 worker + 206 bin/lib) |
| fmt / clippy / R1..R6 | **PASS** (clippy `-D warnings`, R1 ≤ 1.500) |

Log: `/tmp/opencode/st296a-before-gate.log`, `/tmp/opencode/st296a-after-gate.log`,
`/tmp/opencode/st296a-after-tests.txt`.

## 5. Còn lại / blocker

- **Không blocker**. 5 `vs3_worker*` vẫn tách đúng lệnh; 3 binary standalone giữ script soak/host.
- Target sau gom **1,7 G** (trước **1,5 G**): ít binary hơn nhưng 12 link unit lớn hơn; wall gate
  giảm **~86 s**. Có thể tối ưu thêm bằng `split-debuginfo`/profile nếu A0 muốn, ngoài phạm vi.
- Comment trong một số tệp ca còn ghi lệnh `cargo test --test vd270_*` (chỉ là tài liệu, không phải
  test); cập nhật sau nếu W4 muốn, không ảnh hưởng gate.
- **Chưa merge** `main`/tag; nhánh sẵn sàng rebase sau `v0.4.0`. Đĩa cuối sau dọn target **> 10 G**.
