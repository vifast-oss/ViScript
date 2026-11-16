# ST-124 · Review FsGuard TOCTOU + `Value::Cap` (VD-130a/b)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-124** · ngân sách 3 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 (`§10(a) Value::Cap`, `§10(b) FsGuard
TOCTOU`), §6 **ST-124**, §7 **Phụ lục D2**, [`DECISIONS.md`](../DECISIONS.md) **VD-130(a)(b)**.

Đây là task **phân tích có ca**, sản phẩm chính là tệp này. Kết luận có **số** ở §2.5 và §3.8.

---

## 1. FsGuard — chuỗi **kiểm-rồi-mở** hiện tại (trước ST-124)

| # | Bước | Tệp:dòng (trước) | Việc làm |
|---|---|---|---|
| 1 | KIỂM | `crates/vis-cli/src/quyen.rs:401-404` (`FsGuard::read`) | `self.quyen.kiem_fs(p, false)?` — **kiểm trước** |
| 2 | KIỂM (trong) | `quyen.rs:399-427` (`Quyen::kiem_fs`) | `tuyet_doi(p)` → `Self::chuan` = `std::fs::canonicalize` (**giải symlink**) → `starts_with(root)` của từng `cap[Dir]/cap[File]` |
| 3 | MỞ | `quyen.rs:404` → `crates/vis-cli/src/ngoai.rs:247-250` (`Runner::read`) | `std::fs::read_to_string(p)` — mở **lại từ đường dẫn gốc** `p`, **theo symlink** |

⇒ Khe TOCTOU: giữa bước 2 và bước 3, nếu kẻ tấn công **ghi được vào thư mục cha**, họ thay
`p` (hoặc một thành phần của `p`) bằng **symlink** trỏ ra ngoài root. Bước 2 đã kiểm trên đường dẫn
**cũ**, bước 3 lại mở theo **đường dẫn mới** ⇒ nội dung ngoài root lọt ra.

Chuỗi này áp cho **mọi** thao tác đọc: `read` (nội dung), còn `exists_quyen`/`list`/`list_entries`
**không** trả nội dung tệp (chỉ tên/metadata) nên rủi ro thấp hơn — xem §2.6.

## 2. Tái hiện (đỏ trước) — log + sha256

`agent/JOURNAL/st124-toctou-red.log` · **sha256 `9e1897f9a69f5aae6e74e2f7e48838a183a9f1444b72fc21898f3af20eda269`**
· HEAD `4c87cd59` · Darwin 27.0.0 arm64.

Hai ca (module `#[cfg(test)]` trong `crates/vis-cli/src/quyen.rs`, gọi **đúng** `FsGuard` + `Runner`
thật — không dựng lại logic):

### 2.1 Ca **tất định** (`doi_symlink_dung_luc_mo_bi_chan`)

Host giả `HostDoi` **đổi symlink ngay bên trong `read`**: nó thay `root/tep` bằng symlink trỏ
`ngoai.txt` (ngoài root) **rồi mới mở** ⇒ mô phỏng đúng khe "kiểm xong, chưa mở" **không** phụ thuộc
thời gian thật (không `sleep`).

- **Trước sửa**: `phải CHẶN đọc tệp ngoài root, nhưng đọc được: Ok("NGOAI")` ⇒ **tái hiện 1/1**.
- **Sau sửa**: ca xanh (bị chặn).

### 2.2 Ca **đua thật** (`dua_doi_symlink_lap_co_gioi_han`) — luồng phụ, lặp có giới hạn

Luồng phụ `rename` **nguyên tử** `root/tep` giữa "tệp trong root (`TRONG`)" và "symlink → `ngoai.txt`
(`NGOAI`)"; luồng chính đọc **50 000** lần. Không phụ thuộc **thời gian tuyệt đối** (chỉ đếm vòng).

| | `lap` | `ok_trong` | **`ro_ri`** | `chan` | `vong_doi` |
|---|---|---|---|---|---|
| **Trước sửa** | 50 000 | 20 564 | **1 661** | 27 775 | 12 836 |
| **Sau sửa** | 50 000 | 11 980 | **0** | 38 020 | 16 793 |

⇒ Tỉ lệ rò rỉ trước sửa **3,32 %** (1661/50 000). Sau sửa **0/50 000**. `ok_trong > 0` ở cả hai lần
⇒ ca **không** "xanh vì chặn tất".

## 3. Cách sửa **không cần `unsafe`** (đã làm) — mở-rồi-kiểm

Nguyên tắc: **mở tệp trước**, giữ **handle** (handle là thứ quyết định nội dung sẽ trả về), rồi
**kiểm trên đường dẫn tại thời điểm sau khi mở** và **xác nhận danh tính** handle ↔ đường dẫn.

| # | Tệp:dòng (sau) | Nội dung |
|---|---|---|
| 1 | `crates/vis-interp/src/builtin.rs:131-138` | **mới** `FsHost::read_danh_tinh(p) -> Result<(String, u64, u64), String>` — như `read` nhưng trả kèm `(dev, ino)` của **handle đã mở**; default = `(self.read(p)?, 0, 0)` (host cũ/mock không đổi chữ ký) |
| 2 | `crates/vis-cli/src/ngoai.rs:252-270` | **mới** `Runner::read_danh_tinh` (`#[cfg(unix)]`): `File::open` → `f.metadata()` → `read_to_string` ⇒ `(nội dung, dev, ino)` của **chính handle** |
| 3 | `crates/vis-cli/src/main.rs:94-104` | `HostFs::read_danh_tinh` uỷ quyền: `That` → host thật; `Mock` → default `(0,0)` |
| 4 | `crates/vis-cli/src/quyen.rs:398-420` | tách `kiem_fs` → **`kiem_fs_canon`** (trả **đường dẫn canonicalize đã kiểm**) + **`kiem_canon`** (quyết định quyền trên một đường dẫn canonicalize, dùng chung cho kiểm trước và sau) |
| 5 | `crates/vis-cli/src/quyen.rs:422-470` | **mới** `Quyen::xac_nhan_sau_mo(p, truoc, (dev, ino))`: đọc lại `sau = chuan(p)`; nếu có danh tính ⇒ đòi `metadata(sau)` **trùng** `(dev, ino)` **rồi kiểm quyền trên `sau`**; lệch ⇒ `VS0604` ("bị thay đổi trong khi mở"). Không có danh tính (mock/Windows) ⇒ so `sau != truoc` |
| 6 | `crates/vis-cli/src/quyen.rs:509-527` | `FsGuard::read` = kiểm trước (giữ nguyên thông điệp lỗi cũ, **không mở** tệp đã biết ngoài quyền) → **mở một lần** lấy `(nội dung, dev, ino)` → `xac_nhan_sau_mo` |

**Vì sao chặn được** (đúng thứ tự thời gian):

1. Kẻ tấn công đổi `p` → `ngoai.txt` **trước** khi mở: handle = `ngoai.txt`; `sau` = `ngoai.txt`;
   `metadata(sau)` **trùng** handle ⇒ kiểm quyền trên `sau` = **ngoài root** ⇒ `VS0604`. Nội dung
   `ngoai.txt` đã đọc vào bộ nhớ nhưng **không** trả về chương trình.
2. Đổi **sau** khi mở, rồi để nguyên: handle = tệp trong root; `sau` = `ngoai.txt`; `metadata(sau)`
   **khác** handle ⇒ `VS0604` ("bị thay đổi trong khi mở").
3. Đổi **sau** khi mở rồi **đổi lại**: `sau` = tệp trong root, danh tính **trùng** handle ⇒ quyền
   hợp lệ **và** nội dung trả về là của **handle** (tệp trong root) — không lộ gì, vì đọc từ handle
   chứ không đọc lại đường dẫn.
4. Đổi **sau** bước xác nhận: nội dung đã nằm trong handle ⇒ vẫn là tệp đã kiểm.

⇒ Đây là bất biến "**nội dung trả về luôn là nội dung của tệp mà quyết định quyền đã áp**", đạt được
**không `unsafe`** (chỉ `MetadataExt::dev/ino`), không crate mới, không đổi API công khai của ngôn ngữ.

### 3.5 Không hồi quy

`cargo test` các bộ liên quan (chạy thật trên macOS): `quyen` 4/4, `st83_no_nho` 5/6 (1 ca **đỏ do
task khác**, xem §5), `strict_caps` 7/7, `cap` 6/6, `cap_proc` 6/6, `host_conformance` 54/54,
`extern` 13/13, `span_tep` 5/5, `grant_proc_canon` 7/7, `cap_khong_lo` 7/7.

### 3.6 Giới hạn **còn lại** (ghi để Agent 0 quyết Phụ lục D2)

| # | Giới hạn | Vì sao chưa sửa | Mức |
|---|---|---|---|
| a | **Đường GHI** vẫn kiểm-rồi-mở (`FsGuard::write` → `kiem_fs` → `host.write` = `std::fs::write` mở `create+truncate`) | Mở trước để lấy handle là **đã tạo/đã truncate** tệp ⇒ không thể "mở rồi mới kiểm" như đọc. Sửa cần `O_NOFOLLOW`/`openat` theo handle (cần `unsafe` hoặc crate) ⇒ **ngoài VS2** | nợ **#115** |
| b | Host **không báo** danh tính (mock `--fs-mock`, Windows): chỉ còn phép so `sau != truoc` ⇒ chặn được ca "đổi rồi để nguyên", **không** chặn ca "đổi rồi đổi lại" | Không có `(dev, ino)` trên nền đó trong code an toàn; `--fs-mock` là host giả (không có tệp thật) | đã ghi |
| c | Tệp có **hardlink** trong root trỏ cùng inode với tệp ngoài root: đọc qua hardlink trong root **được phép** | Đó **là cùng một tệp**; inode nằm trong root nên quyết định là nhất quán | chấp nhận |
| d | `exists_quyen`/`list`/`list_entries` không đổi (vẫn kiểm-rồi-mở) | Không trả **nội dung** tệp; rò rỉ tối đa là "có tồn tại / tên mục" — cùng mức với `exists` trước ST-124 | chấp nhận |
| e | Khe giữa `read_danh_tinh` (mở) và `xac_nhan_sau_mo` (`canonicalize`+`metadata`) vẫn tồn tại | Nhưng nội dung **đã** đọc từ handle ⇒ khe này **không** làm lộ nội dung khác; nó chỉ có thể làm **từ chối oan** | chấp nhận |

**Đề nghị cho `HOST-BAO-DAM.md`** (Agent 0 quyết, Agent 2 **không** sửa tệp đó): cần **một** dòng
giới hạn cho đường **GHI** (mục a) — không cần dòng nào cho đường đọc nữa, vì đọc đã đóng TOCTOU
trong code an toàn. Văn bản Phụ lục D2 hiện tại chỉ nói về "kiểm đường dẫn rồi mới mở" chung chung;
nếu dán nguyên văn thì **sai** với trạng thái mới của đường đọc ⇒ đề nghị Agent 0 soạn lại D2 cho
đường ghi.

## 4. `Value::Cap` — chứng minh bằng ca (VD-130a)

Tệp ca: `crates/vis-cli/tests/cap_khong_lo.rs` (**7 ca**, chạy cả interp và `--vm` khi ca chạy được;
ca chẩn đoán dùng `vis check`). Ngữ nghĩa nền (đọc code):

- `Value::Cap` **chỉ** do CLI dựng từ `cap` khai trong **CST** của `main`: `quyen.rs:196-215`
  (`Quyen::gia_tri`) ← `main.rs:275` (`quyen::tao`) ← `quyen::ds_cap` đọc **văn bản nguồn**.
- Cưỡng chế đọc từ **`Quyen`** (`FsGuard { quyen }`), **không** bao giờ đọc `Value::Cap` ⇒ giá trị
  kiểu `cap[...]` **không** phải nguồn quyền.

| # | Mục | Ca | Kết quả (đo được) |
|---|---|---|---|
| 1 | **`json.decode`** | `json_decode_khong_tao_cap` | `json.decode("\{…kind/root/write…\}")` ⇒ `Ok(map)`; `to json` in `{"ok":{"kind":"Dir","root":"/","write":true}}` — **dữ liệu**, không có cờ quyền |
| 1b | `json.decode` → biến `cap[Dir]` | `gan_json_vao_bien_cap_khong_sinh_quyen` | tyck **chấp nhận** phép gán (nợ **#114**), nhưng **không** sinh quyền: `fs.read("/etc/hosts")` + `--strict-caps` ⇒ **VS0604 exit 2** |
| 2 | **Đọc tệp** | `doc_tep_khong_tao_cap` | tệp chứa `cap[Dir] @ p"/"` ⇒ `fs.read` trả **`str`** thô `cap[Dir] @ p"/"\n`; chuỗi đó **không** mở quyền: đọc `/etc/hosts` ⇒ **VS0604 exit 2** |
| 3 | **Đối số dòng lệnh** | `doi_so_dong_lenh_la_str` | interp: `main(xs: list[str])` nhận `[cap[Dir] @ p"/"]` — **chuỗi trong danh sách**; `--vm`: nhận `()` (lệch **ngoài ST-124**, nợ **#113**) ⇒ **không** đường nào là `Cap` |
| 4 | **`to json`** | `to_json_cap_ra_null` | `json.encode(cap)` ⇒ `null` (nhánh `_ => null` của `json_tai`, `vis-interp/src/stdlib.rs:716`) — **không** lộ `kind`/`root` |
| 5 | **`print`** | `in_cap_la_loi_tyck` | **VS0412** "`cap` không in được (`cap[Dir]`)" — tyck chặn, không in gì |
| 6 | **So sánh** | `so_sanh_cap_la_loi_tyck` | **VS0412** "`cap` không so sánh được (`cap[Dir]`)" — `==` trên cap bị chặn ở tyck |

### 4.8 Kết luận `Value::Cap`

- **Tạo từ dữ liệu ngoài**: **0/4** đường (json.decode, đọc tệp, đối số, và cả `reader` khác) tạo được
  `Value::Cap`; cả 4 đường chỉ ra `Map`/`Str`/`List(Str)`/`Unit`. Cơ chế: quyền đọc từ `Quyen` (CST),
  không từ giá trị ⇒ **không thể** leo quyền bằng dữ liệu.
- **Lộ quyền**: `print` và `==` bị **tyck** chặn (VS0412); `to json` ra `null`; chuỗi hoá duy nhất còn
  lại là `Show` nội bộ (`<cap[Dir] root>`) dùng cho **thông điệp lỗi của chính runtime**, và `root`
  trong đó là **đường dẫn người dùng đã viết trong nguồn** (không phải bí mật, không phải handle).
- **Rủi ro tương lai**: nếu sau này cưỡng chế đọc `Value::Cap` thay vì `Quyen` thì phép gán `Any` →
  `cap[Dir]` (nợ #114) sẽ thành lỗ hổng thật ⇒ phải vá trước.

## 5. Log đỏ **ngoài phạm vi** (không sửa — tệp của task khác)

`agent/JOURNAL/st124-ngoai-pham-vi.log` (sha256 `b7b635c3b508c716b862b3c7e48f82d23e68cc12566f3e93f815d69e46bd7638`):

- `crates/vis-cli/tests/st83_no_nho.rs::explain_help_theo_lang` **FAILED**: kỳ vọng `"7 blocks, 32 codes"`,
  thực tế `"7 blocks, 33 codes"` ⇒ **task khác** (ST-128, thêm mã chẩn đoán mới) chưa cập nhật ca này.
  **Không** sửa (tệp + mã chẩn đoán thuộc task đó); ghi tên + log rồi tiếp.
- Trong lúc chạy ca đỏ, cây chung **không biên dịch được** vì `crates/vis-cli/src/pkg.rs` +
  `pkg/semver.rs` đang được agent khác sửa (`E0432/E0433` trong `#[cfg(test)]`) ⇒ ca đỏ của ST-124
  chạy trong **bản sao sạch của HEAD** (`git archive HEAD` → `/tmp/vis-st124`), đúng mã nguồn trước
  khi sửa; sau đó chạy lại **trong cây chung** khi cây xanh.

## 6. Tệp đổi

| Tệp | Loại |
|---|---|
| `crates/vis-interp/src/builtin.rs` | **mới** `FsHost::read_danh_tinh` (default) |
| `crates/vis-cli/src/ngoai.rs` | **mới** `Runner::read_danh_tinh` (`cfg(unix)`): mở một lần, trả `(dev, ino)` |
| `crates/vis-cli/src/main.rs` | uỷ quyền `HostFs::read_danh_tinh` |
| `crates/vis-cli/src/quyen.rs` | tách `kiem_fs_canon`/`kiem_canon`; **mới** `xac_nhan_sau_mo`; `FsGuard::read` mở-rồi-kiểm; **mới** 2 ca `#[cfg(test)]` (tất định + đua) |
| `crates/vis-cli/tests/cap_khong_lo.rs` | **mới**: 7 ca `Value::Cap` |
| `agent/JOURNAL/st124-review.md` (tệp này), `st124-toctou-red.log`, `st124-ngoai-pham-vi.log`, `st124-gate-fast.log`, `st124-diff-100k.log` | phân tích + log |

## 7. Nợ mới (đánh số tiếp từ **112**, ST-124 dùng **113**)

- **113** — **VM không đưa `argv` vào tham số không-`cap` của `main`**: interp đặt `argv` = `list[str]`
  (`vis-interp/src/interp.rs:222-241`), VM đặt `()` ⇒ `fn main(xs: list[str])` + `print(xs.len())` in
  `1` ở interp và `VS0501: () không có phương thức len` ở `--vm`. Phát hiện khi làm ca §4 #3; **ngoài**
  ST-124 nên **không** sửa. Ca `doi_so_dong_lenh_la_str` ghim đúng sự lệch này.
- **114** — **tyck chấp nhận `let e: cap[Dir] = json.decode(...)`** (gán `V`/Any ⇒ kiểu `cap[...]`).
  Hiện **vô hại** vì cưỡng chế đọc từ `Quyen` (CST), không từ `Value::Cap`; nhưng nếu sau này có chỗ
  đọc `Value::Cap` để quyết định quyền thì thành lỗ hổng leo quyền. Cần Agent 0 chốt: cấm gán
  `Any → cap[...]` ở tyck, hoặc giữ và ràng buộc "quyền chỉ từ CST" thành luật có test.
- **115** — **đường GHI vẫn kiểm-rồi-mở** (`FsGuard::write`): đổi symlink giữa `kiem_fs` và
  `std::fs::write` có thể truncate/ghi **ngoài root**. Không sửa được theo kiểu mở-rồi-kiểm (mở để
  ghi = đã tạo/đã truncate) ⇒ cần `O_NOFOLLOW`/`openat`/`fstat`-theo-handle = `unsafe` hoặc crate ⇒
  đề nghị Agent 0 ghi **một** dòng giới hạn đường ghi vào `HOST-BAO-DAM.md` ở đợt 24.

## 8. Chỗ không áp được nguyên văn (so với quyết định Agent 0)

- **lệch**: §6 ST-124 "thử tái hiện ... bằng luồng phụ" → **thực tế**: ca **đua thật** (luồng phụ,
  50 000 vòng) chỉ rò rỉ **3,32 %** số lần nên **không** đủ làm ca hồi quy ổn định; vì vậy có **thêm**
  ca **tất định** (host giả đổi symlink ngay trong `read`) làm ca chặn hồi quy chính, ca đua giữ vai
  trò **số đo**. Cả hai đều chạy trong `gate --fast`.
- **lệch**: §7 Phụ lục D2 "chỉ dùng trong ST-124 nếu TOCTOU **cần `unsafe`** để sửa" → **thực tế**:
  đường **đọc** sửa được **không** `unsafe` ⇒ **không** dán D2 cho đường đọc; còn **đường ghi** vẫn
  cần ⇒ đề nghị Agent 0 soạn một dòng riêng cho đường ghi (nợ #115). Agent 2 **không** sửa
  `HOST-BAO-DAM.md` (đúng luật).
- **lệch**: §6 ST-124 "Value::Cap ... `print`, so sánh **không** tạo và **không** lộ `Cap`" → **thực
  tế**: `print`/`==` **không phải** chặn ở runtime mà ở **tyck** (VS0412) ⇒ hai ca là ca chẩn đoán tĩnh
  (`vis check`), không phải ca chạy. Vẫn là "không lộ" (mạnh hơn: không biên dịch được).
