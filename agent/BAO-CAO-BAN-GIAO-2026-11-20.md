<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Báo cáo bàn giao Agent 1 → Agent 0 — VD-257 (cuối tuần 20–23/11/2026)

> **Hiệu lực ủy quyền**: VD-257, từ khi nhận lệnh đến **09:00 JST thứ Hai 23/11/2026**. Báo cáo này
> khóa trạng thái tại thời điểm mọi task được duyệt đã hoàn tất; không tự mở thêm tính năng, không
> đổi runtime mặc định, không tag/release, không mở worker/GC/TLS/API mới.
>
> **Căn cứ**: `agent/DECISIONS.md` VD-257 · `agent/JOURNAL/vd257-{integration-plan,a-coverage-report,
> b-budget-report,c-lifecycle-report,d-fairness-proposal,d-fairness-result}.md`.

## 1. Commit, remote, cây, worktree

| Mục | Giá trị |
|---|---|
| HEAD / remote | `7d4512d3cc03375af9097a624dfb383318a3264f` — `origin/main` cùng hash, đã push |
| Trạng thái cây | **Sạch** (`git status --porcelain` rỗng) |
| Worktree còn mở | **Không** — 4 worktree A/B/C/D đã merge và remove; chỉ `/home/ps/ViScript` (`main`) |
| Tag | Chỉ `pha1-rc1` (= `05e42d0…`, không di chuyển) |
| Baseline lịch sử | `dbae70a` (checkpoint nghiệm thu cũ) |

## 2. Task đã hoàn tất / còn mở / bị chặn

| Task | Trạng thái | Commit tích hợp |
|---|---|---|
| ST-279 — Coverage frame A | **Xong** | `eee4fb0` → merge `e31ce3f` |
| ST-280 — Compatibility + budget B | **Xong** | `0bad843` → merge `a9faed7` |
| ST-281 — Cancellation + lifecycle C | **Xong** | `a36d8d5` → merge `b91b979` |
| ST-282 — Fairness D (phase 1 + 2) | **Xong** (thử nghiệm) | `2de9c51` + `fef5c45` → merge `3a8e1e7` |
| R1 module split + clippy sau merge | **Xong** | `fd4ba32`, `7d4512d` |
| Assertion lỗi thời VD-255 / A | **Xong** | `ff5727b`, `fd4ba32` |

**Còn mở có chủ đích**: coverage các ô §3; ngân số/thuật toán số §5 chưa chốt; step-cap parity; fairness
sản phẩm; Windows runtime; cancellation error-code & error-policy (blocker §6). **Không task nào bị
đóng băng ngoài các blocker đã ghi.**

## 3. Coverage frame — còn thiếu **chính xác**

**Đã phủ trên đường frame thật (source → check → 2 engine)**: `?` (Option/Result + `xs[i]?`, early
return), `chan` (create/send/recv/close + closed-state), `cmd` (từ chối rõ bằng VS1 mặc định),
field/index assignment, host-method vs closure-value dispatch, `if/match`, vòng lặp + `break/continue`,
short-circuit, record/tuple/set/map/enum/with, đối số tên, hằng cấp tệp, **closure callee + capture
dùng chung + writeback**, và **await trong thân closure trên VM** (B sửa `goi_ham_bytecode` — test
`42\n` cả 2 engine). Resume không double-charge (`(pc,kind,site)` charge đúng 1 lần).

**Còn thiếu / từ chối rõ (không fallback đồng bộ)**:
- `chan` blocking `send` trên đầy / `recv` trên rỗng ⇒ `VS0505` deadlock trap — **chưa phải chờ có
  wait-set**; `send` sau close ⇒ `VS0504`.
- `p.x += 1` compound field assign — parser `VS0201`; `r.f(4)` user-method qua field — checker `VS0403`.
- `RedispatchMethod` (`proc.rs`) là continuation phòng thủ, **chưa có ca dương** đi qua nhánh Pending
  của method host.
- `Set`/`Map`/`EnumLit`/`Interp`/`let-else` chỉ có test ở VD-255, chưa nhân bản trong test VD-257.
- Nested async lồng nhau (`fn() -> await src()` với `src` lại `await`) làm interp vượt guard 200 000
  — ghi nhận, chưa sửa.
- Chưa có test riêng cho **final-join error surfaced** sau thay đổi của C (hiện đã surfaced `VS0506`
  qua test VD-256) và chưa đo double-charge trên mọi cấu trúc phủ mới.

**Không gọi là "coverage hoàn tất".** Golden: **không tệp `.out`/`.err` nào bị đổi**; 26 fixture async
(52 tệp gồm out/err) đã phân loại, không cập nhật hàng loạt.

## 4. VERSION/cache & bằng chứng tương thích

Bốn trục được tách rõ:

| Trục | Giá trị |
|---|---|
| Module format | **base `VERSION = 2`**; artifact có trailer `VFEE` mang **`INSTRUMENTED_VERSION = 3`** |
| Event-table | `fee::VERSION = 4` |
| Weight-table | `WEIGHT_TABLE_VERSION = 1` |
| Cache key | gồm cả 4 trục + nhãn `vd257-b-four-version-axes-2026-11-20`; hash ghim `cca587…` → **`98a0b983…550e8`** |

Bằng chứng: runtime **cũ từ chối** artifact v3 ngay ở `decode` — **cả đường nạp trực tiếp**, không chỉ
cache (`lat257_b_compat` 9/9); runtime **mới từ chối** artifact v2/không trailer với thông điệp rõ,
**không âm thầm coi là đã charge**; cache không tái dùng verify của tooling cũ. Số `3` được kiểm lịch sử
(`git log`): core chỉ từng 1→2, `3` là sentinel test — đã đổi sentinel sang `99`, **không chọn số để né
sửa test**. `core VERSION` giữ 2 để không phá 2 assertion thuộc quyền `st246_cache_compat.rs`; nếu A0
muốn global 2→3 cần patch riêng.

## 5. Ngân sách / fuel / Q — số nào thử nghiệm, số nào chưa chốt

| Số | Trạng thái |
|---|---|
| Trọng số v1 `E_STMT/E_EXPR/E_ASSIGN/E_LOOP=1`, `E_BUILTIN/E_AWAIT=2`, `E_CALL/E_SPAWN=4` | **Thử nghiệm**, chưa hợp đồng |
| `BUDGET = 20.000.000` (VD-256 đề xuất) | **Bị A0 từ chối** làm mặc định (chỉ ~11% trên loop 2M) |
| `BUDGET = 40.000.000` (B đề xuất) | **Chưa chốt** — ~2,2× chi phí lớn nhất đo (18M) |
| `SAFETY_FUEL = 10⁷` trong code | **Chưa chấp nhận cuối** — chặn chương trình compiler sinh trước budget |
| `SAFETY_FUEL = 50M` (B đề xuất) | **Chưa chốt**, chưa áp; cần `F ≥ B` (max steps/cost ≈ 1.0) |
| `Q` | **Chưa chọn** — đã quét 7 giá trị × 7 lần (seed `0x…`, debug, ~10s), latency giảm đơn điệu 450.033 → 520 cost units khi Q 1M → 500; Q=500 có 1.050 yield (overhead) |
| Bật ngân sách/quota làm mặc định | **Không** — hook test-only, runtime mặc định nguyên vẹn |

**Root cause step-cap đã xác định** (không phải đổi tên): 2M vòng cost ~18M < 40M nhưng VM chạm fuel
ở 10.000.001 lệnh ⇒ `VS0501-FUEL` trước `VS0501-BUDGET`. Finding **vẫn OPEN** cho tới khi ngân sách mới
thay thế đúng và kiểm chứng.

## 6. B1/B2, latency, tài nguyên sống, môi trường

- **B1-sim** ~201 ms (reactor giả), **B1-real-default** 4,03–4,04 s, **B1-real-frame** ~201–203 ms
  (1/5/20 kết nối, mio/epoll) — chồng lấp trong workload đã đo. **Không** dùng tỷ lệ làm tuyên bố tăng
  tốc mạng sản phẩm.
- **B2**: `vis run` mặc định vẫn `CPU_DONE == IO_START` (đói). Trên **đường frame thử** với quota Q=500:
  `IO_START` và `SHORT_DONE` trước `CPU_DONE`, CPU vẫn hoàn tất; control (tắt quota) tái hiện starvation.
- **Latency** (cost units, monotonic, seed + 7 lần): control 450.033 → Q=200k 200.020 → 50k 50.020 →
  10k 10.020 → 2k 2.020 → 500 520; startup đo riêng 0,5–0,95 ms. Yield count I↔V **lệch ±1 turn**
  (VM bỏ yield ở instruction cuối) — hành vi stdout/exit/snapshot đồng nhất, **không** tuyên bố trace
  byte-equal mọi workload.
- **Vòng đời**: 5 wave `diag_snapshot()` đều về baseline zero; task hết handle + xong join thu hồi;
  handle thoát ra/chia sẻ không bị đóng khi task kết thúc.
- **Môi trường**: Linux x86_64, 12 lõi, 14 GiB; toolchain/MSRV **1.100.0** (VD-252/253); measurement
  **debug**, chưa release.

## 7. Gate trên đúng commit

Tại `7d4512d3cc03375af9097a624dfb383318a3264f`:
- `./vis-dev gate --fast`: **1562 passed / 3 skipped / 0 failed**.
- `./vis-dev nightly --fuzz-minutes 1`: **PASS** — fuzz parser 1 phút (seed ngày) OK, vòng tròn fmt
  475 tệp bất động (bỏ qua 78 ca lỗi cú pháp).
- `./vis-dev rules`: OK; `report-check` 31 báo cáo / 295 hàng QUEUE; cây sạch.
- Không chạy lại full gate trên cùng commit chỉ để tăng số PASS.

## 8. Đề xuất bước tiếp theo (A0 quyết)

1. **Chốt số**: trọng số v1, `BUDGET` (20M đã loại; 40M đề xuất), `SAFETY_FUEL` (50M đề xuất),
   và `Q` từ đường cong đã đo. Chưa áp mặc định khi chưa chốt.
2. **Đóng step-cap**: xử lý đúng quan hệ fuel↔ngân sách, có test chứng minh; không tăng fuel tùy ý.
3. **Khép coverage** các ô §3 nếu muốn thay runtime mặc định; đặc biệt wait-set cho `chan` chặn.
4. **Cancellation**: cần A0 chọn mã lỗi riêng (hiện tái dùng `VS0506`) và chính sách "lỗi task đầu ảnh
   hưởng task khác thế nào" — hiện **giữ nguyên**, không tự đổi.
5. **Windows**: mới chỉ `cargo check --target`; chưa bằng chứng runtime Windows mới.
6. Sau khi các mục trên đạt + gate trên đúng ứng viên: **mới** trình đổi runtime mặc định — lần này
   Agent 1 **không tự kết luận** được đổi.

## 9. Không làm / không kết luận

Không tag/release; không đổi runtime mặc định; không mở worker/GC/TLS/API công khai; không nới sandbox;
không thêm dependency lạ; không hạ tiêu chí hay bỏ test đỏ vô căn cứ. Cây để ở trạng thái xác định,
không tiến trình đang ghi dữ liệu, không worktree còn mở.

**Tóm lược**: bốn luồng cuối tuần đã biến runtime mới từ "chạy được demo" thành "chạy được phần lớn
chương trình hiện có, có tương thích bytecode rõ, có bằng chứng ngân sách/fuel và fairness thử nghiệm"
— nhưng **vẫn chưa đủ điều kiện thay động cơ cũ cho người dùng**. Ngân sách là luật ổn định của chương
trình, không phải hóa đơn cho thao tác nội bộ khác nhau giữa interp và VM.
