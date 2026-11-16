# ST-137 · Bước 0 — Phân tích cửa sổ TOCTOU đường đọc trên Linux (VD-141)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-137** · ngân sách Bước 0: **≤45 phút** (bắt đầu 15:04 JST, xong 15:44 JST)
· HEAD **`926022e4`** · nguồn: [`KE-HOACH-CHOT-23f.md`](../../../KE-HOACH-CHOT-23f.md) §2 **ST-137**,
[`DECISIONS.md`](../DECISIONS.md) **VD-141**, [`reports/R23-viscript.md`](../reports/R23-viscript.md) §4.4 #2,
[`JOURNAL/st124-review.md`](st124-review.md) §3.

> **Số trong tệp này là số ĐO**, mỗi số có tệp log + sha256 đi kèm (không có số viết tay). Máy đo ghi ở §0.

---

## 0. Môi trường đo

| Nền | Máy | Hệ | Kernel | rustc | Ghi chú |
|---|---|---|---|---|---|
| macOS (mps) | Mac arm64 (T8152) | macOS 27.0.1 · build 26A434 | Darwin 27.0.0 | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | máy hiện tại |
| Linux (**ps**) | 12 core | Ubuntu 26.04.1 LTS | `7.0.0-43-generic` x86_64 | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | `bash bin/vis-linux probe`; lsm có `landlock`, ABI **8**; cargo `/root/.cargo/bin/cargo` |

Cây trên `ps` = `926022e4` (đồng bộ bằng `bin/vis-linux`, `VIS_LINUX_NO_SYNC=1` khi cần).

---

## 1. Chuỗi **kiểm-rồi-mở** hiện tại, theo tệp:dòng

`FsGuard::read` (đường đọc duy nhất của `std::fs.read`) đi qua **hai tầng**:

| # | Bước | Tệp:dòng (tại `926022e4`) | Việc làm (syscall) |
|---|---|---|---|
| 1 | **KIỂM trước** | `crates/vis-cli/src/quyen.rs:526` (`FsGuard::read`) → `:405-412` (`kiem_fs_canon`) | `canonicalize(tuyet_doi(p))` → `PathBuf` **`truoc`**; `kiem_canon` trên **chuỗi** đó (`:416-440`, `starts_with` root của từng `cap`) |
| 2 | (trong) | `quyen.rs:385-396` (`Quyen::chuan`) | `std::fs::canonicalize` (**giải symlink**); lỗi ⇒ canonicalize thư mục cha + ghép tên |
| 3 | **MỞ** | `quyen.rs:527` → `crates/vis-cli/src/ngoai.rs:246-256` (`Runner::read_danh_tinh`, `#[cfg(unix)]`) | `File::open(p)` → `f.metadata()` (**fstat handle**) → `read_to_string` (**đọc từ chính handle**) |
| 4 | **XÁC NHẬN sau** | `quyen.rs:452-478` (`Quyen::xac_nhan_sau_mo`), gọi ở `:529` | `:458` `sau = chuan(tuyet_doi(p))` (**canonicalize lần 2**) → `:465` `metadata(sau)` (**stat lại theo TÊN**) → `:466` so `(dev,ino)` với handle → `:469` `kiem_canon(&sau, …)` (**quyết định quyền trên chuỗi `sau`**) |
| 5 | (dự phòng) | `quyen.rs:471-477` | nếu host **không báo** `(dev,ino)` (`(0,0)`: mock, `--fs-mock`) ⇒ chỉ so `sau != truoc` |

Trait/host: `crates/vis-interp/src/builtin.rs:138-140` (`FsHost::read_danh_tinh`, default `(self.read(p)?, 0, 0)`);
uỷ quyền `crates/vis-cli/src/main.rs:83-87` (`HostFs::read_danh_tinh`: `That` → `Runner`, `Mock` → default).

**Nguồn của mọi thứ quyết định**: `Runner::read_danh_tinh` đã **đọc từ handle** (bước 3) — nội dung **không** bị
đọc lại theo tên. Nên câu hỏi duy nhất còn lại là: *quyết định quyền có áp đúng lên **vật** mà handle trỏ tới không?*

---

## 2. Cửa sổ thật ở đâu

**Cửa sổ không nằm giữa bước 1 và bước 3** (chỗ ST-124 đã vá). Nó nằm **bên trong bước 4**, vì bước 4 tự nó là
một **chuỗi kiểm-rồi-quyết-định hai syscall** trên **tên**:

```
:458  sau = canonicalize(p)     ← syscall 1: trả CHUỖI (có thể là /root/tep, TRONG root)
:465  metadata(sau)             ← syscall 2: đi theo symlink HIỆN TẠI (có thể là ngoai.txt, NGOÀI root)
:466  so (dev,ino) với handle   ← "danh tính TRÙNG" chỉ chứng minh: metadata(sau) trỏ cùng vật với handle
:469  kiem_canon(&sau)          ← nhưng quyền lại quyết định trên CHUỖI của syscall 1
```

Hai syscall ở `:458` và `:465` **có thể thấy hai trạng thái khác nhau của cùng một tên**. Kẻ tấn công chỉ cần
thắng **3 lần đổi tên** đúng chỗ:

| Thời điểm | Trạng thái `root/tep` | Hệ quả |
|---|---|---|
| trước `:526` (kiểm trước) | tệp thật `TRONG` (inode A) | `truoc = root/tep` **trong root** ⇒ qua |
| giữa `:526` và `:527` | **symlink → `ngoai.txt`** | `File::open(p)` mở **inode N** (ngoài root); nội dung đọc từ handle = `"NGOAI"` |
| giữa `:527` và `:458` | tệp thật `TRONG` (inode B, mới) | `sau = root/tep` — **chuỗi TRONG root** |
| giữa `:458` và `:465` | **symlink → `ngoai.txt`** | `metadata(sau)` = **inode N** = handle ⇒ "danh tính trùng" (**đúng**, nhưng trùng với vật NGOÀI) |
| `:469` | — | quyết định quyền trên **chuỗi** `root/tep` ⇒ **cho phép** ⇒ trả `"NGOAI"` |

⇒ Bất biến của ST-124 ("nội dung trả về là nội dung của tệp mà quyết định quyền đã áp") **không còn đúng**:
quyết định quyền áp lên **chuỗi** `root/tep`, còn nội dung trả về là của **inode N** ở `ngoai.txt`.
Đây **không** phải "khe giữa mở và kiểm" của ST-124 §3.6e (khe đó chỉ gây **từ chối oan**), mà là **khe
giữa hai syscall của chính bước xác nhận** ⇒ **rò rỉ thật**.

### 2.1 Chứng minh **tất định** (điều khiển thứ tự, không phụ thuộc may mắn)

`agent/JOURNAL/st137-cuaso-that.log` · **sha256 `dcf8b87a15524ae955324ab2ba2b090e20462324eb811041da721941b542d4e8`**
· chạy trên `ps` · chuỗi syscall **chép đúng** `quyen.rs:525-531` + `:452-478`:

```
(A) đổi tên SAU KHI MỞ, TRƯỚC canonicalize-2  ⇒ chan (đúng)
    handle=(dev 47,ino 574179) đọc từ handle="TRONG" | chuỗi-sau=…/ngoai.txt (trong root=false) | metadata(chuỗi)=(dev 47,ino 574178) trùng handle=false
(B) thắng ở MỞ + đổi tên giữa canonicalize-2 và metadata(chuỗi):
    pre-check=…/root/tep | handle=(dev 47,ino 574178) nội dung="NGOAI"
    chuỗi-sau=…/root/tep (trong root=true) | metadata(chuỗi)=(dev 47,ino 574178) trùng handle=true
    ⇒ kết luận của ST-124: LỌT nội dung NGOAI ra chương trình (quyết định quyền trên CHUỖI trong root)
    /proc/self/fd = …/ngoai.txt  ← đường dẫn THẬT của handle (NGOÀI root) ⇒ hướng sửa /proc thấy đúng sự thật
```

(A) = trật tự mà ST-124 chặn được; (B) = trật tự **lọt**, và `/proc/self/fd` chỉ đúng vật gây lọt.

### 2.2 Đỏ-trước **đo được** trên `ps` (ca thật, không mimic)

`agent/JOURNAL/st137-red-ps.log` · **sha256 `70f6e07ff972d5362e9bfd61d0a4509b0bd2fffaacb3e1ad28a597e8df4dc6f5`**
· lệnh: `bash bin/vis-linux run bash -c 'cd viscript && cargo test -p vis-cli quyen::tests::dua_doi_symlink_lap_co_gioi_han -- --nocapture'`

```
TOCTOU dua: lap=50000 ok_trong=1060 ro_ri=92 chan=48848 vong_doi=84906
assertion `left == right` failed: RÒ RỈ: đọc được nội dung ngoài root 92 lần
  left: 92   right: 0
```

**92/50 000** lần rò rỉ (0,18 %) — **đỏ**, đúng như R23 §4.4 #2 ghi **798/50 000** trên cùng commit.
Hai số khác nhau vì tỉ lệ lọt **phụ thuộc lịch trình/độ tải**, không phải hằng số: mimic (đo 3 lần, 50 000 vòng/lần)
cho `ro_ri` = **1 / 0 / 0** — xem `agent/JOURNAL/st137-mimic-cuaso.log` (sha256
`cd71347225ab6bc731d045d1b75b8bba53b32cff0df3f20008cb845440e013ce`). Vì vậy **"0/50 000" một lần đo không
phải bằng chứng đóng** — phải đóng bằng **cơ chế**, không bằng may mắn.

**Kết luận về cửa sổ**: thủ phạm là `quyen.rs:458`+`:465`+`:469` — canonicalize trả **chuỗi**, rồi `metadata`
**theo tên** được dùng để "xác nhận" danh tính, rồi quyền quyết định trên **chuỗi cũ**. `(dev,ino)` **không** neo
được quyết định vào vật đã mở, vì nó được lấy từ **tên** ở một thời điểm khác.

---

## 3. Bước nào trên Linux khác macOS

**Code y hệt nhau** (`#[cfg(unix)]`) — không có nhánh Linux nào trong đường đọc. Khác biệt **không** ở logic mà ở
**hai điểm**:

1. **Tỉ lệ lọt**: cùng ca, cùng mã, `ps` 92–798/50 000; macOS **0**/50 000 (ST-124 §2.2; R23 §4.4).
   Nguyên nhân đo được: luồng phụ đổi tên trên `ps` đi được **~52 800–84 900 vòng** trong một lần chạy 50 000 lần đọc,
   trên macOS chỉ **~11 600 vòng** ⇒ cùng cửa sổ, trên macOS **ít lần** rơi đúng "bánh kẹp" ở §2 (mimic macOS:
   `vong_doi=11830/11577/11664`, `ro_ri=0`). Nói cách khác: **macOS đang xanh vì tỉ lệ, không vì cơ chế** —
   cùng một dòng code vẫn còn cửa sổ (st124-review §2.2 ghi trước khi vá: macOS **1 661**/50 000, tức cửa sổ
   từng lộ rất rõ trên macOS khi đường kiểm chưa có bước xác nhận).
2. **Công cụ để đóng**: Linux có **`/proc/self/fd/<fd>`** — kernel cho biết **đường dẫn thật của chính handle**
   (`std::fs::read_link` + `AsRawFd`, **không** `unsafe`, **không** crate). macOS **không có** `/proc`; đo thử:
   `read_link("/dev/fd/N")` = `EINVAL`, `canonicalize("/dev/fd/N")` = `/dev/fd/<basename>` (đường dẫn giả của
   `fdesc`, **không** phải đường dẫn thật), và `fcntl(F_GETPATH)` cần `libc`/`unsafe` ⇒ **ngoài VS2**.
   ⇒ Trên macOS **không có** primitive fd→path trong std an toàn.

Vì vậy hướng ưu tiên của kế hoạch **đóng được trên Linux** (bằng chứng: §2.1 dòng `/proc/self/fd` chỉ đúng
`ngoai.txt`), còn trên macOS **không áp được nguyên văn** — xem §4.

---

## 4. Hướng sửa và chỗ kế hoạch **không áp được nguyên văn**

Hướng ưu tiên (23f §2): sau khi **mở handle**, lấy **đường dẫn thật của chính handle** qua `/proc/self/fd/<fd>`,
kiểm đường dẫn đó nằm trong vùng được cấp, **đọc từ chính handle**; `/proc` lỗi ⇒ **từ chối** (`VS0604`).

| Nền | Áp được | Ghi chú |
|---|---|---|
| Linux (`ps`) | **được, nguyên văn** | `/proc/self/fd/<fd>` có thật; đọc link **trước** khi đọc nội dung ⇒ `/proc` lỗi là **từ chối ngay**, không đọc nội dung ngoài vào bộ nhớ |
| macOS (mps) | **KHÔNG nguyên văn** | không có `/proc`; "từ chối khi không đọc được `/proc`" nếu áp thẳng ⇒ **mọi** `fs.read` có `cap` bị từ chối ⇒ `ok_trong = 0` (đúng cái tiêu chí cấm: "xanh vì chặn tất") và host suite không thể 54/54 |

**Cách xử lý (sẽ làm ở phần sửa)**: host báo **đường dẫn thật của handle** theo ba trạng thái —
`Co(path)` (Linux `/proc`), `KhongLayDuoc(lý do)` (host thật **không** lấy được ⇒ **từ chối** `VS0604`),
`KhongCo` (host **không có** không gian tên: `--fs-mock`, và nền **không có** `/proc` như macOS
⇒ giữ bước xác nhận của ST-124, **có đo** 0/50 000 và **ghi thành nợ**). Đây là chỗ **duy nhất** kế hoạch
không áp được nguyên văn; không có cách nào khác trong VS2 vì `fcntl(F_GETPATH)`/`O_NOFOLLOW`/`openat2`
đều cần `libc`+`unsafe` (23f §2 loại).

---

## 5. Kết luận Bước 0 (theo yêu cầu 23f: "không đoán rồi sửa")

1. **Cửa sổ thật**: `quyen.rs:458` (canonicalize → **chuỗi**) + `:465` (`metadata` **theo tên**) + `:469`
   (quyết định quyền trên **chuỗi cũ**). `(dev,ino)` của handle chỉ chứng minh "metadata tên trỏ cùng vật với
   handle", **không** chứng minh "chuỗi đã kiểm trỏ cùng vật với handle".
2. **Tái hiện**: tất định (A/B ở §2.1) + đo thật **92/50 000** trên `ps` (§2.2, có log + sha256).
3. **Linux vs macOS**: cùng mã; macOS ít rơi đúng bánh kẹp hơn (đo: 11,6k vs 52,8–84,9k vòng đổi tên);
   macOS thiếu primitive fd→path trong std an toàn.
4. **Hướng ưu tiên đóng được trên Linux** (bằng chứng `/proc/self/fd` = `ngoai.txt` trong đúng lần lọt);
   trên macOS phải giữ bước xác nhận cũ + ghi nợ, báo rõ ở báo cáo ST-137.
5. **Không** cần `unsafe`, **không** crate mới ⇒ **tiếp tục sửa** (không BLOCKED).
