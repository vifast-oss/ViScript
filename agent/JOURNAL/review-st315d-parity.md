<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315d (phần 3) — Gatekeeper kiểm toán độc lập harness parity 66 hàm host

> **Kết luận: `FAIL` (nhẹ)** — cơ chế parity đúng, tất định, cổng xanh, nhưng **hợp đồng
> “ca lỗi so mã/thông điệp” chưa đạt** (`fs` **0** ca lỗi; `net` 10 hàm chỉ so **loại** `Ok/Err`,
> bỏ thông điệp) và **8 hàm `fs` chỉ được gọi, không assert giá trị** (F3). Journal còn khai quá
> mức (“mỗi nhóm có ≥ 1 ca lỗi”, “ca lỗi in thêm mã/thông điệp”). **Không** tìm thấy lệch
> interp↔VM thực tế nào; các mục chặn chỉ là chất lượng phủ, vá ~30 dòng là duyệt lại được.
>
> **Snapshot**: `2fc47130` (nhánh `review/rparity`; harness từ `7436d7ee`, merge `c9f28526`).
> **Ngày**: 2026-11-26 · **Máy**: `ps` (Linux, rustc 1.100.0) · **Không dùng `mps`** ·
> **Không sửa mã** (chỉ tệp này).
> **Target**: `/home/ps/ViScript-rparity/target` — target **riêng** trong worktree,
> `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`. **Không** dùng `/home/ps/ViScript/target`
> sau khi `main` merge `ST-315b` (`Value::Map(Rc<VisMap>)`) để tránh trộn artifact pre/post-W2.

## 0. Phạm vi và lệnh đã chạy

- Đọc toàn bộ `crates/vis-conformance/tests/misc_api_it/host_parity.rs` (**1.011 dòng**),
  `crates/vis-conformance/tests/lib_api_it.rs:81-82`, `JOURNAL/st315d-parity.md`,
  `JOURNAL/st315d-parity-plan.md`, `StdId::ALL` (`crates/vis-hir/src/stdlib/std_id.rs`), và
  host native liên quan (`fs`, `net`).
- Lệnh chính (target riêng):
  - `cargo test -p vis-conformance --test lib_api_it host_parity -- --test-threads=1` **×5**
  - `cargo test -p vis-conformance --test lib_api_it host_parity -- --test-threads=8` **×3**
  - **4 tiến trình test song song** (gọi thẳng binary `lib_api_it`) × 10 test
  - `./vis-dev gate --fast` + `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo nextest list -p vis-conformance --test lib_api_it | grep -c host_parity` → **10**
- Log: `/home/ps/vislogs/rparity-5x.log` (sha256 `6282ecee…95df730`),
  `/home/ps/vislogs/rparity-gate.log` (sha256 `61e48a27…0229fda6a`), ngoài repo.

## 1. Bảng phán quyết

| Hạng mục | Kết quả | Bằng chứng |
|---|---|---|
| Phủ **66/66** hàm host | **ĐẠT** | `HOST_66` = `StdId` host đúng 66 (`host_parity.rs:176-192`); 66 tên đều có lời gọi `tên(` **trong script** (không chỉ bảng) |
| Mỗi ca chạy **cả hai đường** | **ĐẠT** | `hai_duong` (`:156-172`) chạy `vis run` rồi `vis run --vm --no-cache` trên cùng tệp; 10 test |
| So stdout/stderr/exit | **ĐẠT (máy móc)** | `assert_eq!` cả 3 (`:168-170`). Caveat: mọi ca đều exit 0/stderr rỗng; chưa có ca trap/non-zero |
| **Ca lỗi so mã/thông điệp** | **KHÔNG ĐẠT đủ** | `sqlite` + `http` tốt; `net` **bỏ `e.message`** (10 hàm); `fs` **không có ca lỗi** (F1/F2) |
| Assert kết quả từng hàm | **KHÔNG ĐẠT đủ** | 8 hàm `fs` có thể in `E` mà test vẫn xanh (F3) |
| Tất định / ổn định | **ĐẠT** | 5×1 thread: 10/10; 3×8 thread: 10/10; 4 tiến trình song song: 40/40; không đụng cổng/tempdir |
| Dọn dẹp | **ĐẠT khi xanh** | 12 lượt của tôi để lại **0** `/tmp/vis-host-parity-*`; 5 thư mục cũ (12:36/13:08) do lượt trước, xem F7 |
| R8 (0 binary cấp 1 mới) | **ĐẠT** | 38 `crates/*/tests/*.rs` + 1 `vis-host-linux` = **39 ≤ 40**; `vis-cli/tests` = **8 ≤ 15**; `host_parity` nằm trong `misc_api_it/` + `#[path]` |
| R1/R2/R7 | **ĐẠT** | 1.011 dòng ≤ 1.500; **0** `unsafe`; SPDX dòng 1; test chỉ ghi tempdir + `~/.cache`, `git status` sạch |
| `gate --fast` + clippy | **ĐẠT** | `vis-dev check: PASS`; L0 **3.839 passed / 0 failed** + doc-test; `gate --fast: PASS`; `clippy -D warnings` rc=0 |

## 2. Findings

### F1 — `[M · CHẶN]` Nhóm `fs` **không có ca lỗi nào**; trái plan §8 và journal

- **Vị trí**: script `fs_20_ham_parity_tren_tempdir` (`host_parity.rs:241-314`). Mọi lời gọi đều
  trên fixture thành công; `fs.exists`/`is_dir` trả `Ok(false)` **không phải** lỗi. Không có
  `fs.read`/`size`/`remove` trên tệp thiếu, không `mkdir` trùng, không `append` vào thư mục.
- **Trái cam kết**: `st315d-parity-plan.md:48-49` đòi “mỗi nhóm ít nhất một lỗi (file thiếu …)
  — so exit + mã + message”; `st315d-parity.md:52` khẳng định “Mỗi nhóm có ≥ 1 ca lỗi”.
- **Repro (đã chạy, ngoài repo)**: thêm 16 ca lỗi `fs` (`read/size/remove/mkdir/copy/rename/
  read_link/canonicalize/list/walk/glob/append/write/read_bytes/read_lines/write_bytes`) và in
  `e.message`; interp và VM **trùng khớp từng byte** (ví dụ `read không đọc được …: No such file
  or directory (os error 2)`, `append …: Is a directory (os error 21)`). Vậy thêm ca không phải
  “đỏ vì engine”, chỉ là harness thiếu.
- **Tác động**: nhóm `fs` **không kiểm nhánh lỗi nào**; 16 hàm có `Err` (đã probe) đều ngoài
  harness, thêm `current_dir`/`mkdir_all` cũng có nhánh lỗi. Hợp đồng “ca lỗi so mã/thông điệp”
  không đạt cho nhóm này; không có lệch thực tế hôm nay.

### F2 — `[M · CHẶN]` `net`: 10 hàm lỗi chỉ so **loại**, không so thông điệp (kể cả tham số timeout)

- **Vị trí**: `net_loopback_parity` — các nhánh `Err(e)` chỉ in nhãn cố định:
  `:656` “listen busy ok”, `:678` “doc sau dong bi chan”, `:686` “accept_timeout het gio”,
  `:692` “dong-l2 bi chan”, `:696` “connect tu choi”, `:699` “connect_timeout tu choi”,
  `:702/:705/:708` “tay la bi chan”. **Không** chỗ nào in `e.message`.
- **Tác động**: VM truyền sai `timeout` (ví dụ 80 ms → 100 ms) hoặc sai handle vẫn `Err`, test vẫn
  xanh vì stdout chỉ có nhãn. Đây là **lệch parity thật** có thể xảy ra mà harness không bắt;
  không phải chỉ là “host dùng chung”.
- **Repro (đã chạy)**: probe in `e.message` cho 9 ca lỗi; sau khi chuẩn hoá cổng ephemeral, interp
  = VM. Thông điệp thật: “quá thời gian chờ kết nối sau 80 ms”, “bị từ chối kết nối tới
  127.0.0.1:1: Connection refused (os error 111)”, “tay mạng C99 không tồn tại…”.
  `double-listen` chứa cổng ephemeral nên cần chuẩn hoá/không in cổng; các ca còn lại in được ngay.

### F3 — `[M · CHẶN]` 8 hàm `fs` gọi nhưng **không assert giá trị**; có thể in `E` mà vẫn xanh

- **Vị trí**: `fs_20_ham_parity_tren_tempdir` — assert chỉ ở `:318-329` (6 dòng đầu, token
  `A/C/R/M/MA/RM`, `[b.txt]`, `a.txt`, dòng cuối `false`). Không assert:
  `read_lines` (`:259`), `read_bytes` (`:262`), `write` (`:265`), `write_bytes` (`:268`),
  `walk` (`:299`), `glob` (`:302`), `canonicalize` (`:305`), `current_dir` (`:308`).
- **Repro (đã chạy)**: lấy stdout thật của ca `fs`; thay **từng** dòng `[hi]`, `[65, 66, 67]`,
  `W`, `B`, `10`, `[a.txt, …]`, `a.txt`, `true` (dòng 7/8/9/10/23/24/25/26) bằng `E` rồi áp đúng
  các assert `:318-329` → **tất cả vẫn `true`**. Nghĩa là 8 hàm có thể trả `Err` ở **cả hai**
  engine mà test vẫn xanh.
- **Thêm**: `write`/`write_bytes`/`append`/`copy` chỉ kiểm “Ok”, **không kiểm nội dung** đã ghi/
  chép; `append` Ok nhưng không ghi gì vẫn xanh (lần đọc lại `:274` không được assert).
- **Lưu ý mức độ**: lệch **giữa hai engine** vẫn bị `assert_eq!(o1,o2)` bắt; lỗi **chung** của host
  (cả hai engine cùng sai) thì không. Đây là lỗ hổng của **hợp đồng assert kết quả** (và journal
  khẳng định “≥ 1 assert nội dung thật” cho cả nhóm), nên vá trước `DONE`; không phải lỗi parity
  hiện tại.

### F4 — `[L]` `moi_ham_host_co_loi_goi_trong_harness` chỉ là **tìm chuỗi trong chính tệp**

- **Vị trí**: `:197-209` — `include_str!("host_parity.rs")` rồi `src.contains(&format!("{n}("))`.
  Không kiểm lời gọi nằm trong script được chạy, không kiểm kết quả, không bỏ comment/chuỗi.
- **Repro (đã mô phỏng)**: đổi lời gọi thật `fs.read_bytes(d + "/bytes.bin")` thành tên khác và
  thêm comment `// fs.read_bytes(` ⇒ predicate vẫn thấy đủ **66/66**, test vẫn xanh; ca `fs` cũng
  xanh vì `read_bytes` không được assert (F3). Hôm nay **66 lời gọi đều thật** trong script, nên
  đây là lỗ hổng của **hàng rào**, không phải dương tính giả hiện tại.
- **Đề xuất**: bảng `(tên, tag)` + kiểm script của đúng tag, hoặc macro/compile-time; tối thiểu
  strip comment trước khi tìm.

### F5 — `[L]` Journal khai quá mức so với mã

- `st315d-parity.md:12-13`: “ca lỗi in thêm **mã/thông điệp**” — chỉ đúng cho `sqlite`/`http`;
  `net` in nhãn, `fs` không có ca lỗi.
- `st315d-parity.md:52`: “Mỗi nhóm có **≥ 1 ca lỗi**” — `fs` **0**.
- `st315d-parity.md:26`: “mỗi tên … phải có lời gọi `tên(` trong tệp” — đúng câu chữ nhưng là
  tìm chuỗi, không phải “ca chạy thật” (F4).
- `st315d-parity.md:38`: “`explain` … không ghim plan” — đúng plan, nhưng `:480` **ghim tên cột**
  `[id, parent, notused, detail]`; rủi ro thấp (SQLite 3 ổn định), chỉ cần nói rõ.

### F6 — `[L · caveat]` Windows chỉ chạy **64/66**

- `fs_symlink_parity` bị `#[cfg(unix)]` (`:334`); `fs.is_symlink` + `fs.read_link` không chạy trên
  Windows. Bảng `HOST_66` và test tìm chuỗi vẫn tính đủ 66. Journal đã ghi caveat; nếu A0 coi
  “66/66” là 3-OS thì cần `.skip-windows`/cơ chế khác hoặc chấp nhận ngoại lệ POSIX.

### F7 — `[L · caveat]` Dọn tempdir chỉ khi test xanh

- `tam` (`:98-104`) + `remove_dir_all` ở cuối mỗi test; panic/assert fail để lại
  `/tmp/vis-host-parity-*` (đã thấy **5** thư mục cũ mtime **12:36:37–12:36:49** và **13:08:23**,
  trước các lượt của tôi; 12 lượt của tôi không thêm thư mục nào). Nên dùng guard `Drop` nếu muốn
  “tự dọn” đúng cả nhánh đỏ.

### F8 — `[L · caveat]` `net.connect("127.0.0.1:1")` phụ thuộc cổng 1

- `:694`, `:697` dùng cổng **1** cố định cho “bị từ chối”. Trên Linux hiện tại ổn; trên Windows
  firewall có thể hỏi/trễ (journal `:97` đã ghi). Cách `http` đang dùng (bind `:0` rồi `drop`,
  `:950-952`) an toàn hơn; nên áp dụng cho `net`.

### F9 — `[L · caveat]` `vis_bin` giả định profile `debug` + binary đã build

- `:112-125` luôn tìm `<CARGO_TARGET_DIR>/debug/vis`; `cargo test -p vis-conformance` một mình
  (không qua `gate`) sẽ panic nếu chưa `cargo build -p vis-cli`; `--release` cũng trượt.
  `gate --fast` build trước nên cổng xanh; journal `:95` đã ghi. Nếu `CARGO_TARGET_DIR` là đường
  dẫn **tương đối**, `PathBuf` được hiểu theo cwd của test (`crates/vis-conformance`) — nên
  canonicalize theo workspace.

### F10 — `[L · caveat]` Exit 0 / stderr chưa được khẳng định

- `hai_duong` chỉ đòi `c1 == c2`, không đòi `c1 == 0` (`:168`). Một trap **chung** của cả hai
  engine sau khi đã in đủ output sẽ lọt (các nhóm có assert chuỗi đầy đủ ít rủi ro hơn). Nên thêm
  `assert_eq!(c1, 0)` cho ca thành công hoặc một ca trap có chủ đích.

### F11 — `[L · caveat]` Đồng hồ `time` phụ thuộc ngưỡng tuyệt đối

- `:218` đòi `now_ms() > 1_700_000_000_000` (2023-11). Máy có đồng hồ sai trước 2023 sẽ đỏ; hiện
  tại 2026 nên ổn. Có thể thay bằng `now_ms() > 0` + so với `SystemTime` của test (hoặc giữ, ghi rõ).

### F12 — `[caveat]` Server thread có hạn 8 s; chưa thấy flake

- `tcp_server` (`:751-753`) và `http_server` (`:857-860`) dùng deadline 8 s + `stop`; join có hạn.
  12 lượt chạy của tôi (kể cả 4 tiến trình song song và gate nextest) đều xanh; không có bằng
  chứng flake. Nếu máy cực tải, 12 kết nối HTTP + 2×300 ms có thể chạm 8 s — theo dõi, không chặn.

## 3. Bằng chứng tất định và cổng

### 3.1 Năm lượt `--test-threads=1` (target riêng)

| Lượt | Kết quả | Thời gian test | Wall |
|---:|---|---:|---:|
| 1 | 10 passed / 0 failed | 1,42 s | 1,56 s |
| 2 | 10 passed / 0 failed | 1,35 s | 1,49 s |
| 3 | 10 passed / 0 failed | 1,31 s | 1,45 s |
| 4 | 10 passed / 0 failed | 1,32 s | 1,46 s |
| 5 | 10 passed / 0 failed | 1,31 s | 1,44 s |

### 3.2 Tám luồng và bốn tiến trình song song

- `--test-threads=8` ×3: **10/10** mỗi lượt (0,63–0,67 s) — không đụng cổng/tempdir.
- 4 tiến trình `lib_api_it host_parity --test-threads=1` **đồng thời**: **40/40** test xanh
  (1,23–1,24 s/tiến trình); sau đó `/tmp/vis-host-parity-*` **không tăng**.

### 3.3 Cổng

```text
vis-dev check: PASS                      # clippy, R1..R9, DAG, toolchain, self-test
L0 test: cargo-nextest + doc-test: OK    # 3.839 passed / 0 failed
vis-dev gate --fast: PASS                # GATE_RC=0
cargo clippy --workspace --all-targets -- -D warnings   # CLIPPY_RC=0
```

- `host_parity::*` trong nextest: **10/10 PASS** (kể cả khi chạy song song với 3.829 test khác).
- `cargo nextest list … | grep -c host_parity` = **10**; `cargo test … -- --list` = **10**.
- R8: **39** binary cấp 1 (38 + `vis-host-linux/tests/landlock.rs`), `vis-cli/tests` **8**.
- `host_parity.rs`: **1.011 dòng**, 0 `unsafe`, SPDX dòng 1.
- `git status --short`: **sạch** sau các lượt chạy (target nằm trong `.gitignore`).

### 3.4 Probe lỗi (ngoài repo)

- **fs**: 16 thông điệp lỗi (read/size/remove/mkdir/copy/rename/read_link/canonicalize/list/walk/
  glob/append/write/read_bytes/read_lines/write_bytes) **interp = VM từng byte**. `is_file`/
  `is_dir`/`is_symlink` trên tệp thiếu trả `Ok(false)`, không phải lỗi.
- **net**: 9 thông điệp lỗi **interp = VM** sau khi chuẩn hoá cổng ephemeral; thông điệp chứa
  timeout thật (`sau 80 ms`) và lý do (`Connection refused (os error 111)`).
- **http**: `/cham*` in `quá thời gian…` và được assert `contains`; cổng đóng in thông điệp và
  được so parity qua stdout.
- **sqlite**: các thông điệp SQL/params/handle được ghim **nguyên văn** trong `:492-498`,
  `:627-629` — đây là mẫu tốt nhất của harness.

## 4. Đề nghị vá (theo thứ tự)

1. **F1 (chặn)**: thêm ≥ 3 ca lỗi `fs` — `read/size/remove` trên tệp thiếu, `mkdir` trùng,
   `append/write` vào thư mục — in `e.message`; 16 thông điệp đã kiểm là trùng engine hôm nay.
   Tối thiểu phủ đại diện `fs.read` + `fs.mkdir` + `fs.append`.
2. **F2 (chặn)**: in `e.message` cho `net.connect/connect_timeout/read/write/close/
   accept_timeout/close2`; với `double-listen`, in phần không chứa cổng hoặc chuẩn hoá cổng;
   ghim `sau 80 ms`/`sau 100 ms` để bắt lệch tham số giữa hai engine.
3. **F3 (chặn)**: ghim `read_lines`, `read_bytes`, `W`, `B`, `walk`, `glob`, `canonicalize`,
   `current_dir`; assert không có marker lỗi (nên dùng marker duy nhất thay `E`); kiểm nội dung
   `write`/`write_bytes`/`append`/`copy`.
4. **F4**: thay tìm chuỗi bằng bảng `(tên, tag)` kiểm đúng script, hoặc macro compile-time;
   tối thiểu strip comment.
5. **F5/F10**: sửa journal; thêm `assert_eq!(c1, 0)` cho ca thành công hoặc một ca trap.
6. **F7/F8/F9/F11/F12**: guard `Drop` cho tempdir, bind-rồi-thả thay cổng 1, canonicalize
   `CARGO_TARGET_DIR`/profile, ngưỡng đồng hồ tương đối — đều không chặn.

## 5. Phiếu trả lời

- **PASS/FAIL**: **FAIL (nhẹ)** — không đạt đủ hợp đồng “ca lỗi so mã/thông điệp” (F1 `fs`
  không có ca lỗi; F2 `net` bỏ thông điệp) và “assert kết quả” (F3 8 hàm `fs`).
- **Chặn `DONE`**: F1, F2, F3. **Nên gộp**: F4, F5 (rẻ, cùng tệp). **Follow-up không chặn**:
  F6–F12.
- **Không có lỗi runtime/parity thực tế** trong 12 lượt chạy và gate; tất định tốt; R8/R1/R2/R7
  và clippy đạt.
- **Không sửa mã** trong lượt review này; chỉ tạo tệp này và commit
  `[ST-315d] review parity: …` trên `review/rparity` (không push/merge).

---

## 6. Re-audit sau vá (2026-11-26)

> **Merge**: `main` `25790c97` (vá `a45438e6`) vào `review/rparity` tại **`fc85a0c5`**; báo cáo
> `FAIL` cũ giữ nguyên. **Target**: `/home/ps/ViScript-rparity/target` (riêng, không dùng
> `/home/ps/ViScript/target`); **không dùng `mps`**.
>
> **Kết luận re-audit: `FAIL` (nhẹ)** — F1/F2/F3/F5/F7/F8/F9/F10 đã đóng và kiểm được trên Linux;
> **F4 đóng ở mức “mỗi hàm có ≥ 1 lời gọi”** (bọc **cả 3** lời gọi `fs.read_bytes(` thì đỏ), nhưng
> **mutation một lời gọi trong `SRC_FS` vẫn xanh** (F-R1) — đúng câu chữ yêu cầu của lượt này.
> Thêm **F-R2**: `cwd-khop` của F3 có nguy cơ đỏ trên Windows do `std::fs::canonicalize` trả
> `\\?\…` còn `current_dir` thì không (chưa chạy `psvn`). Không có lệch engine; cổng xanh.

### 6.1 Các mục đã đóng (bằng chứng)

| Mục | Trạng thái | Bằng chứng |
|---|---|---|
| **F1** `fs` 0 ca lỗi | **ĐÓNG (Linux)** | `SRC_FS_LOI` 17 ca (`host_parity.rs:511-566`); `fs_loi_moi_ham_parity` ghim **nguyên văn** 17 thông điệp Unix (`:670-688`), nền tảng khác đòi 17 nhãn + `<W>`; test xanh |
| **F2** `net` bỏ message | **ĐÓNG** | Mọi nhánh `Err(e)` in `e.message` (`:1038-1111`); `chuan_hoa_cong` thay `127.0.0.1:<cổng>` bằng `<P>` (`:244-259`); mutation `80→100` làm `net_loopback_parity` **đỏ** tại `:1144` (đã chạy) |
| **F3** 8 hàm `fs` thiếu assert | **ĐÓNG (Linux)** | `SRC_FS` in giá trị thật cho **20/20** hàm; `fs_20_ham_parity_tren_tempdir` so **toàn bộ** stdout (`:578-620`), gồm nội dung sau `write`/`write_bytes`/`append`/`copy` |
| **F4** khoá 66/66 tìm chuỗi | **ĐÓNG một phần** | Chỉ xét `SRC_PHU` (`:99-109`) + bỏ chú thích `#` (`bo_chu_thich_vis`, `:263-297`); self-test `:351-361`; bọc **cả 3** `fs.read_bytes(` ⇒ **đỏ** `["fs.read_bytes"]`. Nhưng một lời gọi trong `SRC_FS` ⇒ **xanh** (F-R1) |
| **F5** journal khai quá mức | **ĐÓNG phần lớn** | `st315d-parity.md` đã viết lại; còn 2 câu quá rộng (F-R4) |
| **F7** dọn tempdir | **ĐÓNG** | `struct Tam` + `Drop` (`:111-131`); sau 5×1 + 2×8 + mutation net **đỏ**, `/tmp/vis-host-parity-*` = **0** |
| **F8** cổng cố định 1 | **ĐÓNG** | `net` bind `:0` rồi `drop` (`:1118-1121`); `http` giữ bind-rồi-drop (`:1390-1393`) |
| **F9** `vis_bin` | **ĐÓNG** | `CARGO_TARGET_DIR` tương đối hiểu theo gốc workspace; profile theo `debug_assertions` (`:148-174`) |
| **F10** exit 0 | **ĐÓNG** | `assert_eq!(c1, 0)` trong `hai_duong_chuan_hoa` (`:230-234`); 12/12 test xanh |
| **R8/R1/R2/R7** | **ĐẠT** | 38 + 1 = **39 ≤ 40**; `vis-cli/tests` **8**; tệp **1.452** dòng ≤ 1.500; **0** `unsafe`; journal ≤ 50 KB; `git status` sạch |

### 6.2 F-R1 — `[M · theo đúng tiêu chí mutation]` một lời gọi `fs.read_bytes(` trong `SRC_FS` **không** làm 66/66 đỏ

- **Vị trí**: `SRC_FS` có **2** lời gọi (`:425` đọc `bytes.bin`, `:438` đọc lại sau `write_bytes`);
  `SRC_FS_LOI` có **1** (`:557`). `moi_ham_host_co_loi_goi_trong_harness` chỉ đòi **ít nhất một**
  lời gọi trong **bất kỳ** script `SRC_PHU` (`:329-340`), không đòi từng script.
- **Repro đã chạy** (`/home/ps/vislogs/rparity-mutations.log`, sha256 `df1533b1…4ae2b5`):
  1. Comment **một** dòng `:425` (`# match fs.read_bytes(...)`) ⇒
     `moi_ham_host_co_loi_goi_trong_harness` **ok** (1 passed) — không đỏ như yêu cầu.
  2. Đổi tên **cả 2** lời gọi trong `SRC_FS` (giữ `SRC_FS_LOI`) ⇒ vẫn **ok** — script lỗi vẫn
     “che” hàm.
  3. Đổi tên **cả 3** lời gọi ⇒ **đỏ** `thiếu lời gọi host trong script được chạy:
     ["fs.read_bytes"]` — đúng như journal W4 §3 đã làm.
- **Bù lại**: `fs_20_ham_parity_tren_tempdir` so **toàn bộ stdout** nên xóa 2 lời gọi thành công
  làm test này **đỏ** (đã kiểm: thiếu `bytes=true`/`write_bytes-noi-dung=true`). Vì vậy **full
  suite** vẫn bắt regression; chỉ **meta-test 66/66** là không bắt.
- **Đề xuất**: nếu hợp đồng là “mỗi hàm có ≥ 1 lời gọi thật ở đâu đó” thì mutation một lời gọi
  là **không hợp lệ** — sửa câu chữ yêu cầu/journal; nếu muốn chặt hơn, thêm bảng
  `(tên, tag)` hoặc đòi `SRC_FS` có đủ 20 lời gọi thành công và `SRC_FS_LOI` đủ 17 lời gọi lỗi.

### 6.3 F-R2 — `[M · Windows]` `cwd-khop` có nguy cơ đỏ trên Windows

- **Vị trí**: `SRC_FS:496-501` — `p = fs.current_dir()`, `q = fs.canonicalize(".")`, in
  `p == q`; expected **`cwd-khop=true`** (`:616`).
- **Lý do**: host thật dùng trực tiếp `std::fs::canonicalize` (`crates/vis-host-native/src/ngoai.rs:672`)
  và `std::env::current_dir` (`:577`). Trên Windows, `std::fs::canonicalize` trả đường dẫn
  **verbatim** `\\?\C:\…` (tài liệu Rust), còn `current_dir` trả `C:\…`; so chuỗi `==` ⇒ `false`.
  `fs_20` không có `#[cfg(unix)]`, nên Windows full gate sẽ đỏ (chưa chạy `psvn` trong lượt này;
  log Windows `c6724b46` là harness **cũ 10 test**, không có `fs_loi`/`bo_chu_thich`/`cwd-khop`).
- **Đề xuất**: so `path.is_absolute(p)` + `q.ends_with(path.basename(p))`, hoặc bỏ tiền tố `\\?\`
  trước khi so, hoặc `#[cfg(unix)]` cho đẳng thức (và expected theo nền tảng). Cần chạy lại
  `cargo test -p vis-conformance --test lib_api_it host_parity` trên `psvn` trước `DONE` 3-OS.

### 6.4 Finding nhỏ / caveat

- **F-R3 [L]**: doc của `moi_ham...` (`:321-322`) nói chú thích Rust `// fs.read_bytes(` cũng làm
  test đỏ; thực tế bộ bóc chỉ hiểu `#` của ViScript, còn `//` nằm trong raw string vẫn bị tính
  (dương tính giả nếu ai đó dùng `//`). Self-test `:351-361` chỉ phủ `#`; sửa doc hoặc bóc thêm
  `//` (nhưng `//` không phải cú pháp ViScript — nên sửa doc là đủ).
- **F-R4 [L]**: `st315d-parity.md:55` “không còn nhóm nào 0 ca lỗi” — `time` (2 hàm) không có
  nhánh lỗi (đồng hồ không lỗi dữ liệu); `:59` nói “8 nhánh lỗi net” trong khi script in nhiều
  nhánh hơn. Sửa câu chữ.
- **F-R5 [L]**: F1/F2 ghim **nguyên văn** chỉ trên `#[cfg(unix)]`; Windows chỉ đòi nhãn/đường dẫn/
  cổng — journal đã ghi, chấp nhận được, nhưng F3 `cwd-khop` chưa có caveat tương ứng.
- **F-R6 [L]**: tệp **1.452/1.500** dòng — chỉ còn 48 dòng trước trần R1; thêm ca phải tách tệp
  hoặc giữ ngắn.
- **F-R7 [caveat]**: `walk-n` (10 Unix/9 Windows), `glob-n=4`, `cwd-khop` phụ thuộc fixture/cwd;
  Linux xanh; macOS/Windows cần chạy thật. `time` chỉ kiểm `now_ms > 0`, `monotonic` không giảm
  (đúng F11).

### 6.5 Bằng chứng chạy lại

| Lệnh | Kết quả |
|---|---|
| `cargo test … host_parity -- --test-threads=1` **×5** | **12/12** mỗi lượt; 1,06–1,09 s; log `rparity-reaudit-5x.log` sha256 `efb3ff65…c491975` |
| `--test-threads=8` **×2** | **12/12** mỗi lượt; 0,62 s |
| `./vis-dev gate --fast` | **PASS** — check + L0 **3.856 passed / 0 failed** + doc-test; 12/12 `host_parity`; `GATE_RC=0` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **rc=0** (`CLIPPY_RC=0`) |
| 4 tiến trình test song song (sau rebuild) | **48/48** — 12 test/tiến trình, 1,07–1,08 s; tempdir 0 |
| Mutation 1 (một lời gọi) | **xanh** (F-R1) |
| Mutation 2 (cả 3 lời gọi) | **đỏ** `["fs.read_bytes"]` |
| Mutation 3 (`accept_timeout` 80→100) | **đỏ** tại `:1144` (chuỗi `sau 100 ms` lệch `sau 80 ms`) |
| Tempdir sau các lượt | **0** `/tmp/vis-host-parity-*` (RAII, kể cả lượt mutation net đỏ) |
| R8 / R1 / R2 / R7 | 39/40 · 1.452 dòng · 0 `unsafe` · journal < 50 KB; `git status` sạch |

### 6.6 Phán quyết re-audit

- **FAIL (nhẹ)** theo **đúng tiêu chí mutation của lượt này** (F-R1: một lời gọi `fs.read_bytes(`
  trong `SRC_FS` **không** làm 66/66 đỏ) và **F-R2** (Windows `cwd-khop` có nguy cơ đỏ; chưa có
  bằng chứng `psvn` cho harness mới).
- **Nếu** hợp đồng chỉ là “mỗi hàm có ≥ 1 lời gọi thật **ở đâu đó**” + chỉ Linux, thì F-R1 là
  **caveat** (full suite vẫn bắt qua `fs_20`), F-R2 cần chạy Windows; khi đó có thể `PASS`.
  Tôi giữ **FAIL** vì yêu cầu nguyên văn “test 66/66 phải đỏ” chưa đạt và F3 chưa được chứng minh
  3-OS.
- Các vá F1/F2/F3/F5/F7/F8/F9/F10 **đạt** trên Linux; **không có lệch interp↔VM**; cổng/clippy
  xanh; R8/R1/R2/R7 đạt.
- **Không sửa mã** trong lượt re-audit; chỉ cập nhật tệp này và commit trên `review/rparity`
  (không push/merge).

---

## 7. Re-audit lần 2 (2026-11-26)

> **Merge**: `main` `347e9cae` (F-R2 `cwd-khop` + F-R4 journal) vào `review/rparity` tại
> **`78965856`**; báo cáo cũ giữ nguyên. **Target**: `/home/ps/ViScript-rparity/target` (riêng).
> **Kết luận: `PASS`** — F-R2 đóng trên Linux (Windows sẽ có **R-Win2** chạy full); **F-R1** là
> **caveat chấp nhận** theo hợp đồng meta-test; 5×1 + 2×8 + gate + clippy xanh; không có lệch
> interp↔VM.

### 7.1 F-R2 — `cwd-khop` canonicalize **cả hai vế** (đóng trên Linux)

- **Mã**: `SRC_FS:496-504` — `p = fs.current_dir()`, `cp = fs.canonicalize(p)`,
  `q = fs.canonicalize(".")`, in `cp == q`; expected `cwd-khop=true` (`:619`).
- **Lý do đúng**: trên Windows cả `cp` và `q` đều ở dạng extended-length `\\?\C:\…` nên khớp;
  trên Linux cả hai là cùng đường dẫn vật lý. Không còn so `current_dir` thô với canonicalize.
- **Kiểm Linux**: 5 lượt `--test-threads=1` + 2 lượt `--test-threads=8` đều **12/12**; riêng
  `fs_20_ham_parity_tren_tempdir` xanh, `cwd-khop=true` (đã có trong stdout kỳ vọng).
- **Windows**: chưa chạy trong lượt này (không `mps`); theo kế hoạch **R-Win2** chạy full trên
  `psvn`. Đây là điều kiện 3-OS còn lại, không phải lỗi Linux.

### 7.2 F-R1 — **caveat chấp nhận**, không chặn

- **Hợp đồng meta-test** (theo chỉ đạo): mỗi hàm cần **≥ 1 lời gọi thật** trong script; xoá
  **toàn bộ** lời gọi của một hàm ⇒ đỏ; xoá **một bản trùng** trong khi bản khác còn ⇒ xanh là
  **hợp lệ**, vì mất hành vi sẽ bị test hành vi (`fs_20`) bắt.
- **Bằng chứng mutation** (`/home/ps/vislogs/rparity-final-mutations.log`, sha256
  `86d900a1…d29c10`):
  | Mutation | `moi_ham…` (66/66) | `fs_20` (hành vi) |
  |---|---|---|
  | A. xoá **một** block `fs.read_bytes` trong `SRC_FS` | **xanh** (hợp lệ) | **đỏ** — thiếu `bytes=true` |
  | B. xoá **cả 2** block thành công trong `SRC_FS` | **xanh** (hợp lệ — còn bản lỗi) | **đỏ** — thiếu `bytes=true`/`write_bytes-noi-dung=true` |
  | C. xoá **toàn bộ 3** lời gọi `fs.read_bytes` | **đỏ** `["fs.read_bytes"]` | — |
  | D. khôi phục | **12/12** | 12/12 |
- **Kết luận**: F-R1 **không còn là finding chặn**; câu “một lời gọi phải đỏ” ở lượt trước là
  **quá chặt** so với hợp đồng function-level. Ghi thành caveat chấp nhận.
- **Residual L**: doc `moi_ham…` (`:321-322`) vẫn nhắc chú thích Rust `// fs.read_bytes(`; bộ bóc
  chỉ hiểu `#` của ViScript. Không ảnh hưởng hợp đồng hiện tại (mutation dùng `#`), chỉ nên sửa
  câu chữ/doc comment khi có dịp.

### 7.3 F-R4 — journal đã khớp

- `st315d-parity.md:55`: “trừ `time` — chỉ bất biến `>= 0`, không có nhánh lỗi” (không còn
  “không nhóm nào 0 ca lỗi”).
- `st315d-parity.md:59`: “**các nhánh lỗi** net” (không còn con số 8 cứng).
- Hàng F4 có **hợp đồng** ≥ 1 lời gọi/hàm; khớp mã và mutation ở §7.2.

### 7.4 Bằng chứng chạy lại

| Lệnh | Kết quả |
|---|---|
| `cargo test … host_parity -- --test-threads=1` **×5** | **12/12** mỗi lượt; 1,04–1,07 s; log `rparity-final-5x.log` sha256 `d1c867ed…e5a677` |
| `--test-threads=8` **×2** | **12/12** mỗi lượt; 0,62 s |
| `./vis-dev gate --fast` | **PASS** — check + L0 **3.856 passed / 0 failed** + doc-test; 12/12 `host_parity`; `GATE_RC=0` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **rc=0** (`CLIPPY_RC=0`) |
| Mutation A/B/C/D | đúng hợp đồng §7.2 |
| Tempdir sau tất cả | **0** `/tmp/vis-host-parity-*` |
| R8 / R1 / R2 / R7 | 39/40 · **1.455** dòng · 0 `unsafe` · journal < 50 KB; `./vis-dev rules` OK |

### 7.5 Phán quyết lần 2

- **PASS** — F1/F2/F3/F5/F7/F8/F9/F10 giữ nguyên đóng; **F-R2 đóng trên Linux**; **F-R1 chấp
  nhận làm caveat** (function-level, behavior do `fs_20` bắt); F-R4 khớp.
- **Còn lại**: **R-Win2** chạy `cargo test … host_parity` + gate trên `psvn` để đóng 3-OS; F-R3
  (doc `//`) và F-R6 (1.455/1.500 dòng) là **L không chặn**.
- Không sửa mã; chỉ cập nhật tệp này và commit trên `review/rparity` (không push/merge).
