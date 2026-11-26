<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-316b — kiểm toán độc lập `#199`/`#48`/`#7`/`#186`/`#145`/`#47`

> **Người kiểm**: Gatekeeper (subagent Agent 1), 2026-11-26 (UTC).
> **Đối tượng**: nhánh `plan/st316b`, commit **`da5590e3fa7ee4f6ef0d0889f4739306575109a9`**
> (`[ST-316b] #199 lowering O(N) + #48 is_pub + #7 Eq long + #186 parity + #145 + #47`),
> so nền `main` **`5cbb785385c06c68ba2a0df218895c2bc39049cc`**.
> **Worktree**: `/home/ps/viswt/review-st316b` (nhánh `review/st316b-audit`, nền `5cbb7853`;
> chỉ commit **tệp báo cáo này**, không push/merge, không sửa mã sản phẩm).
> **Máy**: `ps` (Linux, 12 nhân, tải ~10 do 2 agent khác chạy song song) — mọi phép đo lặp 3 lần,
> so **release ↔ release** để loại nhiễu; log ngoài repo tại `/home/ps/vislogs/` (R7).
>
> **Kết luận: PASS** cả 6 hạng mục. 0 blocker. Có **1 finding thật ngoài 6 hạng mục** (đã xác nhận,
> **có sẵn ở nền `5cbb7853`**, không do lát này) + 1 ghi chú nhỏ (F2).

## 0. TL;DR

| # | Hạng mục | Kết quả | Bằng chứng chính |
|---|---|---|---|
| 1 | `#199` lowering O(N) | **PASS** | 25 000 const: **1,60 s → 0,196 s** (8,2×), 50 000: 4,65 s → 0,251 s (18,5×); JSON **y hệt**; 3/3 test (2 chỉ mục + `doi_khung`) xanh |
| 2 | `#48` `is_pub` + lọc completion | **PASS** | `HRec/HEnum/HAlias/HConst/HTrait.is_pub`; LSP lọc mọi loại item; `lsp_completion_module_chi_item_pub` 1/1 |
| 3 | `#7` `co_float_sau` sâu | **PASS** | 5 ca mới xanh (bản ghi/enum lồng + 3 ca vòng); thủ công: lồng không `float` xanh, tự quy chiếu có `float` → `VS0403` |
| 4 | `#186` parity interp/VM | **PASS** | 3 ca `lang_it/st316b.rs`; thủ công `x = 2` với `let x` cấp tệp: nền interp **exit 1**/VM **exit 0 in `2`**, nay **cả hai exit 2 + `VS0413` y hệt**; `check` nền xanh → nay đỏ |
| 5 | `#145` + `#47` | **PASS** | `chan[int](0)`: `check` **exit 0** (`ok 1`), chạy **VS0505 exit 101** cả 2 engine; `explain` 3 ngôn ngữ **nguyên văn** chuỗi cũ, exit 2 |
| 6 | Luật R1..R9 / DAG | **PASS** | 0 tệp `docs/`,`agent/{DECISIONS,QUEUE,SO-NO}*`; R8 = **39** (trần 40), vis-cli tests = **8** (trần 15); 0 `unsafe` mới; R1 max 1 457 (không tệp nào của lát > 1 152); `rules` + `vis-dag-check` OK |
| — | Cổng `gate --fast` | **PASS** | **exit 0**, 66 binary / **3 870 test chạy, 3 870 pass, 4 skip**, 0 fail |

## 1. `#199` — lowering O(N): chỉ mục `(khung, tên)` + `HashMap` item

**Đọc diff** (`crates/vis-hir/src/symbols.rs`, `crates/vis-types/src/lower/{lower,co_so,khai_bao,sig,than}.rs`):

- `Symbols::chi_muc` (`ChiMuc`) là **dữ liệu dẫn xuất**, dựng tăng dần trong `push_sym`, có `so_sym`
  canh lệch; `chi_muc_khop()` sai ⇒ **mọi** phép tra quay về quét tuyến tính cũ.
- `last_id_in_frame` = đúng `rposition(|s| s.frame == frame && s.name == name)` cũ: lấy bản **cuối**;
  nếu bản cuối **không** thấy được (`duoc_thay`: module khác/không `pub`) thì `resolve` **quét lại
  đúng khung đó** ⇒ giữ nguyên tầm nhìn `pub`/module (đọc kỹ `co_so.rs:648-676`).
- `item_syms: Vec<(String,u32)>` + quét ngược → `HashMap<String,u32>` + `insert`: `insert` ghi đè =
  **lần khai cuối**, đúng ngữ nghĩa `rev().find()` cũ cho cả `item_sym` (`unwrap_or(0)`) và
  `item_sym_cua` (`Option`). Không nơi nào **duyệt** `item_syms` để lấy thứ tự ⇒ không đổi kết quả.
- `than.rs`: sửa thẳng `s.frame` → `doi_khung(p.sym, khung)`; cập nhật cả khoá cũ (lùi về bản trước
  cùng tên nếu có) và khoá mới (chỉ ghi nếu `id` là bản cuối) ⇒ không mất tên sau khi tái dùng sym
  tham số `cmd`. `so_sym` giữ khớp sau mỗi lần push/đổi khung (đã suy luận + test).
- Mảng song song `mod_of`/`pub_of` vẫn push **cùng thứ tự** sau `push_sym` (id = `len` trước push) ⇒
  không lệch chỉ số. `PartialEq for ChiMuc` bỏ qua dữ liệu dẫn xuất (không nơi nào dùng `Symbols ==`).
- **Test**: `chi_muc_khop_quet_tuyen_tinh` (so từng tên với quét tuyến tính), `chi_muc_cu_van_tra_dung`
  (chỉ mục cũ → fallback), **`doi_khung_cap_nhat_chi_muc`** — 3/3 xanh.

**Đo lại** (tự sinh bằng `python3`, `vis check --json`, release ↔ release, 3 lần/corpus):

| Corpus | Nền `5cbb7853` (wall, s) | Mới `da5590e3` (wall, s) | Nhanh hơn |
|---|---|---|---|
| `chain_25000.vis` (mỗi const tham chiếu const trước) | 2,199 / 1,866 / 2,615 (median **2,199**) | 0,191 / 0,244 / 0,235 (median **0,235**) | **~9,4×** |
| `flat_25000.vis` (25 000 const độc lập — đúng yêu cầu) | 1,685 / 1,604 / 1,593 (median **1,604**) | 0,197 / 0,196 / 0,194 (median **0,196**) | **~8,2×** |
| `flat_50000.vis` | **4,654** | **0,251** | **~18,5×** |

- **KPI < 1 s**: **ĐẠT** — 0,196 s @25k; 0,251 s @50k; 0,30 s ở lần đo 50k lặp lại.
- Tuyến tính (mới): 6 250 → 0,04 s; 12 500 → 0,06 s; 25 000 → 0,12 s; 50 000 → 0,30 s.
- **Kết quả không đổi**: `check --json` của nền và mới **byte-y-hệt** (`cmp` = same) trên cả 3 corpus
  (`{"ok":true,...,"diagnostics":[]}`).
- Kiểm chéo tầm nhìn module: 2 tệp cùng `use` khai trùng tên (`pub` + không `pub`) → chẩn đoán
  `VS0302` **y hệt** nền (cả `check` lẫn `run` 2 engine) ⇒ chỉ mục không đổi kết quả giải tên.

> Môi trường: target **dùng chung** tái dùng artifact `vis-diag` cũ của worktree khác (lỗi
> `explain_help not found`) ⇒ đo bằng target **riêng** `/home/ps/visbuild/review-st316b-rel` (exit 0,
> 2m25s, sha256 `90304cbe…a91f6`) — cạm bẫy target chung, **không phải** lỗi mã.

## 2. `#48` — `is_pub` cho mọi item + lọc completion

- HIR: `is_pub` thêm cho `HRec`/`HEnum`/`HAlias`/`HConst`/`HTrait`; `HFn` đã có sẵn.
- Lower: `is_pub: self.has_token(node, "pub")` — `Node::tokens()` chỉ trả **token trực tiếp** của node
  (`Elem::Node ⇒ None`, `cst.rs:259-264`) ⇒ `pub` lồng trong node con **không** làm bẩn cờ.
- LSP `goi_y_module`: lọc theo `is_pub` cho **cả 6** loại; `Extern/Impl/Use` giữ `true` (nợ đã ghi
  trong comment, ngoài phạm vi `#48`). Không còn nhánh "chỉ `fn` mới có cờ".
- Test `lsp_completion_module_chi_item_pub`: `pub fn/type/enum/trait/const/alias` **có**, bản không
  `pub` **không** lọt — 1/1 xanh. Không test cũ nào phụ thuộc việc gợi ý item nội bộ (toàn bộ suite xanh).

## 3. `#7` — `co_float_sau` đệ quy bản ghi/enum lồng

- Hàm nhận thêm `&TypeTable` + `da_tham: Vec<(u8,u32)>`; `Type::Record`/`Type::Enum` đệ quy vào
  trường/biến thể. Mọi tổ hợp của `Type` đều là **OR** thuần (`List/Opt/Map/Result/TypeRef`), nên
  "đánh dấu đã thăm, gặp lại ⇒ `false`" là **đúng** (truy vấn đạt tới được; `true` luôn short-circuit
  toàn chuỗi nên không có âm tính giả). Kiểu tự quy chiếu/vòng nhau dừng nhờ `da_tham`.
- `co_truong_khong_eq` và vòng `VS0403` trong `kiem_derives` đều dùng `da_tham` **mới cho từng
  trường** ⇒ không rò trạng thái giữa các trường.
- 5 ca mới: **5/5 xanh** (`derives_eq_ban_ghi_long_float_bao_vs0403`, `..._enum_long_...`,
  `..._tu_quy_chieu_khong_lap`, `..._enum_tu_quy_chieu...`, `..._hai_ban_ghi_vong_nhau...`);
  `types_it derives_eq` tổng **8/8** (không hồi quy 3 ca cũ).
- Kiểm thủ công (release mới): `type Ngoai derives Eq: t: Trong` (Trong **không** `float`) → 0 chẩn
  đoán; `Node derives Eq: next: Node?` → 0; `Node` tự quy chiếu **có** `float` → `VS0403` (vẫn bắt).

## 4. `#186` — parity interp/VM cho `let` cấp tệp

- Cờ `let_cap_tep[sym]` được đánh dấu **trước** khi kiểm thân hàm (`ghi_nhan_let_cap_tep` phủ mọi
  dạng `HPat`: Bind/Ctor/Tuple/List/Rest/Record/Range/Or; Wild/Lit/Rest bỏ qua). `gan.rs` báo
  `VS0413` cả khi `ty_of` còn `{error}` **chỉ** với ràng buộc `let` cấp tệp; `var` cấp tệp vẫn được
  gán (đối chứng dương xanh, 2 engine in `2`).
- 3 ca `lang_it/st316b.rs`: **3/3 xanh** (gán vào `let` cấp tệp; `let` **pattern** cấp tệp; `var`
  đối chứng). Test khoá `stderr` **byte-y-hệt** giữa 2 engine + `check` đỏ.
- Kiểm chéo thủ công (`d_let_cap_tep_gan.vis`): nền `5cbb7853` **interp exit 1 `VS0405`**, **VM exit 0
  in `2`**, `check` **xanh** ⇒ đúng lỗi được mô tả. Nay: **cả 2 engine exit 2**, cùng
  `d…:4:5: error VS0413: không gán được cho \`let\`/tham số …`, `check` exit 1.
- Chương trình lỗi ở **cấp tệp** (`let x: int = "chuoi"`) và lỗi trong **`fn main`**
  (`let y: int = "chuoi"`): 2 engine **cùng mã/thông điệp/vị trí**, exit 2 (không lệch đường).
- Gán ở **cấp tệp** (`let x` + `x = 2` ngoài hàm): đúng **một** `VS0413` (không nhân đôi); cùng tệp
  với `var` → xanh, 2 engine in `1`.
- Nit (không chặn): comment trong `gan.rs` nói "mã … trùng đường interp (`lvalue_set`)" — thực tế
  `vis-interp` dùng `TYPE_CODES[3]` = **VS0405**, còn tyck dùng **VS0413**; chỉ **thông điệp + vị trí**
  là trùng, còn mã thì khác. Không ảnh hưởng hành vi (tyck chặn trước khi chạy) nhưng câu comment
  nên sửa cho đúng.

## 5. `#145` + `#47`

- `#145`: diff **chỉ đổi comment** ở `vis-runtime/src/rt/ops.rs` (nói đúng: tyck **không** kiểm `cap`,
  runtime từ chối `VS0505`). Kiểm thật: `chan[int](0)` → `vis check` **exit 0 / `ok 1`**, `run` +
  `run --vm --no-cache` đều **exit 101**, cùng `VS0505 … capacity 0 …`; ca mới
  `check_khong_chan_runtime_moi_tu_choi` **1/1** xanh (khoá cả 2 đầu).
- `#47`: chuỗi vi/en/ja dời **một nguồn** sang `vis_diag::explain_help` (số khối/mã vẫn **đếm động**);
  CLI chỉ in 2 dòng. So **nguyên văn**: vi `sổ mã: 7 khối, 57 mã …`, en `code book: 7 blocks, 57 codes`,
  ja `コード帳: 7 ブロック、57 コード`, exit 2, in ra **stderr** như cũ.
  `explain_help_khop_chuoi_cu` **1/1**; `st83_no_nho::explain_help_theo_lang` (khoá byte cả 3 ngôn ngữ
  qua CLI) **1/1**.

## 6. Luật & cổng

| Kiểm | Kết quả |
|---|---|
| Tệp `docs/**`, `agent/{DECISIONS,QUEUE,SO-NO}*` bị sửa | **0** (`git diff --name-only 5cbb7853 da5590e3` chỉ có `crates/**`) |
| R8 — tệp `.rs` cấp 1 trong `crates/*/tests` + `vis-host-linux/tests` | **39** (trần 40); `crates/vis-cli/tests` = **8** (trần 15); ca mới nằm trong `tests/lang_it/` + `#[path]` |
| `unsafe` mới | **0** dòng `+…unsafe` trong diff |
| R1 — tệp dài nhất | 1 457 dòng (`host_parity.rs`, **có sẵn**); tệp bị sửa dài nhất **1 152** (`trait_impl.rs`) < 1 500 |
| `./vis-dev rules` | **OK**, exit 0 (`--- luật R1..R9: OK`) |
| `./bin/vis-dag-check` | **OK**, exit 0 — 23 crate / 5 tầng / 0 cạnh cấm (+ self-test DAG OK) |
| `./vis-dev gate --fast` | **exit 0**: fmt OK · clippy OK · rules OK · DAG OK · release staging OK · self-test âm OK · build OK · doc-examples OK · **L0 3 870/3 870 pass, 4 skip, 0 fail** |

Test tập trung (log riêng): `vis-hir symbols::` 3/3 · `types_it derives_eq` 8/8 · `lang_it st316b::`
3/3 · `async_it check_khong_chan_runtime_moi_tu_choi` 1/1 · `lsp_it lsp_completion_module_chi_item_pub`
1/1 · `vis-diag explain_help_khop_chuoi_cu` 1/1 · `security_it explain_help_theo_lang` 1/1.

## 7. Finding ngoài 6 hạng mục (đã xác nhận, **không** sửa)

**F1 — `main` có ≥ 2 tham số không `cap`: VM chạy với `()` còn interp từ chối (parity).**
Mẫu tối thiểu (`q2.vis`):

```vis
fn main(a: int, b: int):
    print(a)
    print(b)
```

| Engine | Nền `5cbb7853` | Mới `da5590e3` |
|---|---|---|
| interp | exit **101**, stderr `q2.vis:2:1: error VS0501: thiếu đối số \`b\` của \`main\`` | **y hệt** |
| VM `--no-cache` | exit **0**, stdout `[]` rồi `()` (chạy với `b = <unit>`) | **y hệt** |

- Mức độ: **P2** (lệch engine: một bên từ chối, một bên **chạy** và in giá trị rác `()`; lệch cả
  exit code 101 ↔ 0). **Có sẵn ở nền** ⇒ **không do ST-316b**, **ngoài 6 hạng mục** (diff không chạm
  `vis-interp/src/interp.rs` / `vis-vm/src/vm.rs`).
- Ghi chú liên quan, cũng có sẵn và **không lệch engine**: `fn main(a: int)` (1 tham số) ⇒ **cả hai**
  gán `argv` = `[]` (list) vào tham số khai `int` và in `[]`; `check` xanh ⇒ **thiếu kiểm kiểu tham số
  `main` ↔ `argv`** (P3, ngoài phạm vi). Có `mặc định` (`b: int = 5`) thì 2 engine đều in `[]`,`5` —
  không lỗi.
- Kết luận: **xác nhận có thật**, phân loại **ngoài 6 hạng mục / nợ có sẵn**, **không chặn** ST-316b;
  không tự sửa theo đúng phạm vi Gatekeeper.

**F2 — nit tài liệu** trong `crates/vis-types/src/tyck/gan.rs:56-58`: comment nói mã trùng đường
interp, thực tế VS0413 ↔ VS0405 (xem §4). P3, không chặn.

## 8. Log & tái lập

| Nội dung | Đường dẫn |
|---|---|
| Cổng đầy đủ (`gate --fast`) | `/home/ps/vislogs/review-st316b-gate-fast.log` (411 KB) |
| Test tập trung từng hạng mục | `/home/ps/vislogs/review-st316b-focused-tests.log` |
| Dựng release mới (target riêng) | `/home/ps/vislogs/review-st316b-release-iso.log` |
| Corpus + số đo #199 | `/home/ps/vislogs/perf-st316b/` (`chain_25000.vis`, `flat_25000.vis`, `flat_50000.vis`, `tm/`) |
| Ca kiểm thủ công (`#186`, F1, `#7`, module) | `/home/ps/vislogs/perf-st316b/cases/` |
| Nhị phân đo (sha256 đầy đủ) | `cases/sha256.txt`: nền `1bcc54b9…`, mới `90304cbe…` |

Lệnh chính: `CARGO_TARGET_DIR=/home/ps/visbuild/shared CARGO_BUILD_JOBS=2
CARGO_PROFILE_DEV_DEBUG=0 ./vis-dev gate --fast` (dưới `flock /home/ps/visbuild/gate.lock`);
release đo: `CARGO_TARGET_DIR=/home/ps/visbuild/review-st316b-rel cargo build --release -p vis-cli --bin vis`;
đo: `vis check --json <corpus>` × 3.

## 9. Kết luận

**PASS** — `da5590e3` đạt 6/6: `#199` đúng ngữ nghĩa (chỉ mục dẫn xuất + fallback, kết quả
JSON byte-y-hệt, KPI 0,196 s < 1 s), `#48` lọc `pub` đủ 6 loại item, `#7` đệ quy sâu có chống vòng,
`#186` chặn tĩnh `let` cấp tệp cho **cả hai** engine (sửa đúng lỗi nền interp 1 ↔ VM 0), `#145` chỉ
sửa comment + khoá hành vi thật, `#47` gom một nguồn không đổi chuỗi. Luật R1/R8/R9 và DAG sạch,
cổng xanh 3 870/3 870. Finding F1 là **nợ có sẵn ngoài phạm vi** — cần mở task riêng (đề xuất
`#186b`/`#199b`: kiểm kiểu `argv` ↔ tham số `main` + đồng nhất đường gọi `main` nhiều tham số).
