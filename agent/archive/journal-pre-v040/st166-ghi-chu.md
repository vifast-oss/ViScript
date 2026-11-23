# ST-166 — nợ **#132**: 3 ca âm dựng helper bằng `rustc` (thiếu `crt-static`/`rustc` ⇒ đỏ vì môi trường)

Ngày: 2026-11-17 (JST) · Agent 2 dòng ViScript · Máy đo: `mac` (mps, macOS 27.0.1 arm64) và `ps`
(Ubuntu 26.04.1, kernel 7.0.0-43-generic, Landlock ABI 8) · Tệp sửa: `vis-host-linux/tests/landlock.rs`
(**gốc repo**, ngoài `viscript/` — host của ViScript).

## 0. Kết luận một dòng

Nợ **#132 đóng**: phần **dựng helper** không còn phụ thuộc PATH của runner (`RUSTC` → `$CARGO_HOME/bin` →
`$HOME/.cargo/bin` → PATH; PATH của tiến trình `rustc` được bổ sung thư mục toolchain + `/usr/bin:/bin` cho
`cc`) và **cờ tĩnh chỉ đặt trên Linux** (`cfg!(target_os = "linux")`); kỳ vọng của **mọi** ca **không đổi**
(không `#[ignore]`, không `#[should_panic]`, không bỏ ca, không nới ngưỡng). **Đỏ-trước trên `ps` (PATH rút gọn):
6 ca đỏ** = `9 passed; 6 failed` (nguyên văn: `rustc phải dựng được helper tĩnh: error: linker`cc`not found`)
⇒ **xanh-sau 15/15** ở **cả ba** kiểu môi trường. `vis-targets` **18/18**. Cổng macOS **PASS** `[exit=0]`,
**10/10** bước, L0 **1182/0** (131 suite) — lần chạy 2; lần 1 đỏ ở **nợ #146** (không liên quan, xem §6).

## 1. Nguồn đọc trước

- `agent/SO-NO.md` hàng `| 132 |`; `agent/JOURNAL/st132b-read-dir.md` **§7** (nguyên văn nợ: "nếu runner thiếu
  `target-feature=+crt-static`/`rustc` thì ca đỏ vì **môi trường**") và **§6** (macOS **không áp dụng** crate này).
- `agent/JOURNAL/2026-11-17.md` (ST-150: `ps` dựng helper tĩnh **thành công**, "nợ #132 không chặn lần đo này").
- `viscript/agent/JOURNAL/quyet-dinh-a0-2026-11-17-dot29.md` §3.5 ("đêm nay làm #138, #132") + hàng `ST-166`
  (`agent/QUEUE.md`): "đỏ-trước → xanh; `vis-targets` **18/18** cả 3 nền; #132 đóng".
- VD-120 (kernel chặn thật), VD-132, VD-136 (header log), VD-138 (runner `ps` qua SSH), D2-07 (không retry lấy xanh).

## 2. Đỏ-trước (log giữ nguyên, không ghi đè)

### 2.1 `mac` — `JOURNAL/st166-do-truoc-mac.log` (`[exit=101]`)

```
$ cd vis-host-linux && cargo test -p vis-host-linux --test landlock
error[E0425]: cannot find function `prctl` in crate `libc`
error[E0425]: cannot find value `O_PATH` in crate `libc`
error: could not compile `landlock` (lib) due to 6 previous errors
[exit=101]
```

**Lý do môi trường, không phải kỳ vọng sai**: crate `landlock 0.4.7` dùng `libc::prctl`/`SYS_landlock_*` của
Linux; trên macOS **0/15 ca chạy được** (đã ghi ở `st132b-read-dir.md` §6). Vì vậy trên `mac` **không có** "3 ca
đỏ": cả suite không biên dịch, "3 ca đỏ trên mac" **không tái hiện được** — xem §7.

### 2.2 `ps` — `JOURNAL/st166-do-truoc-ps.log` (`[exit=101]`)

- (A) runner chuẩn: `test result: ok. 15 passed` · `A_EXIT=0` — **không** tái hiện được đỏ (khớp ST-150).
- (B) **runner rút gọn PATH** (mô phỏng đúng sự cố R28 §5 #6 ghi ở `viscript/bin/vis-remote`: phiên `ssh`/CI
  không có `$HOME/.cargo/bin` trong PATH): `cargo` vẫn chạy (ghim `RUSTC`), nhưng **tiến trình con** không tìm
  được trình liên kết ⇒ **6 ca đỏ** (`3 ca âm` của #132 + 3 ca đối chứng dùng cùng helper):

```
rustc phải dựng được helper tĩnh: error: linker `cc` not found
  |
  = note: No such file or directory (os error 2)

failures:
    con_doc_ngoai_pham_vi_bi_kernel_chan          <- âm (#132)
    con_ghi_ngoai_pham_vi_bi_kernel_chan          <- âm (#132)
    con_read_dir_ngoai_pham_vi_bi_kernel_chan     <- âm (#132)
    con_doc_trong_pham_vi_chay_duoc               <- đối chứng dương
    con_ghi_vao_thu_muc_chi_doc_bi_kernel_chan    <- đối chứng dương
    con_read_dir_thu_muc_co_quyen_ghi_van_liet_ke_duoc
test result: FAILED. 9 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
B_EXIT=101
```

## 3. Sửa tối thiểu — **chỉ** phần dựng helper (`+79 / −4` dòng, `landlock.rs` 631 → 706 dòng)

1. `duong_rustc()` — `RUSTC` → `$CARGO_HOME/bin/rustc` → `$HOME/.cargo/bin/rustc` → `rustc`: hết phụ thuộc PATH.
2. `path_dung_helper()` — giữ PATH hiện tại rồi **bổ sung** thư mục toolchain + `/usr/bin:/bin:/usr/sbin:/sbin`
   (nơi có `cc`), bỏ trùng, đặt vào **env của tiến trình `rustc`**.
3. `thu_muc_toolchain()` — hai thư mục toolchain theo `CARGO_HOME`/`HOME`.
4. **Cờ tĩnh có điều kiện**: `-O` luôn giữ; `-C target-feature=+crt-static` **chỉ** khi
   `cfg!(target_os = "linux")` (Linux cần helper **tĩnh**: `execve` binary động phải mở `ld.so`/thư viện **ngoài**
   ruleset ⇒ hỏng kỳ vọng; nền khác gắn std tĩnh sẵn). Chú thích đầy đủ ngay trên `dung_helper` (ST-166).

**Không** đổi: chuỗi nguồn helper, mã thoát `0/2/7`, mọi `assert_eq!`, mọi ngưỡng, số ca (15).
**Không** thêm `unsafe`, crate, `#[ignore]`, `#[should_panic]`; tệp 706 dòng < R1 1500; SPDX giữ nguyên
(`// SPDX-License-Identifier: MIT OR Apache-2.0`).

## 4. Xanh-sau

- `ps` — `JOURNAL/st166-sau-ps.log`, sha256 tệp **hai phía khớp** `eb7e729b4e550692a290f974933d348fead948aa694d6d74fc36a1613111a31b`:
  - (A) runner chuẩn: `15 passed; 0 failed` · `A_EXIT=0`
  - (B) **runner rút gọn PATH** (đúng mục 2.2-B): `15 passed; 0 failed` · `B_EXIT=0` (**trước sửa: 6 failed**)
  - (C) PATH rỗng + `CARGO_HOME`/`HOME` ghim: `15 passed; 0 failed` · `C_EXIT=0`
- `mac` — `JOURNAL/st166-sau-mac.log`: (A) suite vẫn **không biên dịch** `[exit(A)=101]` (lý do môi trường như §2.1,
  **không** phải nợ #132); (B) **phần dựng helper chạy tốt trên mac** ở nhánh không-Linux: `rustc -O` ⇒ `[exit=0]`,
  `ls`/`doc`/`ghi` đúng mã thoát (`0`/`0`/`0`), `doc /etc/hosts` = `0` (macOS **không có** Landlock ⇒ đây chính là
  lý do 3 ca âm **không thể** chạy trên mac); (C) `+crt-static` trên mac: `[exit=0]`.
- 3 ca âm (#132) xanh **trên Linux** ở cả (A)/(B)/(C); trên **mac** không có ca nào chạy được — xem §7.

## 5. `vis-targets` — `JOURNAL/st166-vis-targets.log`

`./viscript/bin/vis-targets` ⇒ **18/18 PASS** (`grep -c PASS` = 18), `[exit=0]` (8 crate × 2 target + 2 EXTRA:
`vis-host-wasm × wasm32-unknown-unknown`, `vis-host-linux × x86_64-unknown-linux-gnu`).

## 6. Cổng macOS — **hai** log (D2-07: chạy lại **một** lần, chỉ để **phân loại**)

- `JOURNAL/st166-gate-macos.log` — lần 1: `[exit=1]`, **L0 FAIL** `65 passed; 1 failed` ở
  `quyen::tests::dua_doi_symlink_lap_co_gioi_han` (`RÒ RỈ: đọc được nội dung ngoài root 1 lần`, `ro_ri=1/50 000`,
  `noi_dung="NGOAI"`). Đây **đúng nợ #146** — đã phân loại **(b) TOCTOU THẬT, chỉ macOS**, **BLOCKED-CHO-A0**
  (`agent/BLOCKED.md` §6, ST-163). **Không** liên quan ST-166: `quyen.rs` **không** bị sửa (`git diff` rỗng),
  `vis-host-linux` **ngoài** workspace `viscript/`, không dòng mã nào của `viscript/` bị đụng.
- `JOURNAL/st166-gate-macos-lan2.log` — lần 2 (phân loại): **PASS**, `[exit=0]`, **10/10** bước
  (fmt · clippy · luật R1..R4 · self-test case âm · vis explain · báo cáo · báo cáo fixtures · build · doc-examples ·
  L0), **L0 1182/0** (131 suite), `report-check: PASS (28 báo cáo, 192 hàng QUEUE phủ đủ)`,
  `doc-examples: OK (24 khối: 18 xanh + 6 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3)`.

## 7. Lệch so với đề bài (ghi **số đo thật**, không sửa kỳ vọng)

1. **"đúng 3 ca đỏ"** → đo được **6 ca** ở mục 2.2-B: nợ #132 viết lúc ST-132b mới có **3** ca dùng helper;
   nay có **6** ca dùng **cùng** `dung_helper` (3 âm + 3 đối chứng dương), nên lỗi **môi trường** của helper
   làm đỏ **cả 6** — không thể chỉ 3.
2. **"3 ca đó xanh trên mac"** → **không thực hiện được**: `landlock 0.4.7` chỉ biên dịch trên Linux (6 lỗi
   `libc`), đã ghi ở `st132b-read-dir.md` §6; macOS **không có** Landlock nên 3 ca âm (bản chất là *kernel chặn*)
   **không thể** xanh trên mac. Trên mac đã kiểm chứng **phần dựng helper** (mục 4-B/C) — đó là phần thuộc #132.
3. **"crt-static không hỗ trợ trên mac"** → **SAI với toolchain này**: `rustc 1.99.1 aarch64-apple-darwin` chấp
   nhận `-C target-feature=+crt-static` (`[exit=0]`, ra Mach-O). Vì vậy cờ được `cfg` theo nền để **đúng thiết kế**
   (Linux cần tĩnh, nền khác không cần), **không** phải vì mac từ chối cờ.
4. **Nguyên nhân đỏ thật của #132** (tái hiện được): runner **thiếu `rustc`/`cc` trong PATH** — đúng lớp sự cố
   R28 §5 #6; đã sửa và đo lại ở 3 môi trường.
5. Đề nghị (không tự làm): nếu muốn `cargo test -p vis-host-linux --test landlock` trên mac là **xanh 0 ca**
   thay vì đỏ biên dịch, phải `cfg` hoá **dependency** `landlock` trong `Cargo.toml` — **ngoài** phạm vi "chỉ sửa
   phần dựng helper" của ST-166, cần Agent 0 quyết.

## 8. Tệp đã ghi

- **Sửa**: `vis-host-linux/tests/landlock.rs` (`dung_helper` + 3 hàm phụ + chú thích; kỳ vọng không đổi).
- **Mới (log/ghi chú)**: `JOURNAL/st166-do-truoc-mac.log` · `JOURNAL/st166-do-truoc-ps.log` ·
  `JOURNAL/st166-sau-mac.log` · `JOURNAL/st166-sau-ps.log` · `JOURNAL/st166-vis-targets.log` ·
  `JOURNAL/st166-gate-macos.log` · `JOURNAL/st166-gate-macos-lan2.log` · `JOURNAL/st166-ghi-chu.md`.
- **Không** đụng `QUEUE.md`/`DECISIONS.md`/`SO-NO.md`/`spec/`/`v2/`; `git add` **đường dẫn tường minh**,
  không `git add -A`/`git commit -a` (index dùng chung với dòng OS).
