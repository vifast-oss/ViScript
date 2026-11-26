<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318c — R1 mềm `vis-types`: tách 3 tệp > 1.000 dòng (move nguyên văn)

- **Ngày**: 2026-11-27 (giờ thấp điểm UTC) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st318c`
- **Nền**: `main` @ `2b5ccb51` · **Worktree**: `/home/ps/viswt/st318c` — không chạm repo chính.
- **Phạm vi**: chỉ `crates/vis-types/src/lower/**` + `crates/vis-types/src/tyck/expr*`.
  Không sửa `vis-core`/`vis-hir`/`vis-diag`/`vis-interp`/`vis-vm`/`vis-runtime`/`vis-conformance`;
  không sửa SPEC/QUEUE/DECISIONS/ACTIVE; 0 `unsafe`; 0 tệp test cấp 1 mới.

## 1. LOC trước/sau (`wc -l` thật)

### 1a. `crates/vis-types/src/lower/trait_impl.rs` 1.152 → `trait_impl/`

| Tệp | Dòng | Trách nhiệm |
|---|---:|---|
| `trait_impl/mod.rs` | **199** | doc + `mod` + `TraitSig`/`PhuongThucTrait` + helper dispatch (`ten_kieu`, `static_ty`, `method_sym`, đăng ký/khớp `impl`) + `DeriveNguon`/`PtSinh` |
| `trait_impl/kiem.rs` | **277** | `collect_trait`, `collect_impl`, `kiem_derives`, `kiem_impl_khop` |
| `trait_impl/sinh.rs` | **356** | `sinh_derives` (Eq/Show), `sinh_derive_{rec,enum,derive}`, `sinh_pt`, `sinh_show{,_enum}`, `sinh_eq`, `co_float_sau`, `co_truong_khong_eq` |
| `trait_impl/json.rs` | **359** | `sinh_derives_json`, `json_hop_le`, `sinh_json_{to,from}`, helper literal |
| `trait_impl/sieu.rs` | 38 | **giữ nguyên** (ST-311d) |

API giữ nguyên: `pub use trait_impl::TraitSig`, `PhuongThucTrait`, mọi method `pub(crate)`
(`ten_kieu`, `static_ty`, `method_sym`, `dang_ky_impl`, `khop_impl`, `collect_*`, `kiem_*`,
`sinh_derives`) cùng tên/chữ ký. Chỉ nới nội bộ `pub(super)` cho `co_float_sau`, `json_hop_le`,
`sinh_derives_json`, `sinh_pt` (gọi chéo tệp trong cùng `trait_impl`).

### 1b. `crates/vis-types/src/lower/cmd.rs` 1.115 → `cmd/`

| Tệp | Dòng | Trách nhiệm |
|---|---:|---|
| `cmd/mod.rs` | **37** | doc + `mod` + `ten_kieu` (giữ `pub(super)` cho `sugar.rs`) |
| `cmd/text.rs` | **278** | `nhom_khong_ngoac`, `vach_text`, `lower_vach`, `lower_bieu_thuc_tai`, `kieu_text`, `chuyen_doi{,_text}` |
| `cmd/stage.rs` | **680** | `ha_stage` (hook `get`/`select`/`sort_by`/`where`, khe ống, đối số vị trí) |
| `cmd/complete.rs` | **59** | stage giả `\| complete` |
| `cmd/args.rs` | **120** | `mac_dinh_theo_tep` + `ket_thuc_stage` (tail `ha_stage`: default builtin, tham số bắt buộc) |
| `cmd/doc.rs` / `cmd/hop.rs` | 240 / 213 | **giữ nguyên** (ST-106) |

API giữ nguyên: `ten_kieu` (path `lower::cmd::ten_kieu`), mọi method `pub(super)`/`pub(crate)`.
Nới nội bộ: `vach_text`/`kieu_text` → `pub(crate)` (gọi từ `lower/than.rs`, `lower/sugar.rs` ở
sibling), `ha_stage` → `pub(super)`, `ket_thuc_stage`/`ha_complete` → `pub(super)`. Đường
`super::chieu::tach_phay` đổi thành `super::super::chieu::tach_phay` do sâu thêm một tầng module.

### 1c. `crates/vis-types/src/tyck/expr.rs` 1.079 → `expr/`

| Tệp | Dòng | Trách nhiệm |
|---|---:|---|
| `expr/mod.rs` | **589** | `kiem_expr`/`kiem_expr_trong` (match theo nhóm biểu thức), `kiem_cond`, `show`, `unify_*`, `arity`/`arg_ty`, `try_hop_le`/`ham_tra`, `kiem_enum_lit` |
| `expr/toan_tu.rs` | **160** | `kiem_bin` (toán tử nhị phân, `VS0414`) |
| `expr/builtin.rs` | **366** | `kiem_std` (`std.*`) + `kiem_builtin` (8 tên dựng sẵn + `ls`/`get`/`select`/`sort_by`/`where`) |
| `expr/goi.rs` / `expr/kenh.rs` / `expr/phu.rs` | 444 / 143 / 607 | **giữ nguyên** (ST-285/ST-61) |

API giữ nguyên: `expr::phu` vẫn `pub(super)`; `kiem_expr`/`unify_branch`/`show`/`kiem_cond`
`pub(crate)`; `kiem_std`/`kiem_builtin`/`kiem_bin` nới `pub(super)` (gọi chéo tệp trong `expr`).

**Tổng**: 3 tệp gốc 3.346 dòng → 16 tệp, **max 680** (`cmd/stage.rs`); mọi tệp trong 3 cây
con (kể cả `goi` 444 / `phu` 607 có từ trước) đều < 700. Trần cứng 1.500: OK.

## 2. Cổng (số thật)

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `cargo fmt --all -- --check` | **OK** (exit 0) | — |
| `cargo test -p vis-types` | **221 passed** + 2 doc-test | `/home/ps/vislogs/st318c-test-vis-types.log` · `9d03f6aa…46dfe9` |
| `./vis-dev gate --fast` | **PASS** — nextest **3901/3901**, 4 skipped (87,7 s) | `/home/ps/vislogs/st318c-gate.log` · `a785fa21…154444` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 93.768 chạy / 6.232 lỗi tĩnh, 185,9 s | `/home/ps/vislogs/st318c-diff.log` · `2562a747…09d706` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** | `/home/ps/vislogs/st318c-clippy.log` · `f694a9e0…a9d6e` |
| `./vis-dev rules` R1..R9 | **OK** — **R8 = 38 + 1 = 39/40**, `vis-cli/tests` 8/15 | `/home/ps/vislogs/st318c-rules.log` · `812aacb2…793414` |

Không tạo tệp test cấp 1 mới, không `#[ignore]` mới, không sửa/ di chuyển test (cùng module path
logic, chỉ đổi tệp vật lý). 0 `unsafe` trong toàn bộ tệp đã đổi.

## 3. Commit

| Commit | Nội dung |
|---|---|
| `5c6d441f` | `[ST-318c]` tách `lower/trait_impl.rs` 1152 → mod 199 + `kiem/sinh/json/sieu` |
| `8b368d78` | `[ST-318c]` tách `lower/cmd.rs` 1115 → mod 36 + `text/stage/args/doc/hop` (max 720) |
| `5750f056` | `[ST-318c]` tách `tyck/expr.rs` 1079 → mod 589 + `toan_tu/builtin` (max 589) |
| `62117ecd` | `[ST-318c]` tách `cmd/stage`: `| complete` → `complete.rs` (stage 720 → 680) |

## 4. Việc còn lại

- Không còn tệp nào thuộc 3 mục tiêu > 1.000 dòng. Không mở rộng lane.
- Dọn worktree/nhánh theo `AGENTS.md` §6 sau khi A0 duyệt `DONE` (chưa tự xoá).
- Ghi chú: `cmd/stage.rs` 680 là tệp lớn nhất; `tyck/expr/phu.rs` (607, có từ ST-61) và
  `tyck/expr/goi.rs` (444) giữ nguyên, không cần tách thêm cho R1.
