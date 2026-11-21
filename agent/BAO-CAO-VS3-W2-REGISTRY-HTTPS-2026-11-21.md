<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — registry HTTPS cho `vis pkg` (2026-11-21)

> **Baseline**: `main` `2d6ca2b` (fast-forward từ `b7385e9`, không conflict).
> **Phạm vi**: mở `https://` cho `vis pkg`; **không** proxy/cookie/HTTP2; `http://` vẫn loopback.

## 1. Thay đổi

| Tệp | Nội dung |
|---|---|
| `crates/vis-cli/src/pkg.rs` | `--registry` nhận `https://host[:port][/path]`; usage/doc/error cập nhật; `--registry-key` áp dụng cho HTTP(S) |
| `crates/vis-cli/src/pkg/registry_http.rs` | doc: TLS có sẵn; base HTTPS dùng `http_get`/`Net` (root store, deadline, keep-alive, cap[Net]) |
| `crates/vis-cli/src/net/tls.rs` | hook **debug-only** `VIS_TLS_TEST_ROOT_DER` để test hermetic; release không biên dịch |
| `crates/vis-cli/tests/vs3_registry_https.rs` | 6 ca registry TLS local |
| `crates/vis-cli/tests/vs3_registry.rs` | ca 8 đổi từ "https bị từ chối" → "https được nhận, lỗi nối TLS rõ" |
| `crates/vis-cli/Cargo.toml` | khai báo `[[test]] vs3_registry_https` required-features `tls` |

Luồng không đổi: tải `index.json` → resolver semver → chỉ tải tarball được chọn → kiểm SHA-256
tarball/từng tệp → kiểm chữ ký Ed25519 (nếu cấu hình khoá) **trước** khi tải tarball → ghi `.vis`.
`Net` của registry dùng đúng TLS/root store/deadline/keep-alive của `http.get`; `cap[Net]` vẫn được
`NetGuard` kiểm (khi chạy qua host có quyền).

## 2. Test root cho bản debug (lý do + giới hạn)

`vis pkg` là **binary**, integration test phải chạy tiến trình con; không thể gọi
`Net::moi_voi_goc_tls` từ ngoài. Vì vậy:

- `net/tls.rs::mac_dinh` có `#[cfg(debug_assertions)]` đọc **một** biến `VIS_TLS_TEST_ROOT_DER`
  (đường dẫn chứng chỉ DER) và dùng nó làm root store duy nhất.
- **Release không biên dịch** hook này; không có cờ CLI, không có biến môi trường trong artefact
  phát hành. Bản debug chỉ dùng cho test; tên biến nói rõ mục đích.
- Test thành công tự bỏ qua ở `--release` (ghi chú rõ), không im lặng coi là xanh.

## 3. Test hermetic (`crates/vis-cli/tests/vs3_registry_https.rs`)

Server rustls local + fixture `tests/fixtures/vs3_tls/localhost.*` (đã có từ VS3-4a; SAN IPv4/IPv6);
index + tarball dựng tại chỗ, không Internet.

| Ca | Nội dung |
|---|---|
| `add_qua_https_roi_cai_dung` | `vis pkg add util --registry https://127.0.0.1:PORT` + root test ⇒ exit 0, cài đúng `src/util.vis`, log có **GET /index.json** và **GET /util/0.1.0.tar.b64** |
| `add_https_co_chu_ky` | như trên + `--registry-key` (Ed25519 sinh trong test, ký bằng `pkg_sig::thong_diep_ky`) ⇒ cài đúng |
| `https_sai_cert_loi_ro` | server dùng cert khác root test ⇒ exit 2, lỗi `TLS`/`UnknownIssuer`, **không** tạo `.vis` |
| `https_khong_root_test_loi_ro` | không đặt root test ⇒ cert tự ký bị `webpki-roots` từ chối, không ghi |
| `https_sai_san_van_tu_choi` | root test tin cert nhưng URL `127.0.0.1` sai SAN (`DNS:khong-dung.example`) ⇒ vẫn từ chối |
| `https_duoc_nhan_loi_noi_ro` | `https://127.0.0.1:1` ⇒ lỗi nối TLS rõ, **không** còn "chưa hỗ trợ" |

## 4. Lệnh + kết quả

```text
cargo test -p vis-cli --test vs3_registry_https   6 passed / 0 failed
cargo test -p vis-cli --test vs3_registry        11 passed / 0 failed
cargo test -p vis-cli --test vs3_registry_sig     9 passed / 0 failed
cargo test -p vis-cli --test vs3_tls             12 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive        8 passed / 0 failed
cargo test -p vis-cli --test http                22 passed / 0 failed
cargo test -p vis-cli --bin vis net_quyen::tests  3 passed / 0 failed
cargo test -p vis-cli --no-default-features --test vs3_registry
                                                 11 passed / 0 failed
cargo check -p vis-cli (default/core/wasm-core/win-core) -> Finished
cargo check -p vis-cli --release                   -> Finished (hook test không biên dịch)
cargo clippy ... -- -D warnings                  -> Finished
./vis-dev rules                                  -> OK
./bin/vis-targets                                core 18/18; sqlite 16 + 2 EXPECTED_RED; exit 0
```

Bằng chứng thủ công có Internet (không phải test tự động): `vis pkg add util --registry
https://example.com` ⇒ tải index qua TLS thật rồi nhận `HTTP 404` (root store công khai hoạt động;
không có `.vis` được ghi).

## 5. Còn lại

- `http://` vẫn chỉ loopback (đúng VD-100/105); `https://` host bất kỳ. `vis pkg` là lệnh **CLI**,
  không có `Quyen` của chương trình, nên không có `cap[Net]` để kiểm; nó dùng đúng policy/`Net` của
  `http_get` (không mở đường mạng mới). Khi chương trình dùng `http.get`, `NetGuard` vẫn kiểm
  `cap[Net]` như VS3-4b (kể cả lấy kết nối từ pool).
- Không proxy/cookie/HTTP2/redirect đặc biệt ngoài hành vi `http_get` hiện có.
- Hook root chỉ **debug**; CI release muốn test TLS registry cần cert công khai hoặc cơ chế khác
  (đề xuất giữ nguyên, không đưa hook vào release).
- Chữ ký trong test dùng khoá ephemeral; không có khoá riêng nào lưu trong repo.
- Không đụng `frame/**`; W3 sở hữu `pkg.rs` đã xong, thay đổi ở đây chỉ là nhận diện scheme.
