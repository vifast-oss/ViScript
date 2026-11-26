<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319c2 — `#206` vế frame/async: closure ghi global **không xuyên** + vá P2-1

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2)
- **Nhánh**: `plan/st319c2` · **Worktree**: `/home/ps/viswt/st319c2`
- **Nền**: `plan/st319a` @ `ae0089c0` (closure capture by-value phần lõi)
- **Trạng thái**: mã + 6 ca test xanh (`w19..w24`); `gate --fast` 3921/3921, `vis-diff 100k`
  (`lech=0`), clippy đều PASS sau vá P2-1. **Chờ A0 nghiệm thu — chưa tự tuyên bố DONE.**

## 0. TL;DR

| Đường | Trước | Sau |
|---|---|---|
| frame interp W=1 (CLI `vis run`, chương trình async) | `5\n5` | `5\n0` |
| frame VM W=1 | `5\n0` | `5\n0` |
| non-frame interp/VM (mặc định) | `5\n0` | `5\n0` |

Repro: `var g=0`; `let f = fn() -> int: g = 5; g`; `spawn dummy` + `await`; `print(f())`;
`print(g)`. Kỳ vọng SPEC §4.7 (closure bắt theo giá trị): closure đọc **5**, global ngoài **0**.

## 1. Root cause + cách sửa (lane `vis-interp/src/frame/**`)

`DongBo::ghi` / `ghi_bien_cap_tep` (`frame/worker.rs`) đồng bộ **mọi** lệnh ghi có `sym` trong
`globals`, kể cả khi đang chạy thân closure đã capture by-value ⇒ ghi cục bộ bị đẩy ra global.

- `frame/state.rs`: `CallFrame` thêm `by_value: bool` (khung gọi closure snapshot).
- `frame/expr.rs`: `call_value` đặt `by_value = c.name.is_empty()` — lambda do `MakeLambda`
  tạo (tên rỗng) là snapshot; giá trị hàm **hàm tên** (`let f = tang`) giữ env chia sẻ như
  interp/VM ⇒ `false`. `dispatch` hàm tên giữ `false`.
- `frame/worker.rs`:
  - `DongBo::ghi`: khung **đang thực thi** là closure ⇒ trả sớm, không ghi `globals`, không
    đánh dirty, không đổi `frame_seq`. Hàm tên vẫn xuyên như F2/F3 (`#203`).
  - `nap_vao_env`: chỉ bỏ qua khung `by_value` (closure snapshot); **mọi** khung hàm tên/thường
    — kể cả khung hàm tên nằm **trên** closure trong stack — vẫn nhận đồng bộ F2/F3.
    (Bản checkpoint đầu cắt cả đoạn từ closure trở lên ⇒ hồi quy P2-1: hàm tên gọi closure có
    `await` rồi đọc global ra `0\n3` thay vì `3\n3`; audit bắt, đã vá.)
- Không đổi `vis-vm`, không đổi VBC1, giữ semantics `#203`.

## 2. Test

`crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` (aggregator có, R8):

| Ca | Nội dung | Kết quả |
|---|---|---|
| `w19_frame_w1_closure_ghi_global_khong_xuyen` | repro trên, frame W=1 interp + VM | `5\n0\n`, exit 0, `so_khop` |
| `w20_frame_w1_closure_doc_sau_ghi_task_ghi_global` | task ghi `g=3` giữa 2 lần gọi closure; closure `g = g + 5` + `print(g)` | `5\n5\n10\n10\n3\n`, parity interp ↔ VM |
| `w21_frame_w1_closure_await_ghi_global_sau_resume` | closure `await dat()` (task ghi `g=3`) rồi `g = g + 5` | `5\n3\n` — chạm `nap_vao_env` lúc resume |
| `w22_frame_w1_closure_ghi_global_trong_task` | closure trong thân task ghi `g=5`, task in `f()` rồi `g`, trả `1` | `5\n0\n1\n` |
| `w23_frame_w1_ham_ten_qua_gia_tri_van_xuyen_global` | `let h = tang`; `h()` ghi global trong frame | `1\n` — khoá heuristic `c.name.is_empty()` |
| `w24_frame_w1_ham_ten_tren_closure_nap_van_sync` | **hồi quy P2-1**: `doc()` gọi từ closure, tự `await dat()` rồi đọc `g` | `3\n3\n` (trước vá `0\n3`) |

Mỗi ca assert stdout **literal từng engine** (interp + VM frame W=1), `diagnostics` rỗng,
`exit == 0`, và `so_khop`; không `#[ignore]`.

## 3. Cổng đã chạy (sau vá P2-1)

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all` | sạch |
| `cargo test -p vis-interp` | **PASS** 55 + 2 doc-test |
| `cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` | **PASS 26/26** (w19..w24) |
| `./vis-dev gate --fast` | **PASS** — nextest **3921/3921** (4 skipped), L0 OK |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (`chay=94425`, 183,0 s, seed `3523158054`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (3,85 s incremental) |

Log: `/home/ps/vislogs/st319c2-fix-{st317b,gate-fast,diff100k,clippy}.log` (+ `.sha256`).

## 4. Còn lại / rủi ro

- Sẵn sàng A0 nghiệm thu; **chưa tự tuyên bố DONE**. Không push/merge.
- `w22` in `5\n0\n1\n` đúng literal audit: main chỉ `print(await t)` (kết quả task `1`); nếu
  muốn main đọc thêm global thì phải thêm dòng `print(g)` ⇒ `5\n0\n1\n0\n`.
- Phát hiện ngoài phạm vi (không sửa): gọi **hàm tên** trong closure mà hàm đó ghi global —
  non-frame interp `1,0` vs VM `0,1` (lệch sẵn giữa hai engine mặc định). Frame path hiện theo
  VM/`ghi` xuyên (`0,1`); đầy đủ cần đổi env hàm tên theo caller + merge, nên tách nợ riêng.
