# ST-122 · Span tổng hợp phải mang file (VD-127)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-122** · ngân sách 3 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 Q11 + §6 ST-122,
[`DECISIONS.md`](../DECISIONS.md) **VD-127**.

## 0. Tiêu chí nghiệm thu (viết lại theo **cơ chế thật**)

> Mọi node HIR do lower **sinh ra trong khi hạ một đơn vị tệp `u`** phải mang `span.file` = id của
> `u`; test duyệt HIR của **mọi fixture đa tệp** ⇒ **0** node có `file` khác id của đơn vị đang hạ
> tại thời điểm sinh.

**Vì sao phải viết lại** (không áp được nguyên văn câu "node thuộc module phụ có `file == 0`"):
`file_id` **không** được cấp theo thứ tự nội dung mà theo **thứ tự nạp tiền tự** —
`crates/vis-cli/src/modules.rs:461-464` (`let file_id = self.file_id; self.file_id += 1;` **trước**
khi đệ quy vào `use`) ⇒ **tệp chính = 0**, module = 1, 2, … theo thứ tự `use`; còn `Loaded::files`
xếp **hậu duyệt** nên thứ tự hai bên **khác nhau** (bằng chứng: `test/modules/vong-3/main.vis` cho
`main.vis = 0`, `a.vis = 1`, `b.vis = 2`, `c.vis = 3` — đo bằng audit, xem §3). Hệ quả:

1. Câu "module phụ có `file == 0`" **đúng dạng nợ** nhưng chỉ khi tệp chính = 0; nếu để nguyên cách
   diễn đạt "file 0 = tệp chính" thì audit sẽ **bỏ sót** node sinh ra trong **module** đứng **sau**
   (`file = 1,2,…`) và **báo nhầm** node ở tệp nạp đầu. Vì vậy audit so `span.file` với **id của tệp
   khai item** chứa node, không so với hằng 0.
2. Chỉ **tyck** mới dùng `span.file` để chọn tệp (`modules.rs:809` `n.file_rel.get(d.span.file)`);
   chẩn đoán do **lower** phát ra được gắn `rel` của tệp đang hạ ngay khi đẩy vào `diags`
   (`modules.rs:633`), nên **không** lộ lỗi `file = 0`. Đây là lý do **ca âm phải là chẩn đoán tyck**
   (`VS04xx`/`VS06xx`) rơi trên node tổng hợp — ca lower-only **không** tái hiện được lỗi.

## 1. Mục tiêu

Mọi node HIR **không copy từ CST** (do lower tự dựng) phải mang `file` của **tệp đang hạ**. Trước
ST-122, các chỗ dựng dùng `Span::new(start, end)` ⇒ `file = 0` ⇒ node của **module** bị quy sang tệp
chính; chẩn đoán tyck rơi trên node đó hiện ra **sai tệp + sai dòng:cột** (`main.vis:4:1`).

## 2. Danh sách **mọi** chỗ dựng Span không copy từ CST (đo bằng `grep`, trước khi sửa)

Cột "file lấy từ đâu" = nguồn **đúng** phải dùng. `L` = `Lowerer` của tệp đang hạ.

| # | Tệp:dòng (trước) | Vì sao là node **tổng hợp** (không copy CST) | File lấy từ đâu | Sửa |
|---|---|---|---|---|
| 1 | `lower/than.rs:279` | span của **thân `cmd`** tính từ `t.map` (bản `dedent` → vị trí tuyệt đối), không có token tương ứng | `L.file` | `.with_file(self.file)` |
| 2 | `lower/than.rs:280` | fallback khi `t.map` rỗng (`Span::new(0,0)`) | `L.file` | `.with_file(self.file)` |
| 3 | `lower/than.rs:287` | `HExpr::Error` **giữ chỗ** khi thân `cmd` không hạ được (`dat_loi`) — node HIR thật | `L.file` | `.with_file(l.file)` |
| 4 | `lower/than.rs:326` | token **re-lex** từ `t.dedent` rồi dịch qua `t.map` (thân `cmd` lệnh) | `L.file` | `.with_file(self.file)` |
| 5 | `lower/than.rs:477` | như #4 trong `than_parse` (thân `cmd` khai báo + pipeline) | `L.file` | `.with_file(self.file)` |
| 6 | `lower/than.rs:512` | token của nhóm `( … )` re-lex từ văn bản con rồi dịch `+at` | `L.file` | `.with_file(self.file)` |
| 7 | `lower/than.rs:334` | `parse_voi` dựng **node** CST bằng `elems_span` = `Span::new(first.start,last.end)` ⇒ **node mất `file`** dù token đúng | `L.file` | `parse_voi_file(..., self.file)` |
| 8 | `lower/than.rs:483` | như #7 (`than_parse`) | `L.file` | `parse_voi_file` |
| 9 | `lower/than.rs:514` | như #7 (`lower_nhom`) | `L.file` | `parse_voi_file` |
| 10 | `lower/khai.rs:432` | `parse(prefix)` **parse lại** vùng khai báo `.vish` ⇒ mọi node của **item `.vish`** (fn/cmd/type) mang `file = 0` | `L.file` | `parse_file(..., self.file)` |
| 11 | `lower/khai.rs:444` | `lex(prefix)` cùng lý do #10 (LexOutput đưa vào `lower_into`) | `L.file` | `lex_file(..., self.file)` |
| 12 | `lower/cmd.rs:83` | `lex(&src)` với `src` **đệm byte** cho `( biểu thức )` / stage biểu thức | `L.file` | `lex_file(..., self.file)` |
| 13 | `lower/cmd.rs:84` | `parse(&src)` cùng lý do #12 | `L.file` | `parse_file(..., self.file)` |
| 14 | `lower/cmd/doc.rs:109` | `lex(&src)` đệm cho **closure** `x => …` | `L.file` | `lex_file(..., self.file)` |
| 15 | `lower/cmd/doc.rs:110` | `parse(&src)` cùng lý do #14 | `L.file` | `parse_file(..., self.file)` |
| 16 | `lower/sugar.rs:297` | `lex(&src)` đệm cho **vị từ ngầm `it`** (`where size > 5mb`) | `L.file` | `lex_file(..., self.file)` |
| 17 | `lower/sugar.rs:298` | `parse(&src)` cùng lý do #16 | `L.file` | `parse_file(..., self.file)` |
| 18 | `lower/sugar.rs:160` | span `it.<trường>` **tính lại** từ `toks` của văn bản thân | `L.file` | `.with_file(self.file)` |
| 19 | `lower/sugar.rs:181` | span literal trong so sánh `it.<trường> <op> <literal>` | `L.file` | `.with_file(self.file)` |
| 20 | `lower/sugar.rs:299` | span **khung** `HFrame` cho lambda `it` (khung nằm trong HIR `Symbols`) | `L.file` | `.with_file(self.file)` |
| 21 | `lower/chieu.rs:67` | span **tên trường** của `get`/`select` — không có token riêng (tách từ văn bản stage) | `L.file` | `.with_file(self.file)` |
| 22 | `lower/cmd/doc.rs:44-52` | `lit()` **tổng hợp** literal đối số stage (chương trình ngoài) với `Span::new(0,0)` | `span` của node stage | tham số `span` (gọi #23) |
| 23 | `lower/cmd/doc.rs:174,188` | gọi #22 — literal `exec.<tên>` và stdin rỗng | `node.span` | truyền `node.span` |
| 24 | `lower/khai.rs:466-482` | `span_dong_cot(dòng,cột)` — đổi `(dòng,cột)` → span byte cho **chẩn đoán** `VS0201` của vùng `.vish` | `L.file` | thêm tham số `file` |
| 25 | `lower.rs:400,418` + `lower/khai.rs:455` | 3 chỗ **gọi** #24 | `L.file` | truyền `self.file` |
| 26 | `lower/cmd.rs:942,953` | **default của builtin** (`echo(b="", c="", --upper=false)`) dựng một lần trong `cmdsig::mac_dinh` với `Span::new(0,0)`, rồi **chép** vào HIR của tệp | `L.file` | `mac_dinh_theo_tep(d, self.file)` |
| 27 | `lower.rs:579` | `HSym` của **builtin** (`BUILTIN_VALUE_NAMES`) → `Span::new(0,0)` | — (không thuộc tệp nào) | **không sửa**, xem nợ #105 |

**Không** thuộc phạm vi (chỉ **phân tích**, không sinh node HIR): `lower/than.rs:546` (`kieu_nhom`),
`lower/khai.rs:299,302,312` (`KieuThan`/`tach`/`filler`), `lower/sugar.rs:239` (quét token `it`),
`lower/khai.rs:434`… (đã sửa vì **có** sinh HIR).

## 3. Audit tự động (không hardcode danh sách tệp)

`crates/vis-cli/src/audit_span.rs` — unit test của **bin crate** (integration test không thấy được
`mod modules` của binary). Cơ chế: quét **đệ quy** `test/**` (≥400 tệp), lọc tệp có `use ./`/`use pkg.`
(chỉ hai dạng này mới nạp nhiều tệp), nạp bằng **đúng đường sản phẩm** (`modules::load_full`, không
tyck), bỏ qua đơn vị < 2 tệp, rồi so `span.file` của **từng** node HIR (`HItem/HFn/HStmt/HExpr/HPat/
HArm/HCond/HParam/HField/HVariant`) với id của tệp khai item chứa nó; `hir.stmts` so với **tệp chính**
(`modules.rs:719` chỉ lấy `stmts` của tệp chính). Bản đồ `rel → file_id` dựng lại từ `Loaded::deps`
bằng DFS **tiền tự** đúng như loader cấp id.

| Lần | Fixture | Đơn vị đa tệp | Node HIR | Node sai tệp | Ghi chú |
|---|---|---:|---:|---:|---|
| A | chỉ fixture **cũ** (`test/modules/**`) | 16 | 221 | **0** | ⇒ fixture cũ **không** chạm đường tổng hợp; phải thêm fixture cố ý (§5) |
| B | + 7 fixture mới (`test/modules/span/**`) | 22 | 306 | **55** | **ĐỎ** — toàn bộ `file = 0` |
| C | sau khi sửa #1..#23 (token/lex/parse) | 23 | 321 | 19 | còn node sinh từ **node CST** (`elems_span`) |
| D | + `parse_voi_file` (#7,#8,#9) | 23 | 321 | 3 | còn **default của builtin** (#26) |
| E | + `mac_dinh_theo_tep` (#26) | 23 | 321 | **0** | **XANH** |

Log đỏ (lần B + 5 ca chẩn đoán trước khi sửa): [`st122-do.log`](st122-do.log)
`sha256 = 527c187af3bb1712b7c5e0963a6290395d777e620c07207e4476d4382e6ed51a`
(113 dòng; dòng khoá: `ST-122 audit: 23 đơn vị đa tệp, 321 node HIR, 65 node sai tệp (trong đó 65 node
file=0 sai tệp)` — 65 = 55 của lần B + 10 node của 2 fixture `ngoai`/`get-select` thêm sau; 5 ca chẩn
đoán đều ra `main.vis:4:1` thay vì `mod.vish:3:<cột>`).

## 4. Số đếm trước/sau

| Phép đo | Trước | Sau |
|---|---:|---:|
| node HIR mang **sai** `file` (audit toàn bộ fixture đa tệp) | **65** | **0** |
| ca chẩn đoán ra **sai tệp** (`main.vis:4:1`) | **5/5** | **0/5** |
| ca `test/sem` | 447 | **447** (không đổi — xem §7) |
| parity interp ↔ VM | 398 + 49 | **398 + 49** (không đổi) |
| `vis test` / `--vm` | 447 ok / 447 ok | **447 ok / 447 ok, 0 SKIP** |

## 5. Fixture + ca mới

Fixture **đa tệp** (mỗi ca: `main.vis` = tệp chính `file_id 0`, `use ./mod`; `mod.vish` = module
`file_id 1`) trong `test/modules/span/` — **không** đặt trong `test/sem/` vì `sem.rs`, `diag_cmp.rs`,
`roundtrip.rs`, `fuzz_mutate.rs`, `vs_interp.rs` quét **một tầng** `test/sem/<nhóm>/` và khoá cứng
**447**, còn `vistest.rs::gom` **đệ quy** ⇒ tệp `mod.vish` trong `test/sem/**` sẽ bị `vis test` chạy
như ca độc lập (đỏ) và `vs_interp.rs` hạ **một tệp** trong tiến trình (không nạp module) ⇒ đổi
nghĩa phân hoạch 398+49. Đã **kiểm**: `vis test` vẫn `1..447` (không quét `test/modules/`).

| Ca | Đường lower sinh node | Mã | Vị trí **đúng** (sau sửa) | Trước sửa |
|---|---|---|---|---|
| `cmd-than` | thân `cmd` script (`than_parse`) | VS0412 | `mod.vish:3:5` | `main.vis:4:1` |
| `where-closure` | closure `x => …` (`ha_closure`) | VS0412 | `mod.vish:3:26` | `main.vis:4:1` |
| `where-it` | vị từ ngầm `it` (`sugar`) | VS0412 | `mod.vish:3:67` | `main.vis:4:1` |
| `nhom` | nhóm `( … )` (`lower_nhom`) | VS0412 | `mod.vish:3:7` | `main.vis:4:1` |
| `item-khai` | item `.vish` (`ha_tien_to_khai`) | VS0412 | `mod.vish:3:5` | `main.vis:4:1` |

Fixture **audit-only** (không sinh chẩn đoán, dùng để audit phủ `lit()` của chương trình ngoài và
`get`/`select`): `ngoai`, `get-select`.

Test mới: 6 — `crates/vis-cli/tests/span_tep.rs` **5** (`json_bao_dung_tep_va_dong_cot_moi_ca`,
`tap_bao_dung_tep_va_dong_cot_moi_ca`, `run_interp_va_vm_bao_dung_tep_dong_cot`,
`da_tep_khop_tep_don`, `fixture_audit_hop_le`) + `crates/vis-cli/src/audit_span.rs` **1**
(`moi_node_hir_mang_dung_file_cua_tep_sinh_ra_no`). Ba đường được kiểm cho **từng** ca: `check --json`
(`"file":"mod.vish","line":3,"col":N`), TAP (`not ok 1 - … (mod.vish:3:N)` + `# VS0412 3:N …`),
`run` + `run --vm --no-cache` (exit 2, `mod.vish:3:N: error VS0412:`, hai đường **giống nhau**).
`da_tep_khop_tep_don` **đối chiếu độc lập**: chép `mod.vish` ra tệp đơn ⇒ `(dòng,cột)` **trùng**.

## 6. Lệnh + số đo

```
~/.cargo/bin/cargo test -p vis-cli --bin vis audit_span -- --nocapture
  ⇒ ST-122 audit: 23 đơn vị đa tệp, 321 node HIR, 0 node sai tệp (trong đó 0 node file=0 sai tệp)

~/.cargo/bin/cargo test -p vis-cli --test span_tep
  ⇒ test result: ok. 5 passed; 0 failed

cargo test --workspace                       # xem §6b
bash viscript/bin/vis-dev gate --fast        # xem §6b
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff   # xem §6b
```

### 6b. Kết quả đo (máy dev macOS aarch64, 2026-11-16, worktree có việc song song của lead/ST-123/ST-126)

- `~/.cargo/bin/cargo test --workspace` ⇒ **1072 test ok, 0 failed** trên **118** nhóm kết quả
  (= 1066 của R22 + **6** test mới của ST-122). Log: `/tmp/ws_test.log` (không commit).
- `cd /Users/mps/dvchd/ViFastOs && bash viscript/bin/vis-dev gate --fast` ⇒ dòng cuối
  **`vis-dev gate --fast: PASS (L1 chưa có tới khi có sim ViScript)`**, `EXIT=0`
  (fmt/clippy `-D warnings`/R1..R4/self-test/explain/báo cáo/doc-examples/build/L0 đều OK) —
  log [`st122-gate-fast.log`](st122-gate-fast.log) (1884 dòng),
  `sha256 = 3dd6a98e3018479ad3942d5ff1a08b017ddfe98c7ce9cbea1238058603081b54`.
- `cd viscript && VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` ⇒
  **`DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=164.4 toc_do=608/s seed=20261115`**
  (**lech=0**; `loi_tinh`/`chay` **trùng** R22 ⇒ không đổi ngữ nghĩa) — log
  [`st122-diff-100k.log`](st122-diff-100k.log),
  `sha256 = 75d7d46b9c52c3efa6eeb4bc0d81532579ca4c3ac25a494f640ffabe9de7861c`.
- `./bin/vis test` ⇒ **447 ok, 0 `not ok`, exit 0**; `./bin/vis test --vm` ⇒ **447 ok, 0 SKIP, exit 0**
  (không quét `test/modules/` ⇒ số ca **không** đổi).
- `python3 viscript/test/report/check_report.py --all` ⇒ **PASS (22 báo cáo, 149 hàng QUEUE phủ đủ)**.
- `~/.cargo/bin/cargo fmt --all -- --check` ⇒ sạch; `cargo clippy --all-targets -- -D warnings` ⇒ sạch
  (nằm trong cổng ở trên).

## 7. Vì sao `sem` **không** đổi (đo lại, không tự cộng)

6 ca mới đều **đa tệp** ⇒ không thể nằm trong bộ `test/sem` (một ca = một tệp tự chứa, 3 harness khoá
447 + `vistest.rs::gom` đệ quy). Đo lại sau khi thêm: `ds.len() == 447` ở `sem.rs`,
`(XANH, LOI_BIEN_DICH, LOI_LUC_CHAY) == (364, 49, 34)` ở `diag_cmp.rs`, `(khop, bien_dich) == (398, 49)`
ở `vs_interp.rs`, `so_ca_loi == 83` ở `roundtrip.rs`, `364` module nguồn ở `fuzz_mutate.rs` — **không
sửa hằng nào** (không có ca nào bị thêm/bớt khỏi `test/sem`). Tổng test **tăng 6** (1066 → **1072**).

## 8. Nợ mới (đánh số tiếp từ **105**)

- **105** — `HSym` của **builtin** (`lower.rs:579`, `declare_builtins`) vẫn `Span::new(0, 0)`:
  `Symbols` là bảng **dùng chung đa tệp**, builtin không thuộc tệp nào ⇒ không có `file` đúng duy
  nhất. Ảnh hưởng: `VS0302` khai trùng tên builtin in `pos(prev.span)` = `1:1`. Chưa sửa vì không
  có đáp án đúng theo VD-127; cần Agent 0 chốt (đổi `HSym` sang `Option<Span>` hoặc `file = u16::MAX`).
- **106** — `RecDef`/`FieldDef` của **prelude** (`prelude.rs:76,83`, `cmdsig.rs:58,74,84`) vẫn
  `Span::new(0,0)`; chúng nằm trong `TypeTable` (không phải HIR node) nên audit **không** phủ. Chẩn
  đoán trỏ vào trường prelude vẫn ra `1:1`. Cùng loại với #105.
- **107** — `test/sem` **không** chứa được ca **đa tệp**: 5 harness quét một tầng + `vistest.rs::gom`
  đệ quy. Muốn ca đa tệp vào bộ sem phải thống nhất một định dạng thư mục con (`<nhóm>/<ca>/main.vis`)
  và sửa **đồng thời** 5 harness + 2 khoá đếm; đề nghị Agent 0 quyết ở đợt 24.

## 9. Chỗ không áp được nguyên văn (so với quyết định Agent 0)

- **lệch**: VD-127 "Có test duyệt HIR của mọi fixture đa tệp: không node nào thuộc module phụ có
  file = 0" → **thực tế**: `file_id` cấp theo thứ tự **nạp** (`modules.rs:461-464`), tệp chính = 0, nên
  "file = 0" **không** đồng nghĩa "module phụ"; audit phải so với **id tệp khai item** (§0). Bằng
  chứng: `test/modules/vong-3/main.vis` → `main.vis=0, a.vis=1, b.vis=2, c.vis=3`.
- **lệch**: §6 ST-122 "viết test duyệt ... trong `crates/vis-cli/tests/` (ví dụ `span_tep.rs`)" →
  **thực tế**: audit cần `modules::load_full` — **module nội bộ của binary `vis`**, integration test
  không import được ⇒ audit đặt ở `crates/vis-cli/src/audit_span.rs` (unit test của bin crate, khai
  `#[cfg(test)] mod audit_span;` ở `main.rs:6`); ca **quan sát được** (JSON/TAP/run/--vm) vẫn ở
  `crates/vis-cli/tests/span_tep.rs` đúng như gợi ý.
- **lệch**: §6 ST-122 "≥4 ca sem mới" + "thêm ca sem ⇒ cập nhật đủ ≥7 chỗ đếm" → **thực tế**: ca đa
  tệp **không** đặt được trong `test/sem` (5 harness quét một tầng/khoá 447, `vistest.rs::gom` đệ quy
  ⇒ `vis test` đỏ, `vs_interp.rs` hạ một tệp ⇒ đổi nghĩa phân hoạch 398+49) ⇒ 5 ca đặt ở
  `test/modules/span/**`; **không** chỗ đếm nào phải sửa vì `test/sem` **không đổi** (đo lại §7). Nợ
  #107 ghi việc thống nhất định dạng để sau này đưa được vào `test/sem`.
- **lệch**: §6 ST-122 "node do lower sinh **không copy từ CST**" → **thực tế**: lỗi **không** chỉ ở
  chỗ dựng `Span::new` thủ công; `parse_voi` **dựng lại** span node CST bằng `Span::new(elems_span)`
  (`parse/mod.rs:364-370`) nên node sinh từ **node CST** (Lambda, stage, cả item `.vish`) cũng mất
  `file` — phải dùng `parse_voi_file`/`parse_file`/`lex_file` (§2 #7..#17). Bằng chứng: lần C của §3
  còn **19** node sai sau khi sửa hết `Span::new` thủ công.
