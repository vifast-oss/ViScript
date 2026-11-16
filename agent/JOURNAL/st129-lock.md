# ST-129 · pkg CLI dùng resolver + migration lock (VD-125)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-129** · ngân sách 2,5 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §6 "ST-127..ST-129 · Gói",
[`DECISIONS.md`](../DECISIONS.md) **VD-125**, hàng ST-129 trong [`QUEUE.md`](../QUEUE.md).

## 1. Mục tiêu (nguyên văn + cách đo)

- `pkg install`: lock hiện có **còn thoả** ⇒ **giữ nguyên** (không đổi byte). `pkg update`: giải lại.
- Lock **định dạng cũ** ⇒ đọc được, ghi lại định dạng mới; **ghi lại hai lần cho ra cùng byte**.
- `pkg list --json` có `version_req`.
- **Đo định dạng lock HIỆN TẠI trước** khi thiết kế định dạng mới; dán ví dụ lock cũ + mới vào JOURNAL.

## 2. **Đo định dạng lock TRƯỚC khi thiết kế** (đo, không đoán)

Cách đo: tạo worktree tạm ở **commit ST-128** (`a7867d66`, trước mọi thay đổi của ST-129), build
`vis`, chạy `vis pkg add http@^1 --registry test/pkg/registry`, rồi **sao lại từng byte** `vis.lock`:

| | nội dung | dài |
|---|---|---|
| **lock cũ (định dạng 1)** | `{"lock":1,"packages":[{"name":"http","version":"1.10.0","entry":"src/main.vis","sha256":"d167d77c…","files":[{"path":"src/main.vis","sha256":"b9df4663…"}]},{"name":"util","version":"0.1.0","entry":"src/main.vis","sha256":"3666feaa…","files":[…]}]}` | 510 byte |
| **lock mới (định dạng 2, sau `pkg install`)** | `{"lock":2,"roots":{},"packages":[{"name":"http","version":"1.10.0","version_req":"=1.10.0","entry":"src/main.vis","sha256":"d167d77c…","files":[…]},{"name":"util","version":"0.1.0","version_req":"=0.1.0","entry":"…","sha256":"3666feaa…","files":[…]}]}` | 568 byte |

Bản đầy đủ (nguyên văn hai tệp + `sha256`) ở [`st129-lock-do.log`](st129-lock-do.log)
(`5c924e581112cb88877410dae8edbc43eaa6f180732703cfa9149e010b63ec3c`).
Đo thêm: **hai lần `pkg install` liên tiếp trên lock mới ⇒ `sha256` y hệt**
(`95da92feec667161e343aa9fcc2f72b8c946112e95368df2d925c13d0baedf3a` cả hai lần) — tức "ghi lại hai
lần cho ra cùng byte" **đo được**, không phải suy luận.

**Quan sát từ số đo**: lock cũ **không** có (a) ràng buộc đã chọn ra phiên bản, (b) tập gói người
dùng đòi (`roots`). Hệ quả: `install`/`update` trước ST-129 chỉ có thể **ghim theo `version`** — không
có cách nào biết "còn thoả" nghĩa là gì. Đó là lý do định dạng 2 thêm **hai** trường đó.

## 3. Thiết kế định dạng **2**

```json
{"lock":2,
 "roots":{"http":"^1.0"},                                  // ràng buộc GỐC người dùng đòi (chuẩn hoá)
 "packages":[{"name":"http","version":"1.10.0",
              "version_req":"^1.0",                        // ràng buộc HIỆU LỰC của gói
              "entry":"src/main.vis","sha256":"…","files":[{"path":"…","sha256":"…"}]}]}
```

- `version_req` = **AND** của ràng buộc gốc + mọi ràng buộc dep trỏ tới gói, in **chuẩn hoá** (bỏ
  trùng, sắp theo chuỗi) ⇒ **tất định**, ví dụ `"^0.1, ^0.1.0"`.
- Thứ tự khoá **cố định** (`lock`, `roots`, `packages`; mỗi gói `name`,`version`,`version_req`,
  `entry`,`sha256`,`files`) ⇒ ghi hai lần **cùng byte**.
- Lock cũ (không `version_req`) ⇒ migrate: mỗi gói `version_req = "=<version>"` (**ghim**, bảo toàn
  đúng phiên bản đang có — đúng tinh thần VD-125 "còn thoả thì giữ nguyên"), `roots` = `{}` (không
  biết gói nào là gốc). `update` sau đó lấy mỗi gói làm root `=<version>` (vẫn nhất quán).

## 4. Hành vi từng lệnh (sau ST-129)

| lệnh | hành vi |
|---|---|
| `pkg add <tên>[@req]` | giải **cả đồ thị** từ `roots` (lock cũ ⇒ suy `roots` = `=<version>`); ghi lock **2** (`roots` + `version_req` hiệu lực). Không ghi `@` ⇒ root = `^<phiên bản đã chọn>` |
| `pkg install` | lock **2 còn thoả** ⇒ **không ghi** (byte y hệt); lock **cũ** ⇒ đọc, `version_req = "=<version>"`, **ghi lại 1 lần**; lock 2 **không còn thoả** ⇒ **exit 2** + "chạy `vis pkg update`" (install không có registry) |
| `pkg update [<tên>]` | **giải lại** từ `roots` (lock cũ ⇒ mỗi gói là root `=<version>`); in `không đổi`/`đã cập nhật`; chỉ ghi lock khi có đổi/migrate. Có `<tên>` ⇒ **ghim** các gói khác ở phiên bản hiện có |
| `pkg list --json` | mỗi gói có `version_req`; lock cũ chưa migrate ⇒ in `=<version>` (giá trị **hiệu lực**) để đầu ra không phụ thuộc trạng thái migrate |
| `pkg remove <tên>` | bỏ khỏi `packages` **và** `roots` |

## 5. Ca kiểm — `crates/vis-cli/tests/pkg_lock.rs` (**8 ca**)

1. `install` giữ nguyên byte khi lock 2 còn thoả (+ kiểm `roots`/`version_req` của `add`).
2. lock **định dạng cũ** ⇒ `install` migrate lên 2 (`version_req = "=1.0.0"`), **ghi lại lần 2 cùng byte**.
3. lock 2 **không còn thoả** (`version_req ^2` trên bản 1.0.0) ⇒ exit 2 + chỉ đường `pkg update`.
4. `update` **nâng trong khoảng**: thêm `http@1.11.0` vào registry sao lưu ⇒ `pkg update` in
   `đã cập nhật http 1.11.0`, lock `version_req` giữ `^1.0`, cây cài có bản mới; update lần 2 **cùng byte**.
5. `list --json` có `version_req` đúng **ràng buộc hiệu lực** (`http` = `^1.0`; `util` = `^0.1.0`).
6. `add` hai gói ⇒ `roots` có cả hai (sắp theo tên); `version_req` của dep = **AND** của root + dep.
7. `remove` bỏ cả `roots`; `install` sau đó vẫn chạy.
8. lock **cũ** + `update` ⇒ ghi định dạng 2 **một lần**, lần sau cùng byte.

## 6. Lệnh + số đo

```
cd viscript
~/.cargo/bin/cargo test -p vis-cli --test pkg_lock       → 8 passed
~/.cargo/bin/cargo test -p vis-cli --test pkg            → 24 passed (không hồi quy)
~/.cargo/bin/cargo test -p vis-cli --test pkg_resolver   → 7 passed
~/.cargo/bin/cargo test -p vis-cli --bin vis pkg::       → 38 passed
~/.cargo/bin/cargo clippy -p vis-cli --all-targets -- -D warnings → OK (xem §8 mục 4)
~/.cargo/bin/rustfmt --edition 2021 --check <tệp của tôi> → OK
```

Log phạm vi: [`st129-tests-pham-vi.log`](st129-tests-pham-vi.log)
(`f929681708d673e2f2cb299a718836a04db6b8e19eab8f1df0f27077c47a0656`).
Số khoá **không đổi**: `all_codes` **33**, `VBC1` **53**, FUNCS **52**, sem 447/22, parity 398+49.

## 7. Tệp đổi

`crates/vis-cli/src/pkg.rs` (lock v2: `Khoa`/`doc_lock`/`ghi_lock`, `install`/`update`/`add`/`list`/
`remove`, helper `rang_buoc_hieu_luc`/`khoa_thoa`/`roots_tu_lock`/`tao_khoa`/`giai_va_doc`/`ghi_cay`;
bỏ `gom_goi`/`gom_theo_giai`/`cao_nhat_thoa` không còn dùng),
`crates/vis-cli/src/pkg/resolver.rs` (bỏ `cao_nhat_thoa`; ca `cac_ban_tang_dan_va_loc_theo_req`),
`crates/vis-cli/tests/pkg_lock.rs` (**mới**), `agent/JOURNAL/st129-*.md|log`, `agent/STATE.md`,
`agent/QUEUE.md`.

## 8. Chỗ **lệch** so với quyết định / tự chọn (kèm dòng thực tế)

1. **`pkg add` không ghi `@`** ⇒ root = `^<phiên bản đã chọn>` (không phải `>=0.0.0`): `>=0.0.0` đúng
   nghĩa "mọi bản" nhưng vô dụng trong lock (update luôn nhảy lên bản mới nhất, kể cả major khác).
   Chọn `^…` để `update` **chỉ nâng trong major** — đây là **lựa chọn của Agent 2**, VD-125 không nói.
2. **Migration chọn `=<version>`** (ghim) chứ không `^<version>`: bảo toàn tuyệt đối phiên bản đang
   cài, đúng câu "lock còn thoả thì giữ nguyên"; muốn nâng thì `pkg add` lại với ràng buộc. Ghi ở đây
   để Agent 0 quyết nếu muốn migrate "nới" hơn.
3. **`install` không tự giải lại** khi lock không còn thoả (không có `--registry`, không mạng) ⇒ báo
   lỗi + chỉ đường `pkg update`. VD-125 chỉ nói "install giữ nguyên khi còn thoả", không nói trường hợp
   còn lại.
4. **Clippy**: cổng hiện có **một** lỗi lint ở **tệp của agent khác** (`crates/vis-cli/src/quyen.rs:721`
   `manual_is_multiple_of`, ST-124 đang sửa dở). Tôi chạy clippy với `-A clippy::manual_is_multiple_of`
   để chứng minh **phần của tôi** sạch (`CLIPPY_EXIT=0`); không sửa tệp của người khác.
5. `BTreeSet` vẫn dùng (remove/blob còn dùng) — không còn hàm nào của tôi import thừa.

## 9. Nợ mới (đánh số tiếp từ **112**)

| # | Nợ | Mức |
|---|---|---|
| **112** | Lock 2 **không** ghi lại **quan hệ dep** (chỉ `version_req` tổng hợp). Muốn kiểm "đồ thị còn nhất quán" (mỗi dep thực sự thoả ràng buộc của cha nó) phải **giải lại** — `install` hiện chỉ kiểm từng gói với `version_req` của chính nó. Ghi `deps` (name→req) vào lock là việc của VS3 nếu cần chẩn đoán sâu hơn | VS3 |
| **113** | `pkg update <tên>` **ghim** mọi gói khác (`=<version>`): nếu gói được update cần **nâng** một dep chung, resolver sẽ báo `VS0704` thay vì nâng dep đó. Hành vi an toàn (không đụng gói khác) nhưng có thể gây bất ngờ; cần quyết định chính sách | chờ quyết |
