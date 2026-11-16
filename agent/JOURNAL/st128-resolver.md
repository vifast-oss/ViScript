# ST-128 · Resolver tất định + registry nhiều phiên bản (VD-125)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-128** · ngân sách 3 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 Q9 + §6 "ST-127..ST-129 · Gói",
[`DECISIONS.md`](../DECISIONS.md) **VD-125**, hàng ST-128 trong [`QUEUE.md`](../QUEUE.md).

## 1. Mục tiêu (nguyên văn + cách đo)

- Mỗi gói **đúng một** phiên bản trong đồ thị (VS2).
- Chọn **phiên bản cao nhất thoả mọi ràng buộc**, **quay lui** khi xung đột.
- Không thoả được ⇒ **mã lỗi MỚI trong khối VS07xx** (số **kế tiếp**), thông điệp nêu **chuỗi ràng
  buộc gây xung đột**; thêm vào `explain` **3 ngôn ngữ**; cập nhật **đủ** các chỗ đếm mã.
- **≥12 ca**: chọn cao nhất, quay lui, xung đột, chuỗi ràng buộc; chạy 2 lần cho **cùng byte**.

## 2. Việc đã làm

1. `crates/vis-cli/src/pkg/resolver.rs` (**mới**, 470 dòng): `Resolver` đọc registry lười, sắp ứng
   viên **giảm dần**; DFS **quay lui**; chọn gói **tên nhỏ nhất** chưa chọn (tất định); ghi nhận
   xung đột **nhiều ràng buộc nhất** (tie: tên nhỏ hơn) ⇒ thông điệp ổn định giữa các lần chạy.
   `LoiXungDot { ten, rang_buoc: Vec<(nguồn, req)>, co: Vec<Version>, loi_manh }` — in ra
   `không phiên bản `X` nào thoả đồng thời `^2` (từ a@1.0.0) + `<1.5` (từ b@1.0.0); registry có: …`.
   `cao_nhat_thoa()` cho đường `pkg update` cũ.
2. **Mã mới `VS0704`** trong `MODULE_CODES` (khối `07xx`, số **kế tiếp** sau `703`), đủ **3 ngôn ngữ**
   (vi/en/ja) ⇒ `vis explain VS0704 --lang {vi,en,ja}` chạy qua đường có sẵn.
3. `pkg.rs`: `pkg add` giải **toàn đồ thị** bằng resolver (`gom_theo_giai`); `deps` trong manifest nay
   là **ràng buộc** semver (`VersionReq`), không bắt buộc version chính xác; `gom_goi` (đường `update`)
   giải từng ràng buộc dep qua `cao_nhat_thoa`; `--entry` áp cho **gói gốc** (kể cả khi manifest của
   nó **thiếu** `entry`); lỗi resolver ⇒ `VS0704: <chuỗi ràng buộc>` + exit 2.
4. **Cập nhật đủ chỗ đếm** (32 → **33** mã): `crates/vis-diag/src/lib.rs` (assert + vòng `lookup`
   `701..704`), `crates/vis-diag/tests/messages.rs` (`MODULE_CODES.len()` 3 → 4, `all_codes` 32 →
   33), `crates/vis-cli/tests/explain_moi.rs` (4 chỗ), `crates/vis-cli/tests/st83_no_nho.rs`
   ("7 khối, 33 mã"), `agent/API-CODES.txt` (sinh lại bằng `VIS_FREEZE_REGEN=1`).

## 3. Bỏ / đổi hàm nội bộ (ghi rõ, không giấu)

| hàm | trước | sau |
|---|---|---|
| `chon_cao_nhat` | chọn version cao nhất theo `so_phien_ban` | **bỏ** — resolver `>=0.0.0` làm đúng việc đó (chọn cao nhất) |
| `cmp_phien_ban` | so 2 chuỗi version (lỏng) | **bỏ** — so thứ tự do `semver::Version: Ord`; `so_phien_ban` **giữ** cho **đọc lock cũ** (`1.0` vẫn đọc được) |
| `tach_ten` | trả `Option<String>` version chính xác | trả `Option<VersionReq>`; `@1.2.3` (đủ 3 thành phần) vẫn là **ghim đúng** `=1.2.3` (tương thích ST-70), dạng khác là **ràng buộc** (`^1`, `<1.9`, `>=1,<2`) |
| `GoiCai` | không có deps | +`deps` (để `update` giải ràng buộc dep) |

Ca cũ `pkg::tests::so_phien_ban_theo_so` (dựa `cmp_phien_ban`) đổi thành
`so_phien_ban_long_cho_lock_cu` — **không** phải "sửa kỳ vọng cho xanh": phép so thứ tự vẫn được kiểm
ở `semver::tests::version_in_ra_va_thu_tu` + `resolver::tests::chon_cao_nhat_khi_moi_ban_deu_thoa`;
`so_phien_ban` (đọc lock cũ) vẫn có ca riêng.

## 4. Ca kiểm (≥12 + CLI)

**Unit — `pkg::resolver::tests` (17 ca)**: chọn cao nhất; ràng buộc thu hẹp khoảng; `^0.1` trong
deps; phụ thuộc bắc cầu; hai gốc dùng chung dep ⇒ **một** phiên bản; AND ba gốc; **quay lui một
tầng**; **quay lui hai tầng**; xung đột nêu **nguồn** cả hai ràng buộc (+ bản có); xung đột với
**gốc**; gói không có trong registry; không bản nào thoả ⇒ liệt kê bản có; **tất định: hai lần giải ⇒
cùng byte** (so cả chuỗi `tên@ver\n`); bỏ qua phiên bản **không hợp lệ VD-125** (prerelease/thiếu
thành phần); vòng phụ thuộc; không deps; `cao_nhat_thoa`.

**CLI — `crates/vis-cli/tests/pkg_resolver.rs` (7 ca)**: `@^1.0` ⇒ 1.10.0 + dep; `@1.0.0` vẫn **ghim**;
`@<1.9` ⇒ 1.0.0; `@^2` ⇒ **exit 2 + VS0704** + nêu ràng buộc + bản có, **không** tạo `.vis`/lock;
xung đột **qua dep** ⇒ `VS0704` nêu nguồn `a@1.0.0`; **hai thư mục khác nhau ⇒ lock byte y hệt**,
add lại không đổi byte; ràng buộc sai cú pháp ⇒ exit 2.

## 5. Lệnh + số đo

```
cd viscript
~/.cargo/bin/cargo test -p vis-cli --bin vis pkg::resolver   → 17 passed
~/.cargo/bin/cargo test -p vis-cli --bin vis pkg::semver     → 19 passed (không hồi quy)
~/.cargo/bin/cargo test -p vis-cli --test pkg                → 24 passed (không hồi quy)
~/.cargo/bin/cargo test -p vis-cli --test pkg_resolver       → 7 passed
```

**`all_codes` mới = 33** (7 khối; `VBC1` vẫn 53; FUNCS 52; sem 447/22; parity 398+49 — không đổi).
`next_block()` vẫn **801** (không đổi).

Log phạm vi (chỉ các tệp của ST-128): [`st128-tests-pham-vi.log`](st128-tests-pham-vi.log) —
`sha256 7bb2c92726b17ac491cba56e39c8c4bcd4445de4ba270d3bdd5b23aa403aa62b`: `pkg::` **38**, `--test pkg`
**24**, `pkg_resolver` **7**, `so_ma` **4**, `api_dong_bang` **3**, `explain_moi` **6**, `st83_no_nho`
**6**, `vis-diag` **10 + 11** — tất cả `0 failed`.

### Cổng toàn workspace (ghi rõ ngoài phạm vi)

Lần chạy `cargo test --workspace` **không đạt** vì **tệp của agent khác** đang sửa dở:
`crates/vis-cli/src/json_esc.rs:313` — `error[E0369]: binary operation != cannot be applied to type
Result<Value, String>` (ST-125 fuzz `json_esc`). Log + sha256:
[`st128-cong-ngoai-pham-vi.log`](st128-cong-ngoai-pham-vi.log) `56c3148cc5818db845172c1bffd5e7a3251735367ef8c48eb812b091fa0d2832`.
Đây **không** phải hồi quy của ST-128 (không tệp nào của tôi bị nêu); thử lại cổng khi agent kia commit.

**Lỗi tự tìm & sửa trong lúc làm** (ghi để không giấu): `loi_xung_dot` bản đầu dùng literal
`"VS0700"` làm fallback ⇒ `tests/so_ma.rs::a_la_tap_con_cua_b` **đỏ** (mã dùng mà không có trong sổ);
sửa thành `vis_diag::code(704)` (không thêm literal), test xanh lại — đúng vai của bất biến ST-96.

## 6. Tệp đổi

`crates/vis-cli/src/pkg/resolver.rs` (**mới**), `crates/vis-cli/src/pkg.rs` (nối resolver + `VS0704` +
bỏ 2 hàm cũ), `crates/vis-diag/src/lib.rs` (+`VS0704`, đếm 33), `crates/vis-diag/tests/messages.rs`,
`crates/vis-cli/tests/explain_moi.rs`, `crates/vis-cli/tests/st83_no_nho.rs`,
`crates/vis-cli/tests/pkg_resolver.rs` (**mới**), `agent/API-CODES.txt` (sinh lại),
`agent/JOURNAL/st128-*`, `agent/STATE.md`, `agent/QUEUE.md`.

## 7. Chỗ **lệch** so với quyết định / quan sát (kèm dòng thực tế)

1. **Số mã mới**: VD-125 nói "mã lỗi MỚI trong khối VS07xx"; `next_block()`
   (`crates/vis-diag/src/lib.rs:439`) trả **801** vì nó suy từ **khối đã dành trước**
   (`BLOCKS_RESERVED` 1..7) chứ không từ khối **đang có mã**. Tôi cấp **`704`** đúng "số kế tiếp của
   khối 07xx" theo yêu cầu task, **không** dùng `next_block()`. Hệ quả: từ nay `next_block()` **không
   còn** là nguồn đúng cho khối kế (nó bỏ qua việc `07xx` còn chỗ) — ghi **nợ #109**: hoặc sửa
   `next_block()` để trả "số kế tiếp trong khối đã cấp cuối" (`704`), hoặc ghi rõ nó là "khối **mới**"
   chứ không phải "số kế tiếp". Test `vis-diag` vẫn khoá `next_block() == 801`.
2. **`deps` của manifest nay là ràng buộc**, không phải version chính xác — cần cho VD-125 ("ranges");
   fixture cũ (`"deps":{"util":"0.1.0"}`) vẫn chạy vì `^0.1.0` chỉ có một bản để chọn.
3. **`@1.2.3` trong CLI vẫn là ghim đúng** (`=1.2.3`), không theo "không toán tử ⇒ `^`": lý do là hành
   vi đã có từ ST-70 và có ca kiểm (`add_roi_list`); đổi nó là **đổi ngữ nghĩa CLI** không có VD. Ghi
   nợ/điểm cần Agent 0 xác nhận (#110) nếu muốn đồng nhất tuyệt đối với VD-125.
4. **Resolver không chọn bản có manifest hỏng** nhưng **ghi nhớ** lỗi manifest để thông điệp cuối nêu
   đúng lỗi cấu hình (ví dụ thiếu `entry`) thay vì chỉ "không có phiên bản" — cần cho 2 ca cũ
   (`manifest_thieu_truong`, `entry_tuy_chon_khi_manifest_thieu`) **không** phải sửa kỳ vọng.
5. `pkg update` **giữ** hành vi cũ (đọc lại registry cùng version, cập nhật sha nếu nội dung đổi); chỉ
   **giải lại toàn đồ thị + lock** là việc của ST-129 theo đúng hàng QUEUE.

## 8. Nợ mới (đánh số tiếp từ **109**)

| # | Nợ | Mức |
|---|---|---|
| **109** | `next_block()` trả `801` trong khi khối `07xx` vẫn còn chỗ (`704` đã cấp): hàm đang là "khối **mới**", không phải "số **kế tiếp**". Hoặc sửa hàm, hoặc đổi tài liệu/tên cho khớp — chưa làm vì test `vis-diag` đang khoá `801` | VS3 |
| **110** | Ngữ nghĩa `pkg add tên@1.2.3` = **ghim** (`=1.2.3`) khác "không toán tử ⇒ `^`" của VD-125. Giữ để tương thích ST-70; cần Agent 0 xác nhận (nếu đổi ⇒ sửa `doc_req_cli` + ca `add_roi_list`) | chờ quyết |
| **111** | Resolver chưa xét **yanked**/phiên bản rút; chưa có registry mạng/chữ ký (VS3 theo VD-125) | VS3 |
