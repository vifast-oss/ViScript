<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-323c — Đóng nợ #213: free HOF gọi callback non-frame ghi global phải đánh `Dirty`

- **Ngày**: 2026-11-28 · **Vai**: Builder (Agent 2)
- **Worktree**: `/home/ps/viswt/st323c` · **Nhánh**: `plan/st323c` · **Nền**: `main` @ `eb41d61c`
- **Phạm vi (lane)**: `crates/vis-interp/src/frame/worker.rs` + test
  `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` (thêm `w28`, `w29`).
  **KHÔNG** đụng `crates/vis-interp/src/interp.rs` (đang là lane ST-323d / #214), **không** đụng
  `interp/goi_ham.rs`, `vis-bytecode`/VM; **không** merge/push, không đụng repo chính.
- **Nguồn**: audit sâu ST-320b `4139c46a…` §P2-2 (§2.2/§4.1) + `repro/`; nợ `#213` tại
  `agent/SO-NO.md:302` (pre-existing, **không** gán vào `#212`).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Repro W=1, cả 2 engine | `review-manual-hof-frame2.vis`: interp **`[3],3,0,0`** / VM **`[3],3,0,3`** (exit 0 cả hai) — đúng số audit |
| Nguyên nhân | `DongBo::ghi_ngoai` bỏ qua ca `frame.cur().by_value` ⇒ callback HOF (hàm tên gọi từ lambda) ghi thẳng globals nhưng **không đánh dirty**, main frame giữ cache cũ |
| Sửa | Bỏ early-return đó; vẫn đánh dirty + bơm cache; `nap_vao_env` **vẫn lọc** khung `by_value` ⇒ closure snapshot không bị đè (#206/#209 giữ nguyên) |
| Test `w28` | **đỏ trước** (`"[3]\n3\n0\n0\n"` vs `"[3]\n3\n0\n3\n"`) → **xanh sau**, assert literal riêng cho từng engine |
| Test `w29` (#215) | W=2/4 read-only, hàm tên/HOF trong closure — `[1, 2]\n107\n7\n` cho **mỗi** engine, mỗi W; **xanh** |
| `cargo test -p vis-interp` | **5 + 55 + 2 doc-test PASS** |
| `./vis-dev gate --fast` | **PASS** (fmt + clippy + R1..R10 + self-test + build + doc-examples + L0) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (`chay=94425`, `loi_tinh=5575`, 147,1 s, seed `3523158054`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |
| R8 | 39/40 tệp test cấp 1 (không thêm binary mới; gộp vào aggregator `lib_api_it.rs`) |

## 1. Repro trước/sau (binary `target/debug/vis`, W=1, `--no-cache`)

| Chương trình | Trước: interp | Trước: VM | Sau: interp | Sau: VM |
|---|---|---|---|---|
| `/home/ps/vislogs/review-manual-hof-frame2.vis` (`let h = tang` trong lambda; `where([1,3],h)`; `doc()`, `g` trong lambda; `g` ở main) | `[3]` / `3` / `0` / `0`, exit 0 | `[3]` / `3` / `0` / `3`, exit 0 | **`[3]` / `3` / `0` / `3`**, exit 0 | `[3]` / `3` / `0` / `3`, exit 0 |
| `repro/p1-hof-write-main-frame.vis` | `[1]` / `1` | `[1]` / `1` | `[1]` / `1` (không đổi) | `[1]` / `1` |
| `repro/p2-task-hof-write-frame.vis` | `3` / `3` | `3` / `3` | `3` / `3` (không đổi) | `3` / `3` |
| `repro/p2-named-fn-frame.vis` | `73` / `3` / `3` | `73` / `3` / `3` | `73` / `3` / `3` (không đổi) | `73` / `3` / `3` |

Ba ca sau đã xanh trên nền `eb41d61c` nhờ cơ chế `frame_ghi_ngoai` có sẵn (ST-320b/P1-320b-1);
**chỉ ca closure lambda còn lệch** — đúng phần audit gọi là P2-2 chưa đóng. Log:
`st323c-repro-before.log` (`f8dbe0b4…`), `st323c-repro-after.log` (`b77714b2…`).

## 2. Nguyên nhân

1. `where/each/sort_by` là builtin gọi callback qua `Host::call` → `call_closure` (**non-frame**).
   Callback là hàm tên lấy qua giá trị (`let h = tang`) chạy trên env module (`#212`), nên lệnh
   `g = x` đi thẳng vào `globals`.
2. `Interp::ghi_o`/`lvalue_set` (interp.rs — **không sửa**) gọi `Interp::ghi_ngoai`, chỉ nhả sổ
   khi `Rc::ptr_eq(&self.env, &self.env_module)` — đã đúng: chỉ ghi module mới vào sổ.
3. `buoc_frame_dong_bo` (worker.rs) nhả sổ vào `DongBo::ghi_ngoai` sau mỗi work-item. Tuy nhiên
   hàm này có `if frame.cur().by_value { return; }`: khi callback nằm trong lambda `f()`, khung
   hiện tại **là closure snapshot** (`by_value = true`) ⇒ bỏ qua, **không tăng `seq`, không đánh
   `dirty`**. Main frame (env là clone của globals) không được `nap` ⇒ `print(g)` cuối in `0`.
4. VM in `3`; trước bản vá `#212` ca này cũng in `3` — đây là lệch nền, hệ quả của P2-2 sau khi
   `HExpr::Var` đổi đích env sang module.

## 3. Sửa (1 chỗ, trong lane)

`crates/vis-interp/src/frame/worker.rs`, `DongBo::ghi_ngoai`: bỏ early-return `by_value`, thay bằng
comment giải thích bất biến. Hệ quả:

- Với mọi `sym` trong sổ **đã được xác nhận ghi vào globals**, luôn `seq += 1` + `dirty.insert` ⇒
  peer frame/main frame thấy giá trị mới ở lần `nap` kế tiếp (hoặc ngay trong `buoc_frame_dong_bo`).
- `nap_vao_env` (không đổi) vẫn `filter(|cf| !cf.by_value)` ⇒ **không** đè snapshot closure;
  `print(g)` trong lambda vẫn `0` như VM/SPEC §4.7 (#206/#209).
- `DongBo::ghi` (work-item ghi), const F1, `#203`, `#212` (w25/w26/w27) **không đổi**.

Không sửa `interp.rs`, không đổi field/method nào ngoài `frame/**` — tránh conflict với ST-323d.

## 4. Test

- `w28_frame_w1_free_hof_ghi_global_trong_closure` — repro review-manual; assert literal
  `"[3]\n3\n0\n3\n"` cho **frame interp W1** và **frame VM W1**, `diagnostics.is_empty()`, `exit == 0`,
  `so_khop`. Red-first: `st323c-w28-red.log` (`3da70ea8…`) `left "[3]\n3\n0\n0\n"` /
  `right "[3]\n3\n0\n3\n"`; sau vá `st323c-w28-green.log` (`a5059000…`) **1 PASS**.
- `w29_frame_w2_w4_doc_only_hof_trong_closure` (#215) — task chỉ đọc, `let h = doc` + HOF
  read-only trong closure; W=2 và W=4, literal `"[1, 2]\n107\n7\n"` cho **mỗi** engine;
  `st323c-w28-w29.log` (`e9368a57…`) **2 PASS**.
- Cả `lib_api_it`: **435 passed / 1 failed / 3 ignored**. Ca đỏ
  `vs3_worker_eval_vm::vm_khong_tao_them_thread_sau_khi_join` là flake đo số thread
  (`rò thread: 25->33`) dưới tải song song, **không** liên quan đến frame; chạy riêng **PASS**
  (`st323c-flake-retry.log`, `ef3a7c61…`).

## 5. Cổng

| Lệnh | Log (sha256 rút gọn) | Kết quả |
|---|---|---|
| `cargo fmt --all -- --check` (sau `cargo fmt --all`) | `st323c-fmt-check.log` `c97cbf59…` | exit 0 |
| `cargo test -p vis-interp` | `st323c-test-vis-interp.log` `59413e13…` | 5 + 55 + 2 doc-test PASS |
| `./vis-dev gate --fast` | `st323c-gate-fast.log` `4aee1f81…` | **PASS** |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `st323c-diff-100k.log` `5b39f459…` | `lech=0`, exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | `st323c-clippy.log` `4b395c9c…` | exit 0 |

Môi trường: toolchain **1.100.0** (`rustc 1.100.0 (0ef1bc386)`), `CARGO_BUILD_JOBS=2`,
`TMPDIR=/home/ps/vislogs/tmp323c`, `CARGO_PROFILE_DEV_DEBUG=0`. Đĩa trước commit: **16 GB trống**
(> ngưỡng dừng 5 GB). Log đặt ngoài repo `/home/ps/vislogs/st323c-*.log`.

## 6. Bàn giao A0 / Gatekeeper

- Đề xuất đóng `#213` (đã có red-first + fix tối thiểu + parity cả 2 engine + diff 100k `lech=0`);
  giữ `#212` nguyên vẹn, không gán chéo.
- `#214` (rò tham số `call_hfn_than`) và lane `interp.rs` vẫn thuộc ST-323d — nhánh này **không đụng**.
- `#215` phần W>1 read-only đã được khoá bằng `w29` ở đây; W>1 **ghi** vẫn `VS0509` thiết kế, không đổi.
- Chưa merge/push; chờ Gatekeeper verify độc lập rồi A0 quyết.
