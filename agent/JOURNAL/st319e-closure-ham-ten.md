<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319e — Đóng `#210`: hàm tên gọi từ closure dùng **env module** (interp non-frame)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st319e`
- **Nền**: `main` @ `07a5e2b3` (`[REVIEW] merge ST-319a`, closure capture by-value)
- **Worktree**: `/home/ps/viswt/st319e` — không chạm repo chính.
- **Trạng thái**: mã + test + cổng **PASS**, chờ Gatekeeper audit; chưa tự tuyên bố `DONE` toàn dòng.
- **Commit mã/test**: `327247ea` · **Commit journal**: (tệp này).

## 0. TL;DR

`#210`: trong **interp non-frame**, closure gọi **hàm tên cấp tệp** (`fn tang()`, `fn doc()`)
thì thân hàm tên chạy trên **snapshot env của closure** thay vì **env module**, nên đọc/ghi
global bị lệch với VM (VM đã đúng). Vá: `Interp` giữ thêm handle `env_module`; `call_hfn_trong`
tạm chạy thân hàm tên + default param trên env module, khôi phục trên mọi nhánh thoát;
`HExpr::Var` trỏ hàm tên chụp `env_module` cho `Value::Fn`.

| Trước (interp / VM) | Sau (4 đường: I / V / I-W1 / V-W1) |
|---|---|
| A: `0` / `1` · B: `7` / `9` · C2: `0` / `1` · E: `1\n0` / `0\n1` · P5: `5` / `1` | A `1` · B `9` · C2 `1` · E `0\n1` · P5 `1` — **cả 4 đường khớp** |

**Không đổi** VM, VBC1, `vis-interp/src/frame/**`; `HExpr::Lambda` vẫn **snapshot by-value**
theo ST-319a/SPEC §4.7.

## 1. Nợ `#210` và ngữ nghĩa chốt

`agent/SO-NO.md` dòng `#210` (Audit Sóng 8 · `VD-301`): “Closure gọi hàm tên đọc/ghi global —
non-frame interp `1,0`/`5,0` vs VM `0,1`/`5,5`”. SPEC §4.7 chỉ nói “Closure bắt biến theo giá
trị”, chưa nói env của **hàm tên** khi được gọi từ closure.

Ngữ nghĩa áp dụng (theo nhiệm vụ `ST-319e`, khớp VM):

- Closure vẫn bắt biến **theo giá trị** tại điểm tạo (`HExpr::Lambda` snapshot — ST-319a).
- **Hàm tên cấp tệp** không phải closure: khi được gọi (trực tiếp trong thân closure, qua
  giá trị `let h = tang`, hay qua `Value::Fn` trả về), thân nó đọc/ghi **global module live**.
- Sau khi hàm tên trả về, env của closure được **khôi phục nguyên trạng** (đọc `g` trong
  closure vẫn thấy snapshot của chính closure).

## 2. Root cause

Interp non-frame dùng **một** `Env = Rc<RefCell<HashMap<u32, Value>>>`:

1. `call_closure_trong` (`crates/vis-interp/src/interp/goi_ham.rs`) lưu `saved_env` rồi
   `self.env = c.env.clone()` — từ đây `self.env` là **snapshot** của closure.
2. `call_hfn_trong` cũ chạy thân hàm tên trên `self.env` ⇒ ghi `g = 1` rơi vào snapshot
   closure, global module không đổi; đọc `g` cũng thấy snapshot (cũ), không thấy global live.
3. `HExpr::Var` trỏ hàm tên (`eval.rs:94-103` cũ) tạo `Closure { env: self.env.clone() }`
   ⇒ `let h = tang` **trong closure** mang snapshot closure, nên `h()` cũng ghi/đọc sai chỗ.
4. VM/frame đã đúng: frame có `globals` riêng + `env_for_params(globals)`; VM đọc/ghi mảng
   global live. Vì vậy chỉ sửa interp non-frame.

## 3. Thay đổi mã (commit `327247ea`)

| Tệp | Nội dung |
|---|---|
| `crates/vis-interp/src/interp.rs` | thêm `env_module: Env` (handle env gốc) và `closure_dong: u32` (độ sâu closure non-frame); `Interp::new` khởi tạo `env`/`env_module` cùng một `Rc`. |
| `crates/vis-interp/src/interp/goi_ham.rs` | `call_closure_trong`: closure **ngoài cùng** (`closure_dong == 0`) đồng bộ `env_module = saved_env` — non-frame là env cấp tệp, frame là `globals` do `frame/params.rs` gán vào `it.env`; closure lồng **không** ghi đè. `call_hfn_trong` tách wrapper/thân (`call_hfn_than`): `closure_dong > 0` ⇒ `std::mem::replace(&mut self.env, self.env_module.clone())`, chạy thân + default param, rồi **khôi phục trên mọi nhánh thoát** (`Ok`, `Abort::Ret`, trap/abort); `closure_dong == 0` chỉ đồng bộ handle (không đổi hành vi frame). |
| `crates/vis-interp/src/interp/eval.rs` | `HExpr::Var` trỏ hàm tên: `env: self.env_module.clone()` thay vì `self.env.clone()` ⇒ `let h = tang` trong closure gọi `h()` vẫn dùng env module. `HExpr::Lambda` **giữ nguyên** snapshot. |
| `crates/vis-conformance/tests/misc_api_it/st319a_closure_cap_tep.rs` | thêm 5 ca red-first A/B/C2/E/P5 (helper `bon_duong` có sẵn: interp/VM mặc định + W=1 cả hai engine). 5 ca cũ vẫn xanh ⇒ 10/10. |
| `crates/vis-vm/tests/diff_100k.rs` | generator `ClosureCapTep`: `duoi(4)` → `duoi(8)`, thêm 4 mẫu A/B/C2/E (mẫu cũ giữ nguyên vị trí 0..3). |

Không chạm: `crates/vis-interp/src/frame/**`, `crates/vis-vm/src/**`, `crates/vis-bytecode` (chỉ
generator diff test), `vis-core/value`, `vis-hir`, `vis-diag`, `vis-types`; 0 `unsafe`; không
sửa SPEC/QUEUE/DECISIONS/ACTIVE.

## 4. Test red-first (5 ca)

Mỗi ca chạy **4 đường** (`bon_duong`: `vis_interp::run`, `vis_vm::run`,
`vis_interp::run_workers_local`, `vis_vm::run_workers_local_vm`), assert `stdout`/`exit`/`codes`/
giá trị cuối khớp nhau; **không ignore**.

| Ca | Nguồn rút gọn | Mong đợi |
|---|---|---|
| A | `var g=0; fn tang(): g=1; fn main(): let f=fn(): tang(); f(); print(g)` | `1\n` |
| B | `var g=7; fn doc()->int: g; fn main(): let f=fn()->int: doc(); g=9; print(f())` | `9\n` |
| C2 | A nhưng trong closure `let h = tang; h()` | `1\n` |
| E | closure `tang(); g`; main `print(f()); print(g)` | `0\n1\n` |
| P5 | closure `fn(): tang()` tạo trước; main `g=5`; gọi closure; `print(g)` | `1\n` |

- **Trước vá** (`/home/ps/vislogs/st319e-red.log`): `5 passed; 5 failed` — đúng 5 ca mới đỏ,
  failure là `I↔V: stdout` (interp lệch VM); 5 ca ST-319a cũ vẫn xanh.
- **Sau vá** (`st319e-green.log`): `10 passed; 0 failed`; (`st319e-green4.log`): in thêm stdout
  cả 4 đường cho 5 ca mới — tất cả khớp bảng dưới.

## 5. Output trước / sau (interp | VM | interp W1 | VM W1)

| Ca | Trước — interp | Trước — VM | Sau I | Sau V | Sau I-W1 | Sau V-W1 |
|---|---|---|---|---|---|---|
| A | `0` | `1` | `1` | `1` | `1` | `1` |
| B | `7` | `9` | `9` | `9` | `9` | `9` |
| C2 | `0` | `1` | `1` | `1` | `1` | `1` |
| E | `1\n0` | `0\n1` | `0\n1` | `0\n1` | `0\n1` | `0\n1` |
| P5 | `5` | `1` | `1` | `1` | `1` | `1` |

Nguồn số:
- **Trước — interp**: vế `left` của assertion `I↔V` trong `st319e-red.log` — interp tại đúng
  nền `07a5e2b3` (đã có ST-319a), cùng nguồn với 5 ca red-first.
- **Trước — VM**: vế `right` của cùng assertion (VM đã đúng ở nền này).
- **Sau**: `VIS_319E_DEBUG=1` trong `st319e-green4.log` (cùng nguồn 4 đường; log xen dòng do
  test chạy song song, từng dòng `319e …` đủ `I/V/IW1/VW1`).

## 6. Cổng

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` / `--check` | sạch (exit 0) | — |
| `cargo test -p vis-interp -p vis-conformance` | **PASS**: lib_api_it **416** (3 ignored) + interp lib **55** + các binary khác; tổng 0 fail | `st319e-test-final2.log` |
| `./vis-dev gate --fast` | **PASS** — nextest **3922/3922** (4 skip) + doc-test; `vis-dev check: PASS` | `st319e-gate-fast.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`**: `tong=100000 chay=94425 giay=108.8` | `st319e-diff100k.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0) | `st319e-clippy.log` |

Ghi chú flake (có sẵn từ nền, không do vá): lần chạy `cargo test -p vis-interp -p
vis-conformance` đầu tiên có `vs3_worker_eval_vm::vm_khong_tao_them_thread_sau_khi_join` đỏ
(`rò thread: 25->31`) do đếm thread trong process test chạy song song; chạy riêng
**PASS** (`st319e-flake-thread.log`), chạy lại toàn bộ **PASS** (`st319e-test-final2.log`).
Gate `--fast` (nextest, process riêng) cũng PASS.

## 7. Log + SHA-256 (ngoài repo, luật R7)

| Log | Bytes | SHA-256 |
|---|---|---|
| `/home/ps/vislogs/st319e-red.log` | 4374 | `41e67e43bb96bbdb0d69d5f58af8c85fb3d3d6b2bbc402d105e8b33ad2687415` |
| `/home/ps/vislogs/st319e-green.log` | 1220 | `d51ad9bb8da7d9d3b7700fa8d5de2753bac26363247c3466249786fb302f36f7` |
| `/home/ps/vislogs/st319e-green4.log` | 1808 | `baf9953725d5cecb5579a27ecf457f55828785066eef7951a181b08ea718f6cf` |
| `/home/ps/vislogs/st319e-gate-fast.log` | 417164 | `c9868a714bbc8035763786f4ec96b76cc0b046d780e20acb575f5fdb7949b90d` |
| `/home/ps/vislogs/st319e-diff100k.log` | 266 | `100d438e4128f77f4a3df40245a0e765be581aa2078d254efa49043555a66f18` |
| `/home/ps/vislogs/st319e-clippy.log` | 3619 | `843ea3291c14bce468eba7222b8b9aa19c7d91c1db0ce26b68822fc1dc81e589` |
| `/home/ps/vislogs/st319e-test-final2.log` | 32317 | `a815a8d3aad64607a3500581ee600d936f339d7673278ed41a6d65b40bbd1576` |
| `/home/ps/vislogs/st319e-flake-thread.log` | 376 | `dc1874909bcdd6a11f5deb946aba8770a63234f92eefbc8d464fd25ebe1180e6` |

Định dạng log: lệnh chạy với `TMPDIR=/home/ps/vislogs/tmp-st319e`, `CARGO_BUILD_JOBS=2`,
`PATH=$HOME/.cargo/bin:$PATH` (cargo 1.100.0 theo `rust-toolchain.toml`); target riêng
`/home/ps/viswt/st319e/target`; jobs=2; không `cargo clean`.

## 8. Phạm vi / rủi ro đã kiểm

- **Frame không bị đổi mã**: `env_module` được đồng bộ ở closure ngoài cùng nên khi callback
  non-frame (vd `xs.map(closure)` trong chương trình async) gọi hàm tên, env module là
  `globals` frame — không dùng env rỗng của `Interp::new`.
- **`HExpr::Lambda` không đổi**: snapshot by-value ST-319a (đọc/ghi global trong thân closure
  vẫn là bản riêng; ca E chứng minh env closure được khôi phục sau hàm tên).
- **Giới hạn có sẵn, ngoài phạm vi #210**: gọi hàm tên **qua giá trị** (`let h = tang; h()`)
  đi đường `call_closure` chung nên không áp dụng default param/variadic của `HFn` như
  `call_hfn`; hành vi này có trước, không đổi trong ST-319e.
- `#210` chỉ chốt cho **interp non-frame**; frame/async dùng env `globals` riêng, không nằm
  trong lane này.

## 9. Việc còn lại

1. Gatekeeper audit commit `327247ea` + journal này; chạy lại cổng theo quy trình.
2. A0 cập nhật `agent/SO-NO.md` `#210: mở → đóng` và ghi nhật ký `[ST-00]` docs-only (ngoài
   lane Builder); nếu cần, đóng nợ `#206`/`#210` cùng đợt.
3. Chưa push nhánh `plan/st319e` (Builder chỉ commit trong worktree).
