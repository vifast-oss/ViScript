# ST-137 · Sửa rò rỉ TOCTOU đường đọc trên Linux (VD-141)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-137** · HEAD khi sửa **`926022e4`** · commit **`e47cfb2f`** (trước rebase lên `origin/main` là `1097a81c` — **cùng cây** với `e47cfb2f` trừ các commit `[ST-139]` đã có sẵn trên `origin/main`)
· nguồn: [`KE-HOACH-CHOT-23f.md`](../../../KE-HOACH-CHOT-23f.md) §2 ST-137, [`DECISIONS.md`](../DECISIONS.md) **VD-141**,
[`JOURNAL/st137-phantich.md`](st137-phantich.md) (Bước 0), [`reports/R23-viscript.md`](../reports/R23-viscript.md) §4.4 #2.

> **Số trong tệp này là số ĐO**, mỗi số có log + sha256 (§5). Máy đo: macOS 27.0.1 arm64 (mps) và
> `ps` (Ubuntu 26.04.1 LTS, Linux `7.0.0-43-generic`, rustc 1.99.1, lsm có `landlock` ABI 8).

---

## 1. Cửa sổ thật (kết luận Bước 0, nguyên văn ở `st137-phantich.md`)

Cửa sổ **không** nằm giữa "kiểm" và "mở" (ST-124 đã vá chỗ đó) mà **bên trong bước xác nhận sau khi mở**,
vì bước này tự nó là **hai syscall trên tên**:

| Bước | Tệp:dòng (trước sửa) | Vấn đề |
|---|---|---|
| KIỂM trước | `crates/vis-cli/src/quyen.rs:526` → `:405-412` → `:385-396` | `canonicalize(p)` → chuỗi `truoc`; `kiem_canon` trên chuỗi |
| MỞ + đọc | `quyen.rs:527` → `crates/vis-cli/src/ngoai.rs:246-256` | `File::open` → `fstat` → **đọc từ handle** (đúng) |
| XÁC NHẬN sau | `quyen.rs:458` | `sau = canonicalize(p)` — **syscall 1**, trả **CHUỖI** |
| | `quyen.rs:465` | `metadata(sau)` — **syscall 2**, đi theo **symlink hiện tại** |
| | `quyen.rs:466` | so `(dev,ino)` với handle ⇒ chỉ chứng minh *"metadata(tên) trỏ cùng vật với handle"* |
| | `quyen.rs:469` | **quyết định quyền trên CHUỖI của syscall 1** ← chỗ hỏng |

Kẻ tấn công chen giữa `:458` và `:465`: `canonicalize` thấy `root/tep` (tệp TRONG root) nhưng `metadata` theo
symlink mới thấy `ngoai.txt`; handle lại **chính là** `ngoai.txt` (thắng ở bước mở) ⇒ "danh tính trùng",
quyền quyết định trên `root/tep` ⇒ **cho phép** ⇒ trả nội dung `NGOAI`.

Bằng chứng tất định: `st137-cuaso-that.log` (`dcf8b87a…`) — trật tự (A) bị chặn, trật tự (B) **lọt**, và
`/proc/self/fd` chỉ đúng `ngoai.txt`. Đỏ-trước trên `ps`: **92/50 000** (`st137-red-ps.log` `70f6e07f…`;
R23 đo **798/50 000** trên cùng commit — tỉ lệ phụ thuộc lịch trình, không phải hằng số).

---

## 2. Cách sửa: quyết định trên **đường dẫn thật của handle**, đọc từ handle

Nguyên tắc: **handle là nguồn duy nhất** của cả *nội dung* lẫn *định danh vị trí*. Sau khi mở, lấy đường dẫn
thật của chính handle từ kernel (Linux: `/proc/self/fd/<fd>`), kiểm **đường dẫn đó** nằm trong vùng được cấp,
rồi trả nội dung đã đọc **từ handle**. Không `unsafe`, không crate mới.

| # | Tệp:dòng (sau sửa) | Nội dung |
|---|---|---|
| 1 | `crates/vis-interp/src/builtin.rs:122-145`, `:166-178` | **mới** `enum DuongDanHandle { KhongCo, Co(PathBuf), KhongLayDuoc(String) }`, `struct DocQuaHandle { noi_dung, danh_tinh, duong_dan }`, và `FsHost::read_theo_handle` (default = `read_danh_tinh` + `KhongCo` — impl cũ không đổi chữ ký) |
| 2 | `crates/vis-cli/src/ngoai.rs:243-268` | **mới** `doc_theo_handle(p, goc_proc)`: `File::open` → `fstat` (`(dev,ino)` của handle) → **`/proc/self/fd/<fd>` (readlink)** → **`read_to_string` TỪ handle**. Thứ tự có chủ ý: lấy đường dẫn handle **trước** khi đọc nội dung ⇒ `/proc` lỗi là **từ chối ngay**, không kéo nội dung ngoài vào bộ nhớ |
| 3 | `crates/vis-cli/src/ngoai.rs:269-293` | `duong_dan_cua_handle`: `#[cfg(target_os="linux")]` ⇒ `read_link("/proc/self/fd/<fd>")`, lỗi ⇒ `KhongLayDuoc`; nền **không có** `/proc` (macOS) / không unix ⇒ `KhongCo` |
| 4 | `crates/vis-cli/src/ngoai.rs:323-326` | `Runner::read_theo_handle` = `doc_theo_handle(p, "/proc/self/fd")` |
| 5 | `crates/vis-cli/src/main.rs:89-96` | `HostFs::read_theo_handle`: `That` → `Runner`; `Mock` → default (`KhongCo`) |
| 6 | `crates/vis-cli/src/quyen.rs:497-513` | **mới** `Quyen::xac_nhan_handle(p, truoc, d)`: `Co(dd)` ⇒ `kiem_canon(dd)` (**quyết định trên vật đã mở**); `KhongLayDuoc` ⇒ **`VS0604`** (không quay về kiểm theo tên); `KhongCo` ⇒ `xac_nhan_sau_mo` (đường dự phòng ST-124) |
| 7 | `crates/vis-cli/src/quyen.rs:560-566` | `FsGuard::read`: kiểm trước (giữ thông điệp cũ) → `read_theo_handle` (mở **một lần**, đọc từ handle) → `xac_nhan_handle` |
| 8 | `crates/vis-cli/src/quyen.rs:440-487` | `xac_nhan_sau_mo` giữ nguyên hành vi nhưng **chỉ còn là dự phòng**; ghi rõ giới hạn hai-syscall của nó |

**Vì sao chặn được** — với mọi trật tự tấn công, quyết định luôn rơi trên **vật đã mở**:

| Kẻ tấn công đổi tên | Handle | Đường dẫn thật của handle | Kết quả |
|---|---|---|---|
| trước khi mở (thắng bước MỞ) | `ngoai.txt` (N) | `/…/ngoai.txt` — **ngoài root** | **từ chối** `VS0604` (ca `handle_ngoai_root_bi_chan_tat_dinh`) |
| sau khi mở, trước khi kiểm | `TRONG` (inode trong root) | `/…/root/tep` (hoặc `… (deleted)`) — **trong root** | **đọc từ handle ⇒ `TRONG`** (ca `doc_lay_tu_handle_khi_ten_da_bi_doi`) |
| giữa các syscall của bước xác nhận | bất kỳ | kernel trả đúng vật của handle | không còn syscall "theo tên" nào để chen |
| sau khi đã quyết định | — | nội dung đã nằm trong handle | vẫn là vật đã kiểm |

Đường **ghi** giữ nguyên giới hạn đã ghi trong [`HOST-BAO-DAM.md`](../HOST-BAO-DAM.md) (nợ **109**); ST-137 **không**
sửa tệp đó và **không** mở rộng phạm vi sang đường ghi.

---

## 3. Ca mới (4)

| Ca | Nền chạy | Khẳng định |
|---|---|---|
| `handle_ngoai_root_bi_chan_tat_dinh` | Linux | host đổi tên thành symlink **trước khi mở** ⇒ handle = tệp NGOÀI root ⇒ **`VS0604`**, thông điệp nêu "tệp ĐÃ MỞ", **không** trả `NGOAI`. Ca **tất định** cho đúng trật tự đã lọt trước ST-137 |
| `doc_lay_tu_handle_khi_ten_da_bi_doi` | Linux | host đổi tên **sau khi mở+đọc** ⇒ phải trả **`TRONG`** (nội dung **của handle**), không theo tên đã đổi ra ngoài |
| `proc_khong_doc_duoc_thi_tu_choi` | **cả hai** | host báo `KhongLayDuoc` (`/proc` lỗi/không mount) ⇒ **từ chối `VS0604`**, thông điệp nêu `/proc`, **không** trả nội dung — tức **không** quay về cách cũ |
| `doc_theo_handle_bao_dung_duong_dan_cua_proc` | Linux | gọi **thẳng** host thật: `goc_proc` không tồn tại ⇒ `KhongLayDuoc`; `/proc` thật ⇒ `Co(đường dẫn trong root)`, `noi_dung = "TRONG"`, `danh_tinh.0 != 0` |

Trên Linux: **6/6** ca `quyen` xanh; trên macOS: **3/3** (3 ca Linux-only không biên dịch trên macOS).

---

## 4. Chỗ kế hoạch **không áp được nguyên văn** (bắt buộc báo)

Kế hoạch 23f §2 viết: *"/proc không đọc được/không mount ⇒ **từ chối** (`VS0604`), **không** quay về cách cũ"*.

- **Linux**: áp **nguyên văn** — `KhongLayDuoc` ⇒ `VS0604`, có ca `proc_khong_doc_duoc_thi_tu_choi` + ca host thật
  `doc_theo_handle_bao_dung_duong_dan_cua_proc`.
- **macOS**: **KHÔNG thể** áp nguyên văn, vì macOS **không có `/proc`** và std an toàn **không có** primitive fd→path:
  đo được `read_link("/dev/fd/N")` = `EINVAL`, `canonicalize("/dev/fd/N")` = `/dev/fd/<basename>` (đường dẫn giả
  của `fdesc`); `fcntl(F_GETPATH)` cần `libc`/`unsafe ⇒ ngoài VS2 (23f §2 loại). Nếu áp thẳng "không có `/proc`
  ⇒ từ chối" thì **mọi** `fs.read` có `cap` trên macOS bị chặn ⇒ `ok_trong = 0` (đúng cái tiêu chí cấm "xanh vì
  chặn tất") và host suite không thể 54/54.
  ⇒ Trên macOS **giữ bước xác nhận của ST-124** (`DuongDanHandle::KhongCo`), đo **0/50 000** (§5) và ghi **nợ 132**.

Đây là **chỗ duy nhất** lệch kế hoạch; không có cách nào khác trong VS2 mà vẫn đạt tiêu chí "0/50 000 trên
**cả hai** nền + `ok_trong > 0` + host suite 54/54 hai nền".

**Bằng chứng phụ (không hồi quy)**: dưới `--sandbox` (Landlock ABI=1, mức đầy đủ, `luật=1`), `fs.read` trong
`cap[Dir]` **vẫn chạy** (`TRONG`, exit 0) — `readlink` của `/proc/self/fd` **không** bị Landlock chặn (quyền
`READ_FILE` không phủ `readlink`), nên đường mới **không** làm hỏng sandbox:
`bash bin/vis-linux run bash -c '… vis run --sandbox p.vis'` → `TRONG`, `exit=0`, `sandbox: landlock ABI=1 mức=đầy đủ luật=1 nnp=true`.

---

## 5. Số đo (đỏ-trước / sau sửa), log + sha256

### 5.1 Trước sửa (đỏ) — `ps`

`agent/JOURNAL/st137-red-ps.log` · **sha256 `70f6e07ff972d5362e9bfd61d0a4509b0bd2fffaacb3e1ad28a597e8df4dc6f5`**
· HEAD đo **`926022e4`** · lệnh: `bash bin/vis-linux run bash -c 'cd viscript && cargo test -p vis-cli quyen::tests::dua_doi_symlink_lap_co_gioi_han -- --nocapture'`

```
TOCTOU dua: lap=50000 ok_trong=1060 ro_ri=92 chan=48848 vong_doi=84906
assertion `left == right` failed: RÒ RỈ: đọc được nội dung ngoài root 92 lần
```

**92/50 000** (0,18 %) — **ĐỎ**. R23 §4.4 #2 ghi **798/50 000** trên cùng commit; hai số khác nhau vì tỉ lệ lọt
phụ thuộc lịch trình/độ tải (mimic 3 lần: 1/0/0 — `st137-mimic-cuaso.log` `cd713472…`), **không** phải hằng số.
Vì vậy "0/50 000" một lần **không** đủ làm bằng chứng; phải đóng bằng **cơ chế** (§2) + ca tất định (§3).

### 5.2 Sau sửa — cả hai nền

| Nền | Lệnh | Kết quả | Log + sha256 |
|---|---|---|---|
| **Linux (`ps`)** | `cargo test -p vis-cli --bin vis quyen:: -- --nocapture` | **6/6 ca xanh**; ca đua: **`lap=50000 ok_trong=34142 ro_ri=0 chan=15858 vong_doi=76758`** ⇒ **0/50 000**, `ok_trong > 0` | `st137-sau-ps-sachtree.log` `85ebde53…` (worktree sạch, commit đo `1097a81c` = cây của `e47cfb2f`); bản chạy trước đó trên cây chung: `st137-sau-ps.log` `68fa283a…` — **cùng 0/50 000**, `ok_trong=29244` |
| **macOS** | như trên | **3/3 ca xanh**; ca đua: **`lap=50000 ok_trong=13852 ro_ri=0 chan=36148 vong_doi=15412`** ⇒ **0/50 000**, `ok_trong > 0` | `st137-sau-macos.log` `dd51de41…` |

⇒ **0/50 000 trên cả `ps` và macOS**, `ok_trong > 0` cả hai (29 244–34 142 trên `ps`, 13 852 trên macOS) ⇒
không "xanh vì chặn tất".

### 5.3 Cổng

| # | Lệnh | Kết quả | Log + sha256 |
|---|---|---|---|
| 1 | `bash viscript/bin/vis-host-suite` (macOS) | **54/54** | `st137-host-suite-macos.log` `a0f4f7de…` |
| 2 | `VIS_LINUX_DIR=/root/vifast-st137 bash bin/vis-linux run bash -c 'cd viscript && bash bin/vis-host-suite'` (**ps**) | **54/54** | `st137-host-suite-ps.log` `e44e1d51…` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` (macOS) | **`lech=0`** (`tong=100000 loi_tinh=6230 chay=93770`) | `st137-diff-100k.log` `2a660b59…` |
| 4 | `bash viscript/bin/vis-dev gate --fast` (macOS, **worktree sạch** tại **commit đã push `a85193bd`**) | **PASS**, L0 **1159 passed / 0 failed**, `fmt/clippy/luật R1..R4/doc-examples` đều OK | `st137-gate-fast-macos.log` `1bd6aba0…` (chạy lại lần 2 trên HEAD đã push; lần 1 trên `1097a81c` — L0 **1155/0** — cũng **PASS**) |
| 5 | `python3 viscript/test/report/check_report.py --all` | **PASS** (23 báo cáo, 155 hàng QUEUE) | `st137-report-check.log` `b8ef857f…` |
| 6 | `git diff` không `unsafe`, không crate mới | **0/0** | `st137-diff-khong-unsafe.log` `1fd647e7…` |

Ghi chú #4–#6: chạy trên **worktree sạch** (`git worktree add --detach /tmp/vis-st137 …`) tại commit `1097a81c` **và chạy lại lần 2** tại `a85193bd` (HEAD đã push, sau khi rebase lên `origin/main` có các commit `[ST-139]`) — **cả hai PASS**, để cổng đo mã **của ST-137** chứ không đo tệp chưa commit của agent khác
để cổng chỉ đo mã **của ST-137** — cây chung lúc đo còn tệp **chưa commit của agent khác** (ST-134: `bin/vis-linux`,
`tests/pipeline.rs`, `tests/sandbox.rs`; nhánh OS: `v2/kernel/*`), ngoài phạm vi ST-137.

---

## 6. Nợ mới và giới hạn còn lại

- **132 (mới)** — **macOS vẫn dùng bước xác nhận hai-syscall của ST-124** cho `fs.read` có `cap`: không có `/proc`,
  không có primitive fd→path trong std an toàn (`/dev/fd` `readlink` = `EINVAL`; `fcntl(F_GETPATH)` cần `libc`+`unsafe`).
  Đo **0/50 000** nhưng **không** phải bảo đảm theo cơ chế như Linux. Đề nghị VS3 mở đường `libc` (được duyệt
  `unsafe` có kiểm soát) cho `F_GETPATH`, hoặc chuyển toàn bộ sang `openat2`/`O_NOFOLLOW` khi có crate.
- **109** (đã có, ST-124): **đường GHI** vẫn kiểm-rồi-mở; ST-137 **không** đổi. `HOST-BAO-DAM.md` giữ nguyên đoạn
  giới hạn tạm thời (Agent 0 sở hữu; Agent 2 **không** sửa).
- Chấp nhận (như ST-124): tệp có **hardlink** trong root trỏ cùng inode với tệp ngoài root ⇒ đọc qua hardlink
  trong root **được phép** (cùng một inode, có tên trong root tại lúc quyết định).
- `/proc` **phải** đọc được để `fs.read` có `cap` chạy trên Linux (nếu không ⇒ `VS0604`). Đây là **hành vi đúng**
  theo 23f; ghi ở đây để người vận hành container `hidepid`/không mount procfs biết trước.

---

## 7. Tệp đổi

| Tệp | Loại |
|---|---|
| `crates/vis-interp/src/builtin.rs` | `DuongDanHandle`, `DocQuaHandle`, `FsHost::read_theo_handle` (default) |
| `crates/vis-cli/src/ngoai.rs` | `doc_theo_handle`, `duong_dan_cua_handle` (Linux `/proc`, nền khác `KhongCo`), `Runner::read_theo_handle` |
| `crates/vis-cli/src/main.rs` | `HostFs::read_theo_handle` (uỷ quyền `That`/`Mock`) |
| `crates/vis-cli/src/quyen.rs` | `Quyen::xac_nhan_handle` (3 nhánh), `FsGuard::read` dùng nó, `xac_nhan_sau_mo` thành **dự phòng**, 4 ca mới |
| `agent/JOURNAL/st137-*` | phân tích Bước 0, 4 log đo, log cổng, chứng minh không `unsafe` |
