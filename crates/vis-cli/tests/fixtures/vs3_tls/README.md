<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Chứng chỉ TLS cho ca test VS3-4a

Sinh một lần bằng OpenSSL (không phải dependency lúc build/chạy):

```bash
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -sha256 \
  -not_before 20200101000000Z -not_after 21260101000000Z -nodes \
  -keyout localhost.key.pem -out localhost.cert.pem -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=serverAuth"
openssl x509 -in localhost.cert.pem -outform DER -out localhost.cert.der
openssl pkcs8 -topk8 -nocrypt -in localhost.key.pem -outform DER -out localhost.key.der
```

- `localhost.*` — CN `localhost`, SAN `DNS:localhost,IP:127.0.0.1`; dùng cho ca thành công.
- `sai-ten.*` — chỉ SAN `DNS:khong-dung.example`; dùng cho ca **sai tên** (kết nối bằng
  `127.0.0.1` phải bị từ chối dù gốc đã được tin).
- Chỉ **test** nạp hai chứng chỉ này vào kho gốc; binary `vis` luôn dùng `webpki-roots`.
- Khoá riêng chỉ nằm trong test, không có trong artefact phát hành.
