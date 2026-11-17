<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-4 · Quyền: đường GHI nguyên tử (hết TOCTOU)
**Trạng thái**: CHỜ AGENT 0 DUYỆT (VD-148) · **Phụ thuộc**: QĐ-1 · **Mở đường cho**: —

## 1. Mục tiêu đo được
1. Trên Linux: ghi qua **fd đã mở** (`openat2`/`O_NOFOLLOW`), **0 rò** trong 500 000 vòng ca đua (như `#146`).
2. Trên macOS: **O_NOFOLLOW** + kiểm `dev/ino` sau mở; ghi rõ giới hạn còn lại trong `HOST-BAO-DAM.md`.
3. Windows: giữ "chưa hỗ trợ" (QĐ-5) — **không** hứa.
4. Không `unsafe` trong crate của ta (rustix API an toàn).

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-cli/src/quyen.rs:546` `struct FsGuard`; `:560` `read` (Linux dùng `/proc/self/fd`); `:568` `write` — **kiểm-rồi-mở** ⇒ nợ **#140/#109b**.
- `HOST-BAO-DAM.md` §4 ghi dòng giới hạn (Agent 0 sở hữu).
- macOS: VD-145 (không fd→path) — #137.

## 3. Thiết kế chốt (theo VD-161/VD-164)
- Linux: `rustix::fs::openat2` (RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH) hoặc `openat` + `O_NOFOLLOW`; ghi qua `File` từ fd.
- macOS: `O_NOFOLLOW` + `fstat` so `dev/ino` với `lstat` trước; còn khe hở ghi rõ.
- `rustix` đã có? **chưa** trong `Cargo.toml` của ta ⇒ thêm mới phải qua bảng phụ thuộc (giấy phép MIT/Apache/ISC?) và **Agent 0 duyệt**.

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-4a | Linux: `openat2`/`openat`+`O_NOFOLLOW` cho `write` | ca đua ghi 500k ⇒ 0 rò; L0 xanh | QĐ-1 |
| VS3-4b | macOS: `O_NOFOLLOW` + `dev/ino`; ghi giới hạn | ca đua 500k ⇒ 0 rò (hoặc ghi rõ số) | 4a |
| VS3-4c | Ca đua ghi (đỏ-trước) + `HOST-BAO-DAM` cập nhật | ca đỏ-trước chứng minh lỗ trước sửa; doc khớp | 4a |
| VS3-4d | Rustix: bảng phụ thuộc + cổng licenses | `cargo-deny` xanh; không `unsafe` | 4a |

## 5. Ca đỏ-trước
Đổi symlink giữa kiểm và mở (như `quyen.rs:704` ca tất định) ⇒ phải **chặn**; ca đua 500k ⇒ 0 rò.

## 6. Cổng
`gate --fast` + ca đua 500k trên mac/ps + `cargo-deny`.

## 7. Rủi ro
Nhân kernel cũ (openat2 < 5.6) ⇒ fallback; macOS vẫn còn khe hở bản chất (không fd→path) — phải nói thẳng; Windows không hứa.
## 4b. Hướng dài hạn (VD-176(4)) — dùng chung cơ chế cho đường đọc và đường ghi
Mở **từng thành phần đường dẫn** bằng **`openat` + `O_NOFOLLOW`** qua **`rustix`** (Linux: `openat2` với `RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS`), **bỏ hẳn kiểm theo đường dẫn** (không còn `canonicalize` rồi so). **Đường đọc và đường ghi dùng cùng một cơ chế**; đây là cùng hướng với `#140/#109b` (VS3). **Không** làm ở ST-174 (chỉ vá 1 dòng `symlink_metadata`).
