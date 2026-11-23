# ST-295 — Đóng khe TOCTOU đường ghi Unix + Landlock `Execute` (`#109b`/`#140`/`#125`)

**Nhánh**: `plan/w3b-budget` @ `a48d5b8` (fast-forward sau merge ST-290). **Ngày**: 2026-11-22.
**A0**: VD-276 (phương án A `rustix`; Landlock `#125`). **Trạng thái**: mã + test xong, chưa push/merge/tag.

## 0. Tóm tắt

1. Thêm **`rustix =1.1.6`** (feature tối giản `fs` + `std`) vào
   `[target.'cfg(unix)'.dependencies]` của **`vis-cli`**; Windows/wasm **không** kéo crate.
   Kiểm giấy phép toàn cây: `agent/JOURNAL/st295-license.log` — chỉ **3 gói mới**, cả ba trong
   allowlist ⇒ **KHÔNG BLOCKED**.
2. **Đường đọc + ghi dùng chung `openat`**: module mới `crates/vis-cli/src/quyen_mo.rs`.
   Linux ≥ 5.6 ưu tiên `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)`; `ENOSYS`/nền khác rơi về
   đi **từng thành phần** với `O_NOFOLLOW|O_DIRECTORY`. Gốc `cap` mở bằng `O_NOFOLLOW` (chống
   thay gốc). Áp cho `read`/`read_bytes`/`write`/`append`/`write_bytes`/`mkdir`/`mkdir_all`/
   `remove`/`copy`/`rename`.
3. **Ca đua tất định 100/100 → 0/100**: `quyen_st295.rs` (điểm chèn chỉ test giữa bước kiểm và
   bước mở). Nhánh cũ (kiểm tên → `std::fs::write`) rò **100/100**; nhánh mới **0/100** —
   `agent/JOURNAL/st295-race.log`.
4. **Landlock `#125`**: ruleset **xử lý `Execute`**; khi `--sandbox` + `cap[Proc]` + `--grant`, cấp
   chỉ-đọc + `Execute` cho thư viện chuẩn (`/lib`, `/lib64`, `/usr/lib`, `/usr/lib64`, `/usr/bin`,
   `/bin`, `/sbin`, `/usr/sbin` nếu tồn tại) và **đúng binary canonicalize**. `vis-host-linux`
   **18/18** (3 ca mới: binary trong grant chạy, binary ngoài grant bị chặn, luật lib chỉ-đọc chặn
   ghi). CLI: `st295_sandbox_proc.rs` 3/3 (chạy `test-child` trong grant; `/usr/bin/true` ngoài grant
   vẫn `VS0604`; ghi `/usr/lib` không thành công).
5. **Xoá cảnh báo** “đường ghi còn khe hở kiểm-rồi-mở” trong `agent/HOST-BAO-DAM.md` **cùng commit**
   (điều kiện: ca đua xanh).

## 1. Giấy phép (`st295-license.log`)

- `cargo metadata --format-version 1`; duyệt đồ thị resolve từ `vis-cli` (**89 gói**).
- So `Cargo.lock` với `a48d5b8`: chỉ thêm `rustix 1.1.6`, `linux-raw-sys 0.12.1`, `errno 0.3.15`.
- Cả ba: `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` / `MIT OR Apache-2.0` ⇒ chọn được
  nhánh **Apache-2.0/MIT**, nằm trong allowlist A0.
- Các ngoại lệ NGOÀI allowlist trong cây (`ring` ISC, `webpki-roots` CDLA, `subtle` BSD-3,
  `untrusted` ISC, `unicode-ident` Unicode-3.0) **đã có trước** ST-295 và được duyệt ở VD-271/272;
  ST-295 không đổi phiên bản/nhánh nào của chúng.

## 2. `openat` — vì sao đóng được khe

`FsGuard` cũ: `kiem_fs` → `canonicalize` (syscall 1) → `std::fs::write` (syscall 2). Giữa hai
syscall, thư mục cha bị đổi ⇒ tên trỏ symlink ra ngoài. Bản vá:

- `openat2` với `RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS` mở **một** đường dẫn tương đối từ fd gốc;
  kernel từ chối mọi symlink và mọi `..` thoát gốc. `O_NOFOLLOW` nhắc lại cho final component.
- Fallback: mở lần lượt từng thành phần bằng `openat(O_DIRECTORY|O_NOFOLLOW)` rồi final bằng
  `O_NOFOLLOW`; `..` đã bị chuẩn hoá **từ vựng** trước và `strip_prefix(root)` loại thoát gốc.
- Không dùng `canonicalize` trong quyết định quyền ⇒ không còn “kiểm theo tên rồi mở theo tên”.
- Giữ nguyên thông điệp `[quyền]` cũ cho ngoài root/thiếu `(write)`; symlink/thoát gốc khi mở trả
  `VS0604` với câu “chứa symlink hoặc thoát khỏi gốc `cap` (ST-295)”.

**Bằng chứng đỏ-trước**: `quyen_st295::cu_100_tren_100_ro_ra_ngoai` (nhánh cũ) = **100/100**;
`moi_0_tren_100_ro_ra_ngoai` = **0/100**; đối chứng (không hoán đổi) ghi đúng trong root.

## 3. Landlock `#125`

- Bản cũ **bỏ `Execute` khỏi `handle_access`** nên mọi `execve` trong domain `EACCES` (ghi ở
  `tests/landlock.rs` §4). Nay ruleset xử lý `Execute`; mọi luật read-only cấp
  `ReadFile|ReadDir|Execute`, tệp `ReadFile|Execute`, thư mục ghi `toan_bo`.
- `cli::run::ap_sandbox`: khi `cap[Proc]` có `--grant`, thêm luật **chỉ đọc** cho thư viện chuẩn +
  **đúng binary** trong allowlist (canonicalize, dedup). Quyền chạy **lệnh nào** vẫn do runtime
  `cap[Proc]` quyết định ⇒ binary ngoài grant vẫn `VS0604`.
- `vis-host-linux` **18/18** ca (`cargo test --test landlock`), gồm 3 ca mới:
  - `con_binary_trong_grant_chay_duoc_voi_execute`
  - `con_binary_ngoai_grant_bi_chan` (execve ⇒ `PermissionDenied`)
  - `luat_thu_vien_chi_doc_chan_ghi` (lib giả 0777 vẫn bị `AccessFs::WriteFile` chặn; `/lib` thật
    không tạo được tệp)
- CLI `st295_sandbox_proc`: 3/3.

## 4. Giới hạn còn lại (ghi trung thực)

- **Windows**: không có `openat`; `FsGuard` vẫn kiểm theo đường dẫn ⇒ mức bảo đảm **thấp hơn**
  (giữ nguyên tuyên bố Tier 2, ST-294/297 mới đóng đường dẫn).
- **macOS**: dùng cùng mã `openat`/`rustix` nhưng **chưa verify** lại trên máy Darwin thật (đính
  chính đầu `HOST-BAO-DAM.md`).
- **Hardlink**: vẫn là **giới hạn chính sách** (cây tên), không phải lỗi đua; không đổi.
- **`rename`**: dùng `RENAME_NOREPLACE` (Linux/apple) để không còn khe kiểm-đích-rồi-rename; nền
  khác giữ `statat` + `renameat` với hai fd cha cố định.
- **SO-NO.md**: mục `#109b`/`#140` chưa sửa — ST-296 rà và đóng kèm commit hash (đúng phạm vi
  ST-295 chỉ xoá cảnh báo `HOST-BAO-DAM.md`).

## 5. Cổng (số thật)

| Cổng | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` | **3600 passed / 0 failed / 3 ignored** (357 target) |
| `cargo test -p vis-host-linux --test landlock` | **18/18** |
| `st295_sandbox_proc` (CLI `--sandbox` + `Proc`) | **3/3**; `quyen_st295` **3/3** |
| `vis test test/sem` / `--vm` | **525/525** ×2 |
| `./bin/vis-host-suite` | **54/54** (+ `host_posix_real` PASS) |
| `./bin/vis-targets` | core **18/18**, sqlite **16 PASS + 2 EXPECTED_RED**, 0 FAIL |
| `./bin/vis-diff` (`VIS_DIFF_N=100000`) | **0 lệch** (93.768 chạy, 6.232 lỗi tĩnh, 75,6 s) |
| `./vis-dev gate --fast` | **PASS** (2 m 33 s) |
| fmt / clippy (workspace + `vis-host-linux`) | sạch (`-D warnings`) |
| df | còn **30 G** trên `/` |

Bằng chứng gộp: `agent/JOURNAL/st295-verify.log`; license: `st295-license.log`; đua: `st295-race.log`.
