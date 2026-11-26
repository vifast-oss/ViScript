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

## 4. Phần 2 — `Value::List(Rc<Vec<Value>>)` + handoff đích (đã xong)

- Enum đổi `List(Vec<Value>)` → `List(Rc<Vec<Value>>)`; thêm `Value::list(Vec)` / `Value::list_rc(Rc)`.
  Toàn workspace sửa theo cụm: mẫu khớp giữ `Value::List(rc)`, chỗ dựng đổi `Value::list(vec)`,
  vòng lặp đổi `.iter()`, chỗ cần `Vec` dùng `Rc::unwrap_or_clone`.
- **Handoff mở rộng cho List** (cùng cơ chế Map): `tach_dich` (VM) nhận diện `Rc` của **Map hoặc
  List**, chỉ tách ô đích khi instruction kế là `Assign/StoreLocal` cùng con trỏ.
  - `Instr::CallMethod`: tách ô đích **trước** khi gọi method rồi trả lại kết quả ⇒ `xs.push(v)`
    nối tại chỗ (không chép list).
  - Interp: `Interp::lay_o` lấy hẳn giá trị khỏi ô cho `push` trên `var`; `tach_o` mở rộng cho List
    ở đường `x[i] = v`.
- Sự cố nhỏ đã vá: sed toàn cục `Value::List(` → `Value::list(` lỡ đổi `SendValue::List`,
  `PluginValue::List`, `ProtoValue::List` và các **mẫu khớp** → đã hoàn lại đúng từng chỗ
  (compiler bắt hết; không còn `unsafe`, không đổi hành vi).

### Số đo phần 2 (release, 3 lần, `listbench` = list 10k + 10k đọc + 10k ghi)

| Bản | interp | `--vm` |
|---|---|---|
| `5cbb7853` (nền) | 3.351–3.726 ms | 3.944–4.536 ms |
| `9983c82b` | **21–22 ms** | **14–18 ms** |

⇒ **~165×** interp, **~250×** VM (KPI `listbench ≥ 10×`). `push` riêng: 10k 1.228 ms → **9 ms**,
20k 4.434 ms → **13 ms** (tuyến tính). Alias list `b = a; a[0]=9` ⇒ `a=9, b=1` cả 2 engine.
`writebench` phần 1 giữ nguyên 9–11 ms.

### Cổng phần 2

- `./vis-dev gate --fast` → **PASS** (3.856 test, 4 skip).
- `VIS_DIFF_N=100000 ./bin/vis-diff` → **`lech=0`** (93.768 chạy, 60,9 s, 1.642/s).
- Đã merge nhánh F1 (`b61ec663`, test 2× `Set-Cookie`) vào `plan/st316a` (`213c8ca9`).

## 5. Chưa xong (phần 3, đang chạy song song)

1. F2 (chỉ mục map không alloc khi khoá phức tạp) + F4 (`json.decode` gộp trực tiếp `VisMap`) —
   subagent nhánh `plan/st316a-f24`.
2. F5 (`std.map.remove`/`merge` qua `VisMap` + test non-transitive `N ≥ 8`) — nhánh `plan/st316a-f5`.
3. `bin/vis-bench` (JSON median/p90 + sha256) — nhánh `plan/st316a-bench`.
4. Audit độc lập `ST-316b` — nhánh `review/st316b-audit`.

## 5. Tuyên bố

- Chưa merge/push; commit trên `plan/st316a`. Không sửa `docs/SPEC*`, `agent/DECISIONS*`,
  `QUEUE*`, `SO-NO*` (Agent 0 sở hữu).
- `mps` không dùng. Không tạo tệp `.rs` cấp 1 mới (R8 giữ 39/40).

## 6. Đo lại bằng harness chính thức `bin/vis-bench` (sau khi merge `98d37ef1`)

`./bin/vis-bench --bin <release> --runs 5 --json` (JSON: `/home/ps/vislogs/st316a-bench-list.json`,
binary sha256 `2def425297ab…`; máy có tải nền nên chỉ so trong cùng lượt):

| Chương trình | interp median/p90 (ms) | VM median/p90 (ms) | stdout |
|---|---:|---:|---|
| opt-bench | 47 / 47 | 39 / 43 | 38995450 |
| trim-matches | 33 / 34 | 28 / 30 | 1000009 3 1500003 … |
| mapbench | 46 / 48 | 45 / 47 | 15737616 |
| **listbench** | **32 / 34** | **20 / 22** | 16242400 |
| writebench | 5 / 6 | 3 / 4 | 10100 |

`listbench` trên cùng chương trình, binary nền (trước List CoW, báo cáo bench `98d37ef1`):
**411 / 422 ms** ⇒ **~13× interp, ~19–21× VM** (KPI ≥ 10×). Bản `--runs 3` đó chạy lúc load 1,70;
lượt này có tải nền khác nên tỷ lệ là **thận trọng**, không phóng đại.

Đã merge vào `plan/st316a`: F1 (`213c8ca9`), bench (`6d1d65c1`).
