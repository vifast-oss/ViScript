<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-3 · Net: TLS + HTTP/1.1 keep-alive
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · đã áp [SỬA] tại **ST-178** · **Phụ thuộc**: QĐ-1 (worker/IO) · **Mở đường cho**: QĐ-6

## 1. Mục tiêu đo được
1. `https://` chạy được: GET/POST tới **server thử trong fixtures** (chứng chỉ tự ký nhúng repo) ⇒ 200 + body đúng; **không** gọi mạng ngoài trong test. Ca 200 **khai báo CA riêng**: `http.client(ca: p"fixtures/ca.pem")`.
2. Xác minh chứng chỉ: **mặc định** dùng **kho chứng chỉ hệ điều hành** (`rustls-platform-verifier`); chứng chỉ tự ký **không** nằm trong kho hệ thống và **không** khai `ca:` ⇒ **lỗi** rõ ràng (không im lặng bỏ qua). **Không có tuỳ chọn tắt xác minh** (kiểu insecure) ở 1.0.
   > **đã sửa**: `agent/JOURNAL/vs3-qd-3.md:6` (ST-178) — thêm tuỳ chọn `ca:` để ca 200 và ca từ chối không còn mâu thuẫn.
3. Keep-alive: 2 request liên tiếp **cùng** kết nối (đếm accept ở server thử = 1).
4. **Mặc định an toàn bắt buộc**: timeout kết nối và đọc, giới hạn số lần chuyển hướng, giới hạn kích thước body. **Số đề xuất (Agent 1) — chờ A0 chốt**:

   | Tham số | Đề xuất | Ghi chú |
   |---|---|---|
   | `timeout_connect` | **10 s** | hết ⇒ lỗi rõ ràng |
   | `timeout_read` | **30 s** | hết ⇒ lỗi rõ ràng |
   | `max_redirect` | **5** | chỉ GET (giữ như hiện tại) |
   | `max_body` | **8 MiB** | vượt ⇒ lỗi rõ, **không** cắt im lặng |

   > **Trạng thái số**: **đề xuất, chờ A0 chốt** khi duyệt ST-178 (VD-178 QĐ-3 mục 2). Không dùng số này để “đạt” trước khi A0 chốt.
   > **đã sửa**: `agent/JOURNAL/vs3-qd-3.md:14` (ST-178) — thêm mục 4 (mặc định an toàn) với số đề xuất.
5. Cổng có bước kiểm **licenses + advisories** (`cargo-deny`), không mạng ⇒ dùng cache.

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

- **Tuỳ chọn `ca:`**: Agent 1 xác minh `rustls-platform-verifier` có hỗ trợ **thêm root** hay không. Nếu **không**, cấu hình có `ca:` dùng **`rustls` + `rustls-webpki` với root riêng** — ghi rõ nhánh này trong bảng phụ thuộc khi làm 3a.
- **`ring` trên Windows**: kiểm khi làm 3a (cần NASM?), ghi kết quả vào bảng này.
- **HTTP server có TLS**: **ngoài phạm vi VS3**, ghi nợ (cần cho mục tiêu “làm server”).

Quy tắc: ngoài danh sách giấy phép cho phép (MIT/Apache-2.0/ISC/BSD-2/3/Unicode-3.0/Zlib; CDLA chỉ cho dữ liệu chứng chỉ) ⇒ **BLOCKED-CHO-A0**. Crate của ta vẫn `#![forbid(unsafe_code)]`.

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-3a | Client TLS + platform verifier + **tuỳ chọn `ca:`** + test server tự ký | test offline 200/body với `ca:`; cert lạ **không** `ca:` ⇒ lỗi rõ; **không** có tham số insecure | QĐ-1 |
| VS3-3b | Keep-alive HTTP/1.1 | 2 request = 1 accept | 3a |
| VS3-3c | `cargo-deny` licenses+advisories trong cổng (cache offline) | cổng chạy không mạng; tài liệu cài trên mac/ps/psvn | 3a |
| VS3-3d | Sandbox `net` cap + tài liệu + **mặc định an toàn** | cap thiếu ⇒ `VS0604`; timeout/redirect/body có ca; doc cập nhật “Net có TLS” | 3b |

## 5. Ca đỏ-trước
- HTTPS tới server tự ký **trong fixtures** + `http.client(ca: p"fixtures/ca.pem")` ⇒ 200 (trước: lỗi “không hỗ trợ TLS”).
- Cert tự ký **không** trong kho hệ thống và **không** khai `ca:` ⇒ lỗi xác minh.
- 2 request ⇒ 1 accept.
- `http.client(insecure: true)` (hoặc tương đương) ⇒ **không tồn tại** (không có tham số / lỗi biên dịch).
- Body vượt `max_body` ⇒ lỗi rõ (không cắt im lặng).

## 6. Cổng
`gate --fast` + bước `cargo-deny` (licenses/advisories) + test offline (không mạng).

## 7. Rủi ro
Kho chứng chỉ HĐH khác nhau (mac/ps/psvn); proxy; wasm roots; `ring` build trên Windows (cần NASM? — phải kiểm khi 3a); platform-verifier có thể **không** hỗ trợ thêm root ⇒ dùng nhánh `rustls`+`webpki` cho `ca:`.

## 8. Bảng [SỬA] VD-178 đã áp (ST-178)
| [SỬA] VD-178 | Nội dung | đã sửa |
|---|---|---|
| QĐ-3 mục 1 | Thêm `http.client(ca: p"…")`; mặc định kho HĐH; **không** insecure; nhánh dự phòng rustls+webpki | `agent/JOURNAL/vs3-qd-3.md:6` |
| QĐ-3 mục 2 (áp dụng) | Mặc định an toàn: timeout/redirect/body — **số đề xuất, chờ A0 chốt** | `agent/JOURNAL/vs3-qd-3.md:14` |
