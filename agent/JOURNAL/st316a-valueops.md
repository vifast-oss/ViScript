<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-316a (phần 1) — Một nguồn `vis_runtime::value_ops` + CoW tại chỗ cho `Map`

> Nhánh `plan/st316a`, nền `5cbb7853` (main), commit checkpoint `4ff443c6`.
> Đây là **phần 1** của lát 2,5–3 ngày; phần `Value::List(Rc<…>)` và F1/F2/F4/F5 chưa xong.

## 1. Đã làm

### 1.1 Một nguồn thao tác `Value`

- Tệp mới `crates/vis-runtime/src/value_ops.rs`: `truong`/`doc_truong`, `chi_so`/`chi_so_opt`,
  `dat`/`dat_truong`/`voi`, `Buoc`/`doc_duong`/`gan_duong`.
- `vis-vm/src/vm/vm_helpers.rs` chỉ còn `lit_kind` + re-export tên cũ (`field/index/index_opt/
  set/with`) → **xoá bản sao** thứ nhất.
- `vis-interp/src/gan.rs` còn lớp mỏng gọi `value_ops` → **xoá bản sao** thứ hai; chuỗi
  `VS0501`/`VS0503` nay chỉ có một chỗ sửa.
- Hợp nhất hành vi `Enum`: option bóc `Some(x)`, `e.0` theo chỉ số, atom `e.Ten` — chạy
  `vis-diff 100k` + L0 để chốt parity.

### 1.2 CoW tại chỗ cho `Map` (không đổi VBC1)

- `dat`/`voi`/`dat_truong`/`gan_duong` nhận `&mut Value` (sửa tại chỗ), không `clone()` trước
  `Rc::make_mut`.
- **Chỉ tách đúng ô đích**: trước khi `Set`/`With`, VM đọc instruction **kế tiếp**; nếu là
  `Assign(slot)`/`StoreLocal(slot)` và ô đó đang trỏ **cùng** `Rc<VisMap>` với giá trị trên
  ngăn xếp thì tạm đặt ô = `Unit`, ghi, rồi trả lại `base.clone()`.
  - Ô `>= GLOBAL_SLOT_BASE` là biến cấp tệp (`vm.globals: Vec<Option<Value>>`), còn lại là
    `Frame::locals`.
  - **Không** tách mọi ô cùng `Rc` (bản thử đầu): làm vậy `b = a` sẽ thấy đột biến của `a` —
    sai ngữ nghĩa giá trị. Chỉ ô đích bị tách nên biến chia sẻ vẫn CoW như cũ.
- Interp: `tach_o`/`tra_o` trên `env` làm đúng điều đó sau khi đã đánh giá chỉ số + vế phải;
  `gan_long` tính lá mới **trước** rồi mới tách ô.
- **0 thay đổi định dạng bytecode** (không thêm opcode/operand; chỉ đọc trước instruction kế).

## 2. Số đo (release, cùng máy `ps`, 3 lần/engine)

`writebench` = map 10.000 khoá rồi 100 lần ghi khoá mới (`/tmp/opencode/writebench.vis`):

| Bản | interp | `--vm` |
|---|---|---|
| `5cbb7853` (nền) | 2.392–2.403 ms | 2.339–2.540 ms |
| `4ff443c6` | **9–11 ms** | **8–9 ms** |

⇒ **~240×** (KPI `writebench ≥ 20×`). Lưu ý: lúc đo có 2 subagent build song song nên số nền
có thể nhiễu vài %; mức tăng lớn hơn ngưỡng rất xa.

Kiểm ngữ nghĩa alias (`var a = {1: 10}; var b = a; a[1] = 20`): cả 4 tổ hợp (nền/mới ×
interp/VM) đều in `20` rồi `10` — `b` giữ bản cũ.

## 3. Cổng

- `./vis-dev gate --fast` → **PASS** (fmt, clippy, R1..R9, DAG + self-test, build, doc-examples, L0).
- `VIS_DIFF_N=100000 ./bin/vis-diff` → **`lech=0`** (93.768 chương trình chạy, 72,1 s).
- `cargo check --workspace --all-targets` → sạch.

## 4. Chưa xong (phần 2)

1. `Value::List(Rc<Vec<Value>>)` + `Value::list(...)`: đã thử đổi enum, phát sinh **83 lỗi** trên
   ~86 tệp (chủ yếu `for x in xs`, `.into_iter()` trên `Rc`, chỗ dựng `Value::List(vec)`), nên
   **revert** để giữ cây xanh. Cần một lượt riêng, sửa theo cụm và chạy lại `gate --fast` +
   `vis-diff 100k` sau từng cụm.
2. **listbench** (đọc/pass O(1) ≥ 10×) phụ thuộc mục 1.
3. F1 (2× `Set-Cookie` last-wins), F2 (0 alloc `HashMap` khi khoá phức tạp), F4 (`json.decode`
   gộp trực tiếp `VisMap`), F5 (`std.map.remove`/`merge` + non-transitive `N ≥ 8`).
4. `bin/vis-bench` (release, JSON median/p90, sha256) cho `opt-bench`/`trim-matches`/`mapbench`/
   `listbench`/`writebench`.
5. Sau khi xong: `gate --fast`, `vis-diff 100k`, clippy 3 OS (Linux + `psvn`), rồi `REVIEW`.

## 5. Tuyên bố

- Chưa merge/push; commit trên `plan/st316a`. Không sửa `docs/SPEC*`, `agent/DECISIONS*`,
  `QUEUE*`, `SO-NO*` (Agent 0 sở hữu).
- `mps` không dùng. Không tạo tệp `.rs` cấp 1 mới (R8 giữ 39/40).
