<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-3 · Net: TLS + HTTP/1.1 keep-alive
**Trạng thái**: CHỜ AGENT 0 DUYỆT (VD-148) · **Phụ thuộc**: QĐ-1 (worker/IO) · **Mở đường cho**: QĐ-6

## 1. Mục tiêu đo được
1. `https://` chạy được: GET/POST tới **server thử trong fixtures** (chứng chỉ tự ký nhúng repo) ⇒ 200 + body đúng; **không** gọi mạng ngoài trong test.
2. Xác minh chứng chỉ: chứng chỉ tự ký **không** nằm trong kho hệ thống ⇒ **lỗi** rõ ràng (không im lặng bỏ qua).
3. Keep-alive: 2 request liên tiếp **cùng** kết nối (đếm accept ở server thử = 1).
4. Cổng có bước kiểm **licenses + advisories** (`cargo-deny`), không mạng ⇒ dùng cache.

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-cli/src/net.rs:9` — `TcpListener/TcpStream`; `:17` `struct …{ tay: Vec<(String, TcpStream)> }`; `:87` `accept_tren`.
- `crates/vis-cli/src/modules.rs:550` — module chuẩn `str/list/map/path/time/json/fs/net/http` (chưa TLS).
- 5 câu VD-131 hiện hành: “Net không TLS.” — **phải đổi** khi xong.

## 3. Bảng phụ thuộc (theo VD-161, Agent 1 **phải** xác minh lại bằng `cargo metadata` ở phiên bản ghim)
| Crate | Phiên bản ghim | Giấy phép (khai) | Lý do | unsafe bên trong |
|---|---|---|---|---|
| `rustls` | (chốt khi duyệt) | Apache-2.0/MIT/ISC (**kiểm lại**) | TLS thuần Rust | có (được phép, crate ngoài) |
| `rustls-platform-verifier` | " | Apache-2.0/MIT (**kiểm lại**) | kho chứng chỉ HĐH | có |
| `ring` | " | **Apache-2.0 AND ISC** (đính chính B2) | crypto provider | có |
| `rustls-webpki` | **≥ 0.103.16** | ISC | xác minh chuỗi | có |
| `webpki-roots` | chỉ **wasm** | **CDLA-Permissive-2.0** (chỉ dữ liệu chứng chỉ) | roots cho wasm | không |

Quy tắc: ngoài danh sách giấy phép cho phép (MIT/Apache-2.0/ISC/BSD-2/3/Unicode-3.0/Zlib; CDLA chỉ cho dữ liệu chứng chỉ) ⇒ **BLOCKED-CHO-A0**. Crate của ta vẫn `#![forbid(unsafe_code)]`.

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-3a | Client TLS + platform verifier + test server tự ký | test offline 200/body; cert lạ ⇒ lỗi rõ | QĐ-1 |
| VS3-3b | Keep-alive HTTP/1.1 | 2 request = 1 accept | 3a |
| VS3-3c | `cargo-deny` licenses+advisories trong cổng (cache offline) | cổng chạy không mạng; tài liệu cài trên mac/ps/psvn | 3a |
| VS3-3d | Sandbox `net` cap + tài liệu | cap thiếu ⇒ `VS0604`; doc cập nhật “Net có TLS” | 3b |

## 5. Ca đỏ-trước
- HTTPS tới server tự ký **trong fixtures** ⇒ 200 (trước: lỗi “không hỗ trợ TLS”).
- Cert tự ký **không** trong kho hệ thống ⇒ lỗi xác minh.
- 2 request ⇒ 1 accept.

## 6. Cổng
`gate --fast` + bước `cargo-deny` (licenses/advisories) + test offline (không mạng).

## 7. Rủi ro
Kho chứng chỉ HĐH khác nhau (mac/ps/psvn); proxy; wasm roots; `ring` build trên Windows (cần NASM? — phải kiểm).
