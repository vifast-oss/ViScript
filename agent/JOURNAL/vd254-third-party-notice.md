<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-254 / ST-272 · Giấy phép & thông báo bên thứ ba cho `mio` 1.2.5

**Trạng thái**: **DOC ONLY — CHECKLIST CHƯA THỰC THI**. Tài liệu này chỉ ghi chính sách, soạn mục thông
báo bên thứ ba và chuẩn bị danh mục kiểm `Cargo.lock`. **Không** sửa `Cargo.toml`/`Cargo.lock`, không
thêm crate, không commit, không chạy lệnh làm đổi tệp. Việc tích hợp `mio` do agent ST-271 thực hiện
đồng thời (`agent/QUEUE.md:2072-2073`); khi soạn bản này `mio` **chưa có** trong `Cargo.toml`/`Cargo.lock`
(kết quả `grep -n "mio"` toàn repo: chỉ trúng tài liệu/nhật ký, không trúng manifest hay lockfile).

**Căn cứ**: `agent/DECISIONS.md:2719-2747` (VD-254 §2), `CONTRIBUTING.md:30-41`,
`docs/ViScript-thiet-ke.md:925-927`, `agent/JOURNAL/vd250-dependency-policy.md:10-24,26-35,75-77,92-99`,
`agent/JOURNAL/vd252-selector-proposal.md:44-74,78-108`, `agent/QUEUE.md:2072-2073`.
**Nguồn số liệu crate**: crates.io API `https://crates.io/api/v1/crates/mio/1.2.5` (và `/dependencies`),
truy cập ngày 2026-11-20. *Ghi chú kiểm chứng*: cache index local
(`~/.cargo/registry/index/.../.cache/3/m/mio`) chỉ mới tới 1.2.4 ⇒ **không** lấy được `cksum` 1.2.5 từ
máy; giá trị checksum đầy đủ dưới đây lấy từ crates.io API, phải đối chiếu lại với lockfile sau tích hợp.

---

## 1. Chính sách giấy phép / bên thứ ba thực sự có hiệu lực

| Nguồn | Nội dung có hiệu lực | Trích dẫn |
|---|---|---|
| Luật đóng góp (trong repo, tự chứa) | "**Không** `unsafe`, **không** thêm crate ngoài allowlist giấy phép, **không** `#[ignore]`/`#[should_panic]`"; mỗi tệp có header SPDX theo loại; thay đổi lớn cần mục `VD-##` + task `ST-##`. | `CONTRIBUTING.md:33-34`, `:40-41` |
| Thiết kế §14.2 (D2-21) — allowlist cụ thể duy nhất trong repo | MIT OR Apache-2.0; phụ thuộc qua `cargo-deny` với allowlist **MIT, Apache-2.0, BSD-2/3-Clause, ISC, 0BSD, Zlib, Unicode-3.0, Unicode-DFS-2016**; **cấm MPL/GPL/LGPL**; `THIRD-PARTY.md` **sinh tự động**; LICENSE-MIT/APACHE một bản ở gốc; **không tạo NOTICE**. | `docs/ViScript-thiet-ke.md:926` |
| Hiện trạng vật hoá | `README.md:643-648` chỉ nêu MIT OR Apache-2.0 + trademark. Rà soát thực tế (`find` ở gốc repo, ngày 2026-11-20): chỉ có `LICENSE-MIT`, `LICENSE-APACHE`; **không có** `THIRD-PARTY*`, **không có** `NOTICE*`, **không có** `deny.toml`. | `agent/JOURNAL/vd250-dependency-policy.md:22`, kết quả `find` của bản này |
| Xếp hạng nguồn | QD (ViFastOs) nằm ở repo khác ⇒ không đọc được ở đây, không suy diễn; thiết kế bị QD thắng, nhưng khi QD không đọc được thì báo A0 thay vì tự áp. VD-254 là chữ ký A0 cho riêng crate này. | `AGENTS.md:6-7,16`, `agent/JOURNAL/vd250-dependency-policy.md:10-24`, `agent/DECISIONS.md:2735-2747` |

**Kết luận về tệp thông báo**: **hiện không có tệp thông báo bên thứ ba chuyên biệt nào trong repo**
(không `THIRD-PARTY.md`, không `NOTICE*`, không `deny.toml`). Thiết kế `docs/ViScript-thiet-ke.md:926`
*dự kiến* `THIRD-PARTY.md` sinh tự động và **không tạo NOTICE**, nhưng điều đó **chưa được hiện thực**
(`agent/JOURNAL/vd250-dependency-policy.md:22`). Thông tin giấy phép hôm nay được ghi **tại chỗ khai
báo dependency** kèm số VD — tiền lệ: `sha2` (`crates/vis-bytecode/Cargo.toml:2-4`, VD-54(1)),
`unicode-normalization` (`crates/vis-interp/Cargo.toml:1-3`, allowlist ST-01),
`rusqlite = "=0.37.0"` (`crates/vis-cli/Cargo.toml:42-47`, VD-241 Gate 1) — xem bảng tiền lệ tại
`agent/JOURNAL/vd250-dependency-policy.md:26-35`; ngoài ra có LICENSE-MIT/APACHE ở gốc, header SPDX từng
tệp (`CONTRIBUTING.md:34`, `docs/ViScript-thiet-ke.md:925`) và mẫu exception report
(`agent/JOURNAL/vd250-dependency-policy.md:75-77`). Bản này là **bản ghi thông báo bên thứ ba cho
VD-254** theo đúng vai trò đó; việc có nên vật hoá thành `THIRD-PARTY.md`/`deny.toml` là task riêng,
**chưa được quyết ở đây** (`agent/JOURNAL/vd250-dependency-policy.md:107-108`, câu P2).

---

## 2. Mục thông báo bên thứ ba — `mio` 1.2.5

| Trường | Giá trị đã kiểm chứng |
|---|---|
| Tên crate | `mio` |
| Phiên bản ghim | **`=1.2.5`** (ghim tuyệt đối, theo tiền lệ `rusqlite = "=0.37.0"` — `crates/vis-cli/Cargo.toml:42-47`; `agent/JOURNAL/vd252-selector-proposal.md:48`) |
| Giấy phép | **MIT** — `versions[1.2.5].license` từ crates.io API (2026-11-20); **nằm trong allowlist** `docs/ViScript-thiet-ke.md:926`. Không phải dual-license; không rơi vào nhóm bị cấm MPL/GPL/LGPL. |
| Nguồn registry | `registry+https://github.com/rust-lang/crates.io-index` (đúng định dạng `source` hiện có trong `Cargo.lock:151`) |
| Checksum (crates.io `cksum`) | `20521a903f02b0a573cd6840e9d0a5a0cc043d361fe924092cd70f8556cc9c4a` (khớp tiền tố `20521a90…cc9c4a` ghi trong `agent/JOURNAL/vd252-selector-proposal.md:48`; đối chiếu đầy đủ ở mục 3) |
| Features bật | `default-features = false`, `features = ["os-poll", "net"]` ⇒ **tắt** feature mặc định `log`; giữ `Poll`/`Registry`/`Token`/`Events`. VD-254 §2 `agent/DECISIONS.md:2737`. |
| MSRV | `rust-version = 1.71` ≤ toolchain dự án đã nâng lên 1.100.0 (`agent/JOURNAL/vd252-selector-proposal.md:22-24,51`). |
| Mục đích | Adapter selector I/O cho readiness `read`/`accept` trên **đường thử nghiệm nội bộ (Lát 3)**, chỉ Linux; **không** đổi runtime mặc định, API công khai, `VERSION` hay cache. `agent/DECISIONS.md:2737-2740,2749-2755`; `agent/QUEUE.md:2072`. |
| Phạm vi khai báo | Chỉ crate thực sự sở hữu adapter, trong `[target.'cfg(target_os = "linux")'.dependencies]`; không thêm rộng vào mọi crate, không lọt vào core crate (vd250 P5 `agent/JOURNAL/vd250-dependency-policy.md:111`; `agent/JOURNAL/vd252-selector-proposal.md:50,53`). |

### 2.1 Cây transitive (dữ liệu dependency của crates.io 1.2.5, đối chiếu `vd252-selector-proposal.md:55-64`)

| Crate | Yêu cầu | Điều kiện | Tình trạng trên Linux |
|---|---|---|---|
| `mio` | `=1.2.5` | — | **entry mới duy nhất bắt buộc** trong `Cargo.lock` |
| `libc` | `^0.2.190` | `cfg(any(unix, target_os="hermit", target_os="wasi"))`, normal | **đã có** trong `Cargo.lock:148-152` (hiện `0.2.191`) ⇒ **không được sinh entry mới** |
| `windows-sys` | `^0.61` | `cfg(windows)`, normal | **cfg-gated, không vào bản build Linux**; có thể xuất hiện entry lock do resolver đa target (xem mục 3) |
| `wasi` | `^0.11.0` | `cfg(target_os = "wasi")`, normal | **cfg-gated, không vào bản build Linux** |
| `log` | `^0.4.8` | optional, normal | **TẮT** (`default-features = false`); không được kéo vào |
| `env_logger` `^0.11`, `rand` `^0.9` | dev-dependencies | — | chỉ dev, không vào artifact sản phẩm |

### 2.2 Phương án không chọn

**`polling` KHÔNG được chọn đợt này** — `agent/DECISIONS.md:2746`; `agent/QUEUE.md:2068`. Lý do đã nộp
cùng hồ sơ: `polling = "=3.11.0"` kéo `rustix`/`linux-raw-sys`/`cfg-if` và nhiều bề mặt unsafe hơn vào
graph hiện tại (`agent/JOURNAL/vd252-selector-proposal.md:78-108`). Không thêm `polling` vào manifest
hay lockfile.

---

## 3. Cargo.lock review checklist — **CHƯA THỰC THI**

Điều kiện chạy: **chờ agent ST-271 commit tích hợp** (`agent/QUEUE.md:2072`). Không chạy lệnh nào làm đổi
tệp (`cargo add`, `cargo update`, `cargo build`, `cargo generate-lockfile`...). `cargo metadata --locked
--format-version 1` ở dưới là **read-only**; nếu `mio` chưa có trong manifest/lockfile thì **bỏ qua lệnh
này và ghi rõ "skipped — dependency chưa tích hợp"**. Thứ tự đối chiếu:

1. [ ] Xác nhận đã thấy commit tích hợp của ST-271; ghi hash commit vào artifact review.
2. [ ] `grep -n 'mio' Cargo.lock` — có đúng **một** entry `[[package]] name = "mio" version = "1.2.5"`.
3. [ ] Trong entry đó: `source = "registry+https://github.com/rust-lang/crates.io-index"` (khớp `Cargo.lock:151`).
4. [ ] `checksum = "20521a903f02b0a573cd6840e9d0a5a0cc043d361fe924092cd70f8556cc9c4a"` — so sánh
       **toàn bộ 64 ký tự** với giá trị crates.io API nêu ở mục 2 (không chỉ tin tiền tố `20521a90…cc9c4a`).
5. [ ] `git diff Cargo.lock` — chỉ các entry **dự kiến** được thêm: `mio`; **có thể** có `wasi`/`windows-sys`
       và chuỗi `windows-targets`/`windows-link` đi kèm. Lưu ý kỹ thuật: `Cargo.lock` là resolved cho mọi
       target, nên dependency `cfg(windows)`/`cfg(wasi)` **có thể được ghi vào lock dù không biên dịch trên
       Linux**; nếu xuất hiện, phải là entry cfg-gated thuần (không kéo `rustix`, `linux-raw-sys` —
       đó là dấu vết của `polling`, tuyệt đối không được có).
6. [ ] `libc` vẫn là **một entry** (hiện `0.2.191`, `Cargo.lock:149-152`); **không** được thêm bản libc thứ
       hai hay bị bump version ngoài ý muốn.
7. [ ] **Không có** entry mới nào khác: `log`, `env_logger`, `rand`, `polling`, `rustix`, `linux-raw-sys`,
       `cfg-if`, `smol`, `tokio`, `futures`, `rayon`, `crossbeam`... (đối chiếu danh sách graph hiện tại tại
       `agent/JOURNAL/vd250-dependency-policy.md:34`).
8. [ ] **Không có version bump không liên quan**: so sánh từng dòng `version =` bị đổi trong diff với
       baseline trước tích hợp; mọi thay đổi phải giải thích được bằng việc thêm `mio`.
9. [ ] `git diff Cargo.toml` — khai báo đúng `mio = "=1.2.5"`, `default-features = false`,
       `features = ["os-poll","net"]`, nằm đúng target cfg Linux và đúng crate sở hữu adapter; có comment
       ghi giấy phép + số VD theo tiền lệ (`agent/JOURNAL/vd250-dependency-policy.md:30-32,73`).
10. [ ] Nếu dependency đã hiện diện: chạy `cargo metadata --locked --format-version 1` (read-only) và đối
       chiếu `packages[].license == "MIT"`, `packages[].version == "1.2.5"`, `source`, `resolve.nodes[].deps`
       với mục 2. Nếu chưa hiện diện: **ghi "skipped"**, không tự thêm.
11. [ ] Ghi kết quả từng ô vào artifact review, kèm hash commit đã kiểm; nếu metadata/package thực tế khác
       hồ sơ ⇒ **dừng phần dependency, báo lại A0**, không tự đổi version (`agent/DECISIONS.md:2747`).

---

## 4. unsafe: tách bạch phạm vi

- **unsafe trong mã dự án ViScript vẫn = 0**: `CONTRIBUTING.md:33` cấm; R2 cấm mọi tệp `.rs`
  (`bin/vis-dev:242-253`, trích qua `agent/JOURNAL/vd250-dependency-policy.md:21,96`); VD-254 §2 nhắc lại
  "không thêm `unsafe` trong mã dự án" (`agent/DECISIONS.md:2740`). Adapter Lát 3 không được dùng unsafe để
  né API selector.
- **unsafe trong mio là dependency-contained**: FFI/syscall epoll/kqueue/IOCP nằm trong `src/sys/*` của
  crate (`agent/JOURNAL/vd252-selector-proposal.md:66-70`); API ta gọi (`Poll::wait`, `Registry::register`)
  là safe; không lọt vào API runtime, không nằm ở core crate. Phải đánh giá riêng theo
  `agent/JOURNAL/vd250-dependency-policy.md:92-99`, không tự coi crate "sạch" vì mã dự án sạch.
- **A0 đã chốt**: số chỗ `unsafe` **không phải thước đo độc lập** để kết luận an toàn hơn; chính là lý do
  không chọn `polling` chỉ vì so sánh bề mặt unsafe (`agent/DECISIONS.md:2746-2747`;
  `agent/JOURNAL/vd252-selector-proposal.md:102-104`).

---

## 5. Trạng thái chốt

Mục thông báo bên thứ ba ở mục 2 đã được **soạn** và số liệu crate đã **đối chiếu với crates.io API ngày
2026-11-20** (ngoại trừ checksum đầy đủ không lấy được từ cache index local nên lấy từ API). **Toàn bộ danh
mục kiểm ở mục 3 vẫn CHƯA được thực thi** vì phải chờ commit tích hợp đồng thời của agent ST-271; hiện
`mio` chưa có trong `Cargo.toml`/`Cargo.lock`. Vì vậy **không có nội dung nào trong bản này được tuyên bố là
đã kiểm chứng trên lockfile thực tế** — mọi dòng checklist chỉ là tiêu chí cần đối chiếu sau tích hợp.
