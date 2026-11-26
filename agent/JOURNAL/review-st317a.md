<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Review ST-317a — Gatekeeper (Agent 1) độc lập

**Worktree**: `/home/ps/viswt/st317a` · nhánh `plan/st317a` · HEAD `87985b7e` (mã `fcecb5e6`) · nền `882b7ffa`.
**Kết luận bản gốc (`ec4d7fc6`)**: **PASS** — 0 finding P1; 1 finding P2 (hiệu năng/tài liệu, **không sai ngữ nghĩa**); 3 ghi chú P3.
**Addendum F1 (`2e4090e2`)**: **FAIL** — 1 finding **P1** mới: fast path `r = r with …` bỏ một charge `FeeKind::Expr` (và `steps`/`worker_tick`) nên ngân sách/`--max-budget` lệch interp↔VM. Xem §9.
Không sửa mã sản phẩm trong lượt review này; chỉ thêm tệp này.

## 0. Phạm vi & hợp nhất — PASS

| Kiểm | Bằng chứng |
|---|---|
| Cây làm việc sạch | `git status --short` → rỗng (trước khi thêm tệp này) |
| Lịch sử | `git log --oneline 882b7ffa..HEAD` → `87985b7e` (journal/STATE), `fcecb5e6` (mã) |
| Quy mô | `git diff --stat 882b7ffa..HEAD` → **56 tệp, +1.486/−776** (gồm 2 tệp journal/STATE, 4 tệp conformance mới) |
| Xung đột với `main` | `git merge-tree --write-tree aecb266c HEAD` và `--write-tree main HEAD` (main thực tế `29a7b254`) → **chỉ `agent/STATE.md`** xung đột nội dung; các tệp còn lại auto-merge sạch |
| Tệp nhạy cảm | `vis-runtime/src/worker/proto.rs`, `vis-host-native/src/db/sqlite/mod.rs`, `vis-core/src/value/mod.rs`: **không xung đột**; `main` không đổi 3 tệp này kể từ `882b7ffa` |
| VBC1 / codegen | `git diff 882b7ffa..HEAD -- agent/API-VBC1.txt crates/vis-codegen` → **rỗng** |
| `unsafe` mới | `git diff 882b7ffa..HEAD \| grep '^+.*unsafe'` → chỉ 1 dòng journal (“0 `unsafe` mới”), **0 dòng mã** (R2) |

Xung đột `agent/STATE.md` là xung đột **văn bản thuần**: `main` thêm khối `ST-317b` phần 1 + `ST-317c`,
nhánh này thêm khối `ST-317a`; hợp nhất phải giữ **cả ba** khối, không có xung đột mã/ngữ nghĩa.
`main` thực tế đã đi thêm một commit (`29a7b254`, tách `vis-lsp/lib.rs`) so với mốc `aecb266c` trong
đề bài; kiểm cả hai mốc cho cùng kết quả.

## 1. Layout 32 B — PASS

| Kiểm | Bằng chứng |
|---|---|
| `size_of::<Value>() == 32` | `cargo test -p vis-core --lib value::tests_value_32` → **2 passed** (`size_of_value_bang_32`, `map_rong_chia_se_va_cow_khong_ro`) |
| Khoá ở integration test | `core_it::value_kich_thuoc_va_map_cow` → **PASS** (assert `= 32` trên 64-bit, `≤ 32` nền khác) |
| Payload lớn nhất | `Str/Path/Bytes` = 24 B + tag 8 B ⇒ 32 B; `Record`/`Enum` = `Rc` 8 B, `Cap` = `Box` 8 B |
| Không `unsafe` mới | xem §0 (R2) |
| Không đổi VBC1 | xem §0 |

Test in đúng: `test result: ok. 2 passed; 0 failed; ...` (log `/home/ps/vislogs/review-st317a-test.log`).

## 2. Ngữ nghĩa CoW / singleton — PASS

### 2.1 Conformance hai engine

`cargo test -p vis-conformance --test corpus -- --nocapture`:
`conformance: 567/567 ca xanh` (565 ca nền + 2 ca mới), exit 0.
Harness `vis-conformance::khang_dinh` chạy **cả interp và VM** rồi so byte từng ca
(`crates/vis-conformance/src/lib.rs:131-178`). Log:
`/home/ps/vislogs/review-st317a-conformance.log`, sha256
`49717b0abd1eb08944903569a21a3312657ab283dfa5756c7f3ef79a1dc60c50`.

### 2.2 Kiểm thủ công (cả `vis run` và `vis run --vm`)

Chương trình `/tmp/opencode/st317a-manual.vis` (map rỗng alias, 2 map rỗng, record alias,
record trong list, `?` trên `Some(map)`/`Some(record)`, `Ok`/`Err`). Hai engine cho stdout
**giống hệt** (`diff` rỗng), exit 0:

```text
{}
{"k": 1}
{"a": 1}
{"b": 2}
true
3
P(x: 1, y: 2)
P(x: 1, y: 2)
P(x: 9, y: 2)
1
9
[P(x: 1, y: 2)]
[P(x: 7, y: 2)]
Some(5)
Some(5)
6
Some(5)
Some({"a": 1})
Some({"a": 1})
{"a": 1, "z": 9}
Some(P(x: 3, y: 4))
Some(P(x: 3, y: 4))
P(x: 100, y: 4)
P(x: 3, y: 4)
Ok(7)
Ok(7)
Err(x)
Err(x)
```

Chương trình `/tmp/opencode/st317a-manual2.vis` (map chứa record, gán `m["k"].x`, truyền record
vào hàm, `q.p with x`): hai engine giống hệt, exit 0:

```text
{"k": P(x: 1, y: 2)}
{"k": P(x: 7, y: 2)}
P(x: 3, y: 4)
P(x: 42, y: 4)
Q(p: P(x: 5, y: 6), z: 9)
P(x: 50, y: 6)
```

Đối chiếu yêu cầu: (a) `a = m; b = m; m["k"] = 1` ⇒ `b` vẫn `{}` (dòng 1–2); (b) hai `{}` độc lập
(dòng 3–6); (c) `r2 = r with f: v` không đổi `r` (dòng 7–11, 21–24); (d) alias `Record`/`Enum`
(`Some`/`Ok`/`Err`, payload map/record) đều giữ giá trị gốc, bản sao chỉ đổi sau CoW
(dòng 12–28) — **PASS**.

## 3. Tương thích `Show`/`str`/JSON — PASS

| Kiểm | Bằng chứng |
|---|---|
| Byte-exact toàn corpus | 567/567 ca xanh, gồm các ca `json.*`, `show`, `str`, `format` |
| Stdout bench trước/sau | 5/5 chương trình có `stdout` + `sha256` **giống hệt** giữa `st317a-bench-before.json` và `st317a-bench-after-final.json` (kiểm bằng Python) |
| `Debug` chỉ đổi hình thức | `grep -rn 'format!("{:?}"' crates` → chỉ 1 chỗ cho `Value` (`value/mod.rs:1411`), đã cập nhật đúng dạng mới; 0 golden `.out`/`.md` chứa `Record(`/`Enum(` của `Value` (các hit `Record(`/`Enum(` còn lại là tên instruction `I::MakeRecord`/`I::MakeEnum` trong test bytecode) |
| `Show`/JSON code | diff `show_eq.rs`/`json/mod.rs` chỉ thay truy cập `r.name`/`r.fields`/`e.args`; không đổi chuỗi định dạng |

Ghi chú P3: journal §1.2 nói ca `record-cow.vis` khoá cả “`with` lồng (`q.p with x: 5`)”, nhưng tệp
thực tế chỉ có `a with x: 9`; trường hợp lồng đã được tôi kiểm thủ công ở `manual2.vis` và **PASS**
(`Q(...)` giữ nguyên, chỉ `P` mới đổi). Đề nghị sửa câu journal hoặc thêm ca vào `record-cow.vis`.

## 4. Cổng độc lập — PASS

| Cổng | Kết quả thật | Log / sha256 |
|---|---|---|
| `cargo fmt --all --check` | exit **0** | stdout sạch |
| `./vis-dev gate --fast` | **3882/3882 passed, 4 skipped**; `vis-dev gate --fast: PASS`; exit **0** | `/home/ps/vislogs/review-st317a-gate.log` — `9df9849f82c4b12d9c3da30a7e4b27616cd57229d26c54f36d30954a57a621bd` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `tong=100000 loi_tinh=6232 chay=93768 lech=0 giay=77.7`; exit **0** | `/home/ps/vislogs/review-st317a-visdiff.log` — `68e2e09b75c5b04028ad9008450360b486d92df4b53b5d7a618accaaed7d2457` |
| `cargo test -p vis-core -p vis-runtime -p vis-interp -p vis-vm` | **450 passed, 0 failed**, exit **0** | `/home/ps/vislogs/review-st317a-test.log` — `818cde272222bb228cc2baab06c97b4d4081c4218520d0852272b47836b8924c` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit **0**, 0 warning | `/home/ps/vislogs/review-st317a-clippy.log` — `4aeda7f6832cf9a200d4d988400309e681ef9e0ba70b5d7ed3775379757ce04f` |
| Conformance corpus | **567/567**, exit **0** | `/home/ps/vislogs/review-st317a-conformance.log` — `49717b0abd1eb08944903569a21a3312657ab283dfa5756c7f3ef79a1dc60c50` |

`gate --fast` chạy `cargo-nextest` toàn workspace (gồm `vis-vm::diff_100k`, `fuzz_mutate`,
`vis-conformance`), nên 450 test của 4 crate là lớp kiểm bổ sung, không thay cổng.
Đĩa lúc chạy: 13–14 GB trống (> 6 GB), RAM 14 GB; dùng `CARGO_BUILD_JOBS=3`,
`CARGO_PROFILE_DEV_DEBUG=0`, không `cargo clean`.

## 5. Bench — PASS (không hồi quy, không “nhanh vì sai”)

- JSON tồn tại và khớp báo cáo:
  - `st317a-bench-before.json` sha256 `fae7452e…bcec`, binary `77e64bc0…96b`;
  - `st317a-bench-after-final.json` sha256 `a49f2bca…b605`, binary `8abf6519…2101`;
  - `target/release/vis` hiện tại sha256 **`8abf6519a72d772fb8bdde73b4000b4cf09999ea65162a1b65f9ac2439af2101`** — khớp.
- 5 chương trình bench: `sha256` tệp nguồn khớp JSON; stdout trước/sau **giống hệt** 5/5.
- Kiểm lại A/B 5 vòng từ 10 JSON `/home/ps/vislogs/st317a-ab-{base,after}-{1..5}.json`
  (trung vị của trung vị, ms — interp/VM):

| Chương trình | Nền | Sau | Δ |
|---|---:|---:|---|
| opt-bench | 47 / 40 | 46 / 38 | −2,1 % / −5,0 % |
| trim-matches | 33 / 29 | 31 / 29 | −6,1 % / 0 % |
| mapbench | 46 / 45 | 43 / 44 | −6,5 % / −2,2 % |
| listbench | 29 / 20 | 30 / 19 | +3,4 % / −5,0 % |
| writebench | 5 / 4 | 4 / 3 | −20 % / −25 % |

- Chạy lại trực tiếp `./bin/vis-bench --runs 3 --only writebench,listbench --json`:
  `listbench` interp 27 ms / VM 19 ms; `writebench` interp 4 ms / VM 2 ms; stdout + sha chương trình
  khớp bản nền; exit 0. Các mức chênh 1 ms nằm trong độ phân giải timer, nên **không** dùng để
  tuyên bố cải thiện định lượng; kết luận chỉ là **không hồi quy** và kết quả đúng.
- Không rebuild binary nền (tránh `cargo clean`/đụng worktree 317b đang build); A/B do Builder đo
  được kiểm lại độc lập từ JSON thô, không tin bảng tóm tắt.

## 6. R8 / R1 / R7 — PASS

- R8: `crates/vis-cli/tests/*.rs` = **8/15**; tổng tệp test cấp 1 toàn workspace = **39/40**.
- Không tạo tệp test cấp 1 mới: `git diff --diff-filter=A 882b7ffa..HEAD` chỉ có
  `agent/JOURNAL/st317a-value32.md` + 4 tệp conformance (`test/conformance/lang/st317a_value32/`).
- R1: `crates/vis-core/src/value/mod.rs` **1.415** dòng (< 1.500); quét toàn bộ `*.rs` trong
  `crates/`, `bin/`, `test/` → **0 tệp > 1.500** dòng.

## 7. Finding / rủi ro

| Mức | Nội dung | Tái lập / đề xuất |
|---|---|---|
| **P2** | Journal §1.2 và `agent/STATE.md` nói `Record` được sửa **tại chỗ O(1)** khi không chia sẻ nhờ “engine đã tách ô đích như `Map`/`List`”. Thực tế `ptr_rc` chỉ nhận `Map`/`List` (`crates/vis-interp/src/interp.rs:105-111`, `crates/vis-vm/src/vm.rs:28-34`), nên `tach_o`/`tach_dich` **không bao giờ tách `Value::Record`**; `Rc::make_mut` luôn thấy thêm bản trong ô biến và **chép sâu toàn bộ trường** mỗi lần `r.f = v` / `r with f: v`. **Không sai ngữ nghĩa** (CoW vẫn đúng; mọi ca alias xanh), nhưng lợi ích “in-place O(1)” chưa đạt và lý do tối ưu trong journal chưa có bằng chứng. | Đọc 3 vị trí trên; hoặc thêm `Value::Record` vào `ptr_rc` (cả hai engine) rồi đo lại record-heavy. Nếu không làm trong lát này thì sửa câu journal/STATE cho đúng. |
| P3 | Journal nói `record-cow.vis` phủ `with` lồng (`q.p with x: 5`) nhưng tệp không có ca đó. | Đã kiểm thủ công `manual2.vis` — PASS; thêm ca vào corpus hoặc sửa câu journal. |
| P3 | `EnumArgs::is_empty()` chỉ đúng với `Zero`, trả `false` cho `Many(vec![])` dù `len() == 0`. Hiện **không** caller nội bộ nào tạo `Many` rỗng (`from_vec` luôn chọn `Zero`), nhưng `EnumArgs` là API công khai. | Đổi thành `self.len() == 0` để nhất quán; không ảnh hưởng hành vi hiện tại. |
| P3 | `ApplyTry` (interp, `frame/proc_expr.rs:787-807`) siết `Some`/`Ok` thành **đúng 1 đối số**; bản cũ nhận mọi arity (`unwrap_or(Unit)`). Với giá trị `Option` dị dạng do host API dựng, hành vi `?` đổi từ “lấy Unit” sang “trả sớm như None/Err”. | Thử `Some()`/`Some(1, 2)` → tyck chặn `VS0401` (`Some` cần 1 đối số), nên **không tới được từ mã ViScript**; chỉ là khác biệt cho host/API nội bộ. Giữ nguyên cũng được, nhưng nên ghi chú. |

Không tìm thấy: double-free/alias sai, singleton bị mutate, `Rc` cycle, pattern match thiếu làm sai
ngữ nghĩa, `Debug` vỡ snapshot, hay hồi quy `Enum` bất thường. `enum` thêm 1 cấp phát `Rc` cho
`Many` là caveat đã ghi trong journal; bench không hồi quy.

## 8. Kết luận

**PASS** — đủ điều kiện hợp nhất sau khi xử lý xung đột văn bản `agent/STATE.md` (giữ cả khối
`ST-317a`, `ST-317b` phần 1, `ST-317c`). P2 là vấn đề hiệu năng/tài liệu, không chặn hợp nhất;
3 P3 là ghi chú/hardening. Không sửa mã sản phẩm trong review này.

## 9. Addendum F1 (`2e4090e2`) — **FAIL (P1 ngân sách interp↔VM)**

**Phạm vi**: `git log --oneline ec4d7fc6..2e4090e2` → `2e4090e2`; `git diff --stat` → **9 tệp,
+336/−41**. Cây sạch; không `unsafe` mới; `git diff ec4d7fc6..2e4090e2 -- crates/vis-codegen
agent/API-VBC1.txt` **rỗng**; R8 vẫn **39/40**; không tệp > 1.500 dòng.

### 9.1 Phần vá đúng

- `vis_runtime::value_ops::ptr_rc` (`value_ops.rs:39`) nhận thêm `Value::Record(r)`; `value_ops::tach_neu_cung_rc`
  (`:54`) so con trỏ và đặt ô = `Unit`. `Interp::tach_o` (`interp.rs:337`) dùng helper; `Th::tach_dich`
  (`vm.rs:585`) dùng helper cho ô **local**; VM `Instr::With` giữ nguyên.
- Fast path interp `r = r with f: v` (`stmt.rs:22`, gọi tại `:173-185`): đánh giá base + từng
  field (đúng thứ tự), kiểm đủ trường, rồi `remove` ô đích và áp `value_ops::voi`. Alias còn sống
  ⇒ `Rc::make_mut` CoW; alias giữ bản cũ. Conformance `record-with-gan.vis` (interp + `--vm`) và
  2 ca pointer interp + 1 ca pointer VM **PASS**, đọc test xác nhận đúng: không chia sẻ ⇒ cùng
  `Rc::as_ptr`; có alias ⇒ pointer mới, alias pointer cũ.
- `recordbench` (cùng binary `a6d55492…`): chạy lại `record-fieldbench`/`record-withbench`
  (256 trường × 200k vòng) cho stdout **199999**, wall **0,05–0,06 s** cho cả interp và `--vm`
  (log Builder 54–62 ms) ⇒ số đo **không sai**; nền 1,9 s là thật.

### 9.2 Cổng F1 (độc lập)

| Cổng | Kết quả | Log / sha256 (sau khi ghi log) |
|---|---|---|
| `./vis-dev gate --fast` | **3885/3885 passed, 4 skipped**, `vis-dev gate --fast: PASS`, exit 0 | `/home/ps/vislogs/review-st317a-f1-gate.log` — `64e295ad0fa62ce3e29ac400d83fa2a34f058e7d9294835d90bb7d6f1b662a29` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `tong=100000 … chay=93768 **lech=0**`, 60,0 s, exit 0 | `/home/ps/vislogs/review-st317a-f1-visdiff.log` — `5215f176cc3ade81551ae0cf8ba2e20730eca2bc4745b1ce3c88c29b72e4e3f6` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit **0**, 0 warning | `/home/ps/vislogs/review-st317a-f1-clippy.log` — `d5f7068adfaebad02eb97a8cbfb34e7a3e99dc55f1ef8a7408c9e6cd127023cb` |
| Pointer tests | interp **2 passed**, VM **1 passed** | `/home/ps/vislogs/review-st317a-f1-pointer-tests.log` — `016724fe62ba36afcaeae026cbd09e40a25371129ac37b2ca565305c574914ab` |
| Conformance corpus | **568/568** ca xanh | `/home/ps/vislogs/review-st317a-f1-conformance.log` — `3d7aaade4e7496782e5c5efa22b95eb92bde568cf77f9cd6be32bbafdbf82ee5` |
| `git merge-tree --write-tree main HEAD` (`main` = `d4861fc7`) | **chỉ xung đột `agent/STATE.md`** (văn bản; 3 bản), exit 1 | — |

### 9.3 Finding **P1** — fast path bỏ charge ngân sách, lệch interp↔VM

`HExpr::With` bình thường đi qua `Interp::eval` (`interp/eval.rs:58-69`): tăng `steps`, gọi
`worker_tick`, và **charge `FeeKind::Expr`** (cùng node codegen charge tại `codegen.rs:448`).
Fast path `gan_with_tai_cho` chỉ gọi `eval(base)` + `eval` từng field, **không** charge cho node
`With` (và không tăng `steps`/`worker_tick`). Vì vậy mỗi vòng `r = r with f: i` interp rẻ hơn VM
**1 đơn vị**; `--max-budget` (VS0510) là hành vi người dùng thấy được.

Chương trình tái lập (200k/1000 vòng; bản A dùng fast path, bản D đổi đích `q` để **không** fast
path):
- A: `r = r with f: i`; D: `q = r with f: i`.
- Ngưỡng budget nhỏ nhất chạy được: **A interp 11018**, **A VM 12018**, **D interp 12022**,
  **D VM 12022** (lệch đúng 1000 = số vòng).
- Tại `--max-budget 11500`: A **interp exit 0, in 999**; A **VM exit 101**
  `VS0510 … (E_ASSIGN): cần 1, còn 0`; D **cả hai exit 101**.
  Tại `12017`: A interp exit 0, A VM exit 101 (`E_EXPR`); D cả hai exit 101.
Log: `/home/ps/vislogs/review-st317a-f1-budget-parity.log` —
`4af42dba324896ad1b10312fce2e4aa44467024440c77eb01e96e6b5b21a77b8`.
Đây là **regression mới của F1** (trước F1 cả A và D đều đi qua `eval(With)`), không phải nợ cũ.

**Đề xuất**: trong `gan_with_tai_cho`, trước khi đánh giá base (hoặc tại đúng điểm `eval(With)`
sẽ làm), cộng bookkeeping `self.steps += 1`, `worker_tick(wspan/value.span)` và
`self.charge(FeeKind::Expr, value.span())`; hoặc tách helper chung để fast path không bỏ sự kiện.
Sau đó thêm ca budget parity A/D (ngưỡng phải bằng nhau) và chạy lại `gate --fast` + `vis-diff`.

### 9.4 Ghi chú P3 (không chặn riêng, nhưng nên vá cùng F1)

- Fast path `self.env.borrow_mut().remove(&id)` **trước** `voi`/`ghi_o`; nếu `voi` lỗi bất ngờ
  hoặc `ghi_o` từ chối `let`/ô không hợp lệ, ô biến mất. Precheck trường làm `voi` gần như
  infallible và tyck chặn `let`, nhưng nên `tra_o`/khôi phục ô trong nhánh lỗi để phòng thủ.
- `Th::tach_dich` nhánh **global** vẫn tự so `ptr_rc`/đặt `None` thay vì gọi
  `tach_neu_cung_rc` (nhánh local đã gọi). Hành vi đúng nhưng "một nguồn" chưa trọn; nên dùng
  helper cho cả global.
- Test pointer VM gọi trực tiếp `tach_dich` + `voi`, **không** chạy `Instr::With`/`Instr::Set`
  end-to-end; conformance chỉ chứng minh output. Nên thêm ca chạy `Instr::With` thật (hoặc
  chấp nhận vì `record-with-gan` đã phủ output hai engine).

### 9.5 Kết luận Addendum

**FAIL** vì P1 ngân sách (interp↔VM khác nhau trên `--max-budget`), dù phần pointer/CoW, alias,
conformance 568/568, gate 3885, diff 0 và clippy đều đạt. Không sửa mã; chờ Builder vá
bookkeeping rồi audit lại F1.
