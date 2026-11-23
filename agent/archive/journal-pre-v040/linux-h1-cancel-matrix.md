<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Ma trận flake `vs3_http_cancel` h1 trên Linux — so với Windows ~21 %

> **Người chạy**: W3 (hỗ trợ điều tra ST-287), 2026-11-22 16:24–16:30 JST (~6 phút; time-box 1 h).
> **Phạm vi**: **chỉ đo** trên Linux; không sửa code, không tag/push/merge, không GitHub Actions.
> **Mốc**: `plan/w3-platform` merge `main` **`651537c`** (docs-only so với `a1930cb`:
> `git diff --quiet a1930cb..HEAD -- crates Cargo.toml Cargo.lock` ⇒ **mã y hệt**). Version **0.3.6**.

## 1. Môi trường

| Mục | Giá trị |
|---|---|
| HEAD | **`651537c889822067e23ddb2936cff7a5a6e3b422`** |
| Test | `crates/vis-cli/tests/vs3_http_cancel.rs` sha256 `922898a6…be14c1` |
| Nền | Linux `ps` 7.0.0-43-generic x86_64, **12 CPU** |
| Toolchain | `~/.cargo/bin/cargo`/`rustc` **1.100.0** |
| Target | `target/h1-matrix` (**riêng**), `CARGO_INCREMENTAL=0` |
| Tải | 2 CPU burner (`yes > /dev/null`, PID 2768434/2768435); load trước **3,22 / 1,92 / 2,53**, sau **3,21 / 2,38 / 2,62** |
| Binary test | `target/h1-matrix/debug/build/vis-cli/a8662b95…/out/vs3_http_cancel-a8662b95…`, sha256 `e59e3137…26f` |
| Log | `/tmp/opencode/linux-h1-matrix/` (summary + log fail nếu có; **không có**) |

## 2. Ma trận

Chạy dưới **2 CPU burner** (tải nhẹ, ~2/12 CPU), tuần tự:

| Lần | Lệnh | Số lần | Pass | Fail | Wall |
|---|---|---:|---:|---:|---:|
| **A — ca flake riêng** | `cargo test -q -p vis-cli --test vs3_http_cancel cancel_http_h1_lap_nhieu_vong_khong_ro -- --exact --test-threads=1` | **100** | **100** | **0** | 94 s (~0,94 s/lần) |
| **B — full suite** | `cargo test -q -p vis-cli --test vs3_http_cancel -- --test-threads=1` | **20** | **20** | **0** | 35 s (~1,75 s/lần) |

- **Không có lần đỏ**, không có log fail, không timeout, không rò fd (ca tự assert `dem_tay_dang_mo()==0` và `kiemsau_huy`).
- Tổng: **120/120 lần xanh** (100 exact + 20 full).

## 3. So sánh với Windows (`psvn`)

| Chỉ số | Linux (phiên này) | Windows (`JOURNAL/final-win-v036.md`) |
|---|---|---|
| Ca riêng, lặp | **100/100 pass** (0 %) | ca riêng ×10 **10/10 pass** |
| Full suite `vs3_http_cancel` | **20/20 pass** (0 %) | **3/14 suite đỏ ≈ 21 %**, kể cả `--test-threads=1` |
| Dấu hiệu lỗi | không có | task bị huỷ vẫn in `V:post` sau `WSAECONNABORTED 10053` |
| Feature h2 8/8 | Linux đo được 8/8 ở QA trước | **BLOCKED-ENV** rc 101 (Unix-only) |

**Ý nghĩa thống kê** (nếu tỉ lệ Windows 3/14 = 21,4 % áp đúng cho Linux):

- 100 lần ca riêng: kỳ vọng ~21 lần đỏ; xác suất quan sát **0/100** khi p = 21,4 % là **3,4 × 10⁻¹¹**.
- 20 lần full suite: xác suất **0/20** là **8,0 × 10⁻³** (~0,8 %).
- Gộp 120 lần: **2,7 × 10⁻¹³**.
- Cận trên 95 % cho tỉ lệ Linux khi 0 lần đỏ: **2,95 %** (n=100), **13,9 %** (n=20), **2,47 %** (n=120).

⇒ **Không tái hiện race tương tự trên Linux** dưới tải nhẹ; dữ liệu không ủng hộ giả thuyết "race chung đa nền" với cùng xác suất.
Khác biệt khả dĩ: nhánh lỗi socket **Windows-specific** (`WSAECONNABORTED 10053`) đi vòng qua/đến trước
safe point `frame_cancel_one`, trong khi Linux trả lỗi/EOF theo đường khác; hoặc cần tương tác thứ tự
ca trong suite + timing Windows.

## 4. Kết luận / đề xuất

1. **Blocker Windows vẫn nguyên**: `cancel_http_h1_lap_nhieu_vong_khong_ro` flake ~21 % trên `psvn`;
   phiên này **không** tìm thấy race tương ứng trên Linux (120/120 xanh dưới 2 burner).
2. Hướng điều tra tiếp (không làm trong phiên này):
   - Instrument nhánh `AsyncNet` khi `read`/`write` trả `WSAECONNABORTED 10053` trên Windows: kiểm tra
     cờ huỷ đã được kiểm **trước** khi chạy tiếp thân task chưa; so sánh thứ tự safe-point ↔ lỗi socket
     giữa Linux (EOF/ECONNRESET) và Windows (10053).
   - Trên Windows, chạy lặp **cả suite** (không chỉ ca riêng) với `--test-threads=1`, lưu log từng vòng;
     ca riêng 10/10 xanh gợi ý tương tác với ca chạy trước (server/port/fd) hoặc trạng thái tĩnh.
   - Nếu không tái hiện được sau fix: cân nhắc waiver Tier 2 theo R32 §4 (ghi known-issue, không tuyên bố
     Windows xanh).
3. Không sửa code; không tag/push/merge. Đĩa cuối **8,7 G** (> 5 G).

## 5. Lệnh tái lập

```bash
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=$PWD/target/h1-matrix
yes > /dev/null & b1=$!; yes > /dev/null & b2=$!
for i in $(seq 1 100); do
  cargo test -q -p vis-cli --test vs3_http_cancel \
    cancel_http_h1_lap_nhieu_vong_khong_ro -- --exact --test-threads=1 || echo "FAIL exact $i"
done
for i in $(seq 1 20); do
  cargo test -q -p vis-cli --test vs3_http_cancel -- --test-threads=1 || echo "FAIL full $i"
done
kill $b1 $b2
```
