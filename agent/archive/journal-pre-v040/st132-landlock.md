# ST-132 · Landlock thật — cưỡng chế bằng kernel (2026-11-16)

Người làm: lead (đợt 23). Căn cứ: `JOURNAL/quyet-dinh-a0-2026-11-16.md` §6 **ST-132** + **VD-120**,
điều kiện đóng **Q1** của Agent 0. Runner: `bin/vis-linux` trên host `ps`
(Ubuntu 26.04.1, kernel **7.0.0-43-generic** x86_64, Landlock **ABI = 8**).

## 0. Kết luận một dòng

Landlock **cưỡng chế thật** trên hệ tệp: **12/12 ca xanh** trên runner Linux, trong đó **2 ca âm ở
tiến trình con** chứng minh **kernel** chặn (không phải `FsGuard`); macOS `--sandbox` trả **exit 3**
và ghi rõ "chỉ hỗ trợ trên Linux". Còn **một điểm cần Agent 0 quyết**: giấy phép của
`unicode-ident` (phụ thuộc build-time) không nằm đúng allowlist hẹp (xem §5).

## 1. Đã làm

| Tệp | Việc |
|---|---|
| `vis-host-linux/Cargo.toml` | thêm **một** phụ thuộc `landlock = "=0.4.7"` (ghim) — duyệt có điều kiện ở VD-120 |
| `vis-host-linux/src/sandbox.rs` (mới) | `ap_dung(&[ChoPhep]) -> Result<KetQua, LoiSandbox>`: dựng ruleset `ABI::V1`, `restrict_self()`, trả mức cưỡng chế + số luật + `no_new_privs` |
| `vis-host-linux/src/lib.rs` | `pub mod sandbox;` + cập nhật giới hạn bằng chứng (không còn là "khung") |
| `vis-host-linux/tests/landlock.rs` (mới) | **12 ca** (xem §2) |
| `viscript/crates/vis-cli/Cargo.toml` | `[target.'cfg(target_os = "linux")'.dependencies] vis-host-linux = { path = "../../../vis-host-linux" }` |
| `viscript/crates/vis-cli/src/cli/run.rs` | `ap_sandbox(&Quyen)`: Linux ⇒ áp ruleset từ `cap[Dir]`/`cap[File]`; nền khác ⇒ exit **3** |
| `viscript/crates/vis-cli/src/main.rs` | cờ `--sandbox`, áp **sau** khi nạp chương trình và **trước** khi chạy; tắt cache khi bật sandbox |

Ngữ nghĩa chốt (đúng VD-120): `--sandbox` là **opt-in**; chỉ **hệ tệp**; `cap[Net]`/`cap[Proc]` vẫn
cưỡng chế ở **runtime**; nền không hỗ trợ ⇒ **exit 3** như `--sandbox-check`; **không** `unsafe`
trong code của ta.

## 2. Mười hai ca (đo trên runner)

| # | Ca | Kết quả |
|---|---|---|
| 1 | `doc_trong_thu_muc_duoc_phep` | ok |
| 2 | `doc_ngoai_bi_kernel_chan` | ok — `PermissionDenied` |
| 3 | `ghi_trong_thu_muc_duoc_phep` | ok |
| 4 | `ghi_ngoai_bi_chan` | ok — `PermissionDenied` |
| 5 | `ghi_trong_thu_muc_chi_doc_bi_chan` | ok — `PermissionDenied` |
| 6 | `luat_cho_mot_tep_khong_mo_ca_thu_muc` | ok — luật 1 tệp không mở thư mục chứa |
| 7 | `khong_co_luat_nao_thi_chan_het` | ok — deny-by-default |
| 8 | `nhieu_thu_muc_duoc_cap` | ok — 2 luật, thư mục thứ ba bị chặn |
| 9 | `duong_dan_khong_ton_tai_la_loi_he_thong` | ok — `LoiSandbox::HeThong` |
| 10 | **`con_doc_ngoai_pham_vi_bi_kernel_chan`** | ok — tiến trình con **tĩnh** `exec` được, đọc tệp ngoài ⇒ `EACCES` |
| 11 | **`con_ghi_ngoai_pham_vi_bi_kernel_chan`** | ok — con ghi ngoài ⇒ `EACCES`, tệp **không** được tạo |
| 12 | `con_doc_trong_pham_vi_chay_duoc` | ok — đối chứng: cùng domain, đọc trong phạm vi chạy được |

`cargo test` (vis-host-linux, trên runner): **12 passed; 0 failed** · `finished in 0.13s`.

Bằng chứng "kernel chứ không phải runtime": các ca 10–12 chạy một helper **tĩnh** (`crt-static`) do
`rustc` dựng ngay trong ca; helper **không** đi qua `FsGuard` nào — nó gọi thẳng `std::fs`. Bị
`EACCES` ⇒ **kernel** chặn. Đây là điều VD-120 đòi ("tiến trình con bỏ qua FsGuard vẫn bị EACCES").

## 3. Đo trên hai nền

| Nền | `--sandbox` | Ghi chú |
|---|---|---|
| Linux (runner `ps`) | **chạy thật**: `sandbox: landlock ABI=1 mức=đầy đủ luật=0 nnp=true`, chương trình chạy, exit 0 | đo được |
| macOS (máy dev) | **exit 3**, `sandbox: chỉ hỗ trợ trên Linux (nền này: macos) (exit 3)` | **chưa đo** khả năng cưỡng chế — macOS không có Landlock; ghi đúng chữ "chưa đo", không hứa |

## 4. Phát hiện phải ghi lại: `execve` trong domain cần quyền trên **interpreter**

Lần đầu, hai ca tiến trình con **đỏ** với `EACCES` **ngay khi spawn** `/bin/cat` và `/bin/sh`. Đo
bốn biến thể để tìm nguyên nhân (đều `EACCES`):

1. không xử lý `EXECUTE` trong `handle_access`;
2. xử lý `EXECUTE` + cấp `EXECUTE` trên thư mục chứa;
3. cấp `EXECUTE` trên `/`;
4. chạy bản sao tệp trong thư mục được cấp.

Nguyên nhân thật: `execve` một binary **động** cần quyền `EXECUTE` trên **ELF interpreter**
(`/lib64/ld-linux-…`) — thứ nằm ngoài `cap`. Vì vậy ca kiểm dùng helper **tĩnh**: không có
interpreter ⇒ `execve` chỉ cần quyền trên chính tệp.

**Hệ quả cho sản phẩm (ghi để không nói quá)**: trong `--sandbox`, chương trình **chỉ** chạy được
tệp thực thi nằm trong `cap` (`cap[Dir]`) **và** tĩnh, hoặc phải cấp thêm `EXECUTE` cho thư viện
động. VS2 **không** làm việc cấp quyền cho `/lib*`; nếu Agent 0 muốn `.vish` chạy lệnh ngoài dưới
`--sandbox` thì cần một quyết định mới (mở `cap[Proc]` thành luật `Execute`, kèm rủi ro).

## 5. Điểm cần Agent 0 quyết: giấy phép `unicode-ident`

`cargo metadata` trên runner (log đầy đủ: `JOURNAL/st132-license.log`) — **11 gói**, 10 gói đúng
`MIT OR Apache-2.0`; **một** gói khác:

- `unicode-ident 1.0.27` — `(MIT OR Apache-2.0) AND Unicode-3.0`.
- Là phụ thuộc **bắc cầu của `proc-macro2`** (⇒ `syn`, `thiserror-impl`, `enumflags2_derive`), tức
  **chỉ dùng lúc build**, **không** liên kết vào binary phát hành.

Theo **đúng chữ** của VD-120 ("một phụ thuộc ngoài allowlist thì BLOCKED") thì đây là vi phạm ⇒ lẽ
ra phải BLOCKED. Tôi **không** tự quyết thay Agent 0: hàng QUEUE ST-132 để **DONE + CHỜ DUYỆT** kèm
ghi chú này. Ba lựa chọn để Agent 0 chốt:

(a) chấp nhận: thêm `Unicode-3.0` (chỉ cho phụ thuộc **build-time**) vào allowlist D2-21;
(b) từ chối: ST-132 phải bỏ `landlock` crate ⇒ quay lại nhánh (b) của VD-107 (4) là `unsafe` đóng
    trong `vis-host-linux` — cần một quyết định riêng vì D2-17 cấm `unsafe` ngoài danh sách;
(c) hoãn: giữ code + ca kiểm trong nhánh, không hợp nhất cho tới khi allowlist đổi.

## 6. Lệnh tái lập

```bash
# trên runner Linux (đã đồng bộ viscript/ và vis-host-linux/)
cd vis-host-linux && cargo test                       # 12 passed
cargo metadata --format-version 1 | python3 -c '...'   # danh sách giấy phép
# trên máy dev (macOS)
cd viscript && ./target/debug/vis run --sandbox /tmp/sb.vis   # exit 3
```

## 7. Nợ mới (đánh số tiếp từ 105)

- **105** — `--sandbox` chưa cấp `EXECUTE` cho `/lib*` ⇒ chương trình có lệnh ngoài không chạy được
  dưới sandbox (xem §4). Cần quyết định của Agent 0.
- **106** — `unicode-ident` ngoài allowlist hẹp (xem §5).
- **107** — `ap_dung` mới dùng `ABI::V1`; kernel ABI 8 có thêm quyền (`REFER`, `TRUNCATE`, `IOCTL_DEV`)
  chưa xét — VS3.
