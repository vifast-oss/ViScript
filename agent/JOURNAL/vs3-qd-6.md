<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-6 · Gói: mô hình **phi tập trung kiểu Go** + sổ hash + proxy (viết lại theo VD-180/VD-181)
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · **viết lại theo VD-180 + VD-181** tại **ST-178** · **Phụ thuộc**: QĐ-3 (TLS) · **Mở đường cho**: —

> **Thay thế QĐ-6 mục 1, 2 của VD-178.** Bản cũ (registry tập trung + chữ ký Ed25519 + khoá ký registry) **không còn hiệu lực**.
> **Quyết định không đảo**: tên/đường dẫn gói chốt ở tệp này là **bất biến** sau khi phát hành gói đầu tiên (VD-181 mục 7).

## 1. Mục tiêu đo được
1. **Mô hình phi tập trung kiểu Go, không có kho tập trung ở 1.0**:
   - Tên gói = **đường dẫn**: `github.com/<chủ>/<repo>`, `vifast.org/vis/<tên>`, hoặc domain riêng.
   - `vis.toml`: `<tên ngắn> = "<đường dẫn>@<semver>"`; code chỉ dùng `use <tên ngắn>`.
   - Phiên bản = **tag git semver** (`v1.2.3`); **major ≥ 2 phải có hậu tố `/v2`** trong đường dẫn.
   - Chọn phiên bản: **Minimal Version Selection (MVS)** — **không** có bộ giải ràng buộc.
   - `pkg update X` chỉ đổi **X và phần phụ thuộc bắt buộc của X**, mọi gói khác giữ nguyên; `pkg update` không đối số cập nhật tất cả (giữ **#142**).
2. **`vifast.org/vis/<tên>`**: trang tĩnh trỏ tới nơi chứa mã (**meta tag**), để đổi nơi chứa mà không đổi tên gói. Gói chính thức dùng tiền tố này.
   - `<tên>`: chữ thường `[a-z0-9-]`, **không** `_`, **không** viết hoa. **Không** alias `/x/`; **không** phát hành song song nhiều tiền tố.
   - Tài liệu của ViScript đặt ở `vifast.org/docs/vis/…` để không đụng không gian tên gói.
3. **Tin cậy (thay khoá ký registry)**:
   - `vis.lock` ghi **hash nội dung** (**sha256 của cây tệp chuẩn hoá**) cho mỗi `gói@phiên bản`.
   - **`sum.vifast.org`**: sổ hash công khai, **chỉ ghi thêm**. Giai đoạn đầu là **repo git có commit ký**; `vis` **ghim khoá công khai** của sổ.
   - **Lần đầu tải**: hash phải khớp **sổ** (nếu sổ bật). **Lần sau**: hash phải khớp **`vis.lock`**.
   - **VS0707** = “hash gói không khớp `vis.lock` hoặc sổ”. **Không có lệnh bỏ qua**; chỉ có cấu hình tắt sổ **`VIS_SUMDB=off`** (ghi **cảnh báo**).
   - Gói riêng tư: **`VIS_PRIVATE=<mẫu đường dẫn>`** ⇒ **không** hỏi sổ, **không** qua proxy.
4. **`proxy.vifast.org`**: **thiết kế** giao thức trong tệp này (**GET theo đường dẫn@phiên bản**), **CHƯA vận hành ở VS3**. **`VIS_PROXY=direct`** là mặc định cho tới khi có proxy.
5. **Ca đỏ-trước bắt buộc**: 4 ca ở §5.
6. **Ngoài phạm vi VS3**: proxy chạy thật, `pkg.vifast.org` (tìm kiếm/tài liệu), yank/thu hồi phiên bản.

## 2. Hiện trạng (dẫn chứng)
- `crates/vis-cli/src/pkg.rs` + `crates/vis-cli/src/pkg/resolver.rs`, `semver.rs`.
- `crates/vis-cli/src/modules.rs:5-6` — nạp gói từ `.vis/pkg/` theo `vis.lock`, kiểm **sha** lúc nạp.
- Nợ: **#142** (`pkg update` ghim mọi gói khác), **#143** (NamePolicy::Strict — VS3), **#125** (mạng/chữ ký VS3).
- Hiện tại: registry **cục bộ**, `vis.lock` có SHA-256; **chưa** có tải theo đường dẫn, **chưa** có sổ hash, **chưa** có MVS.

## 3. Bảng host → vai trò (VD-181 mục 5)
| Host | Vai trò |
|---|---|
| `vifast.org` | site + **namespace module** (`/vis/*`); docs ở `/docs/vis/*` |
| `sum.vifast.org` | sổ hash công khai |
| `proxy.vifast.org` | proxy tải gói (**chưa vận hành**) |
| `os.` · `tts.` · `key.vifast.org` | **dịch vụ độc lập**, giữ subdomain; **không** phải nơi chứa gói |

- **Dịch vụ đặt ngang hàng, không lồng dưới `vis.`**: `sum.vifast.org`, `proxy.vifast.org` phục vụ **mọi sản phẩm** ⇒ **không** dùng `sum.vis.vifast.org`/`proxy.vis.vifast.org`.
- **`vis.vifast.org`**: **KHÔNG** dùng làm tiền tố module; nếu cần chỉ là **website** (landing/docs), được phép redirect về `vifast.org/vis`.
- **`viscript.org`** (nếu mua): chỉ là **tài sản thương hiệu**; **KHÔNG** đưa vào đường dẫn gói ở 1.0. Muốn dùng làm tiền tố module phải là quyết định riêng (phiên bản lớn mới), **không** di trú gói cũ.
- **Traefik/reverse proxy KHÔNG phải hợp đồng**: hợp đồng công khai là **nội dung meta tag** trả ở `vifast.org/vis/<tên>` (trỏ nơi chứa mã + tag phiên bản); Traefik chỉ là lớp phục vụ, **được phép thay**. `PathPrefix(/vis/)` → static origin là đủ; **không** cần logic động cho từng gói.
- **Bất biến**: sau khi phát hành gói đầu tiên, các tên/đường dẫn trên **không đổi được** (đổi = phá mọi `use`). **Quyết định không đảo.**

## 4. Thiết kế chốt

### 4.1. Đường dẫn gói, `vis.toml`, phiên bản
```toml
# vis.toml
[dependencies]
json = "vifast.org/vis/json@v1.2.3"
http = "github.com/vifast-oss/vis-http@v0.9.0"
```
- Code chỉ dùng **tên ngắn**: `use json`.
- Phiên bản = **tag git semver**; major ≥ 2 nằm ở **đường dẫn**: `vifast.org/vis/json/v2@v2.0.1`.
- **Không** có trường “registry”; nơi chứa mã do **đường dẫn** quyết định (trực tiếp hoặc qua meta tag).

### 4.2. Chọn phiên bản: MVS
- Mỗi phụ thuộc khai **phiên bản tối thiểu**; `vis` chọn **phiên bản lớn nhất trong các mức tối thiểu** đó (Minimal Version Selection).
- **Không** có bộ giải ràng buộc, **không** có “chọn bản mới nhất”.
- Ví dụ chốt (ca đỏ-trước (b)): hai phụ thuộc cùng cần `X` với `v1.2` và `v1.4` ⇒ chọn **`v1.4`**, **không** chọn `v1.9` dù có.
- Major khác nhau = **đường dẫn khác** (`/v2`) ⇒ không xung đột, cùng tồn tại được.

### 4.3. `vis.lock` + sổ hash `sum.vifast.org`
- `vis.lock` ghi cho mỗi gói@phiên bản: **đường dẫn**, **phiên bản**, **sha256 của cây tệp chuẩn hoá**.
- **Cây tệp chuẩn hoá**: đường dẫn tương đối sắp xếp theo byte, nội dung tệp, **bit thực thi** (chuẩn hoá 0/1), **không** thời gian/mtime, **không** quyền đầy đủ, **không** tệp ngoài danh sách (bỏ `.git`, `target`, tệp bị ignore theo chuẩn gói).
- **Sổ hash**: `sum.vifast.org` chỉ **ghi thêm**; giai đoạn đầu là **repo git có commit ký**; `vis` **ghim khoá công khai** của sổ trong bản phát hành.
- **Kiểm**: lần đầu tải ⇒ hash phải khớp **sổ** (nếu sổ bật); lần sau ⇒ hash phải khớp **`vis.lock`**. Lệch ⇒ **VS0707**, **không** ghi vào `.vis/pkg/`.
- **Không có lệnh bỏ qua**. Chỉ có **`VIS_SUMDB=off`** (tắt hỏi sổ, **ghi cảnh báo** mỗi lần build) — dùng cho môi trường không mạng/không tin sổ, **không** phải đường tắt qua cổng.
- **VS0706** vẫn là “pkg chưa hỗ trợ” (theo R9); **VS0707** = hash không khớp (thay nghĩa “chữ ký sai” của bản cũ).

### 4.4. Gói riêng tư
- **`VIS_PRIVATE=<mẫu đường dẫn>`** (ví dụ `github.com/cong-ty-noi-bo/*`) ⇒ gói khớp mẫu: **không** hỏi sổ, **không** qua proxy; vẫn ghi `vis.lock` và vẫn kiểm hash theo lock.
- Không có cơ chế “bỏ kiểm hash” cho gói riêng tư.

### 4.5. Proxy `proxy.vifast.org` — **chỉ thiết kế, chưa vận hành**
- Mục đích: tăng tốc/cache tải gói; **không** thay đổi mô hình tin cậy (hash vẫn kiểm theo sổ/lock).
- Giao thức (đề xuất, chốt khi làm 6a):
  - `GET https://proxy.vifast.org/<đường_dẫn>@<phiên_bản>` ⇒ `application/x-vis-pkg` (tar.gz **cây chuẩn hoá**), header `X-Vis-Hash: sha256:<hex>`.
  - `GET https://proxy.vifast.org/<đường_dẫn>/@v/list` ⇒ danh sách phiên bản (mỗi dòng một tag).
  - 404/5xx ⇒ **fallback tải trực tiếp** (khi `VIS_PROXY=proxy`).
- **`VIS_PROXY=direct` là mặc định** cho tới khi proxy vận hành; `VIS_PROXY=proxy` là tuỳ chọn.
- Proxy **không** được là nguồn tin cậy duy nhất: hash luôn kiểm theo sổ/lock.

## 5. Ca đỏ-trước bắt buộc (4 ca)
1. **(a) Tag bị đổi nội dung sau khi đã ghi lock** ⇒ **VS0707**, không ghi `.vis/pkg/`.
2. **(b) MVS**: hai phụ thuộc cùng cần gói `X` với `v1.2` và `v1.4` ⇒ chọn **`v1.4`**, **không** chọn bản mới hơn (`v1.9`).
3. **(c) Vanity**: gói `vifast.org/vis/…` (meta tag trỏ GitHub) ⇒ **tải đúng** nơi chứa mã.
4. **(d) Offline + cache**: mạng tắt + đã có cache/`vis.lock` ⇒ **build chạy**.

## 6. Bảng phụ thuộc (Agent 1 xác minh bằng `cargo metadata` khi ghim)
| Crate | Giấy phép (khai) | Lý do | Ghi chú |
|---|---|---|---|
| `sha2` | MIT/Apache-2.0 (**kiểm lại**) | sha256 cây tệp chuẩn hoá | — |
| `ed25519-dalek` | BSD-3-Clause (**kiểm lại**) | xác minh **chữ ký commit của sổ** (nếu sổ dùng khoá ed25519) | có unsafe bên trong |
| (TLS từ QĐ-3) | — | tải gói qua HTTPS | — |

- **`cargo-deny` phải có trước 6a** (giữ VD-178 QĐ-6 mục 4).
- Ngoài danh sách giấy phép cho phép ⇒ **BLOCKED-CHO-A0**.

## 7. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-6a | Client tải gói **theo đường dẫn** (HTTPS) + **meta tag** + cache `.vis/pkg/` | tải gói từ fixtures offline; ghi `.vis/pkg/`; ca (c) | QĐ-3 |
| VS3-6b | `vis.lock` **sha256 cây chuẩn hoá** + **MVS** | lock round-trip; ca (b) chọn `v1.4` | 6a |
| VS3-6c | Sổ hash (chế độ tệp cục bộ trong fixtures) + **VS0707** + `VIS_SUMDB=off` + `VIS_PRIVATE` | ca (a), (d); cảnh báo khi tắt sổ | 6b |
| VS3-6d | **#142** (`pkg update X`) + `pkg gc` + tài liệu | Agent 0 chốt chính sách; doc cập nhật | 6b |

## 8. Cổng
`gate --fast` + test offline (fixtures, không mạng) + `cargo-deny` licenses/advisories.

## 9. Rủi ro
(1) Sổ hash bị tấn công ⇒ khoá ghim + chỉ ghi thêm + commit ký; (2) MVS không giải xung đột “cần bản mới hơn” ⇒ đúng thiết kế, người dùng tự nâng tối thiểu; (3) vanity domain chết ⇒ cache + `vis.lock` giúp build offline; (4) proxy chưa có ⇒ `VIS_PROXY=direct`; (5) `VIS_SUMDB=off` bị lạm dụng ⇒ **ghi cảnh báo** mỗi lần build, không có cờ im lặng.

## 10. Bảng [SỬA] VD-178/VD-180/VD-181 đã áp (ST-178)
| Quyết định | Nội dung | đã sửa |
|---|---|---|
| VD-178 QĐ-6 mục 1 (bị thay) | Mô hình **phi tập trung kiểu Go**, không kho tập trung; tên = đường dẫn; tag semver; `/v2`; **MVS**; `pkg update X` (#142) | `agent/JOURNAL/vs3-qd-6.md:@@L1@@` |
| VD-178 QĐ-6 mục 2 (bị thay) | **VS0707** = hash không khớp `vis.lock`/sổ (thay “chữ ký sai”); **VS0706** giữ “pkg chưa hỗ trợ” | `agent/JOURNAL/vs3-qd-6.md:@@L2@@` |
| VD-180 mục 3 | `vis.lock` sha256 cây chuẩn hoá; `sum.vifast.org` chỉ ghi thêm; `VIS_SUMDB=off`; `VIS_PRIVATE` | `agent/JOURNAL/vs3-qd-6.md:@@L3@@` |
| VD-180 mục 4 | `proxy.vifast.org` **chỉ thiết kế**; `VIS_PROXY=direct` mặc định | `agent/JOURNAL/vs3-qd-6.md:@@L4@@` |
| VD-180 mục 5 | **4 ca đỏ-trước bắt buộc** | `agent/JOURNAL/vs3-qd-6.md:@@L5@@` |
| VD-180 mục 6 | Ngoài phạm vi VS3 | `agent/JOURNAL/vs3-qd-6.md:@@L6@@` |
| VD-181 mục 1 | `vifast.org/vis/<tên>` (path, không subdomain); `<tên>` `[a-z0-9-]`; không `/x/` | `agent/JOURNAL/vs3-qd-6.md:@@L7@@` |
| VD-181 mục 2 | `sum.`/`proxy.vifast.org` ngang hàng, không lồng dưới `vis.` | `agent/JOURNAL/vs3-qd-6.md:@@L8@@` |
| VD-181 mục 3 | `vis.vifast.org` **không** làm tiền tố module | `agent/JOURNAL/vs3-qd-6.md:@@L9@@` |
| VD-181 mục 4 | `viscript.org` chỉ thương hiệu | `agent/JOURNAL/vs3-qd-6.md:@@L10@@` |
| VD-181 mục 5 | **Bảng host → vai trò** | `agent/JOURNAL/vs3-qd-6.md:@@L11@@` |
| VD-181 mục 6 | **Traefik không phải hợp đồng**; meta tag là hợp đồng | `agent/JOURNAL/vs3-qd-6.md:@@L12@@` |
| VD-181 mục 7 | Tên **bất biến**; **quyết định không đảo** | `agent/JOURNAL/vs3-qd-6.md:@@L13@@` |
