<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# PATH-SPLIT-AUDIT — Chỗ tách đường dẫn thủ công (VD-184(3), #149) — 2026-11-18

**Trạng thái: CHỈ LIỆT KÊ — KHÔNG SỬA.** Sửa thuộc task **ST-211 (VS3-5-pre)**, sau khi đóng pha 1 (VD-184(4)). Nợ đính kèm: **`agent/SO-NO.md` §4 #149** (dòng 229, trạng thái **mở** — "danh sách chốt ở task VS3-5-pre; sửa dần khi đụng tệp"). Nguồn luật: **VD-184** (17/11) — (2) `check.rs:82` là **ngoại lệ có tên** cho VD-146, sửa bằng `Path::file_name()`; (3) **liệt kê mọi chỗ tách đường dẫn thủ công** thành nợ, **không sửa** trong đợt này; (5) ca khoá tương đối/tuyệt đối/`C:\…`/`C:/…`/tên tệp chứa `\` trên hai nền + psvn self-test 12/12.

- **Ngày đo**: 2026-11-18 · **trên**: `dem/C` (nền `a9c38ad` + ledger Agent 1 `e06ffb5`; chỉ thêm **docs/** ở C1 — `crates/` không đổi); **`crates/` của `dem/C` đồng nhất với `main` (`733b1e2`)** (`git diff e06ffb5 main -- crates/` rỗng, kiểm 2026-11-18) ⇒ kết luận áp dụng cho cả `main`.
- **Phương pháp**: grep máy (`python3` + `ripgrep`-style) toàn `crates/` (`src/` + `tests/`), các mẫu ở §3 bên dưới; đối chiếu bằng tay mọi hit. **0 hit** của các mẫu yêu cầu được ghi rõ (không bỏ sót).
- **Trả lời câu hỏi VD-184/#149 về `check.rs`**: **DÒNG `rsplit('/')` TƯƠNG KHẮC VẪN CÒN** — nhưng KHÔNG phải dòng 82 cũ: **`crates/vis-cli/src/cli/check.rs:191`** (nhánh `vis check --json` ⇒ `check_all`), chưa được ST-153 sửa. Dòng 82 cũ đã được **ST-153** (commit `2287bad`, VD-152) thay bằng `ten_goc` (tách cả `/` và `\`). Chi tiết ở §4.1.
- **Kết luận chung**: ngoài `check.rs:191`, **không còn chỗ tách `'/'`/`'\'` cho đường dẫn tệp hệ thống** nào khác trong `crates/` khớp mẫu (§5 liệt kê đầy đủ hit còn lại và lý do chúng không phải tách path hoặc mức thấp). **Có** các hit "phương thức tự viết" tên khả nghi (`ten_goc`, `duong_dan`, `split_path` — 0 hit, `fn path_*` — 0 hit, `fn file_name` — 0 hit) và các **trợ giúp path thuần chuỗi** trong `std.path` (interp) — xem §4.2. Tách path **trong URL/diagnostics/registry** được liệt kê riêng ở §5 (không sửa, không mở nợ mới — đã là nợ #149 hoặc không phải path).

## 1. Cách tái hiện (câu lệnh)

```bash
cd <worktree>
# Mẫu yêu cầu (VD-220 C5):
grep -rn "\.split('/')"   crates --include=*.rs
grep -rn '\.split("/")'   crates --include=*.rs
grep -rn "\.split('\\\\')" crates --include=*.rs
grep -rn "\.rsplit('/')"  crates --include=*.rs
grep -rn '\.rsplit("/")'  crates --include=*.rs
grep -rn "\.rsplit('\\\\')" crates --include=*.rs
grep -rn "\.split(':')\|\.rsplit(':')\|\.split(\":\")\|\.rsplit(\":\")" crates --include=*.rs
grep -rn "Path::from(\|PathBuf::from(" crates --include=*.rs
grep -rn "\.nth_back("     crates --include=*.rs
grep -rn "trim_start_matches('/')\|trim_start_matches(\"/\")" crates --include=*.rs
# Mở rộng (nghi ngờ "tự viết"):
grep -rn "\.split_once('/'\|\.rsplit_once('/'\|\.split(\['/'\|\.rsplit(\['/'" crates --include=*.rs
grep -rn "fn ten_\|fn duong_\|fn split_path\|fn path_\|fn file_name" crates --include=*.rs
grep -rn "\.splitn(\|\.rsplitn(" crates --include=*.rs
grep -rn "replace('\\\\', \"/\")" crates --include=*.rs
grep -rn 'find(\[.\x27/.\x27,' crates --include=*.rs
```

(Cách chính xác đã chạy: `python3` với regex tương đương — hit trùng nhau; `Path::from(s.split(...))` = **0 hit**, `.nth_back(0)`/`.nth_back(` = **0 hit**, `trim_start_matches('/')`/`("/")` = **0 hit**, `split('/')` = **1 hit** (URL, §5), `rsplit('/')` = **1 hit** (`check.rs:191`), `rsplit(\"/\")` = **0 hit**, `split('\\')`/`rsplit('\\')` = **0 hit**, `split(\"\\\\\")`/`rsplit(\"\\\\\")` = **0 hit**.)

## 2. Liên quan

- **#149** — `agent/SO-NO.md` §4 dòng 229 (mở) · **VD-184** · **VD-146** · **VD-204** · **ST-153** (`2287bad`, VD-152: `ten_goc` tách `rsplit(['/', '\\'])` — sửa chỗ thuộc VD-184) · **ST-211 (VS3-5-pre)** — **chưa làm** (VD-220 §5: chỉ làm sau khung đo 07:30 nếu còn giờ; làn C **không** làm).
- VD-146: "mọi chỗ khác tự tách `'/'`/`'\\'` phải liệt kê thành nợ, **không sửa** trong đợt này".
- Cách gọi ST-211 dự kiến (gợi ý, **không** thực hiện): thay `check.rs:191` bằng `Path::file_name()` giống `ten_goc`; xét `std.path` (§4.2); giữ nguyên các hit URL/diagnostics (không phải path).

## 3. Bảng (tệp | dòng | mẫu tách | mức nghi ngờ | lý do | ghi chú — sửa ở #149)

### 3.1 Chỗ **đường dẫn tệp hệ thống** tự tách — **ứng viên ST-211**

| tệp | dòng | mẫu tách | mức nghi ngờ | lý do | ghi chú (sửa ở #149) |
|---|---|---|---|---|---|
| `crates/vis-cli/src/cli/check.rs` | **191** | `path.rsplit('/').next().unwrap_or(path).to_string()` — fallback `root` khi `.vish` có `FileUnit.rel` rỗng (`check_all`; phục vụ `vis check --json` / `run_check_json`) | **cao** | cùng lớp lỗi VD-146/#149: chỉ tách `'/'`, không `'\\'`; trên Windows `root` = **cả đường dẫn gốc** ⇒ `phu` luôn khác rỗng ⇒ ở `--syntax` (`kiem_kieu = false`) bộ lọc giữ **mọi** chẩn đoán (kể cả mã không VS01/02/06/07) ⇒ chẩn đoán kiểu/hoc sai tệp, cùng lớp mà VD-184 từng khoá ở `check_mode` (ST-153) nhưng **không** sửa chỗ này | Thay bằng `Path::new(path).file_name()` (đúng khuyến nghị VD-184(2)) hoặc dùng `ten_goc` (dòng 64) — **đổi hành vi nhẹ**: trả tên tệp ASCII thay vì nguyên đường dẫn khi không có `/` (hiện `'/'`-only giữ nguyên cả đường dẫn). Ca khoá #149 (VD-184(5)) cần phủ `--json` |
| `crates/vis-interp/src/stdlib.rs` | **725–730** (`noi_duong`) | `format!("{a}/{b}")` + `trim_end_matches('/')` + `starts_with('/')` | **trung bình** | tự ghép path bằng chuỗi; không tách `'/'` mà **dùng** `/` trực tiếp; không xử lý `\\`; nếu host là Windows và input là path `C:\…` thì kết quả sai phân cách | `std.path.join` — **thuần chuỗi, không chạm đĩa** (doc mục "std.path (thuần chuỗi, không chạm đĩa)"); vẫn là "tự xử lý phân cách"; nên xét khi sửa #149 (dùng `Path` hoặc giữ nguyên có chủ đích + ghi rõ trong doc) |
| `crates/vis-interp/src/stdlib.rs` | **733–739** (`basename`) | `p.trim_end_matches('/')` + `p.rsplit_once('/')` | **trung bình** | tự tách `'/'` cho `std.path.basename`; không nhận `\\` | như trên |
| `crates/vis-interp/src/stdlib.rs` | **741–753** (`dirname`) | `p.trim_end_matches('/')` + `p2.rsplit_once('/')` | **trung bình** | tự tách `'/'` cho `std.path.dirname`; không nhận `\\` | như trên |
| `crates/vis-interp/src/stdlib.rs` | **755–761** (`ext`) | gọi `basename` + `rsplit_once('.')` | **trung bình** | tách đuôi qua basename chuỗi; không path | như trên |
| `crates/vis-interp/src/stdlib.rs` | **763–770** (`stem`) | gọi `basename` + `rsplit_once('.')` | **trung bình** | như trên | như trên |
| `crates/vis-interp/src/stdlib.rs` | **345–362** | handler `"path.join"`, `"path.basename" | "path.dirname" | "path.ext" | "path.stem"`, `"path.is_abs"` ⇒ gọi các hàm trên | **trung bình** | điểm cắm của nhóm trên; đánh dấu để ST-211 không bỏ sót khi sửa | như trên |

### 3.2 Đã sửa ngoại lệ — giữ để đối chiếu (không sửa)

| tệp | dòng | mẫu tách | mức nghi ngờ | lý do | ghi chú (sửa ở #149) |
|---|---|---|---|---|---|
| `crates/vis-cli/src/cli/check.rs` | **64–67** | `path.rsplit(['/', '\\']).next().unwrap_or(path)` — **`ten_goc`** (ST-153, VD-152) | thấp (đã duyệt) | tách **cả** `/` và `\\`; thay cho `rsplit('/')` ở dòng 82 cũ; xử lý đúng gốc VD-146 | **Không sửa** (ST-153 đã đóng); ST-211 nên dùng lại hàm này thay vì viết thêm chỗ mới |
| `crates/vis-cli/src/cli/check.rs` | **364–381** | `ten_goc_windows` / `ten_goc_unix` / `ten_goc_khong_co_dau` (unit tests) | thấp | ca khoá ST-153 (VD-152) | giữ nguyên |

### 3.3 Xử lý phân cách **tay nhưng không phải tách path** — ghi để đầy đủ (không sửa, không mở nợ)

| tệp | dòng | mẫu | mức nghi ngờ | lý do | ghi chú (sửa ở #149) |
|---|---|---|---|---|---|
| `crates/vis-cli/src/modules.rs` | **144, 147, 169** | `.to_string_lossy().replace('\\', "/")` | thấp | **chuẩn hoá** phân cách cho `rel` (đường cache-key cho chẩn đoán multi-file) — cố ý dùng `/` như phân cách chuẩn trong đường trong diagnostics; không phải tách | giữ; ST-211 nên tái sử dụng khi cần so khớp đường dẫn |
| `crates/vis-cli/src/modules.rs` | **523** | `if raw.contains('/')` → `vec![raw]`, ngược lại `raw.split('.')` | thấp | giữ đường tương đối (`./a/b`) nguyên **một đoạn** thay vì tách `.` — **không** tách `/` | giữ |
| `crates/vis-types/src/lower/khai_bao.rs` | **110** | `if raw.contains('/')` → `vec![raw]`, ngược lại `.split('.')` | thấp | như trên (ở tầng hạ HIR) | giữ |
| `crates/vis-cli/src/quyen.rs` | **315** | `if !v.contains('/')` ⇒ dò `PATH` (như `execvp`); ngược lại ghép `goc` | thấp | nhánh PATH-lookup — **không** tách path | giữ |
| `crates/vis-cli/src/pkg.rs` | **806** | `spec.rsplit_once('@')` | thấp | tách `name@version` của registry — không phải path | giữ |
| `crates/vis-cli/src/vistest.rs` | **150** | `.rsplit(':').take(2)` | thấp | parse chẩn đoán `file:line:col` — không phải path (dù chạm vào đường `'/'` trong path cũng không quan tâm vì chỉ lấy 2 số cuối) | giữ |
| `crates/vis-vm/src/vm.rs` | **729** | `duong.rsplit_once('.')` | thấp | tên enum `module.Variant` — không phải path | giữ |
| `crates/vis-vm/src/vm.rs` | **847** | `ten.rsplit('.').next()` | thấp | tên biến/biến thể — không phải path | giữ |
| `crates/vis-types/src/lower/cmd.rs` | **80, 173** (`parse/cmd.rs`) | `ten.split(':')`, `phan.split(':')` | thấp | chữ ký khai báo (tham số `var x: int`…) — không phải path | giữ |
| `crates/vis-cli/src/pkg/semver.rs` | **63** | `s.split('.')` | thấp | tách semver — không phải path | giữ |
| `crates/vis-types/src/lower/khai.rs` | **159** | `t.split('#').next()` | thấp | tách `VSxxxx … # comment` — không phải path | giữ |
| `crates/vis-cli/src/cli/run.rs` | **31–34** | `raw.split(',')` | thấp | đọc `/sys/kernel/security/lsm` (list LSM, dấu phẩy) — không phải path | giữ |
| `crates/vis-cli/src/main.rs` | **213** | `v.split_once('=')` | thấp | `--grant p=path` — tách ở `=`, phần giá trị **giữ nguyên** (đúng: không cắt path tại dấu phẩy) | giữ |

### 3.4 Test-only — **không** phải path tách tay, hoặc đã chuẩn hoá

| tệp | dòng | mẫu | mức nghi ngờ | lý do | ghi chú (sửa ở #149) |
|---|---|---|---|---|---|
| `crates/vis-cli/tests/closure_cmd.rs` | **168, 173** | `cuoi.split(':').nth(2/1)` | thấp | lấy **cột/dòng** từ dòng `…: error VS0414` — không phải path | giữ |
| `crates/vis-cli/tests/lsp.rs` | **703** | `dc.split(':')` | thấp | parse `d:c` trong TAP `# VSxxxx d:c` — không phải path | giữ |
| `crates/vis-cli/tests/diag_cmp.rs` | **163, 166, 169, 194, 195** | `split_once(':')`, `rsplit(':')` | thấp | parse chẩn đoán `file:line:col` — không phải path | giữ |
| `crates/vis-cli/tests/diag_cmp.rs` | **294, 299, 421, 426, 428, 478–479** | `p.as_os_str()` cho `Command` | thấp | truyền OS-native path — đúng | giữ |
| `crates/vis-vm/tests/vs_interp.rs` | **172** | `l.split(':').next()` | thấp–trung bình | lấy phần trước `:` của dòng `A/B … file:line …` để so khớp `KNOWN_LECH` — **nếu** đường dẫn chứa `:` (ổ đĩa `C:`) thì nhận sai; test-only, môi trường CI Unix/mac nên không kích hoạt | ghi nhận để ST-211; không sửa |
| `crates/vis-cli/tests/json_ra.rs` | **241, 243, 268, 269** | Python `rsplit("/",1)[-1]` (trong `python3 -c`) | thấp | `rel` trong JSON **đã chuẩn hoá** `\\`→`/` (`modules.rs:144,147,169`) ⇒ đúng cả trên Windows; test-only | giữ |
| `crates/vis-cli/tests/chan_doan_da_tep.rs` | **47** | Python `f["file"].rsplit("/",1)[-1]` | thấp | như trên | giữ |
| `crates/vis-cli/tests/vish_nhom_vi_tri.rs` | **52–53** | `.split(".vish:").nth(1)`, `.split(": error").next()` | thấp | cắt output diagnostics ở `.vish:` — không phải path | giữ |
| `crates/vis-cli/tests/check.rs` | **66** | `line.trim_start_matches("# ")`, `splitn(3, ' ')` (dòng 67) | thấp | TAP `# VS…` — không phải path; dòng 82 VD-184 thuộc lịch sử (đã ST-153) | giữ |

## 4. Trả lời trực tiếp câu hỏi VD-220 C5

### 4.1 `check.rs` — dòng `rsplit('/')` tương khắc còn không?

**CÒN.** Không phải dòng 82 (đã sửa ở ST-153) mà là **`crates/vis-cli/src/cli/check.rs:191`**:

```rust
.unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path).to_string());
```

- Nằm trong `fn check_all(path: &str, …) -> KetQuaCheck` (dòng 149) — phục vụ `vis check --json` (`run_check_json` → `check_all`).
- Là **fallback** `root` khi `.vish` có `FileUnit.rel` rỗng (comment dòng 182: "`.vish` có thể có `FileUnit.rel` rỗng ⇒ fallback đúng đường dẫn đối số").
- Chỉ tách `'/'` ⇒ trên Windows `path = "C:\\a\\b.vis"` (không có `'/'`) ⇒ `root` = **toàn bộ đường dẫn** ⇒ `phu` luôn khác rỗng ⇒ ở nhánh `kiem_kieu = false` (`--json --syntax`) mọi chẩn đoán đều được giữ (cùng lớp lỗi mà ST-153 sửa ở `check_mode`).
- **Không** tái tạo lỗi khi `main_file.rel` khác rỗng (đa số trường hợp) — chỉ ảnh hưởng `.vish` + `--json` + `--syntax` + Windows.
- **Không sửa ở làn C.** Đề xuất cho ST-211: dùng `ten_goc` hoặc `Path::new(path).file_name()`; thêm ca khoá `# cfg(windows)` `C:\\…` / `C:/…` (đúng VD-184(5)) + `--json`.

### 4.2 Ngoài `check.rs`

**Ngoài `check.rs` — không còn chỗ tách `'/'` thủ công cho path hệ thống khớp mẫu**, nhưng **có** hai cụm "phương thức tự viết" cần nêu (không phải path tệp hệ thống, vẫn là tự tách chuỗi phân cách):

1. `std.path` của interpreter (`crates/vis-interp/src/stdlib.rs:725–770`) — `noi_duong`/`basename`/`dirname`/`ext`/`stem`: tách `/` thủ công, **không** xử lý `\\`; là **cố ý "thuần chuỗi, không chạm đĩa"** (comment mục std.path) nhưng máy móc vẫn là tự tách — xếp vào #149 để ST-211 quyết (giữ/thay).
2. URL handling trong `crates/vis-interp/src/net.rs:28, 33, 92, 166, 169, 197, 203, 209, 211` và `crates/vis-cli/src/net.rs:60`, `crates/vis-cli/src/quyen.rs:577` — toàn bộ là **URL/host:port/authority**, không phải path hệ thống; liệt kê để khép kín mẫu `split`/`rsplit`.

## 5. Toàn bộ dòng khớp mẫu trong `crates/` (máy đếm + đối chiếu tay)

Mẫu `\.split('/')` / `\.split("/")` / `\.split('\\')` / `\.rsplit('/')` / `\.rsplit("/")` / `\.rsplit('\\')` / `\.rsplit("\\")` / `\.split(":")` / `\.rsplit(":")` / `\.split(':')` / `\.rsplit(':')` / `\.nth_back(` / `trim_start_matches('/')` / `split_once('/')` / `rsplit_once('/')` / `splitn(` / `rsplitn(` / `Path::from(` / `PathBuf::from(` / `strip_prefix('/')`:

**Path-class (đường dẫn tệp hoặc URL) — 1+1+9 = 11 vị trí** (chi tiết bảng §3):
`check.rs:191` · `check.rs:64–67` (ten_goc — đã sửa) · `stdlib.rs:725–730, 733–739, 741–753, 755–761, 763–770` (std.path) · `net.rs:166, 169, 197, 203, 209, 210, 211` (URL; `rsplit_once('/')` dòng 210) · `fmt.rs` — **0 hit**.

**Toàn bộ hit khác (không phải path — đầu vào `split`/`rsplit`/`find` của dòng có `'/'`)**:

| tệp | dòng | nội dung rút gọn | loại |
|---|---|---|---|
| `crates/vis-interp/src/net.rs` | 28 | `rest.find(['/', '?'])` | URL authority/path |
| `crates/vis-interp/src/net.rs` | 166, 197, 203, 209 | `split_once('?')`, `split('#')`, `split('?')` | URL query/fragment |
| `crates/vis-interp/src/net.rs` | 33, 92 | `rsplit_once(':')`, `split_once(':')` | host:port / header `k: v` |
| `crates/vis-cli/src/net.rs` | 60 | `.rsplit_once(':')` | host:port |
| `crates/vis-cli/src/quyen.rs` | 577 | `dia_chi.rsplit_once(':')` | host IPs |
| `crates/vis-cli/src/quyen.rs` | 315 | `!v.contains('/')` | PATH lookup |
| `crates/vis-cli/src/pkg.rs` | 806 | `spec.rsplit_once('@')` | `name@ver` |
| `crates/vis-cli/src/vistest.rs` | 150 | `.rsplit(':')` | chẩn đoán `f:d:c` |
| `crates/vis-cli/tests/diag_cmp.rs` | 169, 194 | `split_once(':')` | chẩn đoán |
| `crates/vis-cli/tests/diag_cmp.rs` | 195 | `.rsplit(':')` | chẩn đoán |
| `crates/vis-cli/tests/closure_cmd.rs` | 168, 173 | `.split(':')` | chẩn đoán |
| `crates/vis-cli/tests/lsp.rs` | 703 | `.split(':')` | `d:c` trong TAP |
| `crates/vis-cli/tests/vs_interp.rs` | 172 | `l.split(':').next()` | diff-lệch (path có `:` sẽ sai — ghi nhận) |
| `crates/vis-cli/tests/chan_doan_da_tep.rs` | 47 | python `rsplit("/",1)[-1]` | test (JSON đã chuẩn hoá `/`) |
| `crates/vis-cli/tests/json_ra.rs` | 241, 243, 268, 269 | python `rsplit("/",1)[-1]` | test (như trên) |
| `crates/vis-cli/tests/vish_nhom_vi_tri.rs` | 52, 53 | `.split(".vish:")`, `.split(": error")` | test |
| `crates/vis-cli/tests/check.rs` | 67 | `splitn(3, ' ')` | TAP |
| `crates/vis-syntax/src/parse/cmd.rs` | 80, 173 | `.split(':')` | chữ ký khai báo |
| `crates/vis-types/src/lower/khai.rs` | 159 | `.split('#')` | `VS… # comment` |
| `crates/vis-cli/src/pkg/semver.rs` | 63 | `.split('.')` | semver |
| `crates/vis-cli/src/vistest.rs` | 47, 161 | `split_once('\n')`, `split_once(':')` | TAP |
| `crates/vis-cli/src/cli/run.rs` | 34 | `.split(',')` | LSM list |
| `crates/vis-cli/src/main.rs` | 213 | `split_once('=')` | `--grant p=path` |
| `crates/vis-vm/src/vm.rs` | 729, 847 | `.rsplit_once('.')`, `.rsplit('.')` | `Type.Variant` / tên biến |
| `crates/vis-cli/src/pkg/resolver.rs` | 126 | `it.file_name().to_str()` | đúng chuẩn (read_dir → phiên bản) |
| `crates/vis-cli/src/modules.rs` | 149, 499 | `.file_name()` | tên hiển thị / mod name — đúng |
| `crates/vis-cli/src/lsp/chan_doan.rs` | 19 | `path.file_name()?.to_string_lossy()` | đúng |
| `crates/vis-cli/src/lsp/module.rs` | 140 | `path.file_name()?.to_string_lossy()` | đúng |
| `crates/vis-cli/src/ngoai.rs` | 365, 382 | `e.file_name().to_string_lossy()` | đúng |
| `crates/vis-cli/src/quyen.rs` | 390 | `(p.parent(), p.file_name())` | đúng |
| `crates/vis-cli/src/audit_span.rs` | 34 | `p.file_name().map(...)` | đúng |
| `crates/vis-cli/tests/*.rs` | 15 vị trí | `.file_name()` | đúng (test) |

**0 hit**: `split("/")` · `split('\\')` · `rsplit("/")` · `rsplit('\\')` · `rsplit("\\")` · `.nth_back(` · `trim_start_matches('/')`/`("/")` · `strip_prefix('/')` · `splitn('/')` · `fn split_path` · `fn path_*` · `Path::from(s.split(...))`.

**`PathBuf::from(` = 51 vị trí** — toàn bộ là `CARGO_MANIFEST_DIR` / `current_dir` / `env`/`HOME` / `tempdir` trong test + `main.rs:116` (`PathBuf::from(&c.root)` — c_ root là path CLI, không tách) / `modules.rs:772` (`PathBuf::from(main)`) / `pkg.rs:837` (`PathBuf::from(v)` — giá trị `--reg`) / `quyen.rs:243,349` / `cache.rs:22,157,162` — **không có chỗ nào dựng Path từ tách chuỗi `/`/`\\`**; liệt kê đếm để khép kín, không đưa vào bảng sửa.

## 6. Hàm tự viết tên khả nghi (đã rà tay toàn bộ)

| hàm | vị trí | kết luận |
|---|---|---|
| `fn ten_goc` | `check.rs:64` | tự tách `/`+`\\` — §3.2 (đã ST-153) |
| `fn ten_goc_{windows,unix,khong_co_dau}` | `check.rs:364,370,376` | test của ST-153 |
| `fn ten_nguon` | `lsp/module.rs:68` | `trim_end_matches(".vis"/".vish")` — bỏ hậu tố, **không** tách path |
| `fn duong_dan` | `cache.rs:78` | dựng `.vis/cache/<hash>.vish` từ SHA-256 hex — không tách path |
| `fn duong_dan_cua_handle` | `ngoai.rs:270,286,291` | fd → `/proc/self/fd/…` (Linux) — không tách path |
| `fn duong_thoat_ngoac_van_la_bieu_thuc`, `fn duong_dan_*` | tests | tên test, không phải tách path |
| `fn split_path` | — | **0 hit** |
| `fn path_*` | — | **0 hit** |
| `fn file_name` | — | **0 hit** (không có wrapper tự viết) |
| `fn ten_*` khác (`ten_ty`, `ten_pat`, `ten_lenh`, `ten_bien`, `ten_mac_dinh`, `ten_khai`, `ten_hien_thi_on_dinh`, `ten_bien_bind_toi`, `ten_da_bind_toi`, `ten_cmd_khai`, `ten_khai_bao`, `ten_du_tru_toan_cuc_bi_tu_choi`, `ten_*` trong tests…) | nhiều | không liên quan path (kiểu/biến/lệnh) |
| `fn stem`, `fn ext`, `fn basename`, `fn dirname`, `fn noi_duong` | `vis-interp/src/stdlib.rs:725–770` | §3.1 (tự tách `/`, thuần chuỗi) |

## 7. Kết luận cho #149 (không tự sửa)

1. **#149 hiện trạng**: còn tối thiểu **2 nhóm** có thể vào phạm vi sửa ST-211: (a) `check.rs:191` (`--json` fallback — lớp VD-146); (b) `std.path` trong interp (`stdlib.rs` — chuỗi thay `Path`).
2. Các hit §5 còn lại **không** thuộc ST-211 (URL / diagnostics / registry / semver / signature) — ghi nhận đầy đủ để lần sau không phải rà lại.
3. **Không sửa gì trong làn C** (VD-220 làn C = docs/sổ sách). Đề nghị ST-211: đổi `check.rs:191` → `Path::file_name()`; ca khoá `#[cfg(windows)]` `C:\…`/`C:/…` + `--json --syntax`; quyết giữ hay thay `std.path` (trần chuỗi) — nếu giữ thì thêm ghi chú "phân cách luôn là `/`" vào doc/API (`API-STD.txt` — không thuộc #149).
4. **Liên kết**: `agent/SO-NO.md` §4 dòng 229 (#149, mở) · `agent/DECISIONS.md` **VD-184** (Ngoại lệ VD-146 cho `check.rs:82` — sửa `ten_goc` bằng `Path::file_name()`; ST-153/`2287bad` là hiện thực) · **VD-146** (Windows trong VS2: chỉ đo — VD-184 chính là ngoại lệ hẹp của VD-146) · `agent/JOURNAL/st153-psvn-ca7.log` · ST-211 (VS3-5-pre, chưa làm).
