<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-317a — `Value` 64 B → 32 B, CoW `Record`, singleton map rỗng

- **Nhánh**: `plan/st317a`, nền `882b7ffa` (Sóng 5 `PASS`), worktree `/home/ps/viswt/st317a`.
- **Máy**: `ps` (Linux), target riêng của worktree. `mps`/Windows không dùng.
- **Commit**: `fcecb5e6` (checkpoint đủ cổng; xem §5).
- **Phạm vi**: `ST-317a` (W1 · Sóng 6, `VD-292` §4.1) — nén `Value`, CoW `Record`, đóng F6
  `ST-315b` (map rỗng singleton). Không chạm VBC1, `docs/SPEC*.md`, `agent/DECISIONS.md`,
  `agent/QUEUE.md`, `agent/ACTIVE.md`.

## 1. Thiết kế

### 1.1 `Value` 32 B

Trước lát này `Value` = **64 B**: biến thể lớn nhất là `Cap { String, String, bool }` (56 B
payload) và `Record(String, Vec<(String, Value)>)` (48 B). Ba thay đổi:

| Biến thể | Trước | Sau | Payload |
|---|---|---|---:|
| `Record` | `(String, Vec<(String, Value)>)` | `Record(Rc<VisRecord>)` | 8 B |
| `Enum` | `(Name, Name, Vec<Value>)` | `Enum(Rc<VisEnum>)` | 8 B |
| `Cap` | `{ kind: String, root: String, write: bool }` | `Cap(Box<CapVal>)` | 8 B |

Payload lớn nhất còn `Str`/`Path`/`Bytes` = 24 B + 8 B tag ⇒ **`size_of::<Value>() == 32`**
(unit test `value::tests_value_32::size_of_value_bang_32`, và `core_it` khoá `== 32` trên 64-bit).

- `Enum` giữ `Clone` **O(1)** (tăng refcount) như `Map`/`List`; đối số 0/1 phần tử nằm **trong**
  cấp phát `Rc<VisEnum>` qua `EnumArgs::{Zero,One,Many}` — `Some(x)` vẫn **một** `malloc` như
  `vec![x]` cũ, không thêm cấp phát. `Value::none()` dùng singleton thread-local ⇒ 0 cấp phát.
- `Cap` dùng `Box` (không `Rc`) vì quyền gần như không `clone`; `Clone` của `Value::Cap` là `Box`
  clone — không nằm trên đường nóng.
- Accessor một nguồn: `as_enum`/`enum_names`/`enum_args`/`into_enum`, `as_record`/`into_record`.
  `into_enum`/`into_record` **CoW**: `Rc` chia sẻ ⇒ chép một bản rồi trả sở hữu, nên chỗ gọi cũ
  (`into_some`, `?`, `ApplyTry`, `select`) sửa `Vec` mà không đụng giá trị khác.

### 1.2 CoW `Record`

`dat_truong` (`x.f = v`), `voi` (`x with f: v`), `gan_duong` (đường gán `a.b.f = v`) đều dùng
`Rc::make_mut`:
- kiểm **đủ** trường trước khi sửa (giữ nguyên hợp đồng "lỗi không để lại nửa vời" của `voi`);
- bản ghi **không chia sẻ** ⇒ sửa tại chỗ `O(1)` (engine đã tách ô đích như `Map`/`List`,
  `ST-316a`); chia sẻ ⇒ CoW một lần; `let b = a; a.x = 9` ⇒ `b` giữ bản cũ.
- ca conformance `record-cow.vis` khoá hành vi cả interp và `--vm`, kể cả `with` lồng
  (`q.p with x: 5`).

### 1.3 Singleton map rỗng (đóng F6 `ST-315b`)

`thread_local! { static EMPTY_MAP: Rc<VisMap> = Rc::new(VisMap::new()) }`; `Value::map([])`,
`Value::map_tu(map rỗng)` và `VisMap::thanh_gia_tri` (rỗng) trả `Value::map_rong()` =
`Rc::clone` singleton ⇒ **0 heap alloc cho `{}`** (trước +48 B/map rỗng).
An toàn: singleton **luôn giữ một refcount**, nên `Rc::make_mut` không bao giờ thấy
`strong_count == 1` cho bản dùng chung; mọi đường ghi (`dat`, `voi`, `gan_duong`,
`std.map.remove/merge`, `json.decode`) đều đi qua `Rc::make_mut`.
Ca conformance `map-rong-singleton.vis`: hai map rỗng, sửa lần lượt, cả hai vẫn đúng; `{} == {}`.

## 2. Call site & ngữ nghĩa

- Cập nhật **toàn bộ** workspace: `vis-core` (value/json/reactor), `vis-runtime` (value_ops,
  builtin, render, lift, select, worker/proto, sqlite/*), `vis-interp` (interp/frame/pattern),
  `vis-vm` (`exec.rs`), `vis-http`, `vis-host-native` (sqlite/quyen), `vis-plugin-host`,
  test `vis-conformance`/`vis-interp`.
- `SendValue` **giữ nguyên ABI** (`Record(String, Vec)`, `Enum(String, String, Vec)`,
  `Cap { .. }`); chỉ chuyển đổi ở biên `to_value`/`to_send`.
- `Debug` của `Value::Enum`/`Record` đổi hình (bọc `Rc`) — chỉ một test snapshot `{:?}` cập nhật;
  `Show`/`str()` **không đổi** (conformance 567/567 byte-exact).
- `eq_value`, băm khoá map, `Show`, `json.encode`, pattern match: giữ nguyên luật.
- 0 `unsafe` mới (R2), 0 đổi VBC1.

## 3. Test mới

- `crates/vis-core/src/value/mod.rs::tests_value_32`: `size_of::<Value>() == 32` (64-bit) và
  singleton `{}` chia sẻ + CoW không rò.
- `crates/vis-core/tests/core_it.rs`: khoá `Value == 32` (thay assert 64 cũ).
- `test/conformance/lang/st317a_value32/record-cow.{vis,out}` và
  `map-rong-singleton.{vis,out}` — chạy **cả** interp và `--vm` qua harness dữ liệu.
- Không tạo tệp `.rs` cấp 1 mới (R8 giữ **39/40**).

## 4. Cổng (số thật, log `/home/ps/vislogs/st317a-*.log`)

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all` | PASS |
| `cargo check --workspace --all-targets` | **0 error / 0 warning** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0, `st317a-clippy.log`) |
| `./vis-dev gate --fast` | **PASS** — `check` PASS (fmt, clippy, R1..R9, DAG 5 tầng, self-test, doc/readme-examples) + L0 **3.882 test PASS / 4 skip** (`st317a-gate1.log`) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), 173,0 s, 578/s (`st317a-visdiff.log`) |
| `cargo test -p vis-core -p vis-runtime -p vis-interp -p vis-vm` | PASS (unit + integration, gồm `interp_it`, `vm_it`, `stdlib_diff`) |
| Conformance `corpus` | **567/567** ca xanh (2 ca mới) |

## 5. Đo `bin/vis-bench` (release)

- Nền: `882b7ffa`, binary sha256 `77e64bc0…96b`, JSON `/home/ps/vislogs/st317a-bench-before.json`
  (`sha256 fae7452e…bcec`), đo lúc load ~0,4.
- Sau: `fcecb5e6`, binary sha256 `8abf6519a72d772fb8bdde73b4000b4cf09999ea65162a1b65f9ac2439af2101`,
  JSON `/home/ps/vislogs/st317a-bench-after-final.json` (`sha256 a49f2bca…b605`), 5 mẫu, load ~1,8.
- Máy `ps` có tải nền thay đổi, nên bảng **chuẩn** là A/B xen kẽ 5 vòng (mỗi vòng nền rồi sau,
  `--runs 3`, trung vị của trung vị; JSON `/home/ps/vislogs/st317a-ab-{base,after}-{1..5}.json`):

| Chương trình | Engine | Nền (ms) | Sau (ms) | Δ |
|---|---:|---:|---:|---:|
| opt-bench | interp | 47 | **46** | −2,1 % |
| opt-bench | vm | 40 | **38** | −5,0 % |
| trim-matches | interp | 33 | **31** | −6,1 % |
| trim-matches | vm | 29 | **29** | 0 % |
| mapbench | interp | 46 | **43** | −6,5 % |
| mapbench | vm | 45 | **44** | −2,2 % |
| listbench | interp | 29 | **30** | +3,4 % (10 vòng riêng: 31 → 30,5) |
| listbench | vm | 20 | **19** | −5,0 % |
| writebench | interp | 5 | **4** | −20 % |
| writebench | vm | 4 | **3** | −25 % |

`listbench` đo riêng 10 vòng A/B (5 mẫu/vòng): interp nền 31 / sau **30,5**; VM nền 20 / sau **19**
⇒ không hồi quy ngoài nhiễu. Stdout cả 5 chương trình **giống hệt** nền (harness tự so).

## 6. Caveat & việc còn lại

- `Enum` bọc `Rc`: `Some`/`Ok`/`Err`/enum người dùng thêm **một** cấp phát `Rc` (đối số 0/1 nằm
  trong đó). A/B cho thấy bù lại nhờ `Value` nhỏ hơn (frame/stack/list/map dày gấp đôi); nếu
  Sóng 6 sau này cần tối ưu tiếp, hướng là arena/small-vec cho đối số enum.
- `Debug` của `Value::Enum`/`Record` đổi hình (bọc `Rc`) — không phải hợp đồng; `Show` giữ nguyên.
- `into_enum`/`into_record` chép một bản khi `Rc` chia sẻ (CoW); đúng ngữ nghĩa, nhưng đường nào
  lạm dụng `into_*` trên giá trị chia sẻ nhiều lần có thể tốn O(n) — hiện chỉ dùng ở `?`/pattern.
- `VisMap` singleton theo thread; `SendValue` vẫn snapshot `Vec` khi qua biên worker (không đổi ABI).
- `crates/vis-core/src/value/mod.rs` nay **1.415 dòng** (< 1.500 R1, mục tiêu < 1.000 của
  `ST-317b` sẽ tách phần accessor/`EnumArgs` nếu cần).
- Không chạy fuzz/soak (chế độ Phát triển Kiến trúc `VD-289`); không chạm Windows/macOS.
