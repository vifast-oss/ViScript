<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319c2 — `#206` vế frame/async: closure ghi global **không xuyên** (WIP checkpoint)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2)
- **Nhánh**: `plan/st319c2` · **Worktree**: `/home/ps/viswt/st319c2`
- **Nền**: `plan/st319a` @ `ae0089c0` (closure capture by-value phần lõi)
- **Trạng thái**: **WIP checkpoint** — mã + test xanh trên `check`/target; chưa chạy `gate --fast`,
  `vis-diff 100k`, clippy. **Chưa DONE.**

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
  - `nap_vao_env`: có closure trong stack ⇒ chỉ đồng bộ các khung **phía ngoài** closure;
    không nạp ghi global của frame khác vào snapshot closure đang treo.
- Không đổi `vis-vm`, không đổi VBC1, giữ semantics `#203`.

## 2. Test

`crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` (aggregator có, R8):

| Ca | Nội dung | Kết quả |
|---|---|---|
| `w19_frame_w1_closure_ghi_global_khong_xuyen` | repro trên, frame W=1 interp + VM | `5\n0\n`, exit 0, `so_khop` |
| `w20_frame_w1_closure_doc_sau_ghi_task_ghi_global` | task ghi `g=3` giữa 2 lần gọi closure; closure `g = g + 5` + `print(g)` | `5\n5\n10\n10\n3\n`, parity interp ↔ VM |

## 3. Cổng đã chạy

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all` | sạch |
| `cargo check --workspace --all-targets` | **PASS** (34,65 s) |
| `cargo test -p vis-interp` | **PASS** 55 + 2 doc-test |
| `cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` | **PASS** 22/22 (có w19/w20) |

## 4. Còn lại / rủi ro

- Chạy `./vis-dev gate --fast`, `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0`), clippy trước khi
  báo A0 nghiệm thu.
- Phát hiện ngoài phạm vi (không sửa): gọi **hàm tên** trong closure mà hàm đó ghi global —
  non-frame interp `1,0` vs VM `0,1` (lệch sẵn giữa hai engine mặc định). Frame path hiện theo
  VM/`ghi` xuyên (`0,1`); đầy đủ cần đổi env hàm tên theo caller + merge, nên tách nợ riêng.
- ETA: gate + diff + clippy ~40–60 phút, commit sạch sau đó.
