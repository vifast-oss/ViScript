<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Đề xuất trục kế tiếp sau `ST-314a` — số đo hiệu năng & hai nút thắt có mã nguồn

> **Từ**: Agent 2 (Builder) · **Cho**: Agent 0 · **Ngày**: 2026-11-25 (low-peak, `VD-287`)
> **Trạng thái**: **đề xuất**, chưa sửa `vis-vm`/`vis-core`. Nền `VD-289`/`VD-290` §4: A0 chọn trục
> kế tiếp sau khi `ST-314a` xanh; tài liệu này cung cấp số đo + phương án để A0 quyết.
> **Máy đo**: `ps` (Linux, 12 nhân), build **debug**, binary snapshot sha `caa5e911…a2773`
> (repo đang được sửa song song nên mọi số là **so sánh tương đối trong cùng phiên**, không phải
> hiệu năng tuyệt đối). Log: `/home/ps/vislogs/st300m2_*.{md,json,log}` (R7).

## 1. Số đo (n = 15 mỗi tổ hợp, `# time:` chỉ tính runtime)

| Tổ hợp | median | p90 | RSS đỉnh |
|---|---:|---:|---:|
| `test/bench/opt-bench.vis` · interp | **886 ms** | 891 | 15,4 MB |
| `test/bench/opt-bench.vis` · VM | **1236 ms** | 1252 | 15,5 MB |
| `test/bench/opt-bench.vis` · VM `--unlimited-budget` | **858 ms** | 902 | — |
| `test/bench/trim-matches.vis` · interp | 226 ms | 229 | 28,0 MB |
| `test/bench/trim-matches.vis` · VM | 218 ms | 225 | 28,1 MB |
| Khởi động runtime (1 dòng `print(1)`) | 0,20 ms (interp) / 0,16 ms (VM) | — | 6,4 MB |

- **VM mặc định chậm hơn interp +39,5 %** trên `opt-bench`, nhưng **nhanh hơn 3,2 %** khi tắt
  charge ngân sách (`--unlimited-budget`: 858 ms). Trên `trim-matches` (ít lệnh) VM nhanh hơn 3,5 %
  và `--unlimited-budget` **không đổi** (218 ms). ⇒ Toàn bộ khoảng cách là **overhead
  instrumentation**, không phải dispatch/bytecode.
- `--no-cache` không đổi (0,5 %) ⇒ không phải chi phí nạp/giải mã.
- Chênh tuyệt đối giữa hai phiên đo cùng ngày ~8 % nhưng **tỉ lệ VM/interp ổn định** (1,383 → 1,395)
  ⇒ kết luận theo tỉ lệ là đáng tin.

### 1b. Release `target/release/vis` (2026-11-25 11:20 UTC, binary 12.035.808 B, n=9)

| Tổ hợp | min | median | p90 |
|---|---:|---:|---:|
| `opt-bench` · interp | 328 ms | **331 ms** | 334 ms |
| `opt-bench` · VM | 607 ms | **610 ms** | 615 ms |
| `opt-bench` · VM `--unlimited-budget` | 340 ms | **342 ms** | 360 ms |
| `trim-matches` · interp | 30 ms | **33 ms** | 33 ms |
| `trim-matches` · VM | 26 ms | **27 ms** | 29 ms |
| `trim-matches` · VM `--unlimited-budget` | 26 ms | 27 ms | 28 ms |
| Khởi động runtime (1 dòng `print(1)`) | 0,044 ms | **0,050 ms** | 0,056 ms |

⇒ Trên **release**, VM mặc định chậm hơn interp **+84 %** trên `opt-bench`; tắt charge ngân sách
còn **342 ms** (ngang interp, −3 %). Chênh **268 ms** trên 2,0 M lệnh = **~44 % thời gian VM**
(~134 ns/lệnh) — xác nhận nút thắt §2 **không phải nhiễu debug**. `trim-matches` (ít lệnh) VM nhanh
hơn 18 %, `--unlimited-budget` không đổi.

## 2. Nút thắt 1 — `budget_charge` quét tuyến tính **mỗi lệnh** (ROI cao nhất)

**Mã**: `crates/vis-vm/src/vm_frame/budget.rs:176-228` (gọi từ `crates/vis-vm/src/vm/exec.rs:57`).

Mỗi lệnh, khi ngân sách bật (mặc định CLI **40M**), hàm:

1. khởi tạo `[Option<FeeEntry>; 16]` (MAX_PER_PC) — 16 phần tử mỗi lệnh;
2. `v.iter().filter(|e| e.pc as usize == pc)` — **quét toàn bộ** entry phí của **hàm**, không chỉ
   entry cùng `pc`;
3. mới xử lý các entry khớp.

Hệ quả đo được: `opt-bench` có thân vòng lặp 20 lệnh × 100.000 = **2,0 M lệnh**; chênh
`--unlimited-budget` **1236 → 858 ms** (debug) ⇒ **378 ms ≈ 30,6 %**; trên **release 610 → 342 ms**
⇒ **268 ms ≈ 44 %** thời gian VM (~134 ns/lệnh).
Chi phí là **O(số entry phí của hàm)** cho **mỗi** lệnh ⇒ hàm càng lớn càng tệ (tuyến tính theo
kích thước hàm), dù bảng phí chỉ vài chục–vài trăm entry.

### Phương án A (đề nghị duyệt) — chỉ mục `pc → range`, dựng **một lần** lúc `budget_enable`

- Thêm vào `Vm` (`crates/vis-vm/src/vm.rs:121` cạnh `budget_fee`):
  `budget_idx: Vec<Vec<(u32, u32)>>` (mỗi hàm: `index = pc`, giá trị `(start, len)` vào mảng phẳng
  `budget_flat: Vec<FeeEntry>`), hoặc `Vec<HashMap<u32, Range>>` nếu không muốn cấp phát theo
  `code.len()`.
- Dựng trong `budget_enable` (`budget.rs:100-108`): duyệt bảng phí **theo đúng thứ tự hiện tại**,
  gom entry cùng `pc` ⇒ **giữ nguyên thứ tự trace** (không đổi `budget_trace_for_test`).
- `budget_charge` còn: tra `(start, len)` O(1) + xử lý `len` entry; bỏ mảng `[None; 16]` tạm.
- **Ngữ nghĩa không đổi**: cùng sự kiện, cùng thứ tự, cùng điểm trap; chỉ đổi cách tra.
- **Cổng bắt buộc**: `cargo test -p vis-vm` (nhóm `lat256_budget` — trace có thứ tự, quota, trap
  `VS0510`), `vis-diff 100k lech=0`, `gate --fast`, đo lại `opt-bench` (kỳ vọng VM ≤ interp).
- **Rủi ro**: thấp — khu trú 1 tệp + 1 field; không đổi bytecode/cache/format.
- **Ước lượng**: 0,5 ngày.

### Phương án B (không khuyến nghị) — chỉ `binary_search` nếu chứng minh entry **đã sắp theo pc**

Ít mã hơn nhưng phụ thuộc thứ tự `FeeTable::push` (hiện là thứ tự codegen, **không** có bất biến
"tăng dần theo pc" trong kiểu). Nếu chọn B phải thêm test khoá bất biến + chuẩn hoá lúc `decode`.
Phương án A không cần giả định đó.

## 3. Nút thắt 2 — `map` là `Vec<(Value, Value)>`, tra cứu tuyến tính (**cả hai engine**)

**Mã**: `crates/vis-core/src/value/mod.rs:352` (`Map(Vec<(Value, Value)>)` với chú thích "giữ thứ tự
chèn"); tra cứu tuyến tính ở `crates/vis-runtime/src/builtin/method.rs:208-215` (`map.get`),
`crates/vis-interp/src/gan.rs:134`, `crates/vis-vm/src/vm/vm_helpers.rs:88,167` (index/`get`).

Đo quét N khoá (cùng 100.000 lần `get`). **Debug** (số cũ): N=1 → 376 ms; N=8 → 427; N=79 →
**889**; N=316 → **2387**. **Release** (`target/release/vis`, 1 mẫu/lượt, chương trình
`/tmp/opencode/mapbench/mN.vis`):

| N khoá | interp | VM | VM `--unlimited-budget` |
|---:|---:|---:|---:|
| 1 | 47 ms | 392 ms | 62 ms |
| 8 | 82 ms | 392 ms | 76 ms |
| 79 | **374 ms** | 724 ms | 417 ms |
| 316 | **1429 ms** | 1847 ms | 1530 ms |

⇒ Trên release, phần **quét map** (N=79 so với N=1) chiếm **~327 ms ≈ 87 %** thời gian interp
(47 → 374 ms); N=316 tăng tuyến tính (~1429 ms). Đây là nút thắt **lớn hơn** mọi tối ưu engine và
ảnh hưởng mọi chương trình thật. Lưu ý: với N nhỏ, overhead `budget_charge` của VM (392 ms ở N=1)
còn lớn hơn cả map — càng củng cố thứ tự ưu tiên A trước B.

### Phương án đề nghị — chỉ mục băm **giữ thứ tự chèn** (kiểu indexmap), không đổi ngữ nghĩa

- `Map` = `Vec<(Value, Value)>` (giữ nguyên thứ tự + `Show`/`keys()` ổn định) **+** `HashMap<MapKey, usize>`
  phụ. `MapKey` băm theo **đúng** `eq_value` hiện hành; khoá float `NaN` không bằng chính nó nên
  phải xử lý riêng (từ chối đưa vào index, fallback quét) — giữ đúng hành vi hiện tại.
- Mọi thao tác `get`/`has`/`set`/`remove`/`merge` phải đi qua **một** helper chung để tránh lệch
  interp↔VM; hiện có 4 điểm tra cứu rời.
- **Câu hỏi cần A0 chốt trước khi làm** (vì đụng ngữ nghĩa quan sát được):
  1. `eq_value` cho map là "ánh xạ đã gộp last-wins" — index có được phép **thay** thứ tự quét khi
     khoá trùng kiểu `1` và `1.0` (nếu `eq_value` coi bằng nhau) không?
  2. `json.decode` chọn last-wins (đã chốt ở `VD-242 §3.4`) — index phải tái tạo đúng "vị trí chèn
     đầu, giá trị cuối".
  3. Có cần giữ **byte-for-byte** `Show` của map sau `remove` rồi `set` lại (vị trí cuối) không?
- **Cổng**: `vis-diff 100k`, `net_pkg_it`, `vd270_map_*`, `json`, `gate --fast`; đo lại `opt-bench`
  (kỳ vọng −50 % cho cả hai engine).
- **Ước lượng**: 2–3 ngày (gồm 1 ngày khoá ngữ nghĩa + test đối chứng).

## 4. Khuyến nghị thứ tự

1. **Duyệt Phương án A (budget O(1))** — nhỏ, đo được, không đổi ngữ nghĩa; lấy lại ~25–30 % VM.
2. Sau A, **chốt 3 câu hỏi §3** rồi mở slice map-index — lợi ích lớn hơn cho cả hai engine.
3. **Không** chọn "engine/VM" làm trục kế tiếp: số đo cho thấy VM **ngang hoặc hơn** interp khi
   không bị instrumentation; ưu tiên hai nút thắt trên.
4. Hạ tầng đo: nên có `profile release` + gọi binary trực tiếp (không qua `./vis` — cộng ~130 ms
   kiểm tra freshness của cargo mỗi lần) và đo trên **worktree/binary đóng băng** có sha256.

## 5. Phụ lục — bằng chứng parity hỗ trợ quyết định

Trong cùng đợt low-peak, `ST-314a` đã tăng độ tin cậy của hai engine (chi tiết
`JOURNAL/st314a-hau-kiem.md` §21):

- **1.000.000 chương trình** sinh tự động gọi **73 hàm `std.*` thuần** (str/list/path/json, đầu vào
  biên Unicode/rỗng/số âm/list rỗng/JSON hỏng): **0 lệch** interp↔VM, so cả `stdout`, `exit`, mã
  **và thông điệp** (`crates/vis-vm/tests/vm_it/stdlib_diff.rs`).
- Corpus conformance mở rộng + `.trap` (lỗi **lúc chạy** có mã kỳ vọng), + async/kenh; xem §21.
- Hai lệch **thông điệp** interp↔VM được tìm và vá: `str.join` thiếu `use std.str`, và
  `select_recv` trên đường đồng bộ (`crates/vis-runtime/src/lit.rs` gộp về một nguồn `SIGS`).
