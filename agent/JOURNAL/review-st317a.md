<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Review ST-317a — Gatekeeper (Agent 1) độc lập

**Worktree**: `/home/ps/viswt/st317a` · nhánh `plan/st317a` · HEAD `87985b7e` (mã `fcecb5e6`) · nền `882b7ffa`.
**Kết luận**: **PASS** — 0 finding P1; 1 finding P2 (hiệu năng/tài liệu, **không sai ngữ nghĩa**); 3 ghi chú P3.
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
