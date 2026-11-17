<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-4 · Quyền: đường GHI nguyên tử (hết TOCTOU)
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · đã áp [SỬA] tại **ST-178** · **Phụ thuộc**: QĐ-1 · **Mở đường cho**: —

## 1. Mục tiêu đo được
1. **Linux và macOS dùng cùng một cơ chế**: đi **từng thành phần đường dẫn** từ fd của thư mục gốc được cấp quyền — `openat(dirfd, thanh_phan, O_NOFOLLOW | O_DIRECTORY)` cho thư mục, `O_NOFOLLOW` cho tệp cuối; **đọc và ghi dùng chung một hàm**. Không còn `canonicalize` rồi so, **không cần fd→path**.
   > **đã sửa**: `agent/JOURNAL/vs3-qd-4.md:@@L1@@` (ST-178) — thay thiết kế cũ “O_NOFOLLOW + so dev/ino sau khi mở”.
2. Linux ≥ 5.6: `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)` là **đường nhanh**, có **fallback** đi từng thành phần; test chạy **cả hai đường**.
3. `..`: xử lý **theo từ vựng trong phạm vi gốc**; ra ngoài gốc ⇒ **từ chối**.
4. **macOS không còn khe hở bản chất** như §7 cũ ghi (nhờ không cần fd→path). Giới hạn **còn lại**: **hardlink** (VD-176 mục 3).
5. **Windows**: quyền tệp vẫn kiểm **theo đường dẫn** ở VS3; `HOST-BAO-DAM.md` ghi rõ **mức bảo đảm thấp hơn** (QĐ-5: “hỗ trợ thật” = chạy đúng, **không** cùng mức sandbox).
6. Không `unsafe` trong crate của ta (rustix API an toàn; crate giữ `#![forbid(unsafe_code)]`).

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-cli/src/quyen.rs:546` `struct FsGuard`; `:560` `read` (Linux dùng `/proc/self/fd`); `:568` `write` — **kiểm-rồi-mở** ⇒ nợ **#140/#109b**.
- `HOST-BAO-DAM.md` §4 ghi dòng giới hạn (Agent 0 sở hữu — **cần cập nhật** theo QĐ-4: bỏ khe hở macOS, ghi mức thấp hơn cho Windows; xem §8).
- macOS: VD-145 (không fd→path) — #137.

## 3. Thiết kế chốt (theo VD-161/VD-164 + VD-176(4) + VD-178 QĐ-4)
**§4b là thiết kế chính cho cả Linux và macOS** (thay thiết kế cũ “O_NOFOLLOW + so dev/ino sau khi mở”):
- Đi **từng thành phần đường dẫn** từ fd của thư mục gốc được cấp quyền: `openat(dirfd, thanh_phan, O_NOFOLLOW | O_DIRECTORY)` cho thư mục, `O_NOFOLLOW` cho tệp cuối. **Đọc và ghi dùng chung một hàm.**
- `..`: xử lý theo từ vựng trong phạm vi gốc; ra ngoài gốc ⇒ từ chối.
- Linux ≥ 5.6: `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)` làm **đường nhanh**, có fallback đi từng thành phần; test chạy cả hai đường.
- Không cần fd→path ⇒ **macOS không còn khe hở bản chất**; sửa §1, §7 và `HOST-BAO-DAM.md`.
- Giới hạn còn lại: **hardlink** (VD-176 mục 3).
- **`rustix` được duyệt** (VD-178 QĐ-4 mục 3): chọn giấy phép **MIT** trong lựa chọn của crate; xác minh bằng `cargo metadata` ở phiên bản ghim; dùng API an toàn; crate của ta giữ `forbid(unsafe_code)`.
- **Windows**: kiểm theo đường dẫn ở VS3; mức bảo đảm thấp hơn (không hứa cùng mức sandbox).

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-4a | Linux: `openat2` **đường nhanh** + fallback `openat`+`O_NOFOLLOW` cho `write` **và** `read` | ca đỏ-trước **tất định** 100/100; L0 xanh; test **cả hai đường** | QĐ-1 |
| VS3-4b | macOS: **cùng cơ chế** đi từng thành phần (bỏ `dev/ino` sau mở) | ca đỏ-trước **tất định** 100/100 trên mac; ghi giới hạn hardlink | 4a |
| VS3-4c | Ca đua ghi (đỏ-trước, **tất định**) + `HOST-BAO-DAM` cập nhật | **100/100 đỏ trước, 100/100 xanh sau**; 500 000 vòng chỉ là bằng chứng **phụ**; doc khớp | 4a |
| VS3-4d | Rustix: bảng phụ thuộc + cổng licenses | `cargo-deny` xanh; không `unsafe`; giấy phép **MIT** xác minh bằng `cargo metadata` | 4a |

## 5. Ca đỏ-trước
- **Tất định** (điểm chèn `#[cfg(test)]`, như VD-176): đổi symlink giữa kiểm và mở (như `quyen.rs:704`) ⇒ phải **chặn**; **100/100 đỏ trước, 100/100 xanh sau**.
- Ca đua 500 000 vòng chỉ là **bằng chứng phụ** (không thay ca tất định).
- `..` ra ngoài gốc ⇒ từ chối; hardlink ⇒ ghi rõ giới hạn.

## 6. Cổng
`gate --fast` + ca tất định 100/100 trên mac/ps + ca đua 500k (phụ) + `cargo-deny`.

## 7. Rủi ro
Nhân kernel cũ (openat2 < 5.6) ⇒ fallback đi từng thành phần; **hardlink** vẫn là giới hạn còn lại; Windows mức bảo đảm thấp hơn (kiểm theo đường dẫn).

## 8. Việc còn lại ngoài tệp này (chờ A0)
- `HOST-BAO-DAM.md` §4 (Agent 0 sở hữu): bỏ câu “macOS còn khe hở bản chất”, ghi **hardlink** là giới hạn còn lại, ghi **Windows mức bảo đảm thấp hơn** (kiểm theo đường dẫn). **ST-178 không sửa tệp đó** (ngoài danh sách tệp được giao).

## 4b. Hướng dài hạn (VD-176(4)) — **nay là thiết kế chính ở §3**
Mở **từng thành phần đường dẫn** bằng **`openat` + `O_NOFOLLOW`** qua **`rustix`** (Linux: `openat2` với `RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS`), **bỏ hẳn kiểm theo đường dẫn** (không còn `canonicalize` rồi so). **Đường đọc và đường ghi dùng cùng một cơ chế**; đây là cùng hướng với `#140/#109b` (VS3). **Không** làm ở ST-174 (chỉ vá 1 dòng `symlink_metadata`).

## 9. Bảng [SỬA] VD-178 đã áp (ST-178)
| [SỬA] VD-178 | Nội dung | đã sửa |
|---|---|---|
| QĐ-4 mục 1 | §3 viết lại theo **đi từng thành phần**; `openat`+`O_NOFOLLOW`; `openat2` đường nhanh; macOS hết khe hở; hardlink còn lại | `agent/JOURNAL/vs3-qd-4.md:@@L1@@` |
| QĐ-4 mục 2 | Chứng minh bằng **ca tất định** 100/100 (500k chỉ là phụ) | `agent/JOURNAL/vs3-qd-4.md:@@L2@@` |
| QĐ-4 mục 3 (áp dụng) | **`rustix` được duyệt** (MIT, `cargo metadata`, API an toàn) | `agent/JOURNAL/vs3-qd-4.md:@@L3@@` |
| QĐ-4 mục 4 (áp dụng) | Windows kiểm theo đường dẫn, **mức bảo đảm thấp hơn** | `agent/JOURNAL/vs3-qd-4.md:@@L4@@` |
