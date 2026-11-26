<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318c — Gatekeeper (Agent 1) độc lập

- **Ngày**: 2026-11-27 · **Vai**: Gatekeeper · **Chế độ**: chỉ đọc/kiểm chứng, **không sửa mã**.
- **Nhánh**: `plan/st318c` @ `bcffb8cc` · **Nền**: `main` @ `2b5ccb51` · **`main` hiện tại**: `2c7dbef6`.
- **Worktree**: `/home/ps/viswt/st318c` (không chạm repo chính, không push).
- **Kết luận**: **PASS** — move nguyên văn, API công khai không đổi, toàn bộ cổng độc lập xanh,
  không có P1. Có 1 ghi nhận P2 (nới `pub(crate)` rộng hơn mức tối thiểu) và 3 P3 (hành chính).

## 1. Vệ sinh & phạm vi

| Kiểm | Kết quả |
|---|---|
| `git status --porcelain` | **rỗng** (sạch) |
| `git log --oneline 2b5ccb51..HEAD` | **5 commit** `[ST-318c]` (4 tách tệp + 1 journal) |
| `git diff --stat` | **14 tệp, +2.316 / −2.096** |
| Phạm vi | chỉ `crates/vis-types/src/lower/**`, `crates/vis-types/src/tyck/expr*` + `agent/JOURNAL/st318c-tach-tep.md` |
| Ngoài phạm vi | **0** — không `lib.rs`/`lower.rs`/`tyck/mod.rs`/`Cargo.toml`; không crate khác; không SPEC/QUEUE/DECISIONS/ACTIVE; 0 tệp test thay đổi |

Tệp khai báo module trung gian ở **mọi** commit đều tồn tại (quét `mod X;` ↔ `X.rs`/`X/mod.rs`
trên 5 commit: `5c6d441f`, `8b368d78`, `5750f056`, `62117ecd`, `bcffb8cc` — **0 treo**).

## 2. Move thật (không đổi logic/chẩn đoán/thứ tự)

Phương pháp: tách từng tệp gốc `2b5ccb51:path` thành các **đoạn theo ranh giới hàm**, so **chuỗi
token** (bỏ comment, bỏ `pub`/`pub(super)`/`pub(crate)` — các token còn lại phải trùng **từng token
một**) với tệp mới tương ứng. Kết quả: **không đoạn nào lệch logic**; mọi khác biệt chỉ là:

1. thêm header `impl<'a> … {` / `}` khi một `impl` bị cắt sang tệp khác;
2. dấu `pub(super)`/`pub(crate)` thêm/bớt trên chữ ký (bảng §3);
3. tiền tố đường dẫn `super::` → `super::super::` khi module sâu thêm một tầng
   (`chieu::tach_phay`, `TOAN_TU`, `num`, `thay_gen`, `SymKind`, `FnKind`…);
4. đúng **một** thay đổi thân hàm có chủ đích: `ha_complete` nhận `&Option<(HExpr, Type)>`
   nên `let Some((up, _)) = truoc.clone() else` (thay vì move `truoc`) — hành vi y hệt, chỉ 1
   `clone` khi gặp stage `| complete`.

Bảng token (old → new; "diff" = số token khác, đã đối chiếu thủ công từng dòng):

| Tệp gốc | Đoạn | Token cũ → mới | Khác biệt |
|---|---|---:|---|
| `lower/trait_impl.rs` 1.152 | `mod.rs` (18–177) | 1.076 → 1.077 | +1 `}` đóng impl |
| | `kiem.rs` (178–446) | 1.428 → 1.437 | +header impl |
| | `sinh.rs` (447–467 + 673–948 + 1113–hết) | 2.255 → 2.261 | +header impl, chữ ký `sinh_pt`/`co_float_sau` xuống dòng |
| | `json.rs` (468–672 + 949–1.089) | 2.499 → 2.511 | +header impl, `pub(crate)`→`pub(super)` |
| | `DeriveNguon`/`PtSinh` (1.090–1.112) | 91 → 91 | **trùng tuyệt đối** |
| `lower/cmd.rs` 1.115 | đầu `ha_stage` (272–296) | 131 → 152 | +nhánh gọi `ha_complete` |
| | `| complete` (297–339) | 214 → 233 | +chữ ký `ha_complete`, +`.clone` |
| | thân giữa (340–976) | 4.355 → 4.356 | +1 `}` đóng nhánh |
| | tail builtin (977–1.061) | 516 → 572 | +chữ ký `ket_thuc_stage` |
| `tyck/expr.rs` 1.079 | `mod.rs` (17–483) | 2.730 → 2.731 | +1 `}` đóng impl |
| | `toan_tu.rs` (484–632) | 786 → 801 | +header impl, `super::`→`super::super::` |
| | impl 2 (633–681) | 279 → 289 | +header impl |
| | `builtin.rs` (682–1.025) | 2.199 → 2.210 | +header impl, đường dẫn |
| | enum (1.026–1.078) | 296 → 305 | +header impl |
| | `_giu` (1.079) | 15 → 15 | **trùng tuyệt đối** |

**Thứ tự chẩn đoán/span**: mọi câu `self.diag(...)` giữ nguyên vị trí trong thân hàm và giữ nguyên
tham số `span`; không hàm nào bị đổi thứ tự nhánh. Không có `#[cfg(test)]`/`#[test]` trong 3 tệp
gốc (0 → 0), nên không thể mất test khi tách.

## 3. LOC & API

### 3.1 LOC (`wc -l` tại HEAD)

| Gốc | Sau | Tệp |
|---|---|---|
| `trait_impl.rs` **1.152** | **1.191** | `mod` **199** + `kiem` **277** + `sinh` **356** + `json` **359** (+`sieu` 38 giữ nguyên) |
| `cmd.rs` **1.115** | **1.174** | `mod` **37** + `text` **278** + `stage` **680** + `complete` **59** + `args` **120** (+`doc` 240/`hop` 213 giữ nguyên) |
| `tyck/expr.rs` **1.079** | **1.115** | `mod` **589** + `toan_tu` **160** + `builtin` **366** (+`goi` 444/`kenh` 143/`phu` 607 giữ nguyên) |

Max trong phạm vi: **680** (`cmd/stage.rs`) — đạt mục tiêu < 1.000; trần cứng R1 1.500: OK.
Chênh +39/+59/+36 dòng là doc module, `use`, và header `impl` — không có dòng logic mới.

### 3.2 API công khai — **không đổi**

- `pub` item trước/sau: **2 → 2** — `pub struct TraitSig`, `pub type PhuongThucTrait` (đều ở
  `trait_impl/mod.rs`; `lower.rs:33–34` vẫn `mod trait_impl; pub use trait_impl::TraitSig;`).
- Không thêm `pub fn`/`pub use`/`pub mod` nào; `tyck::expr::phu` vẫn `pub(super)`; `goi`/`kenh`
  vẫn private như cũ.
- Thay đổi hiển thị nội bộ (đều **không** phải API công khai):

| Item | Cũ | Mới | Ghi chú |
|---|---|---|---|
| `sinh_pt`, `co_float_sau`, `json_hop_le` | `fn` (private) | `pub(super)` | gọi chéo trong `trait_impl` |
| `sinh_derives_json` | `pub(crate)` | `pub(super)` | **thu hẹp** — caller chỉ ở `sinh.rs` |
| `ha_stage` | `fn` (private) | `pub(super)` | `cmd/hop.rs` (sibling) gọi |
| `ket_thuc_stage`, `ha_complete` | (tách từ thân `ha_stage`) | `pub(super)` | gọi từ `stage.rs` |
| `vach_text`, `kieu_text` | `pub(super)` (`= lower`) | `pub(crate)` | `lower/sugar.rs`, `lower/than.rs` gọi — xem P2 |
| `chuyen_doi`, `chuyen_doi_text` | `pub(super)` (`= lower`) | `pub(super)` (`= lower::cmd`) | **thu hẹp**, caller chỉ trong `cmd` |
| `kiem_bin`, `kiem_std`, `kiem_builtin` | `fn` (private) | `pub(super)` | gọi chéo trong `expr` |

## 4. Hành vi & cổng độc lập (số thật, chạy lại từ đầu trên HEAD `bcffb8cc`)

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `cargo test -p vis-types` | **exit 0** — 221 unit + **2** `types_it` + **2** doc-test, 0 failed | `vislogs/review-st318c-test-types.log` · `af88769a…6951fe` |
| `./vis-dev gate --fast` | **PASS** — nextest **3901/3901**, 4 skipped; luật R1..R10 OK | `vislogs/review-st318c-gate.log` · `97e1ecd2…a87b5` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — tong=100000, chay=93.768, loi_tinh=6.232, 157,5 s | `vislogs/review-st318c-diff.log` · `81c023cb…562fa` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** | `vislogs/review-st318c-clippy.log` · `ebe93c01…37025a` |
| `cargo fmt --all --check` | **exit 0** | — |
| R1 (≤1.500; mục tiêu <1.000) | OK — max trong phạm vi **680**; max toàn `src` **1.088** (`vis-hir/src/hir.rs`, có sẵn, ngoài phạm vi) | gate log |
| R8 (trần 15/40) | **39/40**, `crates/vis-cli/tests` **8/15** | đếm độc lập + gate log |

Môi trường: toolchain ghim `1.100.0` qua `rust-toolchain.toml` (PATH cần `~/.cargo/bin`);
`CARGO_BUILD_JOBS=2`; không `cargo clean`; đĩa trống lúc bắt đầu **9,2 GB** (> 6 GB).

## 5. Hợp nhất

- `git merge-tree --write-tree main HEAD` → tree `6934ee6f1e53a9c27cfc02955b95e07cf37fc6f2`,
  **exit 0, không xung đột**.
- Chênh lệch khi merge (phía `main`): `agent/{ACTIVE,DECISIONS-INDEX,DECISIONS,QUEUE,ROADMAP,STATE}.md`,
  3 journal mới (VD-296/VD-297/review-st318b + st318b), và
  `crates/vis-conformance/tests/misc_api_it/host_parity/net_http.rs` (từ ST-318b).
- Nhánh này **không** chạm các tệp đó ⇒ merge sạch; `STATE.md` trên nhánh không đổi (xem P3).

## 6. Rủi ro & finding

**P1 — không có.** Không mất `pub use`, không private-hoá sai (mọi caller đều nằm trong phạm vi
hiển thị mới; crate + all-targets clippy biên dịch sạch), không mất test, không đổi thứ tự
chẩn đoán/span, `stage.rs` 680 đã tách `| complete` đúng (nhánh `complete` chuyển nguyên văn sang
`ha_complete`, phần tail builtin chuyển nguyên văn sang `ket_thuc_stage`, `ha_stage` chỉ còn
`return self.ha_complete(...)` + `self.ket_thuc_stage(...)`).

**P2 (thấp, không chặn) — `vach_text`/`kieu_text` nới `pub(crate)`**
- Vị trí: `crates/vis-types/src/lower/cmd/text.rs` (`pub(crate) fn vach_text`, `pub(crate) fn kieu_text`).
- Lý do: sau khi sâu thêm một tầng module, `pub(super)` chỉ còn `lower::cmd`; caller thật là
  `lower/sugar.rs:173` và `lower/than.rs:507,554` (thuộc `lower`).
- Mức tối thiểu đủ dùng là `pub(in crate::lower)`; `pub(crate)` rộng hơn nhưng **không** đổi API
  công khai và không đổi hành vi. Đề nghị (tuỳ A0) siết lại khi có dịp, không cần sửa ở ST này.
- Tái lập: `grep -n 'fn vach_text\|fn kieu_text' crates/vis-types/src/lower/cmd/text.rs`.

**P2 (thấp, ghi nhận) — `truoc.clone()` trong `ha_complete`**
- Vị trí: `lower/cmd/complete.rs:29`. Do chữ ký đổi sang `&Option<(HExpr, Type)>`; hành vi y hệt
  bản gốc (move sau clone), chi phí 1 clone chỉ khi gặp stage `| complete`. Chấp nhận được.

**P3 (hành chính)**
1. Commit `8b368d78` ghi "mod 36" và liệt kê `doc/hop` như tệp mới; thực tế `doc.rs`/`hop.rs` có từ
   ST-106, `cmd/mod.rs` thành **37** dòng sau `62117ecd` (thêm `mod complete;`). Chỉ lệch mô tả.
2. Nhánh không cập nhật `agent/STATE.md` (trong phạm vi cho phép nhưng không bắt buộc). Merge sạch;
   đề nghị QA/A0 ghi dòng ST-318c khi hợp nhất.
3. Chưa build độc lập từng commit trung gian (chỉ kiểm cấu trúc `mod` + build HEAD). Mỗi commit
   đều nguyên tử theo tệp/mod nên rủi ro bisect thấp.

## 7. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"        # toolchain ghim 1.100.0
cd /home/ps/viswt/st318c

git status --porcelain && git log --oneline 2b5ccb51..HEAD
git diff --stat 2b5ccb51..HEAD

CARGO_BUILD_JOBS=2 cargo test -p vis-types
CARGO_BUILD_JOBS=2 ./vis-dev gate --fast
VIS_DIFF_N=100000 ./bin/vis-diff
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

git merge-tree --write-tree main HEAD
```

## 8. Chứng cứ commit

- HEAD review: `bcffb8cc` · base `2b5ccb51` · `main` `2c7dbef6`.
- 5 commit `[ST-318c]`: `5c6d441f`, `8b368d78`, `5750f056`, `62117ecd`, `bcffb8cc`.
- **Kết luận: PASS**, đề nghị A0 `DONE` cho ST-318c (không có điều kiện P1).
