<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-323b — R1 headroom: tách `vis-http/src/lib.rs` (url/header/request/client)

- **Ngày**: 2026-11-28 · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st323b`
- **Nền**: `main` @ `eb41d61c3ef77a6c8384d2f13be40f189ce2676b`
- **Worktree**: `/home/ps/viswt/st323b` — **không** chạm repo chính, **không** merge/push.
- **Trạng thái**: mã + test + cổng **PASS**; chờ Gatekeeper audit.
- **Nguồn đề xuất**: `/home/ps/vislogs/r1-headroom-next-20261127T034711Z.md` §4 (đề bài ghi
  "§4.2, sha `b39aa0f6…`"; tệp thực chỉ có §4 và `grep b39aa0f6` trong `vislogs/` không ra kết
  quả — nội dung §4 khớp đúng yêu cầu: 983 → `url/header/request/client`, `lib.rs` ≈ 40 dòng).

## 0. TL;DR

Tách nguyên văn `lib.rs` (983 dòng) thành 4 module theo ranh giới dòng đã duyệt, để lại barrel
`lib.rs` **52 dòng**. Churn ≈ 950 dòng; **0 đổi hành vi/API công khai** (các hàm `http_get`,
`http_head`, async, `MAX_HTTP`, ... vẫn ở đường dẫn cũ).

| Tệp | Trước | Sau | Trong đó thân move nguyên văn |
|---|---:|---:|---:|
| `src/lib.rs` | 983 | **52** | doc 1–21 + const 27–31 + tail 978–983 |
| `src/url.rs` | — | 238 | 233 (lib 33–265) |
| `src/header.rs` | — | 161 | 154 (lib 267–421) |
| `src/request.rs` | — | 163 | 157 (lib 422–574) |
| `src/client.rs` | — | 417 | 402 (lib 575–977) |

## 1. Thay đổi

- 4 module mới chứa **nguyên văn** các đoạn 33–265 / 267–421 / 422–574 / 575–977 của `lib.rs`
  (kèm SPDX + 1 dòng doc module + `use` tương ứng). Kiểm `difflib` giữa thân 4 module (sau `fmt`)
  và `git show HEAD:crates/vis-http/src/lib.rs`: **0 opcode khác biệt** ở cả 4 tệp; `lib.rs` giữ
  lại đúng các dòng 1–21, 27–31, 978–983. Xem `/tmp/opencode/st323b_verify.py`.
- `lib.rs` khai `mod url/header/request/client;` + re-export.

### Re-export / quyền riêng (bắt buộc để biên dịch, không lọt ra ngoài crate)

- `pub use client::{http_get, http_head};` — **giữ nguyên đường dẫn công khai**; nếu bỏ thì 2 hàm
  rơi vào module riêng và bị `dead_code`.
- `pub(crate) use header::{con_lai_ms, doc_header_day_du, ghep_chunked, tim_header, DauHttp};`
- `#[cfg(test)] pub(crate) use header::doc_header;` — item gốc `#[cfg(test)]`, re-export cũng vậy.
- `#[cfg(test)] pub(crate) use request::chuan_hoa_target;` — chỉ `tests.rs` dùng qua root; nếu re-export
  không điều kiện thì `#[warn(unused_imports)]` ở bản lib thường ⇒ clippy `-D warnings` đỏ.
- `pub(crate) use request::{header_h2, noi_location};`
- `pub(crate) use url::{tach_url, GiaoThuc, UrlHttp};`
- Nâng quyền **nội bộ** (`fn`/`struct`/field riêng → `pub(crate)`) cho các item bị gọi chéo module:
  `UrlHttp::{url_voi_target, url}`, `doc_header`, `chuan_hoa_target`, `LoiHttp` + 2 field +
  `khong_thu_lai`/`ket_noi`, `loi_doc`, `KetQuaGet/Head`, `HeaderThem` + field, `yeu_cau_http`,
  `set_cookie_cua`, `header_gia_tri`. **Không** đổi chữ ký, thân hàm hay item `pub` nào.
- Thêm `#[allow(unused_imports)] use vis_core::host::NetHost;` trong `lib.rs` để liên kết doc
  `[`NetHost`]` vẫn phân giải sau khi tách client (kiểm bằng `cargo doc -D warnings`).

### Khác biệt cơ học do `cargo fmt`

- Bỏ 1 dòng trắng cuối tệp của `header.rs`/`client.rs` (trailing blank line ở EOF).
- Reflow chữ ký `yeu_cau_http` thành 6 dòng sau khi thêm `pub(crate)`. Cả hai đã được chuẩn hoá
  trong script đối chiếu; thân hàm không đổi.

## 2. API / path ngoài không đổi

- `vis_http::http_get/http_head` (re-export từ `client`), `http_get_async/http_head_async`,
  `xoa_trang_thai_frame`, `AsyncHttpEnv`, `FrameReactorBackend`, `ReactorH2Response`, `ReactorOp`
  giữ nguyên như cũ.
- `vis_http::MAX_HTTP` vẫn là `pub const` định nghĩa trực tiếp trong `lib.rs` — `vis-host-native`
  (`src/net/h2_core.rs:22`) dùng path này, không sửa.
- `async_http.rs` và `tests.rs` **không sửa một dòng**, dùng chung re-export tại root.

## 3. Kiểm chứng

Mọi lệnh chạy trong worktree, `PATH=$HOME/.cargo/bin:$PATH` (toolchain **1.100.0** theo
`rust-toolchain.toml`), `TMPDIR=/home/ps/vislogs/tmp` (đĩa), `CARGO_BUILD_JOBS=2`, **không clean**.
Disk `/` lúc commit: **16 G** available (> ngưỡng dừng 5 G).

| Lệnh | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | rc 0 | `st323b-fmt.log` |
| `cargo test -p vis-http` | **8/8**, 0 warning | `st323b-test-http.log` |
| `cargo build -p vis-cli --bins` | rc 0 (cần `vis`/`test-child` cho test host) | `st323b-build-bins.log` |
| `cargo test -p vis-http -p vis-host-native -p vis-runtime -p vis-pkg --no-fail-fast` | **PASS** — 453 test (31+1+9+8+79+8+27+290), 0 fail | `st323b-test-4pkg.log` |
| `cargo test -p vis-conformance --test lib_api_it http` | **4/4 PASS** (`net_http::*` — golden/parity HTTP không đổi) | `st323b-http-golden.log` |
| `./vis-dev gate --fast` | **PASS rc 0** — fmt/clippy/R1..R10/self-test/build/doc-examples + L0 **3942/3942**, 4 skipped | `st323b-gate-fast.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** (100.000 CT, 7 test PASS, 163 s) | `st323b-vis-diff.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS rc 0**, 0 warning (38 s) | `st323b-clippy.log` |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p vis-http --no-deps` | chỉ còn **2 link `Scheduler` đã đỏ từ trước** trong `async_http.rs` (không phải do tách; `gate` không chạy bước này) | `st323b-doc.log` |

**Ghi chú cổng**: lần chạy `gate --fast` đầu tiên đỏ ở fixture `bad-project` do **race một tệp tạm
dùng chung** `/tmp/vrepfx.out` với 2 gate đồng thời (`st323a`, `st323c`); chạy riêng
`bash test/report/run_fixtures.sh` xanh 5/5, và chạy lại `gate --fast` ngay sau đó xanh rc 0.
Log lần race giữ tại `st323b-gate-fast-race.log` để đối chiếu.

## 4. Luật

- **R1**: `lib.rs` **52 < 200** (và < 1500); module lớn nhất mới `client.rs` **417**. Không tệp nào > 1500.
- **R8**: tổng `.rs` cấp 1 trong `crates/*/tests/` + `vis-host-linux/tests/` = **39/40**;
  `vis-cli/tests` = **8/15**. Không thêm binary test nào.
- `unsafe`/`TODO` trong `crates/vis-http/src/`: **0**.
- `git status` trước commit: `M src/lib.rs` + 4 tệp mới + journal — **không** đụng
  `agent/QUEUE.md`/`STATE.md`/`DECISIONS.md`, không đụng golden hay Cargo manifest.

## 5. Kết luận

Di chuyển thuần tuý theo module cohesion (URL / header / request / client), đã đối chiếu `difflib`
với `git show`; cổng máy tái lập được **PASS**. Không merge, không push; chờ Gatekeeper (Agent 1)
audit rồi Agent 0 duyệt.
