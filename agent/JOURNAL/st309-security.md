# ST-309 — kiểm toán bảo mật (phần 1: TOCTOU/cap/fuzz)

**Ngày**: 2026-11-23 · **Nhánh**: `plan/st309-security` từ `c4337ce` · **Máy**: `ps`
(Linux 7.0.0-43-generic x86_64, rustc 1.100.0) · **Môi trường**:
`CARGO_BUILD_JOBS=3 TMPDIR=/home/ps/tmp`, local only (không ssh `mps`/`psvn`).

## 1. Kết luận ngắn

- **0 `unsafe`** trong mã dự án: **10 crate** có `#![forbid(unsafe_code)]`; 28 lần chữ `unsafe`
  trong `crates/` đều là **chú thích** (“không `unsafe`”, “unsafe FFI nằm trong `mio`…”), không có
  khối `unsafe {}`/`unsafe fn`.
- **2 lỗ hổng thật đã vá**, mỗi lỗi có **ca đỏ chạy trước** rồi mới sửa:
  - **F1 (cao)** — gốc `cap` bị escape qua symlink ở **thư mục trung gian** (`quyen_mo::mo_goc`).
  - **F2 (vừa)** — plugin tĩnh ghi xuyên symlink ở **tên cuối** (`PluginCtx::write_text`).
- Landlock: không thấy lỗ; thêm ca âm chứng minh kernel chặn ghi ngoài khi không khai `cap`.
- WASI preopen: symlink trong preopen trỏ ra ngoài vẫn bị từ chối (`VS0604`) — thêm ca.
- Fuzz **1 500 000 ca** decoder+verifier: **0 panic**, 4,95 s (302 916 ca/s).
- `./vis-dev gate --fast`: **PASS** (sẽ điền số cuối ở §6).

## 2. F1 — escape qua symlink trung gian của gốc cap

**Mô tả.** `mo_goc` cũ mở `open(goc, O_NOFOLLOW|O_DIRECTORY)`. `O_NOFOLLOW` chỉ chặn **thành phần
cuối**; các thư mục cha vẫn bị đi theo symlink. `QuyenCap.root` được canonicalize **một lần lúc
khởi động**, nhưng mỗi thao tác lại mở lại theo **tên**. Kẻ tấn công đổi một thư mục cha của gốc
thành symlink sau khởi động ⇒ mọi `openat2(RESOLVE_BENEATH)` sau đó đi từ **fd gốc đã sai** và
đọc/ghi ra ngoài `cap`. Không cần đua: chỉ cần một lần `rename` + `symlink`.

**Ca đỏ (đã chạy trước khi vá).**

```text
cargo test -p vis-host-native --lib goc_bi_thay_bang_symlink_trung_gian
=> FAILED: "gốc bị đổi qua symlink trung gian phải bị chặn: ()"
```

`crates/vis-host-native/src/quyen_st295.rs` dựng `d/cha/con` làm gốc, dời `d/cha` rồi symlink
`d/cha -> d/ngoai` (trong đó có `ngoai/con/`), gọi `FsGuard::write`; trước vá ghi được
`ngoai/con/tep`.

**Vá tối thiểu.** `mo_goc` mở gốc **không theo symlink ở mọi thành phần**: Linux
`openat2(CWD, goc, …, RESOLVE_NO_SYMLINKS)`; kernel cũ/macOS fallback mở `/` rồi đi từng thành
phần với `O_NOFOLLOW`; từ chối gốc không tuyệt đối hoặc có `..`. Sau vá ca xanh, `ngoai/con/tep`
không xuất hiện; các ca ST-295 cũ (100/100 đỏ nhánh cũ, 0/100 nhánh mới) vẫn xanh.

## 3. F2 — plugin tĩnh ghi xuyên symlink tên cuối

**Mô tả.** `PluginCtx::write_text` canonicalize **thư mục cha** rồi
`std::fs::write(canon_parent.join(file_name))`. Nếu **tên cuối** là symlink trỏ ra ngoài `cap`,
`write` đi theo link và ghi đè tệp ngoài — **tất định**, không cần đua. Đường đọc cũ cũng có khe
`canonicalize(path)` → `read_to_string(canon)`.

**Ca đỏ (đã chạy trước khi vá).**

```text
cargo test -p vis-plugin-abi --test parity cap_ghi_khong_di_theo_symlink
=> FAILED: "ghi qua symlink ra ngoài cap phải bị từ chối: ()"
```

**Vá tối thiểu.** Thêm `crates/vis-plugin-abi/src/cap_fs.rs` (Unix): mở gốc bằng
`openat2(NO_SYMLINKS)`/fallback `O_NOFOLLOW`, rồi mở tương đối bằng
`RESOLVE_BENEATH` (đọc) hoặc `RESOLVE_BENEATH|NO_SYMLINKS` (ghi, `O_CREAT|O_TRUNC|O_NOFOLLOW`).
`Cargo.toml` thêm `rustix` **chỉ cho `cfg(unix)`** (crate đã nằm trong allowlist từ VD-120).
Windows giữ đường canonicalize cũ (mức bảo đảm thấp hơn, đã ghi ở `HOST-BAO-DAM.md`).
Sau vá: lỗi `VS0604`, tệp ngoài giữ `NGOAI`; toàn bộ 7 ca `parity` + 6 ca `vs3_plugin` xanh.

## 4. Các bề mặt khác đã rà

| Bề mặt | Kết quả |
|---|---|
| `openat2(RESOLVE_BENEATH\|NO_SYMLINKS)` cho `read/write/append/mkdir/remove/rename/copy` (`quyen_mo`) | Đúng; `rename` dùng `RENAME_NOREPLACE` trên Linux/apple; `copy` nguồn `NOFOLLOW`, đích `O_EXCL` |
| `..` / đường tuyệt đối trong `rel` | `chuan_tu_van` + `strip_prefix` + `thanh()` từ chối `ParentDir`/`RootDir`/rỗng |
| `cap[File]` | Tách cha + tên, mở cha `NOFOLLOW`, tên cuối `NOFOLLOW` |
| `Value::Cap` | Chỉ CLI dựng từ CST `main`; `json.decode`/đọc tệp/argv không tạo được (ca ST-124 cũ giữ xanh) |
| Landlock (`--sandbox`) | Ruleset phủ `AccessFs::from_all`; `Execute` chỉ xử lý khi có `cap[Proc]` + grant; ca âm mới: ruleset rỗng chặn mọi ghi, đối chứng ngoài sandbox ghi được |
| WASI preopen | Preopen đúng `cap[Dir]`, guest `/cap0…`; `..` và tệp ngoài đã có ca; **mới**: symlink trong preopen trỏ ra ngoài bị `VS0604` |
| Plugin tĩnh `read_text` | Nay `RESOLVE_BENEATH` từ fd gốc; symlink ra ngoài bị chặn bởi kernel, không dựa canonicalize |
| Rò rỉ metadata (`size`/`list`/`exists`/`canonicalize`/`read_link`/`is_*`/`walk`) | **Còn nợ**: vẫn kiểm-rồi-dùng theo tên (API `FsHost` không có fd). Không trả **nội dung** tệp ngoài, nhưng có thể lộ metadata qua khe đua — ghi nhận, không sửa trong ST-309 phần này |

## 5. Test âm mới (hữu hạn, tất định)

- `crates/vis-host-native/src/quyen_st295.rs::goc_bi_thay_bang_symlink_trung_gian_khong_duoc_ghi_ra_ngoai`
  — F1 (đỏ trước).
- `crates/vis-plugin-abi/tests/parity.rs::cap_ghi_khong_di_theo_symlink_o_ten_cuoi` — F2 (đỏ trước).
- `crates/vis-plugin-abi/tests/vs3_plugin.rs::preopen_khong_di_theo_symlink_ra_ngoai` — WASI.
- `crates/vis-cli/tests/security_it/st309.rs`:
  - `symlink_ten_cuoi_ra_ngoai_root_bi_chan_ca_doc_va_ghi` — `VS0604`, không rò/ghi tệp ngoài;
  - `sandbox_chan_ghi_ngoai_khi_khong_khai_cap` — Landlock kernel (Linux).
- Đã có sẵn và giữ xanh: `strict_caps.rs` (`..`, thiếu cap, `--strict-caps`), `cap.rs`,
  `vs3_plugin.rs` (ngoài preopen, `..`), `st295_sandbox_proc.rs` (allowlist Proc).

## 6. Fuzz và cổng

**Fuzz decoder+verifier** — `crates/vis-bytecode/tests/st309_fuzz_verify.rs` (seed 8 module; mỗi ca
1–3 đột biến: bit/byte/truncate/insert/remove; `decode` + `verify` + `need_stack` bọc
`catch_unwind`):

```text
VBC_FUZZ_CA=1500000 cargo test -p vis-bytecode --test st309_fuzz_verify -- --nocapture
ST309-FUZZ ca=1500000 decode_ok=171833 verify_ok=149819 vi_pham=0 giay=4.95 ca_giay=302916
```

- Log: `/home/ps/tmp/st309-fuzz.log` · `sha256 c4e6b2be3e024492d99829df12d8de109bbaa801144594e6fc2225686cc5de00`
- Mặc định cổng chỉ chạy 20 000 ca (0,06 s); mốc 1,5 M chạy bằng `VBC_FUZZ_CA` như trên.

**Cổng `./vis-dev gate --fast`** (log `/home/ps/tmp/st309-gate-isolated2.log`):

- Baseline trước thay đổi @ `c4337ce`: PASS **3748** ca.
- Sau thay đổi: **PASS `3754 tests run: 3754 passed, 4 skipped`**, gate **PASS** (log
  `st309-gate-isolated2.log`, `sha256 e632c2d379070027d25d1a22e899c3c5cbda4604d2ab1ba78d1eca3862dba820`).
- Chạy trong **git worktree cô lập** `/home/ps/tmp/st309-sec-gate` (detached @ `c4337ce` + đúng
  13 tệp của ST-309) vì worktree chính đang bị **agent ST-309-spec** sửa đồng thời
  `docs/SPEC.md`/`test/doc-examples/doc_examples.py`; bản sao cô lập cho cùng cây mã, không lẫn
  tệp chưa commit của tác vụ khác.
- `cargo fmt --all --check`: OK; `cargo clippy -p vis-host-native -p vis-plugin-abi -p vis-cli
  -p vis-bytecode --all-targets -- -D warnings`: OK; R1..R7 do `vis-dev check` chạy (PASS).

## 7. Tệp thay đổi

```text
crates/vis-host-native/src/quyen_mo.rs      (F1: mo_goc NO_SYMLINKS mọi thành phần)
crates/vis-host-native/src/quyen_st295.rs   (ca đỏ→xanh F1)
crates/vis-plugin-abi/src/cap_fs.rs         (F2: openat/BENEATH cho plugin tĩnh, mới)
crates/vis-plugin-abi/src/lib.rs            (dùng cap_fs; doc)
crates/vis-plugin-abi/Cargo.toml            (rustix cfg(unix))
crates/vis-plugin-abi/tests/parity.rs       (ca đỏ→xanh F2)
crates/vis-plugin-abi/tests/vs3_plugin.rs   (preopen symlink)
crates/vis-cli/tests/security_it.rs         (+ mod st309)
crates/vis-cli/tests/security_it/st309.rs   (ca âm CLI + Landlock, mới)
crates/vis-bytecode/tests/st309_fuzz_verify.rs (fuzz verifier/decoder, mới)
agent/HOST-BAO-DAM.md                       (cập nhật bảo đảm + nợ metadata)
Cargo.lock                                  (rustix cho vis-plugin-abi)
```

## 8. Việc còn lại (ngoài time-box phần này)

1. **Metadata TOCTOU** (§4): cần API `FsHost` theo fd (`statat/listat/readlinkat`) trên Unix để
   đóng nốt; hiện chỉ lộ metadata, không lộ nội dung.
2. **Plugin WASI bản sao chỉ-đọc**: tên thư mục tạm dựa `pid` + bộ đếm (dự đoán được) và
   `remove_dir_all` → `create_dir_all`; nên tạo nguyên tử (`create_dir` + retry) và mode `0700`.
   Không khai thác được từ test hiện tại; ghi nhận để vá cùng đợt sau.
3. Landlock/Windows/macOS đầy đủ thuộc tổ hợp nghiệm thu ST-309 cuối (ngoài phạm vi phần kiểm toán
   này, chạy local Linux).
