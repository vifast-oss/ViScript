<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R38 — ViScript, vòng 38 (Sóng 5–8: Kiến trúc tối ưu, chưa phát hành)

> **Trạng thái: CHỐT (A0, 2026-11-27).** Báo cáo vòng 38 trên nền **`main` = `4caa600e`**
> (docs `VD-303`; **mã sản phẩm cuối `ee8f252d`**), `origin` sync. Vòng 38 tiếp tục
> **chế độ Phát triển Kiến trúc Tối ưu (`VD-289`)**: không soak 4 h/24 h, không fuzz 8 h,
> không 3× full, không tag; `ST-309` giữ `BLOCKED` vô thời hạn.
>
> **Mốc đo**: cổng cuối `/home/ps/vislogs/final-main-ee8f252d.log` — sha256
> **`904d45a685367c4a15261a82536c93b17454109763c9821b91732ffcb735569f`**, chạy trên
> `ee8f252d`; commit docs chốt sổ là `4caa600e`.
>
> **Quy ước**: chỉ ghi số có log ngoài repo + sha256; ô “CHƯA ĐO” là chưa chạy, không phải 0.
> Không tuyên bố 3-OS/phát hành khi chưa có log `mps`/`psvn` và chain thật.

## 1. Tóm tắt vòng và trạng thái các Sóng

### 1.1 Sóng 5 — `ST-316a`/`ST-316b`/`ST-316d` `DONE`, `ST-316c` `BLOCKED` (`VD-293`)

| Lát | Trạng thái | Bằng chứng chính |
|---|---|---|
| `ST-316a` | **DONE** (tích hợp `882b7ffa`) | `value_ops` một nguồn; true O(1) in-place CoW `Map`/`List`; `Value::List(Rc<Vec<Value>>)`; F1/F2/F4/F5; `bin/vis-bench`: **writebench ~240×**, **listbench 13–21×** |
| `ST-316b` | **DONE** (audit `089b9f4c`) | `#199` lowering `O(N²)→O(N)` (25k `const` 1,604 → **0,196 s**); `#48/#7/#186/#145/#47` |
| `ST-316d` | **DONE** (`ecdb0660`) | DAG cấm `vis-host-native → vis-runtime` (+self-test); `#198` dời `worker_leak_it`; `host_parity` 4 tệp < 500 dòng |
| `ST-316c` | **BLOCKED** | Phần `ps` xong: WASI **398×2 (371+27)**, sem **390/390**, suite **28/28** @`d97773de`; `mps` @`311d2288`, `psvn` @`fe3446d2` lệch; chain 0-LLM **chưa chạy** |

Cổng Sóng 5 (`review/song5` @`882b7ffa`): gate **3.880/3.880** (4 skip), clippy exit 0,
`vis-diff 100k` **`lech=0`** (60,2 s), DAG 23 crate/5 tầng/0 cạnh cấm, **R8 39/40**.

### 1.2 Sóng 6 — `ST-317a`/`ST-317b`/`ST-317c` `DONE` (`VD-294`, `VD-295`)

| Lát | Trạng thái | Bằng chứng chính |
|---|---|---|
| `ST-317a` | **DONE** (merge `426b83a3`) | `Value` **64 → 32 B** (`Record(Rc)`, `Enum(Rc)`, `Cap(Box)`), CoW `Record` in-place, singleton `Rc<VisMap>` cho `{}`; conformance **568/568**; `size_of::<Value>() == 32`; VBC1 không đổi |
| `ST-317b` | **DONE** (`main` @`4e03a19c`) | Tách `pkg/sched/pat/lsp/value/sqlite`; **worker orchestration một nguồn** `orchestrate.rs` **135** dòng; **10 test W1..W10**; audit `e1fe1733`/`ef3e6928`/`4e03a19c` PASS |
| `ST-317c` | **DONE** (merge `aecb266c`) | `StdId` registry **167** hàm một nguồn; R10 `API-CODES.txt` ↔ `SPEC.md` §7.2 (`#75`); `AnalysisHost` (`#81`/`#66`); `lsp_it` **158/158** |

Cổng Sóng 6 `main` `4e03a19c` (`final-main-20261126T1747Z.log`): gate **3901/3901** (4 skip),
diff 100k `lech=0` (61,9 s), clippy/rules 0, R8 **39/40**.

### 1.3 Sóng 7 — **`DONE` 9/9** (`ST-318a..i`, `VD-296`..`VD-299`)

| Lát | Trạng thái | Bằng chứng chính |
|---|---|---|
| `ST-318a` | **DONE** — merge `2cda3a68`; audit F3 `5e79bf54` | Đóng **`#203`** (frame W=1 parity) + **`#204`** (non-frame W≥2 `VS0506`); `st317b` **20/20**; perf var-heavy **1,03–1,05×** main, write-heavy **0,98×** |
| `ST-318b` | **DONE** — `fb725479`; audit `001a9ffb` | `#202` đóng bằng `chuan_hoa_ms`; ca đích **20/20** |
| `ST-318c` | **DONE** — `c02050c0`; audit `4b952956` | `trait_impl.rs` 1.152→max 359; `lower/cmd.rs` 1.115→max **680**; `tyck/expr.rs` 1.079→max 589 |
| `ST-318d` | **DONE** — `82d77e77`; audit `76ca2925` | `hir.rs` 1.088→39 + 5 module (max 447); `vis-diag/lib.rs` 1.070→298 + 5 module (max 587); F1 `2ae1eb2e` |
| `ST-318e` | **DONE** — `9045dd78`; audit `ad404637` | `codegen.rs` 1.069→447 + `expr.rs` 628; `vistest.rs` 1.061→201 + 3 tệp |
| `ST-318f` | **DONE** — `bfe165e2`; audit `683d7a4f` | `lex/mod.rs` 1.033→134; `parse/mod.rs` 1.049→69; `parse/cmd.rs` 1.007→`cmd/mod.rs` 39 + 8 module |
| `ST-318g` | **DONE** — `328a0bad`; audit `634da5c4` | `modules.rs` 1.043→5 tệp (max 555); `builtin/mod.rs` 1.006→4 tệp (max 509) |
| `ST-318h` | **DONE** — merge `2cda3a68`; audit `03c1dc1f` | `vis-interp/src/frame.rs` **1030 → 159** + 4 tệp (`helper 366`, `state 112`, `step 99`, `work 344`) |
| `ST-318i` | **DONE** — merge `8e8b2887`; audit `c2e155da` | 4 intra-doc link `vis-hir`; `cargo doc -D warnings` exit 0 |

**Cổng chốt Sóng 7 (`2cda3a68`)**: gate **3912/3912** (4 skip); diff 100k `lech=0` 84,9 s;
clippy/rules 0; R8 **39/40**; **R1 0 tệp > 1.000**; 0 `unsafe`.

### 1.4 Sóng 8 — **`DONE` 5/5** (`ST-319a/b/c2/d/e`; `ST-319c` chẩn đoán, `VD-300`..`VD-303`)

| Lát | Trạng thái | Bằng chứng chính |
|---|---|---|
| `ST-319a` (W1) | **DONE** — merge `07a5e2b3`; audit F2 `b2033ebd` | Đóng **`#206`**: VM capture closure **by-value** theo `SPEC` §4.7; repro `0/0`, cache bump, **5 test**; gate **3917/3917**, diff 0; VBC1 không đổi |
| `ST-319b` (W2) | **DONE** — merge `eb7f5830`; audit `4dff8b31` | Đóng **`#207`**: doc `frame.rs:5` `[Interp]` → code span `Interp`; `cargo doc -D warnings` exit 0; gate **3912/3912** |
| `ST-319c` (W3) | **DONE (chẩn đoán)** — journal `b1249172`, merge `5408290a` | Tái lập **10 ca**; root cause **codegen thiếu tập `mutable`**; chuyển `ST-319d` |
| `ST-319c2` (W3b) | **DONE** — merge `1ec1105a`; audit `afed0250` | Đóng **`#209`**: frame closure **by-value**, P2-1 `nap_vao_env` `0caa9184`; test **w19–w24**, **26/26**, gate **3921**, w24 `3\n3` |
| `ST-319d` (W4) | **DONE** — merge `ee8f252d`; audit `64e84b25` | Đóng **`#208`**: codegen `mutables` + writeback `CallMethod; Assign; LoadLocal`; **32 test** `st317b`, gate **3929**, diff 0 |
| `ST-319e` (W5) | **DONE** — merge `a9c02abb`; audit `4baa4d2d` | Đóng **`#210`**: env module cho hàm tên trong closure; **A/B/C2/E/P5** 4 đường, gate **3922**, diff 0 |

**Cổng cuối `main` `ee8f252d`** (`final-main-ee8f252d.log`, sha256 `904d45a6…5569f`):
gate **3934/3934** (4 skip), diff 100k **`lech=0`** 79,7 s, `CLIPPY_EXIT=0`, `RULES_EXIT=0`;
R1 **0 tệp > 1.000**, R8 **39/40**, 0 `unsafe`, `API-VBC1.txt` không đổi.

## 2. Bảng số đo chính

| Hạng mục | Kết quả | Mốc / log |
|---|---|---|
| Gate Sóng 5 | **3880/3880**, 4 skip | `882b7ffa` — `review-song5-final.log` |
| Gate Sóng 6 | **3901/3901**, 4 skip | `4e03a19c` — `final-main-20261126T1747Z.log` |
| Gate chốt Sóng 7 | **3912/3912**, 4 skip | `2cda3a68` — `final-main-2cda3a68.log` |
| Gate `319a` (F2 + cache) | **3917/3917**, 4 skip | `b2033ebd`/`5d1d12e8` |
| Gate `319c2` (vá P2-1) | **3921/3921**, 4 skip | cây vá — `st319c2-fix-gate-fast.log` |
| Gate `319d` | **3929/3929** | `64e84b25` — `review-st319d-gate.log` |
| Gate `319e` | **3922/3922** | `4baa4d2d` — `review-st319e-gate.log` |
| **Gate cuối `ee8f252d`** | **3934/3934**, 4 skip | `final-main-ee8f252d.log` — sha256 `904d45a6…5569f` |
| `vis-diff 100k` | **`lech=0`** toàn bộ; cuối **79,7 s** | `ee8f252d` |
| Clippy `--all-targets -D warnings` | **exit 0 / 0 warning** | gate cuối + mọi lát audit |
| R10 (`API-CODES` ↔ `SPEC` §7.2) | **57 mã**, fixtures **20/20** | `review-st317c.md` §2 |
| Rules / DAG | R1..R10 OK; DAG **23 crate/5 tầng/0 cạnh cấm** | gate log |
| `report-check --all` | **PASS — 36 báo cáo, 380 hàng QUEUE** | `main` `4caa600e` |

**Ghi chú đếm**: số test tăng theo lát (319a +5, 319c2 +5, 319d +6, 319e +2) nên 3912 → 3917 →
3921 → 3922 → 3929 → **3934** là các cây khác nhau, không phải hồi quy.

### 2.2 KPI kiến trúc / hiệu năng

| Hạng mục | Số đo | Mốc | Ghi chú |
|---|---|---|---|
| `size_of::<Value>()` | **32 B** | `ST-317a` | `size_of_value_bang_32` + `core_it` |
| CoW `Record` — `r.f0 = i` | interp **1.869–1.875 → 54–55 ms (~34×)**; VM **1.905–1.918 → 55 ms (~35×)** | `ST-317a` | 256 trường × 200.000 vòng, release |
| `writebench` | **2.392–2.540 → 4 ms (~240×)** | `ST-316a` | release, `bin/vis-bench` |
| `listbench` | **411/422 → 21–32 ms (13–21×)** | `ST-316a` | `push` 10k: 1.228 → **9 ms** |
| Lowering `#199` | 25k `const`: **1,604 → 0,196 s**; 50k: **4,654 → 0,251 s** | `ST-316b` | byte-y-hệt |
| `StdId` | **167/167** khớp `API-STD.txt`; dispatcher không `_ =>` | `ST-317c` | test đếm + ca âm |
| `lsp_it` | **158/158**; `vis-lsp --lib` **29/29** | `ST-317c` | 0 `thread_local!` thật |
| Conformance | **568/568** | `ST-317a` | sau nén `Value` |
| WASI (`ps`) | **398×2 = 371 `.out` + 27 `.trap`**, 0 hỏng; sem **390/390**; suite **28/28** | `d97773de` | chưa đo lại trên `ee8f252d` |
| Worker W1..W24 | **32 test** `st317b_worker_orchestrate` | `ST-319d` | 0 `#[ignore]` |
| Rustdoc | `vis-hir` + `vis-interp` `-D warnings` **exit 0** | 318i/319b | 0 warning |
| R1 | **0 tệp > 1.000** (max **1000**, `run_cmd.rs`; `frame.rs` 159); 0 tệp > 1.500 | `ee8f252d` | KPI Sóng 7 13 → 0 |
| R8 | **39/40**; `vis-cli/tests` **8/15** | mọi gate | 38 + `vis-host-linux/tests` 1 |

## 3. Nợ và sổ `SO-NO.md`

Sổ hiện tại (`4caa600e`): **210 = 82 mở + 109 đóng + 16 VS3 + 2 không làm + 1 điều hướng**
(dải 1..210, thiếu `#205` — không cấp).

### 3.1 Nợ đã đóng trong Sóng 7–8

| Số | Nội dung | Trạng thái |
|---|---|---|
| **`#202`** | Flake `host_parity::net_http` do số ms thật | **đóng ở `ST-318b`** (`fb725479`, audit `001a9ffb`) |
| **`#203`** | Frame W=1 lệch interp↔VM | **đóng ở `ST-318a`** (`2cda3a68`, audit `5e79bf54`) |
| **`#204`** | Non-frame W≥2 nuốt lỗi task | **đóng ở `ST-318a`** |
| **`#206`** | Closure interp snapshot vs VM global live | **đóng ở `ST-319a`** (`07a5e2b3`, audit `b2033ebd`; repro `0/0`) |
| **`#207`** | Rustdoc `vis-interp` link `[Interp]` | **đóng ở `ST-319b`** (`eb7f5830`, audit `4dff8b31`) |
| **`#208`** | `xs.push` trong task — frame VM W=1 thiếu writeback | **đóng ở `ST-319d`** (`ee8f252d`, audit `64e84b25`; 32 test) |
| **`#209`** | Frame/async closure ghi global — VM không thấy writeback | **đóng ở `ST-319c2`** (`1ec1105a`, audit `afed0250`; w19–w24) |
| **`#210`** | Closure gọi hàm tên — interp non-frame sai env | **đóng ở `ST-319e`** (`a9c02abb`, audit `4baa4d2d`) |

### 3.2 Nợ còn mở (không thuộc Sóng 8)

- **`#200`** (P2): `fn main(a: int, b: int)` lệch interp (`VS0501`/exit 101) ↔ VM (exit 0).
- **`#201`** (P3): `fn main(a: int)` gán `argv` vào tham số `int` không kiểm kiểu.
- Cả hai giao **VS3/correctness**, đề xuất **Sóng 9** hoặc lát correctness kế tiếp.

## 4. Rủi ro và việc còn lại

| # | Việc / rủi ro | Mức | Cách xử lý |
|---|---|---|---|
| 1 | **`ST-316c` BLOCKED**: thiếu log `mps`/`psvn` 3-OS + chain thật | P1 | Chạy chain `e239c317…` trong cao điểm; chỉ tuyên bố 3-OS khi có log |
| 2 | **WASI 398×2 đo @`d97773de`**, chưa đo lại `ee8f252d` | P2 | Chạy `vis-targets --wasm` + conformance khi có slot; không tuyên bố số mới |
| 3 | **`#200`/`#201`** còn mở | P1 correctness | Sóng 9: chuẩn hoá arity/kiểu argv 2 engine + ca conformance |
| 4 | **P3 còn lại**: `var self` push, closure W>1, `PushBien` diff | P3 | Ghi nhận, xử khi mở lane codegen/closure tiếp theo |
| 5 | **R1 sát ngưỡng**: `run_cmd.rs` đúng **1000** dòng | P2 | Không tăng; tách khi chạm 1.001 |
| 6 | **Worktree `st319d`/`st319e`** còn tồn | P2 hành chính | Dọn sau khi audit/merge xong (`worktree remove` + `branch -d`) |
| 7 | **Chain 0-LLM chưa chạy**; `mps` @`311d2288`, `psvn` @`fe3446d2` lệch | P1 cho 316c | Chạy trong cao điểm; không sửa chain |

**Không tuyên bố**: 3-OS khi chưa có log `mps`/`psvn`; phát hành/tag (`VD-289`); “API-VBC1 rỗng”
theo nghĩa file rỗng — đúng là **diff không đổi**.

## 5. Bảng bằng chứng (log ngoài repo + sha256)

> Log tại `/home/ps/vislogs/`; sha256 tính tại thời điểm chốt. Log có thể đổi hash nếu bị nối thêm.

| Log | Bytes | sha256 | Nội dung |
|---|---:|---|---|
| `final-main-ee8f252d.log` | 414.933 | `904d45a685367c4a15261a82536c93b17454109763c9821b91732ffcb735569f` | **cổng cuối**: 3934/3934, diff 79,7 s, clippy/rules 0 |
| `final-main-2cda3a68.log` | 411.714 | `ec55f13023c6d36f35f7c3b7cdae35dbc6b908ace0f2eb55177ed88e7b834a58` | chốt Sóng 7: 3912/3912 |
| `final-main-20261126T1747Z.log` | 414.767 | `b004475427f6efd7428f2d95088ace47ad5a4d1b895b9d259388519e151f75dd` | chốt Sóng 6: 3901/3901 |
| `review-song5-final.log` | 408.708 | `7254b07a982878b86dc00841867dd281308538ace023c5f43859c1c4e2260e22` | Sóng 5: 3880/3880 |
| `st318h-merged-gate.log` | 411.386 | `db96ce21da7cc8486514229bac3c2f2835648991e9547d552e547fb7575ee37f` | hợp nhất 318a+318h: 3911/3911 |
| `review-st319a-f2-gate.log` | 410.671 | `730d0f43401468d2132c547cb1fcb30e7fde1d6e4f5b4d49ab42cfa14279ef00` | 319a F2: 3917/3917 |
| `st319c2-fix-gate-fast.log` | 150.062 | `263543a3658a66b857aa566f8c174401a2bac2884b096bb444a8725455952a5a` | 319c2 vá P2-1: 3921/3921 |
| `review-st319d-gate.log` | — | (log audit 319d) | 319d: **3929/3929** |
| `review-st319e-gate.log` | — | (log audit 319e) | 319e: **3922/3922** |
| `st316c-wasm-final-20261126T1958Z.log` | 7.736 | `c7d90f60ec6bbe6e2ead3b682cc8df1270018ea93c0a14ca6dbb2c328da82957` | WASI 398×2 (371+27) @`d97773de` |
| `peak1-2026-11-27-chain.sh` | 25.762 | `e239c317b29f6a0b4e75167d423889e659860bceaec15ca448c15a4aa593fa1c` | chain 18 bước, **CHƯA CHẠY** |

## 6. Đề xuất kế tiếp

1. **`ST-316c`**: chạy chain 0-LLM `e239c317…` khi `mps`/`psvn` rảnh; log 3-OS + bench; chỉ đóng
   `BLOCKED` khi có log thật.
2. **Sóng 9 — correctness còn lại**: `#200`/`#201` (arity/kiểu `argv` 2 engine), P3 còn lại
   (`var self` push, closure W>1, `PushBien` diff), dọn rustdoc toàn workspace.
3. **R1**: giữ 0 tệp > 1.000; theo dõi `run_cmd.rs` đúng **1000** — tách khi vượt.
4. **Dọn worktree** `st319d`/`st319e`; giữ `plan/st309-qa`; `worktree ≤ 8`.
5. **Không** mở lại Pha Phát hành (`ST-309`) trừ khi người dùng quyết định; giữ `VD-289`.

## 7. Checklist chốt R38

- [x] Sóng 5/6/7/8 `DONE` (Sóng 8: 5/5 + `319c` chẩn đoán).
- [x] Cổng cuối `ee8f252d`: **3934/3934** (4 skip), diff 100k `lech=0`, clippy/rules 0.
- [x] R1 **0 tệp > 1.000**; R8 **39/40**; 0 `unsafe`; `API-VBC1.txt` không đổi.
- [x] Sổ **210 = 82 mở + 109 đóng + 16 VS3 + 2 + 1**; `#206`–`#210` đóng; `#200/#201` mở.
- [x] `R38-viscript.json` + `.license`; `report-check --all` PASS (36 báo cáo, 380 hàng QUEUE).
- [ ] `ST-316c` 3-OS + chain thật (giữ `BLOCKED`).
