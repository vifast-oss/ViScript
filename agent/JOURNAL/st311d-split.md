# ST-311d — Gom `builtin_*.rs` vào `builtin/`, chia 10 tệp > 1.150 LOC, thử `-zstack-size` WASI (`#193`)

> **Nguồn**: `main` **`0bcb5af`**; nhánh **`plan/st311d-split`** (worktree riêng, không checkout
> `/home/ps/ViScript`). **Local only** — không merge, không push, không tag. Không ssh
> `mps`/`psvn`; không kill fuzz nền `ps`. `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`,
> `CARGO_TARGET_DIR=/home/ps/tmp/st311d-target` (target riêng của worktree).
>
> **Căn cứ**: `VD-286` §3 (`ST-311d` W4), `agent/QUEUE.md` dòng `ST-311d`.

## 1. Kết quả đo

| Hạng mục | Trước | Sau |
|---|---|---|
| Tệp sản phẩm > 1.150 LOC | **10** | **0** (cao nhất `sqlite/mod.rs` 1.133) |
| Tệp phẳng `vis-interp/src/builtin_*.rs` | **6** | **0** (đều ở `builtin/`) |
| `./vis-dev gate --fast` | đỏ (2 ca sai kỳ vọng của `vis-core/tests/core_it.rs`, xem §5) | **PASS** — nextest **3.778 passed / 0 failed / 4 skipped** + `cargo test --doc` OK |
| Luật `bin/vis-dev` | R1..R9 | **R1..R9 OK** (R1 ≤ 1.500 dòng, R2 0 `unsafe`, R7 log ≤ 50 KB) |
| `fmt` / `clippy -D warnings` | — | **OK** |

`R1` (≤ 1.500 dòng/tệp) vốn đã xanh trước ST-311d; mục tiêu 1.150 là **ngưỡng hẹp hơn** do
`VD-286` đặt để chừa chỗ cho tệp phình tiếp. Sau ST-311d không tệp sản phẩm nào > 1.150.

## 2. Việc đã làm

### 2.1. Gom 6 tệp phẳng vào `crates/vis-interp/src/builtin/`

`git mv` (giữ nguyên nội dung, không sửa một dòng logic):

| Cũ | Mới | Khai báo |
|---|---|---|
| `src/builtin_assert.rs` | `src/builtin/assert.rs` | `pub mod assert;` |
| `src/builtin_list.rs` | `src/builtin/list.rs` | `pub mod list;` |
| `src/builtin_ls.rs` | `src/builtin/ls.rs` | `pub(crate) mod ls;` |
| `src/builtin_panic.rs` | `src/builtin/panic.rs` | `pub mod panic;` |
| `src/builtin_sig.rs` | `src/builtin/sig.rs` | `pub(crate) mod sig;` |
| `src/builtin_value.rs` | `src/builtin/value.rs` | `pub(crate) mod value;` |

- **Đăng ký không đổi**: `builtin/mod.rs::call` vẫn trỏ `assert`/`panic`/`$value:` đúng như cũ,
  chỉ đổi đường dẫn `crate::builtin_x` → `self::x`.
- **API công khai không đổi**: `lib.rs` thêm re-export
  `pub use builtin::{assert as builtin_assert, list as builtin_list, panic as builtin_panic};`
  ⇒ `vis_interp::builtin_assert::assert_builtin` (và hai tên kia) vẫn dùng được y như trước.
- 11 chỗ `crate::builtin_ls::…` trong `stdlib/path_{a,b,c,d}.rs` → `crate::builtin::ls::…`.

### 2.2. Chia 10 tệp sản phẩm > 1.150 LOC

Nguyên tắc: **chuyển nguyên văn**, module con **cùng thư mục**, không đổi ngữ nghĩa/API; chỉ nới
`pub(super)`/`pub(crate)` cho item mà module cha còn gọi (bắt buộc khi hàm rời khỏi module cha).

| Tệp | Trước | Sau | Tệp mới (dòng) | Nội dung tách |
|---|---|---|---|---|
| `vis-types/src/tyck/mod.rs` | 1.362 | **773** | `tyck/hop.rs` (599) | impl `Checker`: `hop_nhat`, `same*`, `chuan`, `coerce*`, `resolve_ann/named/cap`, `dang_ky_sig_phuong_thuc` |
| `vis-bytecode/src/codegen/than.rs` | 1.336 | **952** | `than/infer.rs` (310), `than/tests.rs` (90) | suy kiểu `tuple_ty_suy`/`infer_ty`/`bind_pat_ty`/`ctor_arg_ty`; `mod tests` |
| `vis-syntax/src/lex/mod.rs` | 1.259 | **1.033** | `lex/tests.rs` (228) | `mod tests` |
| `vis-cli/src/vistest.rs` | 1.249 | **1.061** | `vistest/ten.rs` (87), `vistest/tests.rs` (114) | SplitMix64/Fisher–Yates/`--shard`/tên đơn vị/`--slowest`; `mod tests` |
| `vis-host-native/src/db/sqlite/mod.rs` | 1.245 | **1.133** | `sqlite/kiem.rs` (125) | `kiem_sql`, `thong_tin_sqlite`, `mo_ta` |
| `vis-types/src/hir.rs` | 1.213 | **1.078** | `hir/tests.rs` (137) | `mod tests` |
| `vis-cli/src/cli/run_cmd.rs` | 1.183 | **1.000** | `run_cmd/host_fs.rs` (192) | `enum HostFs` + `impl FsHost` + `HostFs::tao` |
| `vis-syntax/src/parse/mod.rs` | 1.172 | **1.049** | `parse/api.rs` (134) | `parse`/`parse_file`/`parse_voi`/`parse_voi_file`/`dat_file_cst` (+ `pub use api::*;`) |
| `vis-core/src/http_client/mod.rs` | 1.152 | **977** | `http_client/tests.rs` (176) | `mod tests` |
| `vis-types/src/lower/trait_impl.rs` | 1.152 | **1.122** | `trait_impl/sieu.rs` (38) | `kiem_sieu_trait` (ST-57 siêu trait một tầng) |

Ghi chú kỹ thuật (đã vấp và sửa, giữ lại để lần sau khỏi lặp):

- Tệp `X/tests.rs` là **thân module** `X::tests`, nên nội dung phải bỏ lớp `mod tests { … }` khi
  tách (nếu không sẽ thành `X::tests::tests`).
- Hàm private chuyển **xuống** module con mà module cha vẫn gọi phải nới `pub(super)`
  (`infer_ty`, `chuan`, `tao`, `kiem_sql`, `splitmix64`…); chiều ngược lại (cha → con) thì
  private của con **không** thấy được.
- Doc-comment nằm sát ranh giới cắt phải đi theo hàm, nếu không clippy báo
  `empty line after doc comment` (đã sửa ở `than.rs`, `sqlite/mod.rs`).

## 3. `#193` — thử `-zstack-size` cho WASI (non-blocking)

**Kết luận: `-zstack-size` là điều kiện **cần** nhưng **chưa đủ**; KHÔNG đóng được `#193`.**

Dựng `vis-wasm-run` cho `wasm32-wasip1` (workspace riêng `vis-host-wasm`), chạy 2 ca của `#193`
qua runtime **node:wasi** (máy `ps` **không có** `wasmtime` CLI lẫn Python `wasmtime`; harness
`bin/vis-wasm-sem` tự chọn node):

| Cấu hình | `de-quy-1000` | `de-quy-vo-han` | Ghi chú |
|---|---|---|---|
| stack mặc định (1 MiB), node mặc định | `RangeError: Maximum call stack size exceeded` | như trên | trần **stack JS của V8**, không phải stack wasm |
| `-zstack-size=8 MiB`, node mặc định | như trên | như trên | memory min = **134 trang = 8.781.824 B** ⇒ cờ **có** tác dụng |
| `-zstack-size=8 MiB`, `node --stack-size=8192` | `memory access out of bounds` | như trên | hết **shadow stack** của module |
| `-zstack-size=64 MiB`, `node --stack-size=8192` | **in `1000`** ✓ | **`VS0501 … >2000 khung`** (exit 101) ✓ | cả 2 ca **PASS** |
| `-zstack-size=64 MiB`, node mặc định | vẫn `Maximum call stack size exceeded` | — | trần V8 vẫn chặn |

Diễn giải:

1. Trên **node:wasi**, khung wasm nằm trên **stack native của V8** ⇒ trần thật là
   `--stack-size` của node, **không** phải `-zstack-size` của module. Vì vậy chỉ thêm cờ linker
   **không** làm 2 ca xanh trên runtime này.
2. Khi đã nới stack V8, `-zstack-size` mới là trần thứ hai: 8 MiB **không** đủ cho ~1.000 khung
   ViScript (≈ >8 KB/khung ở bản release), 64 MiB đủ cho cả 1.000 khung lẫn mốc trap
   `MAX_CALL_DEPTH = 2.000`.
3. Trên **wasmtime** (runtime chính theo `docs/SPEC.md`), khung wasm nằm trên **shadow stack**
   trong linear memory ⇒ `-zstack-size` mới là trần duy nhất và rất có thể đủ để đóng `#193`
   — **nhưng chưa kiểm chứng được** trên `ps` (`BLOCKED-ENV`: thiếu `wasmtime`/Python `wasmtime`).

Vì vậy **không** đưa cờ vào build mặc định (64 MiB × mỗi instance là ~1.025 trang bộ nhớ khởi
tạo, đắt cho runner/plugin). Đề xuất cho A0 (chọn 1):

- **(a)** Kiểm lại trên máy **có wasmtime** (`mps`) với `RUSTFLAGS="-C link-arg=-zstack-size=33554432"`
  và `bin/vis-wasm-sem --runtime wasmtime`; nếu 390/390 ⇒ đóng `#193` bằng cờ + ghi chú bộ nhớ.
- **(b)** Nếu vẫn đỏ ⇒ sửa gốc: **trampoline/khử đệ quy** ở `vis-interp` (đúng hướng "trampoline
  interp" mà `SO-NO` đã ghi), hoặc hạ `MAX_CALL_DEPTH` riêng cho WASI (không khuyến nghị — đổi
  ngữ nghĩa theo nền).
- **(c)** Trước mắt: ghi `--stack-size` vào tài liệu chạy WASI bằng node (không sửa mã).

## 4. Cổng máy

```
./vis-dev gate --fast
  fmt OK · clippy OK · luật R1..R9 OK · toolchain PASS · release PASS
  self-test case âm OK · vis explain OK · báo cáo OK · fixtures OK
  build OK · doc-examples OK (31 khối) · readme-examples OK
  L0 test: cargo-nextest → Summary 3778 tests run: 3778 passed (1 slow), 4 skipped
            + cargo test --doc OK
  vis-dev gate --fast: PASS
```

Không thêm `unsafe` (R2 xanh), không thêm tệp log > 50 KB (R7 xanh), không đổi `Cargo.toml`/
dependency, không đụng `vis-dev`/cổng.

## 5. Phát hiện ngoài phạm vi (cần A0 biết)

`main` **`0bcb5af`** có sẵn **2 ca đỏ** trong `crates/vis-core/tests/core_it.rs` (tệp do **ST-311c**
thêm ở `a320e02`) — gate `--fast` **đỏ từ trước ST-311d**:

| Ca | Kỳ vọng trong tệp | Hành vi thật (đã đối chiếu `main`) |
|---|---|---|
| `json_roundtrip_va_escape_duy_nhat` | `decode(esc(s))` | `esc` trả **nội dung** trong dấu `"` (không thêm dấu bao) ⇒ phải bọc `"…"` trước khi `decode` |
| `value_parse_size_duration_va_exit_code` | `5kb = 5120`, `250ms = 250` | `kb`/`mb` là **SI** (1000), `kib` mới là 1024; `parse_duration` trả **nanosecond** |

Đã sửa **kỳ vọng của test** cho khớp hợp đồng đã ghi trong `json/esc.rs` + `value/mod.rs`
(không đổi một dòng sản phẩm). **Nợ mới phát hiện**: `parse_duration("sai")` trả `Some(0)` thay vì
`None` (khác `parse_size`) — ST-311d **không** vá vì phạm vi là chia tệp; test tạm ghi
`Some(0)` + chú thích `Nợ (JOURNAL ST-311d §5)`. Đề nghị cấp mã nợ riêng (`#196`) để vá
`parse_duration` trả `None` khi không có cặp `<số><đơn vị>` nào.

**Flaky đã quan sát (không do ST-311d)**: `vis-cli::cli_it repl_term::format_la_la_loi_dung_exit_2`
đỏ **một lần** khi máy đang chạy song song 2 build wasm (`BrokenPipe` lúc ghi stdin của PTY con);
chạy lại **3/3 xanh** và xanh trong cổng cuối. Nên ghi nhận là ca nhạy tải (không sửa).

## 6. Việc chưa làm / bàn giao

- Không merge/push/tag (đúng chỉ đạo). Nhánh `plan/st311d-split` giữ 2 commit: `[ST-311d]` (chia
  tệp + `#193`) và `[ST-311d]` (sửa kỳ vọng `core_it` của ST-311c).
- `agent/QUEUE.md`/`DECISIONS.md`/`ROADMAP.md` **không** sửa (A0 đồng bộ ở commit `[ST-00]`).
- `#193` **vẫn mở** (xem §3), `#196` **đề nghị mở** (§5).
- Chưa chạy `nightly` (fuzz + vòng tròn fmt) và chưa chạy ma trận 3-OS: ngoài time-box 2,5 h của
  lát này; `gate --fast` là cổng bắt buộc và đã xanh.
