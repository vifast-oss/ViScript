# ST-290 — Ngân sách mặc định `40M`, `VS0509`/`VS0510`, cache có bảng phí (`VS3-1a`)

**Nhánh**: `plan/w3b-budget` @ base `12c9092`. **Ngày**: 2026-11-22. **A0**: VD-273 (chốt ngân sách
mặc định + cấp mã) mở bởi VD-280. **Trạng thái**: mã + test xong, chưa push/merge/tag.

## 0. Tóm tắt

1. Đăng ký **cả** `VS0509` (ghi biến cấp tệp chéo worker, W > 1) và `VS0510` (vượt ngân sách thực
   thi ngữ nghĩa) vào `crates/vis-diag/src/lib.rs` + `agent/API-CODES.txt` (54 → **56** mã), đủ
   `vi/en/ja`, đúng quy trình VD-129 (không tái dùng số, không đổi nghĩa mã cũ).
2. Bật ngân sách ngữ nghĩa **mặc định** `BUDGET_DEFAULT = 40_000_000` cho **interp** (đường
   thường + frame/async) và **VM**; giữ `VM_SAFETY_FUEL = 100_000_000`.
3. Cờ CLI `vis run --max-budget <N>` / `--unlimited-budget` đặt parsing ở `cli/run.rs`, không thêm
   dòng vào `interp.rs` (đang 1.458 dòng, sát trần R1).
4. VM nhận bảng phí qua cache: cache ghi artifact `INSTRUMENTED_VERSION` (trailer `VFEE`); lần
   chạy ấm vẫn charge/trap đúng. Bump `PHIEN_BAN` + re-pin `khoa_cache_ghim`.
5. Vượt ngân sách báo **`VS0510`** (không phải `VS0501`/`VS0501-FUEL`) với cùng dòng/cột/thông
   điệp trên hai engine.
6. **Quyết định 40M**: AN TOÀN cho toàn corpus đo được (đỉnh thật 18.019 trên 525 `test/sem`;
   workspace test lớn nhất ~12M; `diff 100k` chương trình nhỏ). Workload dài hơn 40M (ví dụ vòng
   `10^7` có thân gán: **60.000.013**) bị chặn **có chủ ý** theo VD-273 và có cờ thoát; không phải
   "trap oan" của corpus. Không cần A0 đổi ngưỡng.

## 1. Mã lỗi & API

| Mã | Nghĩa | Nơi dùng |
|---|---|---|
| `VS0509` | ghi biến cấp tệp chéo worker ở chế độ song song (W > 1) | cấp sẵn cho `ST-292` (W1) |
| `VS0510` | chương trình vượt ngân sách thực thi ngữ nghĩa | `BUDGET` của interp, `budget_charge` của VM, budget worker |

- `RUNTIME_CODES`: 8 → **10**, giữ thứ tự id tăng dần; test `so_runtime_05xx_khoa_theo_vd42` cập
  nhật.
- `API-CODES.txt`: 54 → **56**; `api_dong_bang::api_codes_khop_code` xanh.
- Các test đếm số mã (`explain_moi`, `st83_no_nho`, `vd270_explain_json`, `vis-diag/messages`,
  unit test trong `vis-diag`) cập nhật 56 kèm lý do.
- Mọi chỗ trước đây kỳ vọng chuỗi `VS0501-BUDGET` đổi sang `VS0510` (mã giả `VS0501-BUDGET` bị
  xoá; `BUDGET` nay trỏ `RUNTIME_CODES[9]`).

## 2. Ngân sách mặc định — thiết kế

### 2.1 Interp

- `crates/vis-interp/src/frame/fee.rs` (include `fee_shared.rs`) thêm chính sách **một nguồn**:
  `GIOI_HAN_PHIEN: AtomicU64 = BUDGET_DEFAULT`; `dat_gioi_han(n)`, `bo_gioi_han()`,
  `gioi_han_phien()`.
- `bat_dau_phien()` gọi ở **cửa vào runtime** (`run_with_hosts_caps_mode` và
  `run_async_with_hosts_caps_backend`): nếu test-hook chưa đặt `explicit`, bật charge với hạn
  phiên, `frame_emit = true` (đường `async` không né phí) và **không lưu trace** (chạy 40M sự kiện
  không được ngốn hàng trăm MB).
- `da_dung_phien()` cho nghiệm thu/đo (`VIS_BUDGET_REPORT=1`).
- Hook cũ (`enable_for_test`, `enable_measure_for_test`, quota) giữ nguyên hành vi và **thắng**
  chính sách mặc định nhờ cờ `explicit`.
- `tat_ngan_sach_phien_for_test()` cho ca cần chứng minh "tắt" (VD-257/stream A).

### 2.2 VM

- VM **không thể** tự suy bảng phí từ `Module` (bảng phí là side table theo PC). Vì vậy:
  - codegen thêm `compile_kiem_voi_bang_co_phi` (cùng cổng chẩn đoán front end);
  - CLI biên dịch `(Module, FeeTable)`, ghi cache **có trailer** (`cache::luu_co_phi`), lần sau
    `cache::nap_co_phi` trả lại cả hai; hạ dấu `INSTRUMENTED_VERSION` 3 → `VERSION` 2 sau khi tách
    bảng phí (bytecode không đổi) để `Vm::run` → `verify` nhận;
  - `vm_frame::dat_phi_lan_chay(fee)` đặt bảng phí **một-lần** cho lần chạy kế tiếp trên luồng;
    `Vm::run`/`run_frame_vm_driver` lấy hạn từ `gioi_han_phien()` và bật budget;
  - `budget_charge` gom sự kiện tại chỗ (`[Option<FeeEntry>; 16]`) thay vì cấp `Vec` mỗi lệnh;
    chỉ test-hook mới lưu trace (`budget_trace_on`).
- Cache cũ (v2 trơn) bị `load_verified_with_fee` từ chối ⇒ biên dịch lại và ghi đè. `PHIEN_BAN`
  thêm `+vd273-budget-default-40m-2026-11-22`; khoá ghim mới
  `e5373caf155dfad15e8173df5f5218651847d17d0963e9f0784b42c8ccdf026d`, `assert_ne!` với khoá
  `2c0382…bbbf` (ST-289).

### 2.3 Sửa `verify_fee` (bắt buộc để artifact thật nạp được)

Ràng buộc cũ "`site` phải nằm trong span của **đúng** lệnh tại `pc`" **từ chối chính artifact do
codegen sinh**: sự kiện `E_STMT` mang span **cả câu lệnh**, còn span lệnh đầu tiên là span hẹp của
biểu thức con (ví dụ `var n = 0` site 15, span lệnh `0` 23..24). Đã nới thành: `site != 0` phải
nằm trong **biên văn bản suy từ span mọi lệnh** (`site <= max_end`). Không đụng hàng rào chống né
phí: `pc`, `kind`, số sự kiện/PC, CFG chu trình và chu trình gọi vẫn kiểm nguyên; test crafted
(`site = 9999`, bảng rỗng, xoá phí hàm có vòng lặp) vẫn đỏ.

### 2.4 Giới hạn đã biết (ghi để W1/W3 sau không hiểu nhầm)

- Đường **worker pool W > 1** (opt-in) vẫn dùng `SharedBudget` riêng; chính sách mặc định chưa
  cấp `SharedBudget` cho pool ⇒ `--workers` (ST-292) phải nối ngân sách phiên vào pool. `W = 1`
  chạy caller nên đã charge.
- API thấp `vis_vm::run*` gọi trực tiếp **không kèm bảng phí** thì không charge (như trước); CLI
  luôn kèm. Đây là ranh giới của VM: bảng phí do compiler cấp.

## 3. CLI

```
vis run --max-budget <N>      # N nguyên dương; vượt ⇒ VS0510, exit 101
vis run --unlimited-budget    # bỏ hạn ngân sách; VM_SAFETY_FUEL 100M vẫn giữ
```

- `N = 0`, không phải số, tràn `u64`, hoặc dùng **cả hai** cờ ⇒ stderr + exit **2**.
- `--unlimited-budget` chạy được chương trình vượt 40M; `VS0501-FUEL` không xuất hiện trong các ca
  đo.
- Hook đo (không phải API người dùng): `VIS_BUDGET_REPORT=1` in
  `# budget: used=<N> limit=<L|unlimited> file=<tệp>` ra stderr **trên đúng luồng chạy**.

## 4. Đo đỉnh ngân sách (số thật)

Môi trường: Linux `ps`, debug, `CARGO_TARGET_DIR` riêng, `CARGO_INCREMENTAL=0`, rustc 1.100.0.

| Corpus / workload | Đỉnh (`standard_weight`) | Ghi chú |
|---|---:|---|
| **`test/sem` 525 ca — interp** | **18.019** | `ham/de-quy-1000.vis`; 470 ca chạy tới runtime, 55 ca lỗi tĩnh/đường khác |
| **`test/sem` 525 ca — VM** | **18.019** | cùng ca, **trùng khít** interp |
| `for 0..2_000_000` + 1 gán | 12.000.013 | ca biên VS3-1a |
| `for 0..2_000_000` + 3 gán | 20.000.014 | ví dụ `vs3_budget_measure` |
| đệ quy 200 khung | 2.815 | |
| 1000 `spawn` | 17.013 | |
| **vòng `10^7` + 1 gán (hợp lệ, dài)** | **60.000.013** | **vượt 40M** — trap `VS0510` trên **cả hai** engine, cùng `file:4:13`; `--unlimited-budget` chạy xong |
| 3.594 workspace test | không ca nào trap `VS0510` | toàn bộ xanh (xem §5) |
| `diff 100k` | chương trình ≤ 200 token | 93.768 chạy / 6.232 lỗi tĩnh / **0 lệch**, không ca nào trap |

**Kết luận đo**: không chương trình nào thuộc corpus nghiệm thu (workspace + sem + host + diff) bị
trap ở 40M. Chỉ workload **cố tình dài** (≈ 3,3M vòng có gán trở lên) vượt trần; đây là hành vi
theo thiết kế VD-273 và có `--unlimited-budget`. **Đủ điều kiện bật 40M**, không cần A0 đổi ngưỡng.

## 5. Test / cổng (số thật)

| Cổng | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` (debug) | **3.594 passed / 0 failed / 3 ignored** (356 target) |
| `vis test test/sem` / `--vm` | **525/525** ×2; cả hai engine cùng đỉnh **18.019** |
| `./bin/vis-host-suite` | **54/54** (+ `host_posix_real` PASS) |
| `./bin/vis-diff` (`VIS_DIFF_N=100000`) | **0 lệch**; 93.768 chạy, 6.232 lỗi tĩnh; 132,8 s |
| `./vis-dev gate --fast` | **PASS** (fmt/clippy/R1..R6/self-test/build/doc-examples + L0), 3 m 18 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | sạch |
| `cargo fmt --all -- --check` | sạch |
| R1 | không sửa `interp.rs` (1.458); mọi tệp sửa < 1.450 dòng |
| cache ghim mới | `e5373caf155dfad15e8173df5f5218651847d17d0963e9f0784b42c8ccdf026d` (≠ `2c0382…bbbf`) |
| df | còn **6,2 G** trên `/` (> 5 G) |

## 6. Ghi chú cho Gatekeeper / W1

- `VS0509` **đã cấp nhưng chưa dùng** ở ST-290 (đúng VD-280: W1/ST-292 nối cross-write). Cross-write
  hiện vẫn báo `VS0501` qua `loi()`; ST-292 chỉ cần đổi mã ở 3 điểm trap + test kỳ vọng.
- `ST-292` khi mở `--workers` phải cấp `SharedBudget` từ `gioi_han_phien()` cho pool (hoặc từ chối
  `--workers` khi `--unlimited-budget`); hiện pool W > 1 không charge ngân sách phiên.
- `vis test --vm` chạy `--no-cache` nên không phụ thuộc cache có bảng phí; `vis run --vm` thì có.

