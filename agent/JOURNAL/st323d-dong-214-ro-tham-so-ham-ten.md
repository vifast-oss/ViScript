<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-323d — đóng nợ `#214` (P2 latent): rò tham số hàm tên trong closure

- **Nhánh**: `plan/st323d`, nền `main` = `eb41d61c`; worktree `/home/ps/viswt/st323d`.
  **Không** đụng repo chính, **không** merge/push (VD-279).
- **Lane**: `crates/vis-interp/src/interp/**` + `crates/vis-cli/tests/lang_it/main_sig.rs`.
  **Không** sửa `interp.rs` (điểm chạm chung với ST-323c — đã kiểm `git -C /home/ps/viswt/st323c
  status --short | grep interp.rs` ⇒ sạch), **không** đụng `frame/**`, bytecode/VM.
  `env_module` (#210) và closure by-value (#206/#209) giữ nguyên ngữ nghĩa: hàm tên vẫn chạy
  trên env module do `call_hfn_trong` chọn; chỉ đổi chỗ/điểm khôi phục ô tham số.
- **Môi trường**: cargo 1.100.0 (`rust-toolchain.toml`), `CARGO_BUILD_JOBS=2`,
  `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/vislogs/tmp/st323d-tmp` (đĩa, không tmpfs).
  Đĩa đầu **19 G**, cuối **16 G** (> ngưỡng dừng 5 G).

## 1. Xác lập minh bạch: có rò thật, nhưng chỉ quan sát được ở API Rust

`call_hfn_than` ghi từng tham số vào `self.env` (env module) rồi mới `self.eval(d)?` cho
giá trị mặc định / `return Err` khi thiếu đối số. Hai nhánh thoát sớm đó nhảy thẳng qua vòng
restore cũ (đặt **sau** vòng lặp gán + sau `exec_body`) ⇒ ô tham số đã ghi lọt lại env module.

- **Bằng chứng đỏ-trước (Rust API)**: unit test mới trong `goi_ham.rs` dựng HIR từ nguồn,
  chạy `Interp::run_program`, rồi tra `sym` của tham số `a` trong `it.env`. Trước bản vá,
  **2/2 test FAILED** với `#214: ô a của h vẫn còn trong env module sau trap sớm` và
  `... sau Ret sớm` (`st323d-red.log`). Sau bản vá **2/2 ok** (`st323d-green-unit.log`).
- **Không quan sát được qua stdout/CLI** (`st323d-repro.log`, binary debug nền `eb41d61c`):
  ca default `1 / 0` trap (`VS0502`, exit 101, stdout rỗng) và ca closure gọi hàm tên với
  default `3` (stdout `3`, exit 0) cho **kết quả byte-bằng nhau trước và sau**. Lý do: trap
  làm chương trình dừng ngay (`Abort::Trap` fatal); `None?` trong default là `Abort::Ret`,
  bị `call_closure_trong` bắt thành giá trị `None` như thiết kế. Vì vậy đây là nợ **latent ở
  mức ngôn ngữ**, nhưng **rò có thật ở mức interpreter state** — đủ căn cứ để sửa có kiểm
  chứng, không sửa ẩu/journal "không tái lập".

## 2. Patch

| Tệp | Thay đổi |
|---|---|
| `crates/vis-interp/src/interp/goi_ham.rs` | Tách vòng gán tham số thành `gan_tham_so(...) -> R<()>` (ghi giá trị cũ vào `old`) + helper `khoi_phuc_tham_so(&old)`. `call_hfn_than`: nếu setup trả `Err` (trap/Ret/arity/thiếu đối số) ⇒ **restore rồi trả lỗi**; nếu setup Ok ⇒ chạy thân, rồi restore như cũ (Ok/Ret/abort/`Pending` đều đi qua đây). |
| `crates/vis-cli/tests/lang_it/main_sig.rs` | 2 ca tích hợp mới (xem §3), nằm trong aggregator `lang_it.rs` có sẵn — **R8 giữ 39/40**, không tệp `.rs` cấp 1 mới. |

Lưu ý đã tự sửa sai trong lúc làm và bị `async_it/vd270_recv_opt_infer` (10 ca) bắt: restore
**vô điều kiện trước `exec_body`** xoá mất `self` mà thân impl method cần. Bản chốt chỉ restore
trước thân **khi setup lỗi**; full `cargo test` sau đó xanh 0 fail. `mutable` (cờ `var self`) cố ý
không đưa vào restore: cờ này là thuộc tính tĩnh đơn điệu của sym, thay đổi có thể làm closure
bắt `self` mất quyền ghi sau lời gọi (nằm ngoài phạm vi `#214`).

## 3. Test

- Unit `vis-interp`: `st323d_khong_ro_tham_so_khi_trap_default` (default `1 / 0` ⇒ Trap) và
  `st323d_khong_ro_tham_so_khi_ret_default` (default `boom()?` ⇒ `Ret(None)`, stdout `None\n`)
  — cả hai tra env module sau khi chạy.
- CLI `main_sig`: `st323d_closure_goi_ham_ten_default` — closure `let f = fn(): h()` gọi hàm
  tên có `--n: int = 3`; interp = VM, exit 0, stdout `3\n`, stderr rỗng byte-bằng nhau.
  `st323d_closure_goi_ham_ten_trap_som` — default `1 / 0` trap trước vòng restore; interp = VM,
  exit 101, stdout rỗng, stderr cùng `VS0502`.

## 4. Cổng (log `/home/ps/vislogs/st323d-*.log`)

| Cổng | Kết quả | sha256 log |
|---|---|---|
| `cargo test -p vis-interp -p vis-cli` | **0 fail**; `lang_it` 500/500 (gồm 2 ca mới), `cli_it` 125, `interp` unit 7/7 (gồm 2 ca mới) | `417f684a…0978` |
| `./vis-dev gate --fast` | **PASS**, nextest **3946 passed / 4 skipped** (nền 3942 + 4 ca mới); fmt/clippy/R1..R9/self-test/build/doc-examples OK, L0 OK | `57dab2fa…f781cf0` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (`tong=100000`, `chay=94425`, `loi_tinh=5575`, 61,8 s, seed `3523158054`) | `85e02c53…113deefd` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **OK** | `1094a368…88698c` |
| Repro trước/sau (CLI + unit red/green) | xác nhận latent ở stdout, rò thật ở env | `4e0f4b00…e52d3` |
| Unit đỏ trước bản vá | 2 FAILED (bằng chứng rò) | `c961c906…d2edf0` |

Sha256 đầy đủ: `st323d-repro.log` `4e0f4b005c3a9de23091c1dd6ee3e69831d40f2cd5b8910cd2f290549a9e52d3`;
`st323d-red.log` `c961c90636764a5bc0601799296106eabd80468cb582879d23eb2dfe5ad2edf0`;
`st323d-green-unit.log` `e7d31ea9baa6da09e6fc3c9a053954e9554744dc24f104ee2f9d9792f9d61e48`;
`st323d-test.log` `417f684acb3a7777c3a4c7ba6137c43889ebb88d6be78e16b3644243056a0978`;
`st323d-gate-fast.log` `57dab2fad65652a200dc33eea327713bf36ca74b67f6251a2e0312bc9f781cf0`;
`st323d-vis-diff-100k.log` `85e02c537f507c406285600b34d15a2dfc4c534b4ad69e15f6f3b470113deefd`;
`st323d-clippy.log` `1094a36867300c1838cdc4ee40efe961d83bac574665f8ee57a40d5b1e88698c`.

## 5. Bàn giao Agent 0 / Agent 1

- Đề xuất A0 chuyển `#214` `mở → đóng ở ST-323d` trong `agent/SO-NO.md` (Builder **không** sửa
  sổ theo luật). Căn cứ: unit red→green + cổng xanh ở trên.
- Không merge/push; chờ Agent 1 nghiệm thu rồi A0 merge theo VD-279. Không tag phát hành,
  VD-289 tiếp diễn. Không nợ mới.
