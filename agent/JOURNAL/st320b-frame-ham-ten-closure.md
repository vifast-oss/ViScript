<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-320b — Frame: hàm tên qua giá trị **trong closure** dùng env module (`#212`, P2-2 ST-319c2)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2)
- **Worktree**: `/home/ps/viswt/st320b` · **Nhánh**: `plan/st320b`
- **Nền**: `main` @ `d1254b05` (mã sản phẩm cuối `ee8f252d`, đã có 319a/c2/e)
- **Phạm vi**: `crates/vis-interp/src/interp.rs` + `crates/vis-interp/src/frame/{expr,params,worker}.rs`
  + test `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` (thêm `w25`).
  **Không** chạm VM/bytecode/loader/cache/VBC1, **không** merge/push (main frozen).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Lỗi | hàm tên qua giá trị tạo **trong lambda** chụp `frame.cur().env` (snapshot closure) rồi `merge` chép ngược ⇒ frame lệch VM |
| Sửa | `Value::Fn` hàm tên chụp `it.env_module()`; gọi hàm tên chạy trên env module và **không merge** vào `c.env`; lambda giữ nguyên by-value (#206/#209) |
| Đồng bộ | `env_module` được gán cùng `globals` tại **2** điểm frame gán `it.env` (`params.rs`, `worker.rs`) |
| Test `w25` | **đỏ trước** (`1,1,1` vs `0,0,1`) → **xanh sau**; write-only + read-only/`await`, frame interp + frame VM W=1, literal `0,0,1` / `903,203,303,403` |
| `st317b` | **33/33 PASS**, 0 ignore (gồm `w19..w24`) |
| `cargo test -p vis-interp` | **5 + 55 + 2 doc-test PASS** |
| `./vis-dev gate --fast` | **PASS** — nextest **3935/3935**, 4 skip; fmt/clippy/R1..R10 OK |
| `VIS_DIFF_N=100000` | **`lech=0`**, `chay=94425`, seed `3523158054`, 98,9 s |
| clippy `--workspace --all-targets -D warnings` | **exit 0** |

## 1. Nguyên nhân

P2-2 của review `ST-319c2` (§8.6) và P2-2 `ST-319e` (§7): `let h = tang` **bên trong lambda** đi
qua nhánh `HExpr::Var` riêng của frame (`frame/expr.rs:48`), nhánh này chụp
`frame.cur().env` — tức snapshot của closure — thay vì env module. Khi gọi `h()`:

1. `call_value` dựng `CallFrame` với `by_value = c.name.is_empty()` = `false` (đúng: hàm tên),
   nhưng `merge: Some(c.env.clone())` trỏ **snapshot closure**;
2. thân hàm tên ghi global qua `DongBo::ghi` → `globals` đúng, nhưng khi khung xong `merge`
   chép toàn bộ env của hàm tên (đã được `nap_vao_env` đồng bộ `g`) ngược vào `c.env`;
3. closure đọc `g` từ snapshot của nó ⇒ thấy giá trị global vừa ghi, trong khi SPEC §4.7/VM giữ
   closure **by-value**.

Thêm bẫy: `env_module` (thêm bởi ST-319e) **không được đồng bộ** khi frame gán `it.env = globals`
(`frame/params.rs:20`, `frame/worker.rs:104`), nên nếu chỉ đổi `HExpr::Var` sang `env_module` thì
giá trị hàm tên sẽ chụp map rỗng của `Interp::new` — đúng P2-2 review 319e.

## 2. Sửa (tối thiểu, interp-only)

| Tệp | Thay đổi |
|---|---|
| `interp.rs` | thêm `dat_env_module(env)` + `env_module()` (ST-320b/#212) — một chỗ đặt bất biến "env module" cho frame |
| `frame/params.rs` | sau `it.env = globals.clone()` gọi `it.dat_env_module(globals.clone())` (đường có `main`) |
| `frame/worker.rs` | như trên ở nhánh **không `main`** (thân cấp tệp chạy trực tiếp trên `globals`) |
| `frame/expr.rs` `HExpr::Var` | `Value::Fn` của hàm tên: `env: it.env_module()` thay `frame.cur().env` |
| `frame/expr.rs` `call_value` | `la_closure = c.name.is_empty()`; closure giữ `goc = c.env`, `merge = Some(c.env)`, `by_value = true`; **hàm tên** `goc = it.env_module()`, `merge = None`, `by_value = false` |

Không đổi `MakeLambda` (lambda vẫn snapshot `frame.cur().env`), không đổi `DongBo`/`nap_vao_env`
(#203, #206 giữ nguyên).

## 3. Repro trước/sau

Đo bằng binary `target/debug/vis` (frame = `vis run`, VM = `vis run --vm`), W=1:

| Ca | Chương trình | Trước (frame) | VM | Sau (frame) |
|---|---|---|---|---|
| write-only | `tang` ghi `g=1`; closure `let h = tang; h(); g`; main `print(f())` ×2 rồi `tang(); print(g)` | **`1,1,1`** | `0,0,1` | **`0,0,1`** |
| read-only + await | closure tạo trước task ghi `g=3`; trong closure `let h = doc; spawn dat; await t; print(900+h())` … `print(400+h())` | **`900,200,300,400`** | `903,203,303,403` | **`903,203,303,403`** |

- Trước vá, `h()` đọc snapshot `g=0` (900/200/300/400) và `merge` làm closure thấy `g=1`
  (1,1,1). Sau vá, `h()` chạy trên env module (`g=3` → 903/203/303/403) còn snapshot closure
  vẫn `0` (write-only `f()` trả `0`).
- Red-first thật với test `w25` (stash 4 tệp mã, giữ test):
  `left: "1\n1\n1\n"` / `right: "0\n0\n1\n"` — `st320b-w25-red.log`.
- W=2/4 (kiểm tay, ngoài phạm vi `w25`): write-only giữ `0,0,1` cả frame lẫn VM; read-only báo
  `VS0509` ở **cả hai** engine (task ghi global khi W>1 — đúng thiết kế).

## 4. Test `w25`

`w25_frame_w1_ham_ten_qua_gia_tri_trong_closure` — một test, hai chương trình (write-only và
read-only + `await`), chạy **frame interp W=1** (`run_async_with_hosts_caps`) và **frame VM W=1**
(`run_async_with_full_hosts_caps`); mỗi engine assert **literal riêng**, `diagnostics.is_empty()`,
`exit == 0`, và `so_khop`. Không `#[ignore]`. Tên ca nằm trong tệp con + aggregator sẵn có ⇒ R8
giữ **39/40** (không thêm tệp `.rs` cấp 1).

## 5. Cổng máy (log ngoài repo)

| Cổng | Kết quả | Log · SHA-256 |
|---|---|---|
| `st317b_worker_orchestrate` | **33/33 PASS**, 0 ignore | `/home/ps/vislogs/st320b-st317b.log` · `5df0669f9e306c61e123e4f3e6fc590023e579147d9eadb2be0bd7f12315a9bf` |
| `cargo test -p vis-interp` | **5 + 55 + 2 doc-test PASS** | `/home/ps/vislogs/st320b-interp-test.log` · `0b017f21260547c8c6fa9a8a8a032c714aea75d678b18c910b9bd58ecb5496a2` |
| `./vis-dev gate --fast` | **PASS** — nextest **3935/3935** (4 skip), fmt/clippy/R1..R10 | `/home/ps/vislogs/st320b-gate-fast.log` · `7126ba8756bf27e39ffa750b0d47d2d781505588769847a65aa35256d03e7bd6` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — `tong=100000 chay=94425 giay=98.9 seed=3523158054` | `/home/ps/vislogs/st320b-diff100k.log` · `1a2ddbb11f7019d494db7c3be217136d0cf0e62a48211456543d28c63725c7de` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** | `/home/ps/vislogs/st320b-clippy.log` · `3ccc5bd4a22ea789e80c3ea32a18ec2dd2275a6afc171069fc18d957fb82b227` |
| red-first `w25` | FAIL đúng dự kiến (`1,1,1` vs `0,0,1`) | `/home/ps/vislogs/st320b-w25-red.log` · `85cfe18092bda120b1d05546b5e52a836d14c0c65e0fe263acd4270aa61ec6db` |
| green `w25` | **PASS** | `/home/ps/vislogs/st320b-w25-green.log` · `e476d2a23a39c40acf30e96b9ab89d4726dbaab05c509cf9e36094226b6e42ca` |

Tái lập: `PATH=$HOME/.cargo/bin:$PATH CARGO_BUILD_JOBS=2 TMPDIR=/home/ps/vislogs/tmp-st320b`
(toolchain `1.100.0`; `cargo` hệ thống 1.93.1 không dựng được workspace). Đĩa bắt đầu ~23 GB,
không `cargo clean`.

## 6. Rủi ro / ghi chú

- **Hàm tên qua giá trị không còn `merge` vào `c.env`**: đúng ngữ nghĩa (hàm tên không bắt biến);
  trước đây merge còn kéo `let` cục bộ của hàm vào env gọi. `w23` (`let h = tang` ở `main`) vẫn
  xanh nhờ `DongBo` ghi `globals` + `nap_vao_env` đồng bộ khung `!by_value`.
- **Closure/lambda không đổi**: `MakeLambda` snapshot; `call_value` nhánh `name` rỗng giữ
  `merge = Some(c.env)`, `by_value = true` ⇒ `w19..w24` giữ nguyên literal.
- **#203 không đổi**: task/main ghi global vẫn xuyên qua `DongBo`; `w11/w14/w15/w16/w18/w23` xanh.
- **Không sửa** P2-1 (rò tham số khi trap sớm trong `call_hfn_than`) và P2-3 (test W>1 read-only)
  của review 319e — ngoài phạm vi brief; W>1 chỉ kiểm tay, không thêm ca.
- Không chạm cache/`PHIEN_BAN`/VBC1 (interp-only), không đổi output non-frame.
- **Chưa merge/push** — chờ A0; nhánh `plan/st320b` giữ checkpoint này.
