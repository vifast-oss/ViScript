<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-320a — `var self` + `push`: chốt ngữ nghĩa + khoá parity (`#211`, đề xuất Sóng 9)

- **Nhánh**: `plan/st320a`, worktree `/home/ps/viswt/st320a`, nền `main` = **`d1254b05`**
  (mã sản phẩm cuối `ee8f252d`). **Không** merge/push (main frozen cho chain).
- **Lane thực tế**: **chỉ test** — `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs`
  (aggregator `lib_api_it.rs` đã có `#[path]`; **không** tạo tệp `.rs` cấp 1 mới ⇒ R8 giữ 39/40).
  **0 dòng sản phẩm** bị sửa: không chạm `vis-bytecode`/`vis-interp`/`vis-vm`, không đổi VBC1/cache.
- **Nguồn**: gap #1 `/home/ps/vislogs/song9-de-xuat-20261126T2132Z.md` §1.1; nợ P3-1
  `agent/JOURNAL/review-st319d.md:104-106`; SPEC §4.7/§4.8; VD-51.

## 1. Ngữ nghĩa đã chốt (theo interp = oracle + SPEC)

1. **`var self` chỉ là tham số được gán** (SPEC §4.7); gán lồng `a.b = v` chỉ trên gốc `var`
   (§4.8). Method nhận `self` **theo giá trị**: thay đổi trong thân **không** xuyên ra người gọi
   — đã khoá sẵn bởi `test/sem/ban-ghi/var-self-gan.out` (`42\n41`), `vd270_frame_nested_lib.rs:150`
   (`9\n1`), `vd284_spawn_method.rs` (`2\n1`).
2. **`push`** (VD-51): ghi lại **ô biến** khi receiver là `HExpr::Var` trỏ biến `var`/`var self`;
   receiver `let`/biểu thức tạm trả list mới, **không** ghi lại.
3. **`self.xs.push(2)`**: receiver là **`Field`** (`self.xs`), không phải `Var` trực tiếp ⇒ cả hai
   engine coi là biểu thức tạm: trả `[1, 2]`, **không** rebind `self.xs` (đọc lại trong hàm vẫn
   `[1]`). Codegen `expr.rs` rơi nhánh `_ => None` (không `Assign`); interp `goi_ham.rs` cũng chỉ
   ghi khi `base` là `Var` + `mutable`. **Hai engine khớp** ⇒ chọn phương án **(a)** của đề xuất
   Sóng 9 §5.1: giữ hành vi, chỉ thêm test + ghi chú; **không sửa codegen**.
4. Đường `mutables` + `is_var_self` **có** chạy và đúng: `self.push(2)` với `var self` kiểu list
   (dạng `fn keo(var self)` — dạng duy nhất có receiver `Var` trỏ `var self`; method `impl` có
   `self` kiểu record nên `self.push` không hợp lệ, còn tham số tự do `var self` không nhận chú
   thích kiểu — VS1b `{error}`) ⇒ trong thân `self` = `[1, 2]`, `len = 2`; người gọi vẫn `[1]`.

## 2. Test mới — 3 ca cuối `st317b_worker_orchestrate.rs` (35 ca cả module)

Helper `sau_duong_st320a` chạy **6 đường** cho mỗi ca: `vis_interp::run`, `vis_vm::run`
(mặc định), `run_workers_local`/`run_workers_local_vm` (W=1 non-frame), `frame_1`/`frame_vm_1`
(frame W=1 — đường production `vis run` cho chương trình async); assert `stdout` literal, `exit 0`,
`diagnostics.is_empty()` **từng đường** + `so_khop` từng cặp với interp mặc định.

| Ca | Chương trình | Kỳ vọng (mọi đường) |
|---|---|---|
| `st320a_var_self_push_truong_parity` | `self.xs.push(2)` ở biểu thức cuối **và** trong `let`, đọc lại `self.xs` trong hàm | `[1, 2]` / `[1]` / `[1]` / `[1, 2]` / `[1]` |
| `st320a_var_self_gan_truong_cuc_bo` | đối chứng `self.xs = self.xs.push(2)` (gán tường minh, §4.8) | `[1, 2]` / `[1, 2]` / `[1]` (chỉ cục bộ) |
| `st320a_var_self_la_list_push_ghi_o` | `fn keo(var self): self.push(2)` — nhánh codegen `(Var, "push")` + `mutables.is_var_self` | `[1, 2]` / `[1, 2]` / `2` / `[1]` |

**Red proof cho nhánh `is_var_self`** (tạm bỏ `st.g.mutables.insert(p.sym)` ở
`crates/vis-bytecode/src/codegen.rs:324`, đã `git checkout` khôi phục):
`st320a_var_self_la_list_push_ghi_o` **ĐỎ** — VM `[1]\n[1, 2]\n1\n[1]\n` ≠ kỳ vọng
`[1, 2]\n[1, 2]\n2\n[1]\n` (interp vẫn đúng vì `Interp.mutable` độc lập). Nhờ vậy ca này thật sự
khoá `is_var_self`, không phải test xanh hình thức. Log `/home/ps/vislogs/st320a-redproof.log`.

## 3. Cổng

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all -- --check` | sạch (exit 0; `git diff --check` sạch) | `/home/ps/vislogs/st320a-fmt.log` |
| `cargo test -p vis-conformance --test lib_api_it st320a` | **3/3 PASS** (sau khôi phục codegen) | `/home/ps/vislogs/st320a-test5.log` |
| `cargo test ... st317b_worker_orchestrate` | **35/35 PASS** | `/home/ps/vislogs/st320a-test3.log` |
| `cargo test -p vis-conformance --test lib_api_it` (toàn bộ) | **431 passed, 0 failed, 3 ignored** (434) | `/home/ps/vislogs/st320a-test4.log` |
| `./vis-dev gate --fast` | **PASS** — **3937/3937** test, 4 skip, clippy/R1..R10/self-test OK, `GATE_EXIT=0` | `/home/ps/vislogs/st320a-gate-fast.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **OK** (exit 0; gate cũng đã chạy) | `/home/ps/vislogs/st320a-clippy.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — tổng 100000, `loi_tinh=5575`, `chay=94425`, seed `3523158054`, 114,6 s (không chạm mã sản phẩm, chạy theo yêu cầu task) | `/home/ps/vislogs/st320a-diff100k.log` |
| R8 | 39/40 tệp `.rs` cấp 1; `vis-cli/tests` 8/15 — **không tệp mới** | — |
| VBC1 / cache | **không đổi** (`crates/vis-bytecode` 0 diff) | — |

## 4. Ghi chú cho A0

- Đề xuất Sóng 9 §5.1 đã được thực thi theo khuyến nghị **(a)**: giữ `self.xs.push` là biểu thức
  tạm, không rebind field. Nếu A0 chọn **(b)** (push trên field của gốc `var` phải writeback như
  ô `var`), hai ca `st320a_var_self_push_truong_parity` + `st320a_var_self_la_list_push_ghi_o` sẽ
  là **test đỏ có chủ đích** để mở task codegen riêng (lane `vis-bytecode/src/codegen/**` +
  `vis-interp/src/interp/goi_ham.rs`; khi đó mới cần bump `PHIEN_BAN` cache, VBC1 giữ 2).
- Không cấp/tham chiếu số nợ mới ngoài `#211` (đề xuất); P3-1 `ST-319d` coi như **đã đóng ở mức
  test**: nhánh `is_var_self` có ca + red proof.

## 5. sha256 log (`/home/ps/vislogs/`)

```text
st320a-test3.log        22b2a759d4d9771afad6f43e4ffe9d56259d523e3dae996a416d5df95dbd71e8
st320a-test4.log        5ec64b14cca93c76bcfd1d213aa6c8ecd540e9f611cdcb3f60b1ff5d98894e30
st320a-test5.log        080c1be1a26d66009c51b381361b909a88c8953ab09cfc3f6078c14dc2691422
st320a-redproof.log     b11943a4f50ecbf6a857152522dea20b5fb7fa5adbd7bb7d7b0e91f455bcbf62
st320a-gate-fast.log    3d51f2b17d1b513fe2ff4bd8755592141698cd777f3ac0021e4d2daa7627b2ce
st320a-clippy.log       0eab183f2f19c1610d8f73cce66c1f8a77050378e62aaebf1347ff38329c6d6a
st320a-diff100k.log     678994910e69f350468c180e1d86768b6ed9ee6c857f9a442b9a90ee1bfc0d7c
```

`st320a-fmt.log` rỗng (`cargo fmt --all -- --check` không in gì, exit 0).
