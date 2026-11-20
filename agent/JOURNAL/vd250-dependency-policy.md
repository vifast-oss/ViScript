<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-250 / ST-266 · Nháp **chính sách dependency thực sự có hiệu lực trong ViScript**

**Trạng thái**: **DRAFT — CHỜ A0 CHỐT**. research + draft only; không sửa `Cargo.toml`/`Cargo.lock`, không thêm crate, không commit, không chạy gate.
**Căn cứ**: `agent/DECISIONS.md:2444`, `:2525-2527` (VD-250 §1/§5), `AGENTS.md:3-16`, `CONTRIBUTING.md:30-41`, `docs/ViScript-thiet-ke.md:879`, `:926-927`, `:1086`.
**Quy ước**: nhận định "hiện trạng" có `file:line` (hoặc kết quả lệnh kiểm chứng); mọi đề xuất gắn **[CHỜ A0 CHỐT]**. **Không chọn dependency cụ thể nào** — tài liệu này chỉ xác định chính sách và quy trình.

---

## 1. Chính sách nào đang có hiệu lực (không áp tự động allowlist ViFastOs)

A0 dặn: **không áp tự động** allowlist ViFastOs; phải xác định chính sách ViScript thực sự có hiệu lực (`agent/DECISIONS.md:2525-2527`).

### 1.1 Phân tầng nguồn lực

| Tầng | Hiện trạng có trích dẫn | Hệ quả |
|---|---|---|
| Luật repo ViScript | `AGENTS.md:3-5`: đây là bản tách, gốc repo là cây ViScript; `:6-7`: luật chung ViFastOs (QD) **vẫn áp nhưng nay nằm ở repo ViFastOs**; `:9-14`: thứ tự nguồn sự thật = code+test → SPEC → `DECISIONS.md` → thiết kế; `:16`: **QD thắng tài liệu thiết kế**. | QD không nằm trong repo này (`AGENTS.md:6-7`) ⇒ **không thể đọc allowlist ViFastOs tại đây** và càng không được suy diễn nội dung của nó. Mọi xung đột phải trình A0, không tự áp. |
| Quy ước đóng góp (có hiệu lực, trong repo) | `CONTRIBUTING.md:33`: "**Không** `unsafe`, **không** thêm crate ngoài allowlist giấy phép, **không** `#[ignore]`/`#[should_panic]`"; `:40-41`: thay đổi lớn cần mục `VD-##` + task trong `QUEUE.md`. | Đây là luật ViScript **tự chứa** đầu tiên về dependency: cấm crate ngoài allowlist, cấm unsafe trong mã dự án. |
| Thiết kế §14.2 (tham chiếu, bị QD thắng) | `docs/ViScript-thiet-ke.md:926` (**D2-21**): MIT OR Apache-2.0; phụ thuộc qua `cargo-deny` với allowlist **MIT, Apache-2.0, BSD-2/3-Clause, ISC, 0BSD, Zlib, Unicode-3.0, Unicode-DFS-2016**; **cấm MPL/GPL/LGPL**; `THIRD-PARTY.md` sinh tự động; LICENSE-MIT/APACHE một bản ở gốc; **không** tạo NOTICE. `:879`: phụ thuộc Rust **phải thuần Rust** và giấy phép tương thích MIT/Apache. `:927`: `unsafe` chỉ trong danh sách crate đóng (`vis-vm` heap, `vis-plugin` ABI), có Miri; core crate (`vis-syntax`, `vis-types`, `vis-bytecode`, `vis-vm`) không phụ thuộc OS, build wasm32. `:1086`: gate kiểm `unsafe` + R1 có case âm. | Đây là **allowlist ViScript tự chứa**, không phải allowlist ViFastOs. Tuy nhiên là tài liệu thiết kế ⇒ `AGENTS.md:16` xếp dưới QD; khi QD không đọc được ở repo này, phải **báo A0** thay vì tự coi là chốt. |
| Thực thi cổng | `bin/vis-dev:242-253` — **R2 cấm `unsafe` trong mọi tệp `.rs`**, thông điệp "vòng 1 chưa crate nào được phép" (danh sách trống); R1 `:195-234`; R3 `:255+`; R4 `:287-289`. | **Mâu thuẫn cần A0 chốt**: thiết kế `:927` cho phép unsafe trong `vis-vm`/`vis-plugin`, nhưng cổng hiện tại cấm **tất cả**. Trước khi A0 đổi, **không được** thêm unsafe vào mã dự án. |
| Vật hoá chính sách | Kiểm `find . -maxdepth 2 -iname 'deny.toml' -o -iname '*third*party*'` ⇒ **rỗng** (không có `deny.toml`, không có `THIRD-PARTY.md` ở gốc repo). `README.md:643-648` chỉ nêu MIT OR Apache-2.0 + trademark. | Thiết kế `:926` đòi `cargo-deny` + `THIRD-PARTY.md` sinh tự động nhưng **chưa có trong repo** ⇒ hiện chưa có hàng rào tự động cho license allowlist; phải làm bằng tay/quy trình và báo gap cho A0. |

**Kết luận [CHỜ A0 CHỐT]**: chính sách có hiệu lực lớn nhất là `CONTRIBUTING.md:33` + R2 (`bin/vis-dev:242-253`) + VD-250 (`agent/DECISIONS.md:2525-2527`). Allowlist cụ thể chỉ có nguồn trong `docs/ViScript-thiet-ke.md:926`, phải trình A0 xác nhận vẫn áp dụng (vì QD ở repo khác và `AGENTS.md:16` cho QD thắng).

### 1.2 Tiền lệ dependency đã có trong repo (để A0 thấy cách xử lý ngoại lệ)

| Crate | Căn cứ | Điểm đáng chú ý |
|---|---|---|
| `sha2 = "0.10"` | `crates/vis-bytecode/Cargo.toml:2-4`; chú thích ghi rõ "MIT OR Apache-2.0, **Agent 0 duyệt allowlist, VD-54 (1)**", liên kết QD §9.3 ở `crates/vis-bytecode/src/format.rs:22-23` | Ngoại lệ/duyệt được **ghi ngay tại chỗ khai báo** + số VD. |
| `unicode-normalization = "0.1"` | `crates/vis-interp/Cargo.toml:1-3` ("license MIT OR Apache-2.0 + Unicode-3.0, **đã allowlist ở ST-01**"); cũng ở `crates/vis-syntax/Cargo.toml` | Phủ thêm `Unicode-3.0` — khớp allowlist `docs/ViScript-thiet-ke.md:926`. |
| `rusqlite = "=0.37.0"`, features `bundled`, `hooks`, optional | `crates/vis-cli/Cargo.toml:42-47` — ghi rõ **VD-241 Gate 1**, kéo `libsqlite3-sys` (SQLite 3.50.2, `sqlite3.c` public domain; rusqlite MIT), **cần C compiler/linker** | **Tiền lệ ngoại lệ quan trọng**: chấp nhận mã C (mâu thuẫn với "thuần Rust" ở `docs/ViScript-thiet-ke.md:879`) nhưng **ghim version tuyệt đối** và nêu rõ lý do + VD. |
| `vis-host-linux` (chứa `landlock`) | `crates/vis-cli/Cargo.toml:50-54` — chỉ `cfg(target_os = "linux")`; `Cargo.lock` có `landlock` | Ngoại lệ nền tảng có điều kiện; ma trận target vẫn giữ được. |
| Tổng graph | `Cargo.lock` chứa `bitflags, block-buffer, cc, cfg-if, cpufeatures, crypto-common, digest, enumflags2*, fallible-iterator*, find-msvc-tools, foldhash, generic-array, hashbrown, hashlink, libc, libsqlite3-sys, pkg-config, proc-macro2, quote, rusqlite, sha2, shlex, smallvec, syn×2, thiserror*, tinyvec, typenum, unicode-ident, unicode-normalization, vcpkg, version_check` + 10 crate nội bộ | `cc`/`libc`/`vcpkg`/`pkg-config` là dấu vết của đường build C từ rusqlite bundled. Mọi selector mới sẽ nằm **trên graph hiện tại này**. |

---

## 2. Quy trình đề xuất một thư viện selector **[CHỜ A0 CHỐT TOÀN BỘ]**

A0 **được đề xuất** thư viện selector phù hợp nhưng **chưa duyệt dependency cụ thể**, không bắt buộc tự viết reactor (`agent/DECISIONS.md:2444`, `:2525-2527`). Không chọn crate ở ST-266.

### 2.1 Bộ ứng viên cần đánh giá (danh sách đánh giá, **không** phải lựa chọn)

**[CHỜ A0 CHỐT]** — đề xuất đưa vào vòng đánh giá, không hơn:

| Ứng viên | Lý do đưa vào vòng đánh giá | Cần trả lời |
|---|---|---|
| `mio` | selector mỏng, phổ biến, API readiness rõ | transitive tree, license, unsafe bên trong, target Windows/wasm |
| `polling` | tối giản hơn, phù hợp "adapter I/O mỏng" (VD-250 §1 `agent/DECISIONS.md:2443`) | cùng như trên + hỗ trợ nền |
| `async-io` / `smol` | nếu A0 muốn thêm timer cùng registry | có kéo executor/futures không? (VD-250 chưa mở worker/API mới `:2452`) |
| `tokio` (chỉ phần `net`/`time`, **không** runtime đầy) | so sánh chi phí transitive | có bắt buộc runtime/executor không; có vi phạm "một luồng do ViScript quản lý" không |
| `crossbeam` / `futures` | chỉ đưa vào để **loại có lý do**, không phải ưu tiên | có kéo shared heap/parallel không |

**Không tự ý thêm** crate nào vào `Cargo.toml` trước khi A0 chốt. Mỗi ứng viên phải đi qua §2.2–§2.4.

### 2.2 Các bước đánh giá bắt buộc

**[CHỜ A0 CHỐT]**

1. **Cây transitive** — chạy và lưu output thành artifact đính kèm phiếu:
   ```sh
   cargo tree -p <candidate> -e normal,build,dev --prefix none
   cargo tree -i <candidate>
   cargo metadata --format-version 1 > /tmp/opencode/<candidate>-metadata.json
   ```
   `cargo metadata` trường `packages[].license` / `license_file` là nguồn license; `resolve.nodes[].deps` là cây phụ thuộc thật. Dùng `/tmp/opencode` theo cấu hình môi trường.
2. **License check** — đối chiếu từng crate trong cây (kể cả transitive) với allowlist `docs/ViScript-thiet-ke.md:926`: MIT, Apache-2.0, BSD-2/3-Clause, ISC, 0BSD, Zlib, Unicode-3.0, Unicode-DFS-2016; **từ chối** MPL/GPL/LGPL; dual-license dạng `MIT OR Apache-2.0` hợp lệ. Ghi rõ crate nào dùng `license-file` thay vì `license`. Nếu repo chưa có `deny.toml` (§1.1), đề xuất thêm file này với allowlist trên — **[CHỜ A0 CHỐT]**, không tự thêm.
3. **OS / target check** — `cargo check --target wasm32-unknown-unknown -p vis-syntax -p vis-types -p vis-bytecode -p vis-vm` (quy tắc core crate không phụ thuộc OS `docs/ViScript-thiet-ke.md:927`); nếu selector chỉ dành cho `vis-cli`/host adapter thì ghi rõ điều đó, không để lọt vào core. Đồng thời kiểm ma trận target hiện hành (`crates/vis-cli/tests/targets.rs`, `bin/vis-dev` R4 `bin/vis-dev:287-289`).
4. **MSRV / build** — `rust-version = "1.99.1"` `Cargo.toml` (workspace `[workspace.package]`); dependency phải build được với toolchain ghim `rust-toolchain.toml`. Nếu cần C compiler như rusqlite (`crates/vis-cli/Cargo.toml:42-47`), đó là **ngoại lệ** phải xin riêng.
5. **unsafe inventory** — tách thành hai nhóm: *unsafe trong mã dự án* (xem §3) và *unsafe nằm trong dependency* (xem §3). Không được coi dependency "sạch" chỉ vì API của nó an toàn.
6. **Tác động định dạng/cache** — selector không được làm đổi bytecode/VERSION; chỉ adapter host (VD-250 §5 `agent/DECISIONS.md:2508-2516`: cho phép sửa interface nội bộ, giữ bề mặt `net.*`). Khẳng định điều này trong phiếu.
7. **Ca thử nghiệm** — chạy các ca mạng hiện có (`crates/vis-cli/tests/net.rs`, `net_server.rs`, `http.rs`) + deterministic fixture theo `vd250-determinism-contract.md` §4 lớp T; **không** đổi golden hiện hữu.
8. **Ghi chép tại chỗ** — nếu A0 duyệt, khai báo trong `Cargo.toml` kèm comment `VD-##` + allowlist/license như tiền lệ `sha2` (`crates/vis-bytecode/Cargo.toml:2-4`) và `rusqlite` (`crates/vis-cli/Cargo.toml:42-47`); cập nhật danh sách phụ thuộc trong tài liệu nếu A0 yêu cầu.

### 2.3 Mẫu báo cáo ngoại lệ (exception report) **[CHỜ A0 CHỐT]**

Phiếu gửi A0 phải có các mục: (1) tên + **version ghim**; (2) lý do cần (liên kết VD-250 §5 + lát cắt dùng); (3) cây direct/transitive từ `cargo tree`/`cargo metadata`; (4) bảng license từng crate + đối chiếu allowlist; (5) bảng unsafe (crate, file, mục đích, có Miri không, có lọt vào core không); (6) yêu cầu build (C compiler, OS target, wasm32); (7) phương án thay thế đã cân nhắc và lý do loại; (8) khẳng định không đổi VERSION/bytecode/API ngôn ngữ; (9) số `VD-##` + task `ST-##` đề xuất (`CONTRIBUTING.md:40-41`, `AGENTS.md:25`). **Chưa có chữ ký A0 thì không sửa `Cargo.toml`.**

### 2.4 Lệnh tổng hợp để trình A0 **[CHỜ A0 CHỐT]**

```sh
cargo tree -p <candidate> -e normal,build,dev --prefix none
cargo metadata --format-version 1 > /tmp/opencode/<candidate>-metadata.json
cargo check --target wasm32-unknown-unknown -p vis-syntax -p vis-types -p vis-bytecode -p vis-vm
cargo test -p vis-cli --test net --test net_server --test http
```

(Không chạy `./vis-dev gate --fast` ở ST-266 — đây là draft; cổng chỉ chạy ở task hiện thực sau khi A0 duyệt.)

---

## 3. unsafe: phân biệt hai chính sách **[CHỜ A0 CHỐT]**

| Nhóm | Trạng thái hiện tại | Quy trình đề xuất |
|---|---|---|
| **unsafe trong mã dự án ViScript** | A0 **chưa duyệt unsafe mới** (`agent/DECISIONS.md:2527`); `CONTRIBUTING.md:33` cấm; R2 `bin/vis-dev:242-253` cấm **mọi** `.rs` (danh sách crate đóng trống, dù `docs/ViScript-thiet-ke.md:927` nhắc `vis-vm`/`vis-plugin`). | Giữ nguyên: **không thêm unsafe**. Muốn đổi phải có VD riêng của A0 + đổi R2 + case âm chứng minh cổng bắt được (`docs/ViScript-thiet-ke.md:927`, `:1086`). Không được lợi dụng prototype Lát 0 để lén đưa unsafe. |
| **unsafe nằm trong dependency** | Chưa có chính sách riêng được chốt; các crate hiện tại có unsafe ẩn (vd. đường C của `libsqlite3-sys` `crates/vis-cli/Cargo.toml:42-47`; `libc`/`cc` trong `Cargo.lock`). | Đề xuất đánh giá **riêng**, theo bảng: (a) unsafe có lọt vào API public mà runtime dùng không; (b) nằm ở core crate hay chỉ host adapter; (c) có audit/Miri ở phía crate đó không (Miri của ta chỉ chạy được cho mã mình giữ); (d) license/OS/target; (e) có ghi rõ trong exception report §2.3 không. Không tự coi "unsafe trong dependency" là được phép vì "mã dự án sạch". |

**Điểm cần A0 làm rõ**: `docs/ViScript-thiet-ke.md:927` nói unsafe được phép trong danh sách crate đóng có Miri, nhưng `bin/vis-dev:242-253` và `CONTRIBUTING.md:33` nói cấm — đề xuất A0 chọn **một** hướng trước khi bất kỳ selector nào được duyệt. **[CHỜ A0 CHỐT]**

---

## 4. Câu hỏi gửi A0

| # | Câu hỏi | Khuyến nghị |
|---|---|---|
| P1 | Allowlist `docs/ViScript-thiet-ke.md:926` có còn là chính sách ViScript hiệu lực không (vì QD ở repo khác, `AGENTS.md:6-7,16`)? | Xác nhận lại bằng VD mới; không tự áp allowlist ViFastOs. |
| P2 | Có cho thêm `deny.toml` + `THIRD-PARTY.md` để vật hoá allowlist không (§1.1 — hiện đều thiếu)? | Có, nhưng là task riêng, không gộp vào ST-266. |
| P3 | Giải quyết mâu thuẫn unsafe giữa thiết kế `:927` và R2 `bin/vis-dev:242-253` thế nào (§3)? | Giữ cấm tới khi A0 đổi rõ ràng. |
| P4 | Có chấp nhận ngoại lệ "thuần Rust" kiểu rusqlite (`crates/vis-cli/Cargo.toml:42-47`) cho selector không? | Chưa; đánh giá từng crate, ưu tiên thuần Rust. |
| P5 | Phạm vi cho phép của selector: chỉ `vis-cli`/host adapter, hay có được lọt vào core crate (VD-250 §1 `agent/DECISIONS.md:2443`)? | Chỉ host adapter; core giữ wasm32. |
| P6 | Khi nào được phép chọn crate cụ thể? | **Sau khi** A0 chốt P1–P5 và nhận exception report §2.3; ST-266 không chọn. |

---

## 5. Giới hạn phạm vi

Không sửa `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, không cài crate, không đổi R1–R6, không commit, không chạy gate. Tài liệu này chỉ để A0 chốt chính sách và quy trình.
