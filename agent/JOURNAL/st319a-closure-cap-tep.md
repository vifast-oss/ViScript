<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319a — Đóng `#206`: closure đọc/ghi biến cấp tệp theo **by-value**

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st319a`
- **Nền**: `main` @ `d97773de` (`VD-299`, Sóng 7 `DONE`)
- **Worktree**: `/home/ps/viswt/st319a` — không chạm repo chính.
- **Trạng thái**: **F2** — đã vá cache version + bổ sung test bền/lồng; chưa tuyên bố `DONE`.

## 0. TL;DR

| Trước | Sau (default interp ↔ VM) |
|---|---|
| `var g = 0` / closure đọc `g`: interp `0,0`, VM `0,1` | cả hai `0,0` |

Hướng: **codegen** chụp biến cấp tệp **có xuất hiện trong thân closure** vào `MakeClosure`
(ô cục bộ riêng + `LoadCaptured`/`Assign` prologue — cùng cơ chế `var` capture ST-60b), nên
closure đọc/ghi **bản snapshot**; **không đổi VBC1**, không đổi `vis-vm`.

## 1. Thay đổi

| Tệp | Nội dung |
|---|---|
| `crates/vis-bytecode/src/codegen/bien_tu_do.rs` (mới) | quét `sym` được tham chiếu trong `HBody`/`HStmt`/`HExpr` (đi xuyên lambda lồng); sym là duy nhất nên không cần miền ràng buộc |
| `crates/vis-bytecode/src/codegen/expr.rs` | `HExpr::Lambda`: sau snapshot `f.locals`, chụp thêm global **được tham chiếu** (dedupe, sắp theo slot ⇒ closure không chạm global giữ bytecode cũ). `HExpr::Var`: ưu tiên ô cục bộ (closure đã chụp) trước kho global |
| `crates/vis-bytecode/src/codegen/cau.rs` | `slot_cua`: `captured` → `f.locals` → `globals` → cấp ô mới (ghi trong closure không xuyên global) |
| `crates/vis-conformance/tests/misc_api_it/st319a_closure_cap_tep.rs` (mới, `#[path]` trong `lib_api_it.rs`) | 3 ca: đọc snapshot, ghi không xuyên + bền qua nhiều lần gọi, lambda lồng giữ snapshot closure ngoài; mỗi ca chạy 4 đường (interp/VM mặc định + W=1 cả hai engine) |
| `crates/vis-vm/tests/diff_100k.rs` | thêm nhóm sinh `ClosureCapTep` (3 mẫu cố định) để `vis-diff` bắt được lớp lỗi này về sau |

## 2. Kết quả cổng (tại checkpoint)

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all` | sạch |
| `cargo check --workspace --all-targets` | PASS (18,74 s) |
| `./vis-dev gate --fast` | **PASS** — nextest **3915/3915** (0 fail), có 3 ca `st319a_closure_cap_tep` |
| `cargo test -p vis-bytecode -p vis-vm -p vis-interp` | PASS (bytecode 103 + …, vm 56 + diff, interp 55) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `tong=100000 chay=94425 lech=0` (80,2 s) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `vis test test/sem` / `--vm test/sem` | **525/525** cả hai engine |

## 3. Việc còn lại / phát hiện ngoài lane

- **Đường frame/async (`run_async_*`, tức `--workers`/chương trình có `spawn`-`await`)**: closure
  **ghi** global vẫn lệch — interp frame in `5\n5` (ghi xuyên qua `DongBo::ghi` trong
  `vis-interp/src/frame/worker.rs`), VM in `5\n0`. Đọc thì đã khớp (`0\n0`). Tệp `worker.rs`
  **thuộc lane 319c / bị cấm ở 319a** ⇒ không sửa; đề xuất A0 giao 319c hoặc mở nợ mới.
- Khi hoàn tất: chạy lại `gate --fast` + `vis-diff 100k` sau journal/commit cuối, rồi báo A0
  nghiệm thu (chưa tự tuyên bố `DONE`).

## 4. F2 — audit `7f209dfb` (PASS có điều kiện)

### 4.1 Cache version (`crates/vis-loader/src/cache.rs`)

Nhãn mới cuối `PHIEN_BAN`: **`+st319a-closure-cap-tep-2026-11-27`** (giữ nguyên `CARGO_PKG_VERSION`,
`VERSION` VBC1/ABI/định dạng). Ghim lại `cache::tests::khoa_cache_ghim`:
`de308705a335d3da380dc8ff57b57b50f2ffccbf281af4f1a3bc45eb2e117bba`, kèm `assert_ne` khoá cũ
`c5c4d5ac…` (“cache sinh trước closure-capture-by-value không được tái dùng”).

**Repro có cache** (`/tmp/st319a-repro.vis`, `vis run --vm`, `XDG_CACHE_HOME` riêng):

| Bước | Binary | Cache dir | stdout | `.vbc` |
|---|---|---|---|---|
| Trước | pre-fix (`ae0089c0^`) | rỗng | `0,1` | 1 |
| Trước (tái dùng) | post-fix codegen, **PHIEN_BAN cũ** | còn `.vbc` pre-fix | **`0,1`** (lỗi audit) | 1 |
| Sau | post-fix, **PHIEN_BAN mới** | vẫn dir trên | **`0,0`** | 2 |
| Sau (cache hit) | post-fix | dir trên | **`0,0`** | 2 |

### 4.2 Test bền / lambda lồng ghi (audit bắt gap)

- `ghi_global_qua_closure_ben_qua_nhieu_lan_goi`: `g = g + 1; g`, gọi 2 lần, `print(g)` ⇒
  **`1\n2\n0\n`** (ca `g = 5` cũ idempotent không chứng minh state bền).
- `lambda_long_ghi_global`: `ngoai` tạo ở `g=0`, sau `g=7`, inner `g = g + 1; g` gọi 2 lần +
  `print(g)` ⇒ **`1\n2\n7\n`**.
- Generator `vis-vm/tests/diff_100k.rs` thêm mẫu thứ 4 `g = g + 1` (bắt lớp reset-env).
- `st319a_closure_cap_tep`: **5/5** PASS (mỗi ca 4 đường interp/VM default + W=1).

### 4.3 Nợ mới (không sửa — pre-existing, ngoài lane 319a)

**Closure gọi hàm cấp tệp đọc/ghi global** vẫn lệch (interp dùng env **động** của closure,
VM đọc/ghi global **live**):

```vis
var g = 0
fn doc() -> int: g
fn main():
    let f = fn() -> int: doc()
    g = 9
    print(f())          # interp 0, VM 9
```

```vis
var g = 0
fn ghi() -> int:
    g = 5
    g
fn main():
    let f = fn() -> int: ghi()
    print(f())          # interp 5, VM 5
    print(g)            # interp 0, VM 5
```

Không do ST-319a (có trước, `vis-diff` không sinh mẫu). Đề xuất A0 cấp **nợ mới** lane
`vis-interp` (cùng họ `#206`), hướng: chốt SPEC “hàm gọi trong closure thấy global live hay
snapshot”, rồi sửa một phía + ca parity.

