<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315b — Kiểm toán độc lập (Gatekeeper/Agent 1), 2026-11-26

- **Verdict: PASS** — hợp đồng `VD-291` §3.2 (Q1–Q3, CoW `Value::Map(Rc<VisMap>)`,
  chỉ mục băm lười, `size_of::<Value>() == 64`, `SendValue` ABI) đạt; 6 finding ban đầu;
  **F3 đã vá, F5 phần lớn đã vá trên `main` @ `79f1fcee`** (fix `085546d0`) — recheck §11;
  còn F1/F2/F4/F6 không chặn.
- **Nhánh kiểm**: `review/r315b`; audit gốc @ `8ca08441` (nền `2fc47130`), recheck sau
  `git merge main` @ `79f1fcee`; lát gốc `7dc21cf2`
  (`[ST-315b] core: Value::Map(Rc<VisMap>) CoW + chi muc bam luoi`).
- **Worktree**: `/home/ps/ViScript-r315b`; **target riêng** `/home/ps/ViScript-r315b/target`;
  `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`; không dùng `mps`; không sửa mã nguồn;
  audit gốc không push/merge; recheck chỉ `git merge main` fast-forward theo yêu cầu.
  Chỉ tạo/cập nhật tệp này.
- **Diff kiểm**: `git diff 2fc47130..8ca08441` (49 tệp) + đọc trực tiếp `map.rs`,
  `show_eq.rs`, `value/mod.rs`, `json/{mod,derive}.rs`, `runtime`, `interp`, `vm`, `lsp`,
  `pkg`, `plugin`, `http`, `fsmock`, test `st315b_map_cow.rs`, `core_it.rs`, journal gốc.

## 1. Kết luận nhanh

| Hạng mục | Kết quả | Bằng chứng |
|---|---|---|
| Q1 — khoá trùng `eq_value`: cập nhật tại chỗ, giữ khoá gốc + vị trí đầu | **ĐẠT** | `map.rs:257-268`; test `giu_thu_tu_va_last_wins_tai_cho`, `khoa_trung_eq_value_int_float_percent`, `get_float_khi_co_int_va_percent_tra_entry_cu_nhat`; fuzz ngoài repo |
| Q2 — `json.decode` trùng khoá: vị trí đầu, giá trị cuối | **ĐẠT** | `json/mod.rs:112-122`; test `q2_json_trung_khoa_vi_tri_dau_gia_tri_cuoi` |
| Q3 — `remove` rồi `set` chèn cuối, `Show`/`keys` byte-for-byte | **ĐẠT** | `map.rs:270-277`; test `remove_roi_set_lai_chen_cuoi`, `q3_remove_roi_set_lai_chen_cuoi` |
| `Value::Map(Rc<VisMap>)` CoW; `Value::clone` không chép sâu map | **ĐẠT** | `Rc::make_mut` tại `gan.rs:51-56`, `vm_helpers.rs:116-124`; grep toàn cây chỉ 2 điểm ghi production; test `value_kich_thuoc_va_map_cow`, `co_chep_khi_chia_se_va_ghi_tai_cho_khi_doc` |
| Chỉ mục băm lười ngưỡng 8; insert/remove/clone đồng bộ; bucket trả entry cũ nhất | **ĐẠT** | `map.rs:45-103`, `map.rs:222-253`; fuzz 200 chuỗi × 400 thao tác + 2 000 merge + 20 000 eq, 0 lệch |
| `Value::map`/`map_tu` là nguồn dựng duy nhất; không còn `Rc::new(VisMap...)` production | **ĐẠT** | `grep`: 0 `Value::Map(vec` production; 0 `Rc::new(VisMap` ngoài `value/mod.rs`, `map.rs`, test |
| `SendValue::Map(Vec<...>)` giữ nguyên ABI | **ĐẠT** | `worker/proto.rs:51`; diff chỉ đổi `Value::Map(...)` → `Value::map(...)` khi dựng lại |
| `size_of::<Value>() == 64` | **ĐẠT** | test `<= 64`; đo độc lập **64 B**, `VisMap` 32 B, `MapIndex` 56 B |
| KPI release `opt-bench` ≤ 120 ms cả hai engine | **ĐẠT** | interp 49 ms, VM 45 ms (15 mẫu) |
| KPI `mapbench N=316` ≥ 10× nền 1,429 ms | **ĐẠT** | 47 ms ⇒ 30,4× nền 1,429 s; A/B 26,9× |
| KPI `N=1` không hồi quy > 5 % (nền 44,5 ms) | **ĐẠT** | 42 ms; A/B nền 47 → 43 ms (nhanh hơn) |
| Cổng bắt buộc | **ĐẠT** | gate --fast PASS 3 852 test/66 binary + doc-test; clippy 0 warning; vis-diff 100k lệch 0 |

## 2. Phương pháp

1. Đọc toàn bộ `crates/vis-core/src/value/map.rs` (413 dòng) và diff `show_eq.rs`; kiểm từng
   bất biến bằng lập luận + ca biên (Int/Float/Percent không bắc cầu, NaN, ±0.0, Unit, list,
   record, enum, map lồng).
2. Kiểm mọi điểm ghi `Value::Map` trên toàn workspace: chỉ `gan.rs` và `vm_helpers.rs` cần
   `Rc::make_mut`; `Rc` không có `DerefMut` nên mọi chỗ khác phải dựng map mới — biên dịch là
   chốt.
3. **Harness fuzz ngoài repo** (`/tmp/opencode/r315b/fuzz_map.rs`, rustc 1.100.0, link rlib
   `vis-core` hiện tại), không sửa repo:
   - 200 chuỗi × 400 thao tác `insert`/`get`/`remove`/`co_cap` + clone/write, so từng entry,
     thứ tự, `len`, `contains_key`, `get`, `co_cap` với `Vec<(Value,Value)>` tham chiếu (đúng
     `find`/`Vec::remove` cũ), gồm cả `from_pairs` và `merge`;
   - 20 000 ca `eq_value` map (8–27 khoá, trùng/không trùng, NaN, ±0.0, Int/Float/Percent,
     Unit, list, record, enum) so với **logic cũ** `gop` + so một chiều;
   - 2 000 ca eq một vế **không có chỉ mục** (`VisMap::clone` bỏ index) so với vế có index;
   - 2 000 ca `VisMap::merge` so với chèn tuần tự.
   Kết quả cuối: **0 lệch**, **0 bất đối xứng mới** (xem §7 về bất đối xứng cũ).
4. Chạy chương trình `.vis` thật trên interp + `--vm` cho CoW map lớn (16 khoá), map lồng,
   map trong list, map qua hàm, `map.remove` + `set`, `map.merge`, `map.get_or`,
   `map.contains_key`; đối chiếu thêm binary nền `987b98b8` khi hữu ích.
5. Chạy lại toàn bộ cổng + đo release trên target riêng.

## 3. CoW, ABI, kích thước

- `Value::Map(Rc<VisMap>)`: `Value` clone chỉ tăng refcount. Test `Rc::ptr_eq` + ghi
  `make_mut` không đụng bản gốc đạt.
- Ghi production: `crates/vis-interp/src/gan.rs:51-56` và `crates/vis-vm/src/vm/vm_helpers.rs:116-124`
  dùng `Rc::make_mut(&mut kv).insert(k, v)`. Không có `Rc::get_mut`/`DerefMut`/ghi trực tiếp
  nào khác trên map (grep + biên dịch).
- `VisMap::clone` **bỏ** `index` (`map.rs:120-128`); `Rc::make_mut` chỉ gọi clone khi refcount
  > 1, nên map chia sẻ không nhân bản `HashMap`; đúng chủ ý.
- `VisMap` dùng `Deref<Target=[(Value,Value)]>` cho 45 điểm đọc; `IntoIterator for &VisMap`
  giữ `for (k,v) in &m`; không còn chỗ nào giả định payload là `Vec` (biên dịch workspace +
  clippy all-targets).
- `SendValue::Map(Vec<(SendValue, SendValue)>)` giữ nguyên; `ToSend` đi qua `kv.iter()`; chỉ
  `SendValue::to_value` dựng `Value::map` ở biên. Test worker 1 000 khoá đạt.
- Đo độc lập: `size_of::<Value>() = 64`, `VisMap = 32`, `MapIndex = 56`. Test `core_it` chỉ
  khẳng định `<= 64` (F5).

## 4. Chỉ mục băm — kiểm chi tiết

- `MapIndex::dung` chỉ dựng khi `entries.len() >= 8` (`map.rs:53-65`); map nhỏ không cấp
  `HashMap` (nhưng vẫn có `Rc`, xem F2/F4).
- `them`: chèn đầu bucket; `sau[i]` = head cũ hoặc `usize::MAX` (`map.rs:68-71`). Invariant
  `sau.len() == entries.len()` được giữ bởi: dựng từ đầu, `insert` mới (push cả hai),
  `remove` dựng lại toàn bộ; `insert` cập nhật giá trị không đổi độ dài.
- `tim`: duyệt mới→cũ, ghi đè mỗi lần khớp ⇒ trả entry **cũ nhất**; đúng `iter().find()` cũ
  (`map.rs:76-86`). Ca `Int(1)` + `Percent(1.0)` (không `eq_value` nhau) cùng khớp
  `Float(1.0)`: `get(Float)` trả entry chèn trước; `insert(Float)` sửa entry cũ nhất — test
  + fuzz.
- `co_cap`: trả true nếu **một cặp** bất kỳ khớp cả khoá lẫn giá trị; đúng phép "any" cũ,
  không chỉ entry đầu khớp khoá (`map.rs:90-102`).
- Hợp đồng băm: `bam_khoa_vo_huong` (`show_eq.rs:214-227`) phủ đúng các kiểu mà `eq_value`
  so được; `Int`/`Float`/`Percent` cùng miền `f64`, `±0.0` chuẩn hoá, `Size`/`Duration`,
  `Str`/`Path`, `Bool`/`Char`/`Bytes` tag riêng. Đã đối chiếu từng arm `eq_value` với hàm
  băm: mọi cặp `eq_value` đều cùng băm; khoá phức tạp/Unit/NaN đều fallback quét tuyến tính
  (NaN không khớp chính nó — giữ hành vi cũ).
- Clone/mutate: `VisMap::clone` bỏ index; `Value::clone` chia sẻ nguyên `VisMap` (giữ index);
  `Rc::make_mut` chỉ clone khi cần. Không có đường nào sửa `entries` mà không cập nhật/dựng
  lại index.
- `Rc::try_unwrap` dùng đúng ở `iter.rs:102-106`, `json/mod.rs:316-322`, `plugin convert.rs:27-33`:
  `Ok` lấy entry, `Err` clone entry; không giả định sai quyền sở hữu.
- Fuzz xác nhận index không stale sau insert/remove/clone/merge/from_pairs với cả khoá vô
  hướng lẫn phức tạp.

## 5. Ngữ nghĩa — kiểm bằng chương trình thật

| Ca | interp | VM | Nền `987b98b8` (khi so) |
|---|---|---|---|
| CoW 16 khoá: `b=a; b["k3"]=99; b["moi"]=7` | a giữ 16 entry, b 17, `a["k3"]=3` | giống | giống |
| Map lồng `b["m"]["x"]=2` | a `{"m":{"x":1}}`, b `{"m":{"x":2}}` | giống | giống |
| Map trong list `ys[0]["a"]=2` | xs giữ 1, ys 2 | giống | giống |
| Map qua hàm (tham số + `var x=m; x[...]`) | caller không đổi | giống | giống |
| `map.remove` rồi `p["b"]=9` | `{a,c,b}` (b cuối), `keys=[a,c,b]` | giống | giống |
| `map.merge` / `get_or` / `contains_key` | khớp thứ tự + khoá gốc | giống | giống |
| `json.decode` trùng khoá (test Rust) | vị trí đầu/giá trị cuối | dùng chung core | — |
| Map 8–316 khoá `m["k{i}"]` | tổng lý thuyết khớp | — | — |

Không phát hiện chỗ nào còn giả định `Value::Map` là `Vec` (grep `Value::Map(vec`, `Map(vec!`,
`Rc::new(VisMap` production = 0; clippy all-targets biên dịch toàn bộ test/example).

## 6. Cổng (chạy lại trên target riêng)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-core` | **PASS** — 26 unit + 4 `core_it`, 0 fail; gồm `value_kich_thuoc_va_map_cow`, toàn bộ test `value::map` |
| `cargo test -p vis-interp --test interp_it` | **PASS** — 55/55 (sau khi build `target/debug/vis`; lần đầu đỏ vì thiếu binary `lat0_suspend`, không phải lỗi mã) |
| `cargo test -p vis-vm --test vm_it stdlib_diff` | **PASS** — 2/2, 400 chương trình, 0 lệch |
| `./vis-dev gate --fast` | **PASS** — `check` PASS (fmt, clippy, R1..R9, DAG 23 crate/5 tầng, self-test, doc-examples, release), L0 **3 852 test / 66 binary / 0 fail / 4 skip** + doc-test |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0, không cảnh báo) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** — `tong=100000`, `loi_tinh=6232`, `chay=93768`, 111,5 s |
| `cargo fmt` (trong gate) | **PASS** |
| `size_of::<Value>()` (đo độc lập) | **64 B** |

Ghi chú: số L0 3 852 (không phải 3 836 trong journal gốc) vì nền `8ca08441` đã gồm test
`ST-315c`/`ST-315d`; không phải hồi quy.

## 7. Đo release (target riêng, `--release`)

Binary mới (`8ca08441`): `sha256 dff3d6295e69fc8447eed3c644c532c0792c514718c6b2a9fce1095e93526b22`
(`vis 0.6.0`). Binary nền `987b98b8` dựng độc lập trong `/tmp/opencode/r315b-base-target`:
`sha256 5eb2973aed13f13be0fb361a18c61eea4c64535b92f16465fa436374e1833ceb`.
(Hash khác `32284f9...` trong journal gốc vì journal đo trên cây `7dc21cf2`; cây review
`8ca08441` gồm thêm `ST-315c`/`ST-315d` — mã `ST-315b` không đổi.)

### 7.1 `opt-bench` — 15 mẫu mỗi engine (load avg ~1,57 lúc đo)

| Engine | Nền `987b98b8` | ST-315b | KPI ≤ 120 ms |
|---|---:|---:|---|
| interp | 341 ms | **49 ms** (48–58) | **PASS** |
| VM | 351 ms | **45 ms** (44–48) | **PASS** |

Đầu ra `38995450` khớp ở cả hai engine.

### 7.2 `mapbench` (100 000 lần `m["k{i%N}"]`, 15 mẫu)

| N | Nền `987b98b8` | ST-315b | Tỉ lệ |
|---:|---:|---:|---:|
| 1 | 47 ms (media A/B) | **42 ms** (40–49) | 1,09× |
| 8 | — | **44 ms** | — |
| 79 | — | **44 ms** | — |
| 316 | 1 266 ms (A/B) | **47 ms** (46–51) | **26,9×** |

- KPI `N=316 ≥ 10×` nền 1 429 ms: **PASS** — 47 ms ⇒ **30,4×**; A/B với nền tự dựng
  1 266 ms ⇒ 26,9×. Đo lúc máy tải cao hơn (load ~4–8): nền 1 439 ms / mới 61 ms ⇒ 23,6×,
  vẫn PASS.
- KPI `N=1` không hồi quy > 5 %: **PASS** — 42 ms so 44,5 ms (W2) và 43 ms so 47 ms (A/B);
  lúc tải cao 56 ms so 59 ms (A/B), vẫn không hồi quy.
- `N=8`/`N=79` phẳng so `N=1` (44/44 ms), không có bậc thang ở ngưỡng 8.
- Đầu ra lý thuyết khớp: N=1 `0`, N=8 `350000`, N=79 `3899545`, N=316 `15737616`.

## 8. Finding

### F1 — Map header HTTP làm rơi header trùng (hành vi API) — **Thấp**

- **Vị trí**: `crates/vis-http/src/async_http.rs:422` (`ket_qua_head_meta`),
  `crates/vis-http/src/async_http.rs:530` (`ket_qua_head`),
  `crates/vis-runtime/src/stdlib/fs_net/dispatch.rs:340` (đường blocking `http.head`).
- **Trước**: `Value::Map(headers.iter().map(...).collect())` — `Vec` giữ **mọi** cặp, kể cả
  trùng tên (`Set-Cookie`, `Warning`, `Link`…). `Show`/`keys`/JSON encode thấy đủ; `get`
  trả bản ghi **đầu**.
- **Sau**: `Value::map(...)` → `VisMap::from_pairs` last-wins ⇒ mỗi tên còn **một** entry
  (giá trị cuối, vị trí đầu). `res.headers.get("set-cookie")` đổi từ first → last; vòng
  `for (k, v) in res.headers` mất các cookie trùng. Cookie jar **không** đổi (đọc `Vec` trực
  tiếp ở `vis-http/src/lib.rs:547-555`, `async_http.rs:624-628`); chỉ map trả cho script đổi.
- **Mức**: **Thấp** — map `map[str,{error}]` vốn không biểu diễn được đa trị nên đây có thể
  là sửa hợp lý, nhưng là **thay đổi hành vi quan sát được, không có test**. Journal gốc §5
  viết "mọi constructor cũ đã last-wins" là **không đúng** với 3 điểm này.
- **Khuyến nghị**: ghi rõ trong CHANGELOG/journal (hoặc giữ `list[(str,str)]` cho header nếu
  muốn đa trị); thêm test server gửi 2 `Set-Cookie` để khoá hành vi.

### F2 — `MapIndex` cấp `dau` theo `entries.len()` cả khi toàn khoá phức tạp — **Thấp (bộ nhớ)**

- **Vị trí**: `crates/vis-core/src/value/map.rs:53-65` (`HashMap::with_capacity(entries.len())`),
  `:68-71` (khoá phức tạp chỉ đẩy `usize::MAX` vào `sau`).
- **Vấn đề**: map ≥ 8 entry toàn khoá `List`/`Map`/`Record`/`Enum` vẫn dựng `dau` rỗng với
  sức chứa N (và `sau` 8 B/entry), trong khi mọi lookup vẫn quét tuyến tính. Map 100 000 khoá
  list: `co_chi_muc() == true`, `dau` không có bucket nào nhưng đã cấp phát; lãng phí cỡ vài
  trăm KB–vài MB tuỳ allocator.
- **Mức**: **Thấp** (chỉ bộ nhớ, không sai ngữ nghĩa); nên dựng index khi gặp khoá vô hướng
  đầu tiên hoặc chỉ cấp `dau` theo số khoá vô hướng.

### F3 — Index rơi sau CoW: ghi đè khoá cũ không dựng lại; ghi khoá mới dựng lại O(N) — **Thấp (hiệu năng) — đã vá @79f1fcee, recheck §11**

- **Vị trí**: `crates/vis-core/src/value/map.rs:120-128` (`Clone` bỏ index),
  `:257-268` (`insert` chỉ dựng index ở nhánh **khoá mới**; nhánh cập nhật giá trị trả sớm),
  `:275` (`remove` dựng lại toàn bộ).
- **Đã vá (`085546d0`)**: `map.rs:257-262` dựng lại index trước `vi_tri` khi
  `index.is_none() && len >= NGUONG_CHI_MUC`; test F3/F5 chạm đúng đường này (§11).
- **Hệ quả 1 (đọc) — trước vá**: `let b = a` (Rc chia sẻ) rồi `b["k"] = v` với `k` **đã có**:
  `doc_o` clone → `Rc::make_mut` gọi `VisMap::clone` (index None) → `insert` tìm tuyến tính,
  cập nhật, **không** dựng index. Mọi `b.get(...)` sau đó quét N cho tới khi có insert khoá
  mới/remove. Không hồi quy so nền cũ (nền cũ luôn tuyến tính), nhưng mất lợi ích index mới.
- **Hệ quả 2 (ghi)**: mỗi lần `Rc::make_mut` + chèn khoá mới dựng lại `HashMap` O(N); đo
  `writebench` (khởi tạo + 100 lần ghi) N=10 000: nền 2 869 ms → mới 3 285 ms (**+14,5 %**);
  N=5 000 × 200: 756 → 877 ms (**+16 %**). Phần lớn là chi phí copy CoW sẵn có; index làm
  tăng hằng số. Không nằm trong KPI đọc.
- **Mức**: **Thấp** (không sai; ngoài KPI 4.1/4.2). Sau vá §11: đường đọc sau cập nhật khoá
  cũ đã O(1); chi phí ghi khoá **mới** sau CoW vẫn còn (3 164→3 508 ms N=10k; 800→917 ms
  N=5k), chấp nhận ngoài KPI. Nếu muốn tối ưu thêm: giữ index qua `Clone`/dựng lười khi đọc
  (cần `&mut`/`RefCell` — cân nhắc).

### F4 — `json.decode` đệm **mọi** cặp trùng trước khi gộp — **Thấp (bộ nhớ, đầu vào đối kháng)**

- **Vị trí**: `crates/vis-core/src/json/mod.rs:112-114` (`kv.push` mọi cặp) rồi
  `Value::map(kv)` ở `:97`/`:122`.
- **Trước**: decoder gộp last-wins **ngay trong `kv`** (chỉ giữ unique key); **sau**: giữ
  toàn bộ cặp đến cuối object. Object 1 triệu khoá trùng `{"a":0,"a":0,...}`: kết quả vẫn 1
  entry nhưng đỉnh bộ nhớ chứa **1 triệu cặp** (tỉ lệ số cặp, kể cả trùng) thay vì 1; giá
  trị lồng lớn làm khuếch đại thêm. Input JSON lớn giới hạn thực tế (HTTP 8 MiB) nên **Thấp**, nhưng là
  hồi quy bộ nhớ so logic cũ với dữ liệu trùng nhiều.
- **Khuyến nghị**: gộp ngay bằng `VisMap` tạm trong decoder, hoặc ghi nhận trade-off trong
  journal (đổi O(N²) thời gian lấy O(N) bộ nhớ đỉnh).

### F5 — Khoảng trống test + claim journal — **Thông tin — phần lớn đã vá @79f1fcee, recheck §11**

- **Đã vá (`085546d0`)**: `st315b_map_cow.rs:94-123` dùng 64 khoá **phân biệt**, assert
  `truoc == 64`, `co_chi_muc()` sau khi sửa khoá **đã có** `k0`, đọc lại `k0`/`k63`;
  `core_it.rs:70-75` assert `size_of::<Value>() == 64` trên 64-bit. Cả hai pass (§11).
- Còn: `map.rs:359` ca Int/Percent không bắc cầu chỉ 2 khoá (chưa có bản ≥ 8); chưa có
  test non-transitive ở ≥ 8 entry. Các điểm `remove`/`merge` dưới đây vẫn nguyên.
- `VisMap::remove`/`VisMap::merge` **không** được `std.map.remove`/`std.map.merge` gọi
  (`stdlib.rs:619-626` lọc + `Value::map`; `:600-609` `from_pairs` + `insert`), nên đường
  dựng lại index của `remove`/prebuild của `merge` chỉ được harness ngoài kiểm; journal §1.2
  ghi "xoá/gộp qua `remove`/`merge`" là chưa khớp mã.
- Không có test header trùng (F1).

### F6 — Mỗi map cấp thêm `Rc` + `VisMap` trên heap (kể cả map rỗng) — **Thông tin (bộ nhớ)**

- **Vị trí**: `crates/vis-core/src/value/mod.rs:409-410` (`Value::map` → `Rc::new(VisMap)`),
  `value/map.rs:109-112` (`VisMap` 32 B), `Rc` header 2 × `usize` = 16 B ⇒ **~48 B heap/map**
  (chưa kể overhead allocator), kể cả `{}`.
- **Trước**: `Value::Map(Vec)` nằm trong 64 B của `Value`; map rỗng **0 heap**, map nhỏ chỉ
  một cấp `Vec` entry.
- **Hệ quả**: 100 000 map nhỏ cùng tồn tại +~4,8 MB; đổi lại `Value::clone` map rẻ. Đo CPU
  không hồi quy: vòng tạo map rỗng 100k lần 39 ms (nền 40 ms); map 3 khoá trong vòng 100k
  106 ms (nền 125 ms — nhanh hơn nhờ clone Rc). Đây là trade-off có chủ ý, nhưng cần ghi
  nhận nếu chương trình giữ nhiều map nhỏ; tối ưu inline-small-map để sau, không chặn lát.
- **Mức**: **Thông tin/Thấp (bộ nhớ)**.

## 9. Quan sát **tiền tồn tại** (không do ST-315b; new khớp old)

1. **`eq_value` map không bắc cầu / không đối xứng**: dựng
   `A = {(Int(1),5),(Percent(1.0),5)} + 6 cặp đệm`, `B = {(Float(1.0),5),(Int(2),7)} + 6 cặp
   đệm` ⇒ cả logic cũ và mới cho `A == B` **true**, `B == A` **false** (hai khoá của A cùng
   khớp một khoá Float của B, B còn khoá Int(2) không khớp). Đây là hệ quả `eq_value` không
   bắc cầu đã có từ `ST-239`/`VD-270`; ST-315b giữ nguyên (fuzz 20 000 ca + ca chủ đích).
2. **Map có khoá `NaN` không bằng chính nó**: `m.eq_value(&m)` false (NaN không `eq_value`
   NaN), `contains_key(NaN)` false, `get(NaN)` None; chèn NaN tạo entry mới. Cả cũ/mới giống;
   `map.rs` test `nan_khong_khop_chinh_no` khoá hành vi này.
3. `Int(1)` và `Percent(1.0)` là **hai** entry (không eq nhau) nhưng cùng khớp `Float(1.0)`;
   `get(Float)`/`insert(Float)` trả/sửa entry **cũ nhất** — đã kiểm cả index lẫn fallback.

Nếu A0 muốn sửa (1)/(2) thì là task riêng (thay đổi `eq_value`/SPEC), không phải phạm vi
ST-315b; sửa trong lát này sẽ phá parity đã khoá.

## 10. Việc đã kiểm nhưng không thấy lỗi

- `VisMap` bất biến một khoá/entry qua `insert`/`from_pairs`/`merge`/JSON; key gốc + vị trí
  đầu; `remove` giữ thứ tự còn lại, set lại chèn cuối.
- Index: dựng ngưỡng 8, bucket trả cũ nhất, cập nhật khi insert mới/remove, bỏ khi clone,
  fallback khoá phức tạp/Unit/NaN, hash khớp `eq_value` từng arm.
- `Value::clone` không chép sâu map; `Rc::make_mut` ở mọi điểm ghi production; không còn
  `Rc::new(VisMap...)` rời nguồn; `SendValue::Map` ABI nguyên; `size_of::<Value>() = 64`.
- `Show`/`keys`/`values`/JSON/`eq_value`/`map.get`/`has`/`len`/`is_empty`/`m[k]`/`m[k]=v`/
  `map.remove`/`map.merge`/`map.get_or`/`contains_key`/`m.get`/`json.decode` (JSON trùng khoá)
  trên interp ↔ VM; `vis-diff` 100k lệch 0.
- LSP/pkg/plugin/http/fsmock/worker: biên dịch + test cổng; chỉ F1 là thay đổi hành vi.

## 11. Recheck F3/F5 sau vá `main` (`79f1fcee`, fix `085546d0`)

- **Merge**: `git merge main` trong `/home/ps/ViScript-r315b` là **fast-forward**
  `04163a7c..79f1fcee`; báo cáo cũ giữ nguyên. `main` gồm fix `085546d0`
  (`map.rs` +5, `core_it` +6, `st315b_map_cow` +19/−9; docs/STATE/QUEUE đi kèm).
- **F3 — nguồn**: `crates/vis-core/src/value/map.rs:257-262` dựng `MapIndex::dung` trước
  `vi_tri` khi `index.is_none() && entries.len() >= NGUONG_CHI_MUC`; sau đó mới cập nhật
  khoá cũ (`:263-265`) hoặc push + `them` (`:266-271`). Không đổi last-wins/thứ tự.
- **F3 — kiểm độc lập** (`/tmp/opencode/r315b/f3_recheck.rs`, link rlib hiện tại):
  - map 64 khoá → `a.clone()` (`Rc::ptr_eq`); `Rc::make_mut` cho `VisMap` index None;
    `insert("k0", 100)` ⇒ `co_chi_muc() == true`, `get("k0")==100`, `get("k63")==63`,
    bản gốc `a` vẫn `k0=0`.
  - `VisMap::clone` của map 64 khoá có index None; `insert` khoá cũ ⇒ index true;
    `insert` khoá **mới** ⇒ index true, `len 65`, tra được cả cũ/mới.
  - Ngưỡng: clone 7 khoá + insert khoá cũ ⇒ vẫn **không** index; 8 khoá ⇒ **có** index.
  - **Đo** (`f3_bench.rs`, N=10 000): 10 000 `get` trên bản clone **không** index = 259,20 ms;
    sau `insert` khoá cũ (index dựng lại), 1 000 000 `get` = 102,62 ms (~0,10 µs/get so
    25,9 µs/get tuyến tính).
- **F5 — kiểm độc lập**: `cargo test -p vis-interp --test interp_it st315b` ⇒ **6/6 pass**
  (gồm `co_chep_khi_chia_se_va_ghi_tai_cho_khi_doc` mới, assert index); `cargo test -p vis-core`
  ⇒ **26 unit + 4 `core_it` pass**, gồm `value_kich_thuoc_va_map_cow` với `==64` trên 64-bit.
  Gate L0 **3 856 test/66 binary, 0 fail, 4 skip**; clippy `-D warnings` exit 0.
- **Fuzz §2 chạy lại trên mã sau vá**: 200 chuỗi × 400 thao tác + 20 000 ca eq + 2 000 ca
  merge = **0 lệch** (kể cả clone + insert khoá cũ/mới).
- **Chi phí ghi còn lại** (không phải KPI): AB 5 cặp `writebench` N=10 000×100:
  nền 3 164 ms → mới 3 508 ms (+10,9 %); N=5 000×200: 800 → 917 ms (+14,6 %). Nguyên nhân:
  mỗi lần CoW + chèn **khoá mới** vẫn dựng lại `HashMap` O(N); fix F3 chỉ giữ index cho
  đường đọc sau cập nhật khoá cũ. Chấp nhận được (ngoài KPI; CoW copy chi phối).
- **KPI sau vá** (release mới `sha256 1bcc54b9f02a3d51179a605966e0ae9c63afb8d8112eeaff279e05813152eda0`):
  AB `opt-bench` interp 340 → **49 ms**, VM 350 → **45 ms**; AB `mapbench` N=1 44 → **41 ms**,
  N=316 1 257 → **47 ms** (**26,7×** nền A/B; 30,4× nền 1 429 ms); 15 mẫu N=8/79 = 44/44 ms.
  KPI ≤120 ms, ≥10×, N=1 không hồi quy: **PASS**.
- **F1** (header trùng) vẫn nguyên: đã ghi nhận là thay đổi hành vi cần A0/CHANGELOG, không
  sửa trong lát này. F2/F4/F6 vẫn low/info.
- **Kết luận recheck: PASS** — F3 đóng; F5 phần lớn đóng (còn ghi chú coverage `remove`/`merge` và non-transitive ≥ 8); không phát hiện hồi quy mới từ fix.

## 12. Tuyên bố

- Không sửa mã nguồn, SPEC, DECISIONS, QUEUE, STATE; chỉ tạo/cập nhật tệp này.
- Không push; chỉ merge `main` vào nhánh review theo yêu cầu recheck (fast-forward), không merge ngược vào `main`; không dùng `mps`.
- Commit review trên `review/r315b` (`git log -1` — không push).
- **PASS**; F3 đã vá, F5 phần lớn đã vá và recheck @79f1fcee (§11); F1/F2/F4/F6 vẫn là finding không chặn.
  F1 cần được ghi nhận công khai trước khi phát hành (nếu A0 coi map header là đa trị thì
  phải sửa/wontfix tường minh).
