<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW Sóng 5 — `ST-316a` + `ST-316b` + `ST-316d` (2026-11-26)

> Nhánh `review/song5`, nền `5cbb7853` (main lúc mở Sóng 5). Tất cả cổng chạy trên cây **tích hợp**
> (a+b+d + audit), log tại `/home/ps/vislogs/review-song5-final.log`.

## 1. Tóm tắt phán quyết (Agent 2 tự đánh giá, chờ A0 phê chuẩn)

| Lát | Trạng thái | Bằng chứng chính |
|---|---|---|
| `ST-316a` | **REVIEW — PASS** | `value_ops` một nguồn; CoW tại chỗ cho `Map` **và** `List`; F1/F2/F4/F5; `bin/vis-bench`; writebench ~240×, listbench 13–21×; gate 3.864/3.864; diff 100k `lech=0` |
| `ST-316b` | **REVIEW — PASS** (Gatekeeper độc lập) | `#199` 25k const 1,604→0,196 s; `#48/#7/#186/#145/#47`; audit `089b9f4c`; gate 3.870/3.870 |
| `ST-316d` | **REVIEW — PASS** | DAG cấm `vis-host-native → vis-runtime` (+self-test); `#198` dời `worker_leak_it` + settle 500 ms; `host_parity` tách 4 tệp < 500 dòng; gate 3.858/3.858 |
| **Tích hợp** | **PASS** | gate **3.880/3.880** (4 skip), clippy `--all-targets -D warnings` **exit 0**, `vis-diff 100k` **`lech=0`** (60,2 s), `vis-dag-check` 23 crate/5 tầng/0 cạnh cấm, `rules` OK (R8 **39 ≤ 40**) |

## 2. `ST-316a` — chi tiết

- **Một nguồn** `vis_runtime::value_ops`: `truong/doc_truong`, `chi_so/chi_so_opt`, `dat/dat_truong/voi`,
  `Buoc/doc_duong/gan_duong`. `vm_helpers.rs` + `gan.rs` chỉ còn lớp mỏng — hết hai bản sao
  `VS0501`/`VS0503`.
- **CoW tại chỗ, 0 đổi VBC1**: trước `Set`/`With`/`CallMethod`, VM đọc instruction **kế tiếp**; nếu là
  `Assign(slot)`/`StoreLocal(slot)` và ô đích trỏ **cùng `Rc`** với giá trị thì tạm đặt ô = `Unit`,
  ghi, rồi trả lại. Chỉ **ô đích** bị tách ⇒ biến alias vẫn giữ bản cũ (đúng ngữ nghĩa giá trị).
  Interp tương ứng `tach_o`/`lay_o` (+ `tra_o`).
- **`Value::List(Rc<Vec<Value>>)`** + `Value::list/list_rc`; handoff mở rộng cho List
  (`push`, `xs[i] = v`).
- **F1**: test 2× `Set-Cookie` — `res.headers` last-wins, cookie jar đủ hai (commit `b61ec663`).
- **F2**: `MapIndex::dung` trả `None` **trước khi** cấp phát khi 0 khoá băm; test 16 khoá `List`.
- **F4**: `json.decode` gộp **trực tiếp** `VisMap`; test 100k cặp trùng ⇒ 1 entry.
- **F5**: `std.map.remove/merge` gọi thẳng `VisMap`, sửa tại chỗ khi không chia sẻ; test
  non-transitive `N=8` (`Int(1)` ↔ `Percent(1.0)`) + E2E corpus 565/565.
- **`bin/vis-bench`**: script Python, chạy interp + `--vm`, warmup 1, `--runs N`, JSON có
  median/p90/wall + sha256(binary/program/stdout), chống "nhanh vì sai" (stdout phải khớp 2 engine).

### Số đo (release, `bin/vis-bench --runs 5`, binary `78c36b64…`)

| Chương trình | interp median/p90 (ms) | VM median/p90 (ms) | Nền |
|---|---:|---:|---|
| opt-bench | 68 / 69 | 50 / 57 | 45–49 (máy êm) |
| mapbench | 47 / 48 | 47 / 51 | 42–46 |
| **listbench** | **32 / 33** | **21 / 21** | **411 / 422** ⇒ **13–20×** |
| **writebench** | **4 / 7** | **4 / 4** | **2.392–2.540** ⇒ **~240×** |

`push` riêng: 10k 1.228→**9 ms**, 20k 4.434→**13 ms** (tuyến tính). Alias map/list đều đúng
(`a` đổi, `b` giữ bản cũ) trên cả 2 engine.

### Caveat F5 (trung thực)
- Khoá non-transitive không dựng được bằng literal có kiểu trực tiếp (checker chặn `VS0412`);
  ca E2E dùng đường `{error}` qua lambda `id(x)`. Tầng Rust đã có test trực tiếp trên `VisMap`.
- `std.map.remove` ở mức ngôn ngữ chỉ nhận khoá `str`; phần Int/Percent kiểm ở tầng `VisMap`.
- `Float(1.0)` và `Percent(1.0)` `eq_value` bằng nhau; cặp non-transitive thật là
  `Int(1)` ↔ `Percent(1.0)`.

## 3. `ST-316b` — audit độc lập (Gatekeeper)

`JOURNAL/review-st316b.md` (commit audit `089b9f4c`): PASS 6/6; `#199` đo lại 25k **1,604 → 0,196 s**,
50k **4,654 → 0,251 s**, `check --json` nền/mới **byte-y-hệt**; `#186` trước lệch
(interp `VS0405`/exit 1 vs VM exit 0) nay **cả hai `VS0413`/exit 2**.

**Finding ngoài phạm vi (P2, có sẵn ở nền)**: `fn main(a: int, b: int)` → interp `VS0501` exit 101,
VM chạy với `b = ()` exit 0 — lệch stdout/exit. P3: `fn main(a: int)` cả hai gán `argv` vào tham số
`int` mà không kiểm kiểu. Đề nghị A0 đăng ký `SO-NO`, chưa sửa trong Sóng 5.

## 4. `ST-316d`

- `vis-runtime` → `[dev-dependencies]` ở `vis-host-native`; thêm cạnh cấm DAG + self-test âm
  (tổng 4 cạnh cấm).
- `#198`: `stress_dai_5x10k_cancel_budget_print_cross` dời sang `worker_leak_it.rs`, settle 500 ms
  cho `Threads:`, **0 binary R8 mới**.
- `host_parity.rs` 1.457 dòng → `host_parity/{mod,fs,sqlite,net_http}.rs` (388/370/299/435), 12 test
  giữ nguyên; thêm ca `MAX+1` (plugin) + truncate `cap[Dir]` chỉ-đọc (Landlock) + ghi nhận ca 2 SCC
  đã có từ `9bbb77dd`.

## 5. Commit chính trên `review/song5`

- `plan/st316a`: `4ff443c6` (value_ops+Map CoW), `9983c82b` (List CoW), `213c8ca9` (F1),
  `6d1d65c1` (bench), `f13eedbc` (F2+F4), `6ea3a3b7` (F5), các commit nhật ký.
- `plan/st316b`: `da5590e3`; audit `089b9f4c`.
- `plan/st316d`: `ecdb0660`.
- Merge cuối: `c71af2d4` (ST-316a full), `b1db35b6` (audit ST-316b), trên nền `2c46cd4e`/`940f6ac0`
  (b+d).

## 6. Chưa làm / kế tiếp

1. **A0 phê chuẩn** rồi merge `review/song5` → `main` (hoặc chỉ định người merge); hiện **chưa push
   `main`**.
2. `ST-316c`: macOS `mps` (khi máy rảnh) + `vis-targets --wasm` + đối chuẩn `vis-bench` 3-OS; cân nhắc
   finding P2/P3 của audit.
3. Sự cố môi trường đã gặp (không phải lỗi mã): `/tmp` tmpfs đầy 86% làm linker `signal 7`, gây đỏ
   oan 3–4 test; đã dọn và gate xanh lại. Khi chạy soak/fuzz dài cần kiểm `df -h /tmp` trước.
