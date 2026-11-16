# ST-127 · Semver: `Version` + `VersionReq` + so khớp (VD-125)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-127** · ngân sách 2,5 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 Q9 + §6 "ST-127..ST-129 · Gói",
[`DECISIONS.md`](../DECISIONS.md) **VD-125**, hàng ST-127 trong [`QUEUE.md`](../QUEUE.md).

## 1. Mục tiêu (nguyên văn + cách đo)

Ngữ nghĩa **chốt sẵn, không tự thêm**:

- Phiên bản `MAJOR.MINOR.PATCH` **số nguyên**; **prerelease/build metadata bị từ chối** kèm chẩn đoán.
- Toán tử `^` `~` `=` `>` `>=` `<` `<=`; **dấu phẩy = AND**; **không toán tử ⇒ `^`** (giống Cargo, kể cả
  luật `^0.x`).
- **≥ 40 ca đơn vị** phủ mọi toán tử + `^0.x` + từ chối prerelease. **Không crate mới.**

## 2. Việc đã làm

`crates/vis-cli/src/pkg/semver.rs` (**mới**, 622 dòng — dưới R1 1.500):

| thành phần | nội dung |
|---|---|
| `LoiSemver { goc, ly_do }` | lỗi có **nguyên văn** chuỗi + lý do tiếng Việt; `Display` = `` `gốc`: lý do ``; hằng `LY_DO_PRERELEASE` nhắc đúng **VD-125** |
| `Version { major, minor, patch }` | `parse` **đúng ba** thành phần; `Display`; `Ord` theo số (derive) |
| `ToanTu` | đúng **bảy** toán tử của VD-125 (+ `dau()`) |
| `Khoang { tt, major, minor, patch, n }` | một ràng buộc; `n` = **số thành phần đã ghi** ⇒ luật chặn trên của `^`/`~` giống Cargo (`^0` ≠ `^0.0` ≠ `^0.0.0`); `khop()`; `Display` in **chuẩn hoá** theo `n` |
| `VersionReq { ds }` | `parse` (dấu phẩy = AND, không toán tử ⇒ `^`), `khop` = **mọi** khoang, `ds()`, `chuan_hoa()`; `PartialEq` là **bình đẳng ngữ nghĩa** (không so chuỗi gốc) |

Từ chối tường minh (kèm lý do): rỗng, `||`, khoảng `a - b`, `!=`, `*`/`x`, thành phần rỗng giữa hai
dấu phẩy, `^`/`~` không có số, prerelease/build, số có 0 đứng đầu, tràn `u64`.

**Nối vào code sản phẩm** (để không phải `allow(dead_code)` cho cả module): `chon_cao_nhat()` trong
`pkg.rs` nay dùng `semver::Version::parse` làm **cổng kiểm** phiên bản registry — thư mục
`registry/<tên>/1.2` (thiếu PATCH, prerelease…) bị **bỏ qua** đúng VD-125.

## 3. Bảng ngữ nghĩa đã cài (đối chiếu Cargo, đo bằng ca)

| ràng buộc | khoảng tương đương | ca |
|---|---|---|
| `^1.2.3` | `>=1.2.3, <2.0.0` | `xanh 1.2.3/1.9.9`; `đỏ 1.2.2, 2.0.0` |
| `^0.2.3` | `>=0.2.3, <0.3.0` | `xanh 0.2.9`; `đỏ 0.3.0` |
| `^0.0.3` | `>=0.0.3, <0.0.4` | `đỏ 0.0.4` |
| `^0.0` | `>=0.0.0, <0.1.0` | `xanh 0.0.9`; `đỏ 0.1.0` |
| `^0` | `>=0.0.0, <1.0.0` | `xanh 0.9.9`; `đỏ 1.0.0` |
| `^0.0.0` | `=0.0.0` | `đỏ 0.0.1` |
| `~1.2.3`, `~1.2` | `>=1.2.3, <1.3.0` / `>=1.2.0, <1.3.0` | `đỏ 1.3.0` |
| `~1` | `>=1.0.0, <2.0.0` | `đỏ 2.0.0` |
| `~0.0.3`, `~0.0` | `<0.1.0` | `đỏ 0.1.0` |
| `=1.2` | `=1.2.0` (điền 0) | `đỏ 1.2.1` |
| `>1.2` | `>1.2.0` | `đỏ 1.2.0` |
| `<2` | `<2.0.0` | `đỏ 2.0.0` |
| `1.2.3` (không op) | `^1.2.3` | `xanh 1.9.0`; `đỏ 2.0.0` |
| `>=1.2.0, <1.5.0` | AND | `xanh 1.4.9`; `đỏ 1.5.0, 1.1.9` |
| `^1, <1.4` | AND trộn | `đỏ 1.4.0` |

## 4. Red-first

**Không có log đỏ kiểu test-đỏ** vì API này **chưa từng tồn tại** (viết test trước chỉ ra lỗi biên
dịch, không phải log đỏ hành vi — không hợp lệ nên **không** dựng giả). Thay bằng **đo hành vi CŨ**
để chứng minh khoảng trống mà task phải lấp:

| đo | log | sha256 |
|---|---|---|
| `vis pkg add "http@^1.0"` trên registry mẫu ⇒ **exit 2**, `vis pkg: version `^1.0` không phải số (phần `^1`)` — tức CLI **chưa** hiểu ràng buộc; ST-128/129 sẽ nối resolver để ca này chạy | [`st127-truoc-range.log`](st127-truoc-range.log) | `cd579e277ac6ae6d9fdf8996f9b161144aeda82e3d942d297c0709262cac19dc` |
| 19 test / **114 `assert`** + 27 chuỗi trong vòng `for` (ca từ chối) | [`st127-semver-tests.log`](st127-semver-tests.log) | `a432911c61ef26f3e17c19a6f0854bb48419a7785ef03a8c673e9a43b4c9b700` |

## 5. Lệnh + số đo

```
cd viscript
~/.cargo/bin/cargo test -p vis-cli --bin vis pkg::semver   → 19 passed (114 assert, 27 ca vòng for ≥ 40 ca)
~/.cargo/bin/cargo test -p vis-cli --test pkg              → 24 passed (không hồi quy registry/lock/update)
~/.cargo/bin/cargo clippy -p vis-cli --all-targets -- -D warnings → OK
~/.cargo/bin/cargo fmt -p vis-cli -- --check               → OK
```

Số khoá **không đổi**: FUNCS **52**, `all_codes` **32**, `VBC1` **53**, sem 447/22, parity 398+49,
allowlist 6. Không thêm crate (parser tự viết), không `unsafe`, R1 max không đổi (semver.rs 622).

## 6. Tệp đổi

`crates/vis-cli/src/pkg/semver.rs` (**mới**), `crates/vis-cli/src/pkg.rs` (+1 `pub mod semver;`,
`chon_cao_nhat` dùng cổng `semver::Version`), `agent/JOURNAL/st127-*`.

## 7. Chỗ **lệch** so với quyết định / tự chọn (ghi rõ)

1. **Quy ước cho toán tử so sánh với phiên bản thiếu thành phần** — VD-125 chỉ chốt luật `^` (giống
   Cargo). Tôi chọn: `=`,`>`,`>=`,`<`,`<=` **điền 0** cho thành phần thiếu (`>1.2` = `>1.2.0`,
   `<2` = `<2.0.0`). Cargo dùng biểu diễn khác cho `>1.2` (nới thành `>=1.3.0`); **không** chép vì
   VD-125 không yêu cầu và "điền 0" là quy ước đơn giản, tất định, có ca kiểm. Ghi ở đây để Agent 0
   quyết nếu muốn đổi.
2. `Version::parse` **bỏ khoảng trắng hai đầu** nhưng **không** chấp nhận khoảng trắng bên trong
   (`">= 1.0.0"` đỏ) — Cargo cũng vậy; ghi lại vì đây là lựa chọn.
3. Từ chối **số có 0 đứng đầu** (`01.2.3`) — VD-125 không nói; chọn **từ chối** (đúng Cargo) để khoá
   lock không có hai cách viết cùng một phiên bản.
4. `chon_cao_nhat` **siết** thêm: thư mục `registry/<tên>/1.2` giờ **bị bỏ qua** (trước đây nhận vì
   `so_phien_ban` lỏng). Đây là hệ quả **trực tiếp** của VD-125 ("phiên bản MAJOR.MINOR.PATCH"), không
   phải sửa kỳ vọng cho xanh: không fixture/test nào dùng phiên bản thiếu thành phần (đo: 4 manifest
   trong `test/pkg/registry` đều 3 thành phần).
5. `mod semver` khai `pub mod` trong `pkg.rs` và có `#![allow(dead_code)]` **tạm** vì `VersionReq`
   chưa nối vào đường chạy cho tới ST-128/129 — nợ **#107** (bỏ `allow` khi ST-129 xong).

## 8. Nợ mới (đánh số tiếp từ **107**)

| # | Nợ | Mức |
|---|---|---|
| **107** | `pkg/semver.rs` còn `#![allow(dead_code)]` cho tới khi resolver (ST-128) + lock (ST-129) dùng hết `VersionReq`/`Khoang`; **phải bỏ** ở ST-129 và để clippy `-D warnings` làm hàng rào | đóng ở ST-129 |
| **108** | Quy ước "điền 0" cho `>`,`<`,`<=`,`>=`,`=` với phiên bản thiếu thành phần là **của Agent 2**, không phải VD-125; nếu Agent 0 muốn theo Cargo chặt hơn (`>1.2` = `>=1.3.0`) thì phải sửa `Khoang::san()` + 6 ca | VS3 |
