# ST-173 — Landlock chỉ Linux (VD-175 §2)

Ngày: 2026-11-17 (JST) · Agent 2 dòng ViScript · Máy đo: `mac` (mps, macOS 27.0.1 arm64) và `ps`
(Ubuntu 26.04.1, kernel `7.0.0-43-generic`, LSM có `landlock`) · Tệp sửa: `vis-host-linux/Cargo.toml`,
`viscript/agent/HOST-BAO-DAM.md` (không sửa `tests/landlock.rs` — xem §1).

## 0. Kết luận một dòng

`landlock =0.4.7` đã nằm trong `[target.'cfg(target_os = "linux")'.dependencies]`; **mac** hết đỏ biên
dịch (`101` → `0`, 0 ca), **số ca Landlock trên `ps` KHÔNG đổi (15 → 15)**; cổng `--fast` **PASS 10/10**
trên cả mac (L0 **1182/0**) và `ps` (L0 **1186/0**). Không đổi kỳ vọng/ngưỡng, không `#[ignore]`,
không bỏ ca, không crate mới.

## 1. Thay đổi

- `vis-host-linux/Cargo.toml`: `landlock = "=0.4.7"` chuyển từ `[dependencies]` sang
  `[target.'cfg(target_os = "linux")'.dependencies]` (kèm chú thích ST-173). Đây là **thay đổi nguồn
  duy nhất** ngoài tài liệu.
- `vis-host-linux/tests/landlock.rs`: **đã có** `#![cfg(target_os = "linux")]` (dòng 12, thêm từ
  `f9ced6c6` ST-132) ⇒ **không có diff**; `git log -S` xác nhận. `src/lib.rs` cũng đã có từ ST-132.
  Trên mac/Windows tệp test là crate **rỗng** ⇒ 0 ca, không lỗi biên dịch.

## 2. macOS — trước/sau (`cargo test -p vis-host-linux`, toàn crate)

| Lúc | Lệnh | Kết quả | Log |
|---|---|---|---|
| Trước | `cd vis-host-linux && cargo test -p vis-host-linux` | **`[exit=101]`** — 6 lỗi biên dịch trong crate `landlock` (`libc::prctl`, `O_PATH`, `SYS_landlock_*`, `__u32`) | `JOURNAL/st173-mac-truoc.log` |
| Sau | như trên | **`[exit=0]`** — `Compiling vis-host-linux` (không kéo `landlock`), **0 ca** ở lib/test/doc-test | `JOURNAL/st173-mac-sau.log` |

## 3. `ps` — 2 số ca Landlock (`cargo test -p vis-host-linux --test landlock`)

| Lúc | Số ca | Log |
|---|---|---|
| Trước | **15 passed; 0 failed** (`[exit=0]`) | `JOURNAL/st173-ps-truoc.log` |
| Sau | **15 passed; 0 failed** (`[exit=0]`) | `JOURNAL/st173-ps-sau.log` |

- Trước: `Cargo.toml` sha256 `1d3d5193…` (bản gốc), HEAD `795df046`.
- Sau: `Cargo.toml` sha256 `32e93b6d…` (bản đã sửa), **xoá artifact cũ** (`cargo clean -p
  vis-host-linux`) rồi mới chạy ⇒ log có `Compiling landlock` + `Compiling vis-host-linux`, chứng minh
  cấu hình mới **biên dịch và chạy đủ 15 ca** trên Linux (không dựa binary cũ).
- Cây `ps` chỉ được sửa **tạm** một tệp cho phép đo rồi **khôi phục nguyên trạng** (sha256 `1d3d5193…`,
  chủ `501:50`, mtime giữ nguyên); `git status -- vis-host-linux` chỉ còn `?? target/` (có sẵn).
- Bối cảnh: cây `ps` đang bẩn **2152 mục** (nợ **ST-175**, không do task này); HEAD `ps` chậm hơn mac
  nhưng **mã `viscript/` không đổi** giữa `795df046..712178bc` (chỉ `viscript/agent/`).

## 4. Cổng `./vis-dev gate --fast`

| Máy | Kết quả | L0 | Log |
|---|---|---|---|
| mac (HEAD `712178bc`) | **PASS 10/10** (`--- L0 test: OK`, `[exit=0]`) | **1182 passed / 0 failed** | `JOURNAL/st173-gate-macos.log` |
| `ps` (HEAD `795df046`) | **PASS 10/10** (`[exit=0]`) | **1186 passed / 0 failed** | `JOURNAL/st173-gate-ps.log` |

10 bước: fmt · clippy · luật R1..R4 · self-test case âm · `vis explain` · báo cáo R* (report-check)
· fixture âm báo cáo · build · doc-examples · L0 test. Lần chạy mac **không retry**.

## 5. Giới hạn nền (đã ghi `HOST-BAO-DAM.md`)

Landlock **chỉ Linux**; mac/Windows **không chạy ca Landlock** (0 ca), hành vi bảo vệ **chỉ đo trên
Linux**. Cổng trên mac **không** phát hiện hồi quy Landlock ⇒ phải chạy suite trên `ps`. Trên Linux
dependency vẫn **bắt buộc** (thiếu `landlock` ⇒ build đỏ), không có đường âm thầm bỏ bảo vệ.

## 6. Tệp thay đổi/thêm (chưa commit — Agent 1 commit)

- Sửa: `vis-host-linux/Cargo.toml`, `viscript/agent/HOST-BAO-DAM.md`
- Thêm: `viscript/agent/JOURNAL/st173-mac-truoc.log`, `st173-mac-sau.log`, `st173-ps-truoc.log`,
  `st173-ps-sau.log`, `st173-gate-macos.log`, `st173-gate-ps.log`, `st173-ghi-chu.md`
