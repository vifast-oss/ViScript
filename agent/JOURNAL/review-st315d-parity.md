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
