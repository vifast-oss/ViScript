# ST-123 · `--grant` cho `cap[Proc]` theo **đường dẫn tuyệt đối đã canonicalize** (VD-130c)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-123** · ngân sách 2 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 (mục `§10(c) --grant so basename`),
§6 **ST-123**, [`DECISIONS.md`](../DECISIONS.md) **VD-130(c)**.

## 0. Mục tiêu

`cap[Proc]` + `--grant` phải là **allowlist theo đường dẫn tuyệt đối đã canonicalize**, không theo
tên tệp. Trước ST-123, `khop_lenh` so **basename** ⇒ một tệp thực thi **cùng tên** ở thư mục khác
được phép chạy. Đó là lỗ hổng; Agent 0 **không chấp nhận** (§2 bảng quyết định, dòng `§10(c)`).

## 1. Hiện trạng trước khi sửa (tệp:dòng)

| # | Tệp:dòng (trước) | Vai trò | Lỗ hổng |
|---|---|---|---|
| 1 | `crates/vis-cli/src/ngoai.rs:45-66` | `khop_lenh(ten, g)`: `ten == g` → `basename(ten) == basename(g)` → `canonicalize(ten) == canonicalize(g)` | nhánh **basename** cho phép mọi tệp cùng tên; `canon` fallback về **chuỗi gốc** khi canonicalize lỗi ⇒ hai đường dẫn khác nhau vẫn "khớp" nếu chuỗi bằng nhau |
| 2 | `crates/vis-cli/src/ngoai.rs:95-106` | `chay_raw` áp `allow_proc` trước khi `spawn` | chỉ kiểm **một lần**, bằng hàm ở #1 |
| 3 | `crates/vis-cli/src/quyen.rs:204-224` | `Quyen::tao` gom `--grant` của `cap[Proc]` vào `allow_proc` **nguyên văn** (không phân giải) | giá trị `echo` khớp mọi `/…/echo`; giá trị trỏ tệp **không tồn tại** vẫn được nhận, không chẩn đoán |

## 2. Đỏ trước — log + sha256

`agent/JOURNAL/st123-red.log` · **sha256 `5ffafd6e9b4e1843a4c3fcec83326e6cdeee4273bcbea87967f6cfcb279a0c99`**
· HEAD `4f54d610` · Darwin 27.0.0 arm64.

Tóm tắt log:

1. `cp /bin/echo <tmp>/echo-copy` ⇒ sha256 **trùng** `9f13adc0…e8e92c` (bản sao **nguyên byte**).
2. macOS **SIGKILL** bản sao binary hệ thống (`exit=137`): chữ ký nền tảng gắn với đường dẫn/binary
   gốc ⇒ **không** dùng được bản sao `/bin/echo` làm ca chạy. Vì vậy ca dùng **tệp thực thi cùng
   basename `echo`** tự tạo trong thư mục tạm (`#!/bin/sh`, sha256 `23a19dfc…28f2`): quyết định quyền
   chỉ dựa trên **đường dẫn**, nên cùng tên là đủ chứng minh lỗ hổng basename.
3. `--grant p=/bin/echo` + chạy `<tmp>/echo` ⇒ **exit 0, stdout `[SAO-CHEP]`** — cả `interp` **và**
   `--vm --no-cache` (M1=0, M2=0). Đây là **đỏ**.
4. Đối chứng: `--grant p=/bin/echo` + chạy `/bin/echo DUNG` ⇒ exit 0 `[DUNG]` (ca xanh, phải giữ).

## 3. Luật mới + cách sửa

Luật (VD-130c): giá trị grant **không có `/`** ⇒ phân giải qua `PATH` **một lần lúc khởi động** thành
đường dẫn tuyệt đối; **hai phía** (grant và lệnh sẽ chạy) đều **canonicalize** rồi so **bằng nhau**.

| # | Tệp:dòng (sau) | Nội dung |
|---|---|---|
| 1 | `quyen.rs:283-331` `fn phan_giai_lenh(v, goc) -> Result<PathBuf, String>` | **mới**: giá trị **không** có `/` ⇒ dò `PATH` (`std::env::split_paths`), nhận mục **đầu tiên** là tệp chạy được, rồi `canonicalize`; có `/` ⇒ ghép `goc` nếu tương đối rồi `canonicalize`. Không phân giải được ⇒ `Err` chẩn đoán rõ |
| 2 | `quyen.rs:333-337` `pub fn chuan_lenh(ten) -> Option<PathBuf>` | **mới**: cùng luật cho **lệnh sẽ chạy** tại thời điểm chạy; `None` = không phân giải được ⇒ không thể khớp allowlist |
| 3 | `quyen.rs:265-282` `fn la_tep_lenh(p)` (+ `la_tep_thuc_thi` gọi lại) | **mới**: Unix = tệp thường + bit `x`; nền khác = tệp thường. Giữ `la_tep_thuc_thi` cho ca `cap[Dir]` trỏ tệp thực thi |
| 4 | `quyen.rs:204-231` (`Quyen::tao`, nhánh `cap[Proc]`) | **sửa**: mỗi giá trị `--grant` đi qua `phan_giai_lenh`; lỗi ⇒ `Err` ⇒ CLI in rồi **exit 2**. `allow_proc` giữ **đường dẫn tuyệt đối đã canonicalize**; `QuyenCap.root` **giữ nguyên** chuỗi người dùng (`Value::Cap` in ra không đổi) |
| 5 | `ngoai.rs:45-56` `fn khop_lenh` | **sửa**: chỉ còn `chuan_lenh(ten) == Path::new(g)`; **bỏ** nhánh basename và fallback "so chuỗi gốc" |

## 4. Ca (7 ca · mỗi ca chạy **cả** interp và `--vm --no-cache`, so **byte** cả stdout+stderr+exit)

Tệp: `crates/vis-cli/tests/grant_proc_canon.rs`.

| # | Ca | Kỳ vọng | Trước sửa |
|---|---|---|---|
| 1 | `tuyet_doi_khop_thi_chay` — grant `/bin/echo`, chạy `/bin/echo a` | exit 0, `[a]` | xanh (giữ) |
| 2 | `sao_chep_trung_ten_bi_chan` — grant `/bin/echo`, chạy `<tmp>/echo SAO` | **exit 2**, `VS0604` + "không nằm trong allowlist", stdout rỗng | **chạy được** (đỏ §2) |
| 3 | `symlink_canonicalize_hai_phia` — `<tmp>/ln-echo -> /bin/echo`, grant `/bin/echo` | exit 0, `[a]` | xanh |
| 3b | (cùng ca) grant `<tmp>/ln-echo`, chạy `/bin/echo b` | exit 0, `[b]` — canonicalize **hai phía** | xanh |
| 4 | `grant_ten_tran_phan_giai_path` — grant `echo` (không `/`), chạy `/bin/echo a` | exit 0, `[a]` | xanh, **nhưng vì basename**; nay vì `PATH` ⇒ `/bin/echo` |
| 4b | (cùng ca) grant `printf`, chạy `printf abc` và `/usr/bin/printf abc` | exit 0, `[abc]` | xanh |
| 4c | (cùng ca) grant `/bin/cat`, chạy `/usr/bin/printf x` | exit 2 `VS0604` | xanh |
| 5 | `grant_duong_dan_tuong_doi` — grant `./bin/echo`, chạy `./bin/echo` | exit 0, `[TUONG-DOI]` | xanh |
| 5b | (cùng ca) cùng grant, chạy `<tmp>/echo` (cùng tên, khác thư mục) | exit 2 `VS0604` | **chạy được** |
| 6 | `grant_binary_khong_ton_tai` — grant `/khong/co/lenh` | exit 2, "--grant p=/khong/co/lenh" + "không phân giải được", stdout rỗng | **chạy** (không hề chẩn đoán) |
| 6b | (cùng ca) grant `khong_co_lenh_nay_trong_path` | exit 2, "không thấy lệnh" + "PATH" | **chạy** |
| 7 | `khong_grant_khong_gioi_han` — **không** grant, chạy `<tmp>/echo TU-DO` | exit 0, `[TU-DO]` (không siết oan) | xanh |

Ghi chú ca 4: **không** dùng `echo` trần làm *lệnh sẽ chạy* — trong chế độ lệnh `echo` là **builtin**
(`vis-interp/src/interp.rs:1215`, `builtin.rs:886`), không spawn tiến trình nên **không** đi qua
allowlist (đúng ngữ nghĩa: `cap[Proc]` chỉ áp cho tiến trình ngoài). Ca dương dùng `printf` (lệnh ngoài thật).

## 5. Lệnh + số đo

```
~/.cargo/bin/cargo test -p vis-cli --test grant_proc_canon        ⇒ 7 passed; 0 failed
~/.cargo/bin/cargo test -p vis-cli --test cap_proc_grant --test cap_proc --test quyen --test extern
                                                                  ⇒ 6+6+13+8 passed; 0 failed
bash bin/vis-host-suite                                           ⇒ 54/54 ca xanh (macOS)
bash bin/vis-dev gate --fast                                      ⇒ PASS (log: JOURNAL/st123-gate-fast.log)
  cargo test --workspace: 1084 ca / 125 suite / 0 đỏ (log cùng tệp; trong đó 7 ca là của ST-123,
  phần còn lại thuộc cây chung đang được các task khác sửa song song)
python3 viscript/test/report/check_report.py --all                ⇒ PASS (22 báo cáo, 149 hàng QUEUE)
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff            ⇒ xem JOURNAL/st123-diff-100k.log
R1: tệp lớn nhất của ST-123 = quyen.rs 527 dòng (≤ 1.500)
```

## 6. Ca cũ đổi kỳ vọng — **không có**

Rà **toàn bộ** chỗ dùng `--grant` cho `cap[Proc]` trong repo (grep `--grant` + `p=`):

| # | Ca cũ | Có dựa vào so **basename**? | Kết quả |
|---|---|---|---|
| 1 | `crates/vis-cli/tests/cap_proc_grant.rs` (a) `--grant p=/bin/echo` + `/bin/echo` | không (đường dẫn tuyệt đối, canonicalize hai phía) | **không đổi**, vẫn xanh |
| 2 | (b) `--grant p=/bin/echo` + `/usr/bin/grep` | không — âm tính, vẫn bị chặn | **không đổi**, vẫn xanh |
| 3 | (c) `--grant p=echo` + `/bin/echo` | **có** — nhưng `PATH` phân giải `echo` ⇒ `/bin/echo` nên **vẫn khớp** theo luật mới | **không đổi kỳ vọng**; chỉ sửa **chú thích** trong tệp (nói rõ xanh vì `PATH`, không vì basename) |
| 4 | (d) không grant ⇒ không giới hạn | không | không đổi |
| 5 | (e) hai grant + pipeline 2 stage | không | không đổi |
| 6 | (f) grant Proc cho `cap[Dir]` ⇒ lỗi kiểu | không | không đổi |
| 7 | `test/host/suite.txt:65` `cap_grant` | **không** — đây là `--grant d=…` cho `cap[Dir]`, không đi qua `khop_lenh` | **không đổi kỳ vọng**, 54/54 xanh |
| 8 | `crates/vis-cli/tests/cap_proc.rs` (không có `--grant`) | không | không đổi |

⇒ **0 ca** phải đổi kỳ vọng. Đúng tinh thần §6 ST-123: chỉ đổi khi ca **dựa vào** so basename; ca (c)
dựa vào **hành vi quan sát được** (`echo` ⇒ `/bin/echo`) mà luật mới vẫn bảo toàn qua `PATH`.

## 7. Tệp đổi

| Tệp | Loại |
|---|---|
| `crates/vis-cli/src/quyen.rs` | sửa: `phan_giai_lenh`/`chuan_lenh`/`la_tep_lenh` + nhánh `cap[Proc]` của `Quyen::tao` |
| `crates/vis-cli/src/ngoai.rs` | sửa: `khop_lenh` so đường dẫn canonicalize; bỏ basename |
| `crates/vis-cli/tests/grant_proc_canon.rs` | **mới**: 7 ca (× interp/--vm) |
| `crates/vis-cli/tests/cap_proc_grant.rs` | chú thích (luật mới + vì sao ca (c) vẫn xanh) |
| `agent/JOURNAL/st123-red.log` | log **đỏ trước** + sha256 |
| `agent/JOURNAL/st123-gate-fast.log`, `st123-diff-100k.log` | log cổng |

## 8. Nợ mới (đánh số tiếp từ **112**)

- **112** — `khop_lenh` phân giải `ten` tại thời điểm chạy nhưng **tiến trình thật** vẫn do
  `Command::new(ten)` để OS phân giải `PATH` **lần nữa** (`ngoai.rs:110`). Giữa hai lần phân giải,
  một `PATH` bị sửa hoặc một thư mục trong `PATH` bị thay nội dung có thể khiến tệp **chạy** khác tệp
  **đã kiểm**. Sửa triệt để = `spawn` **đúng** đường dẫn đã canonicalize (hoặc `fexecve`), nhưng đổi
  `argv[0]` (chương trình phân nhánh theo `argv[0]` như `busybox`/`gunzip` sẽ đổi hành vi) ⇒ cần
  quyết định riêng, **chưa** làm trong VS2.

## 9. Chỗ không áp được nguyên văn (so với quyết định Agent 0)

- **lệch**: §6 ST-123 "chép `/bin/echo` vào thư mục tạm, chạy chương trình gọi **bản sao**" → **thực
  tế**: macOS **SIGKILL** bản sao binary hệ thống (`exit=137`, log §2 (1)) ⇒ ca chạy bằng **tệp thực
  thi cùng basename** tự tạo. Bản sao nguyên byte **vẫn được chứng minh** trong log bằng `sha256`
  trùng; lỗ hổng được đo là lỗ hổng **chính sách đường dẫn**, không phụ thuộc nội dung tệp.
- **lệch**: cùng câu trên, "gọi bản sao với `--grant p=/bin/echo`" → ca (2) chạy `<tmp>/echo` là tệp
  **cùng tên khác thư mục**, đúng ngữ nghĩa "bản sao trùng tên"; không dùng `cp` vì lý do trên.
- **không lệch**: "≥6 ca (symlink, PATH, tương đối, trùng tên) × interp/--vm" → **7 ca** (12 phép
  khẳng định), mỗi ca so byte hai đường.
