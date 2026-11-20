<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Checkpoint sau Lát 1 — chờ A0 duyệt Lát 2 (VD-251, 2026-11-20)

> **Trạng thái**: Lát 1 đạt tiêu chí, **DỪNG TẠI ĐÂY** — theo VD-251 §7, không tự mở reactor Lát 2.
> Tag/release **PAUSED**; baseline lịch sử `dbae70a`; chưa mở VS3/worker/GC/TLS/API mới.
> **Runtime mặc định, bytecode trên đĩa, `VERSION`, cache key, API ngôn ngữ và cờ CLI công khai
> không đổi.** Chỉ có đường test/debug nội bộ.

## 1. Kết quả Lát 1 (ST-267)

**Sản phẩm**: `crates/vis-interp/src/frame.rs` (mới), `crates/vis-vm/src/vm_frame.rs` (mới),
một nhánh `Abort::Pending` trong `rt.rs`/`interp.rs` và hook 1 dòng ở `vm.rs:860` (CallBuiltin),
test `crates/vis-cli/tests/lat1_frame.rs` + `crates/vis-interp/tests/lat1_frame.rs`.
Báo cáo chi tiết: `agent/JOURNAL/vd251-lat1-report.md`.

**Không phải Lát 0 lặp lại**: test đi qua **source → lower → check → HIR/bytecode thật →
`Interp`/`Vm` thật → `builtin::call`/`Value`/`Trap`/`Scheduler` thật** (`tien_trinh`,
`lat1_frame.rs:15`). Lát 0 vẫn 10/10, không sửa.

| # | Tiêu chí A0 | Bằng chứng (cả 2 engine) |
|---|---|---|
| 1 | A Pending → B chạy → A resume | log `wait:1:Event{key:1,gen:0}` → `wake:1` → `wake:0`; stdout `A:pre/B:run/A:post/48/5` |
| 2 | Local/capture đúng; không lặp side-effect | `48 = add1(7+40)`; `A:pre`/`B:run`/`A:post` mỗi cái đúng 1 lần |
| 3 | Gọi hàm lồng + await biểu thức lồng | `print(add1(await a))`: frame con thật (interp) / `Instr::Call` thật (VM) |
| 4 | Lỗi thật `Trap`/`VS0506`, await lần 2 trả cache | `1 % 0` ⇒ `VS0502` gốc, bọc `VS0506`; `bad:run` in **1 lần** |
| 5 | Task không await vẫn final-join, không auto-cancel | `main\nlonely\n` — driver xếp task `Moi` vào cuối `frame.ready` |
| 6 | Wake trùng/wake cũ không chạy sai | gen + seq trace; log `duplicate:1`, `stale:1`, `wake:1` đúng 1 lần |
| 7 | Thu hồi theo ownership | `DiagSnapshot{live_tasks:0,frames:0,waiters:0,ready:0}` + 3 dòng `reclaim:` |
| 8 | Fixture cũ phân loại, không đổi golden | 3 fixture chạy qua đường mặc định, stdout khớp; `git status` không đổi `test/**` |

**Tất định**: FIFO; task chờ không nằm trong `ready`; wake nối đuôi; thứ tự theo **khóa ghi trong
trace** (seq/key/gen), không hash map/địa chỉ; mỗi task tối đa 1 mục ready; event đăng ký cũ bị bỏ
qua gen. Hai engine cùng trace ⇒ **byte-identical** stdout, chẩn đoán, log và snapshot.

**Validation**: `./vis-dev gate --fast` **PASS — 1450/1450** sau `cargo clean` (xem §3);
`cargo test -p vis-interp -p vis-cli` **1007/0**; test Lát 1 **8/8** (interp smoke 1/1); Lát 0 **10/10**.

**B1/B2 baseline (không yêu cầu cải thiện)**: B1 ~**4,04 s** cả 2 engine (baseline 4,11 s);
B2 `CPU_DONE` = `IO_START` cùng ms, `IO_DONE−IO_START = 201 ms`. I/O vẫn chặn, fairness CPU chưa giải.

## 2. Giới hạn trung thực

- Interp frame evaluator **(đường test)** chỉ phủ `Lit`/`Var`/`Bin`/`Call`/`Await`/`Spawn` với callee
  là tên hàm cấp tệp hoặc builtin, `Let`/`Var`/`Expr`/`Return`, pattern `Bind`/`Wild`. **Chưa phủ** `&&`/`||`,
  vòng lặp, `if`/`match`, field/index, record/tuple, closure callee, hằng cấp tệp, đối số tên,
  multi-frame qua closure capture. **Checker vẫn chấp nhận mọi vị trí đó như cũ** — đây là gap của đường
  test, không phải regression hay thu hẹp ngôn ngữ.
- VM dùng bytecode thật nhưng task frame trong test mode khởi tạo `captured`/`writeback` rỗng — closure
  của async task chưa được kiểm chứng ở frame mode.
- **Step-cap parity vẫn MỞ** (VD-249/ST-260): interp đếm vòng/đếm sự kiện, VM đếm lệnh, cùng 10⁷;
  2M vòng ⇒ interp exit 0 / VM `VS0506→VS0501`. Lát 1 **không sửa, không cam kết parity** cho workload
  chạm giới hạn. Hàng rào hiện hành giữ nguyên.
- **Không có** budget mới/hạn ngạch CPU/reactor/dependency/unsafe mới.
- `diag_snapshot()` **test/debug-only**; không cờ CLI/API công khai.

## 3. Drift toolchain (cần A0 biết)

Hồ sơ nghiệm thu trước ghi `rustc/cargo 1.99.1`. Hiện `rustc/cargo` đang là **1.100.0**: repo có
`rust-toolchain.toml` ghim `channel = "stable"` (commit cũ `5b5937c`, ST-171i) và **stable channel vừa
tự cập nhật hôm nay 14:57**. Default rustup vẫn là `1.99.1`. **Tôi không chạy lệnh nâng cấp nào.**
Hệ quả: artifact cũ trong `target/` bị stale ⇒ các test đọc `test/sem` báo `NotFound` giả. Đã
`cargo clean` (xóa ~10 GiB, 33 662 tệp) và build lại ⇒ gate **1450/1450 PASS**. Không đổi mã vì
lỗi này; đây là drift môi trường, không phải regression Lát 1.

## 4. Đề xuất Lát 2 để A0 duyệt (CHƯA triển khai)

Lát 2 = **I/O mạng loopback không chặn** (`read`/`accept`) qua adapter/host registry, bỏ
`thread::sleep(5 ms)` busy-poll, kèm timer tối thiểu. Tiêu chí đề xuất: B1 `4,11 s → ~0,3 s` trên
cả 2 engine, 20/20 đọc ok, thứ tự theo trace; `connect`/`write` **giữ đồng bộ tạm và phải công bố rõ
có thể chặn vượt deadline** (A0 VD-251 §1); không công bố "async mạng thực dụng" khi còn đường chặn.
**Trước Lát 2 phải chốt**: 5 câu tính phí + bảng phụ/opcode + chính sách dependency/unsafe
(đã gửi bảng ngắn `agent/JOURNAL/vd251-charging-short.md`, 5 draft VD-250). Không ấn `VERSION`.

## 5. Quyết định A0 cần

1. Chấp nhận checkpoint Lát 1 (kể cả giới hạn phủ của đường test và drift toolchain §3).
2. Duyệt hay không **Lát 2** theo §4, sau khi chốt các hợp đồng còn mở.
3. Nhắc lại: tag/release PAUSED; VS3/worker/GC/TLS/API chưa mở; step-cap parity vẫn mở.

**Kết luận**: Lát 1 đã chứng minh **chương trình ViScript thật suspend/resume đúng** trên cả hai
engine, không phải hai mô hình. Đó là nền móng phải khoá trước khi đưa mạng thật vào scheduler
(Lát 2) — và Lát 2 chỉ bắt đầu khi A0 duyệt.
