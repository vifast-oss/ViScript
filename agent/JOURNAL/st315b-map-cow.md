<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315b — `Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười (`VD-291` §3.2)

- **Nhánh**: `plan/st315b` (nền `987b98b8`), worktree `/home/ps/ViScript-w2`.
- **Máy**: `ps` (Linux). **Target riêng**: `/home/ps/ViScript-w2/target` — sau sự cố hạ tầng
  dùng chung `CARGO_TARGET_DIR` giữa nhiều worktree (artifact bị ghi đè), target được **xoá sạch
  và dựng lại từ đầu**; **mọi** số liệu dưới đây đo trên target riêng này, không dùng
  `/home/ps/ViScript/target`. Không dùng `mps`; không chạy Windows/macOS.
- `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`.

## 1. Thiết kế

### 1.1 `VisMap` (`crates/vis-core/src/value/map.rs`, mới)

```rust
pub struct VisMap {
    entries: Vec<(Value, Value)>,   // giữ thứ tự chèn
    index: Option<Box<MapIndex>>,   // chỉ mục băm, dựng khi len >= NGUONG_CHI_MUC
}
pub struct MapIndex {
    dau: HashMap<u64, usize, BuildHasherDefault<Fnv>>, // băm -> entry mới nhất
    sau: Vec<usize>,                                   // chuỗi bucket (usize::MAX = hết)
}
pub const NGUONG_CHI_MUC: usize = 8;
```

- `Value::Map(Rc<VisMap>)`: `Value::clone` chỉ tăng refcount; `size_of::<Value>() == 64`
  (biến thể lớn nhất vẫn là `Cap { String, String, bool }`; `Map` từ 24 B payload còn **8 B**).
  Khoá test `<= 64` nằm ở `crates/vis-core/tests/core_it.rs`.
- Ghi qua `Rc::make_mut` (CoW). `Clone for VisMap` **cố ý bỏ** `index` để `make_mut` không nhân
  bản `HashMap` của map lớn; bản sao dựng lại chỉ mục theo nhu cầu (khi đã `>= NGUONG`).
- `Deref<Target = [(Value, Value)]>` + `IntoIterator for &VisMap` giữ nguyên 45 điểm khớp mẫu
  chỉ đọc (`.iter()`, `.len()`, `[i]`, `for (k, v) in &m`) — không nhân bản luật ở nơi gọi.
- Chỉ mục dùng **cùng** `bam_khoa_vo_huong` với `eq_value` (`show_eq.rs`, nay `pub(super)`):
  `Int`/`Float`/`Percent` cùng miền số `f64`, `-0.0 == +0.0`, `Size`/`Duration`, `Str`/`Path`,
  `Bool`/`Char`/`Bytes` mỗi loại tag riêng. Khoá phức tạp (`List`/`Map`/`Record`/`Enum`/…)
  **luôn** rơi về quét tuyến tính nên chỉ mục không thể trả âm tính giả; bucket còn kiểm
  `eq_value` nên trùng băm không sai.
- **Ngưỡng 8**: map nhỏ (JSON-RPC LSP, `{}`/`{a:1}`) không cấp phát `HashMap`; đo `N=1,4,8,16`
  cho cùng thời gian (xem §4) nên 8 là điểm khởi động chỉ mục sớm mà không phí cho map nhỏ.
- **Tìm khoá trả entry CŨ NHẤT** trong bucket (chuỗi mới→cũ, ghi đè lần khớp cuối) — đúng
  `iter().find()` cũ. Cần thiết vì `eq_value` **không bắc cầu**: `Int(1)` và `Percent(1.0)` là
  hai entry (không `eq_value` nhau) nhưng cùng khớp `Float(1.0)`; `get`/`insert` phải chọn entry
  chèn trước. Ca khoá ở `map.rs::tests::get_float_khi_co_int_va_percent_tra_entry_cu_nhat`.
- `eq_value` map: hai vế đều có chỉ mục ⇒ `len` bằng + mỗi cặp `(k, v)` của vế A có **một cặp**
  `(k, v)` trong bucket B (`co_cap`, không chỉ entry đầu khớp khoá — đúng phép "any" cũ); còn lại
  dùng nguyên `eq_map_vo_huong`/`eq_map_cham`.

### 1.2 Gom một nguồn

- Dựng map: `Value::map(Vec<(Value, Value)>)`/`Value::map_tu(VisMap)` ⇒ `VisMap::from_pairs`
  (last-wins tại chỗ). Đã thay **mọi** constructor: literal interp (`eval.rs`,
  `frame/proc_expr.rs`), `Instr::MakeMap` (VM `exec.rs`), `json.decode` (bỏ vòng quét O(N²)),
  `json::boc`/`derive`, worker snapshot→`Value`, plugin convert, HTTP headers, LSP, pkg, test.
- Tra: `VisMap::get`/`contains_key`/`co_cap` dùng chỉ mục; đã chuyển `builtin/method.rs`
  (`map.get`/`has`), `stdlib.rs` (`map.get_or`, `map.merge`, `map.remove`, `map.keys/values`),
  `mapcontainskey`, index `m[k]` của interp (`frame.rs`, `gan.rs`, `interp/eval.rs`,
  `frame/proc_expr.rs`) và VM (`vm/vm_helpers.rs`).
- Gán: `gan.rs`/`vm_helpers.rs::set` dùng `Rc::make_mut(&mut rc).insert(k, v)`; gộp:
  `VisMap::merge`; xoá: `VisMap::remove`.
- `SendValue::Map(Vec<(SendValue, SendValue)>)` **giữ nguyên** (snapshot qua biên worker, không đổi
  ABI); chỉ chuyển sang `Value::map` khi dựng lại trong heap.

### 1.3 Ba phán quyết (giữ nguyên)

- **Q1**: khoá trùng theo `eq_value` (`1`/`1.0`, `0.5`/`50pct`) ⇒ `slot.1 = v` **tại chỗ**, giữ
  khoá gốc + vị trí chèn đầu. `NaN != NaN` nên mỗi lần chèn `NaN` vẫn là entry mới (như cũ).
- **Q2**: `json.decode` last-wins, **vị trí đầu / giá trị cuối**, một entry mỗi khoá; decoder chỉ
  đẩy cặp, `from_pairs` gộp O(1) khấu hao.
- **Q3**: `Show`/`keys()` theo thứ tự chèn; `remove` bỏ mục (giữ thứ tự còn lại), `insert` lại
  chèn **cuối** — byte-for-byte như `Vec` cũ.

## 2. Test mới

- `crates/vis-core/src/value/map.rs` (unit): thứ tự + last-wins tại chỗ, `Int`/`Float`/`Percent`,
  `NaN`, `remove`→`set` chèn cuối, ngưỡng chỉ mục + khoá phức tạp fallback, `Int`+`Percent` cùng
  khớp `Float` trả entry cũ nhất.
- `crates/vis-core/tests/core_it.rs`: `size_of::<Value>() <= 64` + CoW (`Rc::ptr_eq`, ghi
  `make_mut` không đụng bản gốc).
- `crates/vis-interp/tests/interp_it/st315b_map_cow.rs` (khai `#[path]` trong aggregator
  `interp_it.rs` — **R8: 0 binary cấp 1 mới**): Q1/Q2/Q3 qua `Value`, JSON trùng khoá, CoW,
  ngưỡng chỉ mục, `eq_value` đường chỉ mục (kể cả đảo thứ tự entry).
- Không sửa `docs/SPEC*.md`, `agent/DECISIONS.md`, `agent/ACTIVE.md`, `agent/QUEUE.md`,
  `agent/STATE.md`.

## 3. Cổng (target riêng, dựng lại từ đầu)

| Cổng | Kết quả | Ghi chú |
|---|---|---|
| `cargo fmt --all` | PASS | |
| `./vis-dev gate --fast` | **PASS** | `check` PASS (fmt, clippy, R1..R9, DAG 23 crate/5 tầng, self-test, doc-examples, readme-examples) + L0 **3836 passed / 0 failed / 4 skipped** (`/tmp/opencode/st315b-gate-private.log`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0) | `/tmp/opencode/st315b-clippy-private.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** | 100 000 chương trình (`93768` chạy), 105,7 s (`st315b-visdiff-final.log`) |
| `cargo test -p vis-vm --test vm_it stdlib_diff` | **0 lệch** | 73 hàm thuần / 400 chương trình; sugar 152/200, 0 lệch |
| R8 / R1 | PASS | 0 tệp `.rs` test cấp 1 mới; `map.rs` 413 dòng |

## 4. Đo release (`target/release/vis`, sha256 `32284f9cbde90478364871eb5a97e5b598c17bfa7f929482ec2c928a469d661b`)

Máy `ps` có tải song song từ luồng khác, nên đo **A/B xen kẽ** với binary nền `987b98b8`
(cùng máy, cùng thời điểm; nền dựng ở worktree tạm, target riêng, đã xoá sau đo):

`mapbench` (100 000 lần `m["k{j%N}"]`; ms, 6 cặp AB, lấy trung vị):

| N | nền `987b98b8` | `ST-315b` | tỉ lệ |
|---:|---:|---:|---:|
| 1 | 44,5 | **42** | 1,06× (không hồi quy) |
| 8 | 58,5 | **43,5** | 1,34× |
| 79 | 341 | **43,5** | 7,8× |
| 316 | 1328 | **49,5** | **26,8×** |

- KPI `N=316 ≥ 10× nền 1.429 ms`: **PASS** (49,5 ms ⇒ ~28,8× so với 1.429 ms; phẳng so với N=8).
- KPI `N=1` không hồi quy > 5 %: **PASS** (42 ms vs 44,5 ms — nhanh hơn).
- `N=4`, `N=16` (5 mẫu): 42–45 ms interp, 44–46 ms VM — không có bậc thang ở ngưỡng 8/16.

`opt-bench` (10 cặp AB xen kẽ; ms):

| Engine | nền `987b98b8` | `ST-315b` | KPI |
|---|---:|---:|---:|
| interp | 368 | **48** (47–65) | ≤ 120: **PASS** |
| VM | 382 | **46** (43–61) | ≤ 120: **PASS** |

Đầu ra `opt-bench` = `38995450` ở cả hai engine; `mapbench` khớp tổng lý thuyết
(N=316: `15737616`).

## 5. Caveat & việc còn lại

- **Hạ tầng**: không dùng chung target với worktree khác nữa (artifact bị ghi đè giữa các
  worktree cùng tên/phiên bản crate). Mọi số ở trên là target riêng, dựng lại 100 % từ mã hiện tại.
- `Value::map` gộp last-wins **khi dựng**: mọi constructor cũ đã last-wins nên không đổi ngữ nghĩa;
  nhưng nếu sau này ai dựng map có chủ đích giữ entry trùng (không phải ánh xạ) thì phải dùng
  kiểu khác — `VisMap` là **ánh xạ**, bất biến một khoá một entry.
- `VisMap::get`/`insert` với khoá **phức tạp** vẫn quét tuyến tính (chỉ mục chỉ phủ khoá vô
  hướng); map lớn toàn khoá list/record sẽ không hưởng O(1) — chấp nhận, đúng hợp đồng băm hiện có.
- Các điểm tra map ngoài lõi (`vis-lsp::protocol::jsonrpc::lay`, `vis-pkg` parse `Value::Map`
  slice) vẫn quét tuyến tính trên slice; map JSON-RPC nhỏ (< 8 khoá) nên không cần chỉ mục, và
  không nằm trong danh sách "một nguồn" của lát này.
- `SendValue::Map` vẫn là `Vec` snapshot — **không** đổi ABI worker; `ST-316a` (CoW cho
  `Value::List`) có thể dùng lại khuôn `VisMap`/`MapIndex`.
- Không còn hạng mục KPI nào treo cho `ST-315b`.

## 6. Đo độc lập sau merge (main `b61a03f7`, máy êm load 1,66, n=15)

Binary `target/release/vis` sha256 `dff3d6295e69fc8447eed3c644c532c0792c514718c6b2a9fce1095e93526b22`
(`# time:` chỉ runtime, 1 lần warm cache; log `/home/ps/vislogs/st315b-main-bench-quiet.txt`):

| Tổ hợp | median | p90 | min |
|---|---:|---:|---:|
| `opt-bench` interp | **45 ms** | 49 | 41 |
| `opt-bench` VM | **42 ms** | 48 | 40 |
| `mapbench` N=1 interp / VM | 43 / 45 ms | 46 / 48 | 37 / 41 |
| `mapbench` N=8 interp / VM | 44 / 46 ms | 50 / 50 | 39 / 44 |
| `mapbench` N=79 interp / VM | 46 / 47 ms | 52 / 50 | 43 / 44 |
| `mapbench` N=316 interp / VM | **48 / 50 ms** | 52 / 56 | 46 / 47 |

KPI: opt-bench ≤ 120 ms **PASS**; N=316 ≥ 10× nền 1.328 ms (≈27×) **PASS**; N=1 không hồi quy
so nền 44,5 ms (43 ms) **PASS**; phẳng N=8→316. Số này khớp bảng A/B của W2 (sai lệch ≤ 3 ms).

## 7. Hậu-kiểm R-B (`review/r315b`, `04163a7c`) — PASS, finding thấp

- R-B tái lập độc lập: differential/fuzz `VisMap` ↔ `Vec` cũ **0 lệch** (200 chuỗi × 400 thao tác,
  2.000 merge, 20.000 `eq_value`, NaN/±0.0/Int/Float/Percent/list/record/enum); gate **3.852/0**,
  clippy PASS, `vis-diff 100k lech=0`; benchmark target riêng: opt-bench **49/45 ms**, N=316
  **47 ms** (A/B **26,9×**, KPI ~30,4× nền 1.429 ms); `size_of::<Value>()=64`, `VisMap=32`.
- **F1 (chấp nhận, cần A0 ghi CHANGELOG/wontfix)**: `Value::map` gộp last-wins nên 3 điểm dựng
  header HTTP (`vis-http/async_http.rs`, `fs_net/dispatch.rs`) **mất header trùng tên**
  (`Set-Cookie`…); trước giữ đủ cặp, `get` trả first. Đây là thay đổi hành vi API; chưa sửa vì
  `map[str,str]` không biểu diễn được nhiều giá trị/khoá — cần quyết định A0 (đổi kiểu/CHANGELOG).
- **F3 (đã vá + test)**: `VisMap::insert` dựng lại index trước khi cập nhật khoá **đã có** trên bản
  clone (trước đó `b=a; b["k"]=v` để index rỗng ⇒ `get` quét N). Test CoW dùng 64 khoá phân biệt
  và assert `co_chi_muc()` sau ghi.
- **F5 (đã siết)**: `core_it` assert `size_of::<Value>() == 64` trên 64-bit (vẫn ≤ 64 nền tảng khác).
- F2/F4/F6 (bộ nhớ index cho khoá phức tạp; JSON đệm cặp trùng; +48 B/map rỗng) ghi nhận **không chặn**.
