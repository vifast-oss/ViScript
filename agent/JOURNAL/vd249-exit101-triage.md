<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-249 / ST-261 · Phân lập **exit 101** của `vis run --vm`
**Trạng thái**: research + draft (Agent 2) · **không commit, không đổi mã sản phẩm**
**Đề bài**: `agent/DECISIONS.md:2384-2385` (VD-249 §3) — "báo cáo ghi VM exit 101 dù đã in `VS0501`/`VS0506` … phải xác định đây là mã thoát CLI chủ ý hay panic/đường lỗi khác; **không mặc định đúng** chỉ vì có chẩn đoán."

## Verdict

**Exit 101 là mã thoát CLI chủ ý (`ExitCode::from(101)`), KHÔNG phải Rust panic, cũng không phải đường lỗi khác.**

Đường dẫn đầy đủ, tất định:

1. `crates/vis-vm/src/vm.rs:97` — `MAX_STEPS = 10_000_000`; `:736` — `vm.steps += 1` **cho mọi lệnh**; `:737-743` — vượt ngưỡng ⇒ `Abort::Trap(Trap::new(RT, "chạy quá lâu (>10^7 lệnh)", span))`. `RT` = `&RUNTIME_CODES[0]` = `VS0501` (`crates/vis-interp/src/interp.rs:17`; `crates/vis-diag/src/lib.rs:444-452`).
2. VM dùng chung máy tác vụ của interp: `crates/vis-vm/src/vm.rs:228-234` gọi `vis_interp::rt::join_all(self)`; lỗi thân tác vụ bị bọc ở `crates/vis-interp/src/rt.rs:225` → `wrap_task_error` (`:232-239`) ⇒ `TASK_ERR` = `&RUNTIME_CODES[5]` = `VS0506` (`interp.rs:23`; `vis-diag/src/lib.rs:489-498`), thông điệp `tác vụ \`busy\` lỗi: VS0501: chạy quá lâu (>10^7 lệnh)`.
3. `crates/vis-vm/src/vm.rs:246-252` — `Err(Abort::Trap(t))` ⇒ `RunOutput { diagnostics: vec![t.to_diag(self.text)], exit: 1, … }`. **Trường `exit: 1` này bị CLI bỏ qua** trên đường có chẩn đoán (xem bước 5) — đây là điểm dễ hiểu nhầm.
4. `crates/vis-cli/src/main.rs:610-625` — ánh xạ chẩn đoán thành tuple `(mã, dòng, cột, thông điệp, tệp)` và trả ra khỏi `thread::scope`; `:747` — `Some("run") => run_run(&args[1..])`.
5. `crates/vis-cli/src/main.rs:640-658` — `if !diags.is_empty()`: in từng dòng ở `:647`, rồi `:657` `ExitCode::from(ma_thoat_khi_loi(&ma_loi))`.
6. **`crates/vis-cli/src/cli/run.rs:8-21`** — `ma_thoat_khi_loi`: `[quyền]…` ⇒ 2; **`d.0.starts_with("VS05")` ⇒ 101**; còn lại ⇒ 1. Cả `VS0501` lẫn `VS0506` đều khớp `VS05`, nên việc bọc VS0506 **không đổi mã thoát**.

Mã này có gốc từ bảng mã thoát VD-20 (`agent/DECISIONS.md:47`: "101 panic") và được R9/VD-178 chốt lại: `agent/DECISIONS.md:1087` — "bảng mã thoát (0/1/2/**101** panic) — test cũ mong exit 1 cho VS0502/0503 đổi sang 101". Chữ "panic" ở `cli/run.rs:11` và trong VD là **nhãn phân loại** (lỗi runtime kiểu panic), không phải lời gọi `panic!`.

### Vì sao KHÔNG phải panic
- Không có `panic!` / `unwrap()` trên đường này; kết quả là `Result<_, Abort>` được xử lý thành `RunOutput` rồi thành `ExitCode` (`vm.rs:246`, `main.rs:625`, `main.rs:657`).
- `RUST_BACKTRACE=1` vẫn không có dòng `thread '…' panicked`, không backtrace (xem bảng reproduce).
- Ở `crates/vis-cli/src/main.rs:627-629` có `.expect("không tạo được luồng chạy")` / `.expect("luồng chạy lỗi")` — đây là **đường panic thật duy nhất** trong đoạn, nhưng chỉ kích hoạt nếu luồng bị panic; trong ca này luồng trả về bình thường nên không đi qua.
- `usage()` (`main.rs:31-36`) trả 2, không liên quan.

## Reproduce (Program tinh smallest, đặt trong /tmp, không vào repo)

Lệnh: `PATH="$HOME/.cargo/bin:$PATH" /home/ps/ViScript/vis run [--vm] --no-cache <tệp>`; stderr/free chạy trực tiếp `target/debug/vis` (cùng artifact mà shim `bin/vis` gọi qua `cargo run -q`, `bin/vis:11`) cho kết quả y hệt.

| Ca | Chương trình | interp exit | VM exit | stderr (trừ dòng toolchain của shim) |
|---|---|---|---|---|
| (a) 2M vòng **trong async task** | `async fn busy() -> int: var i=0; while i < 2000000: i = i + 1; return i` rồi `await busy()` | **0**, stdout `2000000` (~3,47 s) | **101** (~1,32 s) | `VS0506: tác vụ \`busy\` lỗi: VS0501: chạy quá lâu (>10^7 lệnh)` |
| (a2) 2M vòng **trong main** (không task) | `fn main(): var i=0; while i < 2000000: i=i+1; print(i)` | **0**, stdout `2000000` (~2,98 s) | **101** (~1,40 s) | `VS0501: chạy quá lâu (>10^7 lệnh)` |
| (b) trap runtime thường | `print(1 / 0)` (từ `test/sem/loi/chia-0.vis`; kỳ vọng `test/sem/loi/chia-0.err` = `VS0502`) | **101** | **101** | `VS0502: chia cho 0 (\`/0\`)` |

Ghi chép:
- Ca (b) chứng minh **101 là mã thoát bình thường của mọi lỗi runtime `VS05xx`**, không phải dấu hiệu riêng của lỗi ngân sách.
- `RUST_BACKTRACE=1`: cả (a), (a2), (b) đều **không** có thông điệp panic/backtrace; grep `panicked|backtrace|thread '` trên stderr ⇒ rỗng.
- Dòng `vis: chọn cargo …` ở stderr khi dùng shim do `bin/vis:8-11` in, không phải chương trình ViScript.
- Tệp repro: `/tmp/vd249-loop2m.vis`, `/tmp/vd249-loop2m-plain.vis`, `/tmp/vd249-chia0.vis` (ngoài repo).

## Hệ luận cho A0

- **Cái exit 101 là đúng theo hợp đồng hiện hành** (R9/VD-178); không sửa `ma_thoat_khi_loi`.
- **Cái sai là chỗ đặt hàng rào**: cùng một chương trình 2M vòng, interp đếm số **lần lặp while** (`crates/vis-interp/src/interp.rs:535-547`, `guard > 10_000_000`) nên vượt qua; VM đếm **mọi lệnh** (`vm.rs:736-743`) nên trap sớm. Đây chính là divergence cần thay bằng điểm tính phí ngữ nghĩa chung (`vd249-budget-draft.md`).
- **Không được ngụ ý sửa bằng cách nâng `MAX_STEPS`** — VD-249 §3 cấm rõ (`agent/DECISIONS.md:2370-2371`).
- **Không được coi việc in chẩn đoán là bằng chứng exit đúng**: ở đây chẩn đoán đúng nhưng nguyên nhân trap sai engine.
- Câu hỏi mở cho A0: có phát hành **mã chẩn đoán riêng** cho "vượt ngân sách" thay vì tái dùng `VS0501`/`VS0506` hay không. Dù chọn gì, `ma_thoat_khi_loi` (`cli/run.rs:13-21`) vẫn cho **101** nếu mã mới nằm trong họ `VS05xx`; nếu mã mới nằm ngoài `VS05xx`, phải sửa cả hàm này — cần A0 chốt trước khi triển khai.
