# ST-296 — `vd270_test_slowest` chịu tải (không `#[ignore]`)

**Nhánh**: `plan-w3b-budget` @ `5a38680` (merge main). **Không sửa mã sản phẩm** — chỉ test.
**Thời điểm**: 2026-11-23 (JST). **Môi trường**: `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`,
`CARGO_PROFILE_DEV_DEBUG=0`, target riêng của worktree.

## 1. Vấn đề

Soak kết hợp (22/11, `fa9a39d`, trước `c42d4a5`) bắt flake
`vis-cli::vd270_test_slowest::slowest_repeat_2_ten_k_tren_n`: dưới load **36**, ca “nhanh”
(1 `assert true`) đo **74 ms** > ca “chậm” **66/60 ms** (busy loop 50 ms), nên hai dòng đầu
không phải `cham`. Đây là **test đo thời gian tường**, không phải lỗi sản phẩm; nhưng test cũ
có thêm lỗ hổng: tệp nguồn đặt `a_cham` trước `b_nhanh`, trùng thứ tự thời gian ⇒ không phân
biệt được “sắp theo thời gian” với “giữ nguyên thứ tự nguồn”.

`main` đã có bản nới 50 → 300 ms (`c42d4a5`), nhưng vẫn là ngưỡng tuyệt đối mỏng. Bản này làm
test **chịu tải có căn cứ** và khoá regression mạnh hơn.

## 2. Sửa (chỉ `crates/vis-cli/tests/vd270_test_slowest.rs`)

1. **Ca chậm 1000 ms**, ngưỡng **800 ms** (80% nominal; vẫn ≥ 10× fast đo được). Căn cứ: dưới
   load 36, `vis test` spawn ca nhanh mất tối đa **74 ms** (đo soak); 1000/74 ≈ **13,5×**.
   Busy loop dùng `time.monotonic_ms`, không `sleep`; ngưỡng 800 ms vẫn bắt regression thời gian.
2. **Đảo thứ tự nguồn**: `a_nhanh.vis` (nhanh) trước `b_cham.vis` (chậm). Nếu `--slowest` giữ
   thứ tự nguồn thay vì sắp theo thời gian ⇒ hai dòng đầu là `nhanh` ⇒ **đỏ**. Đây là kiểm
   regression thật, không chỉ “ca chậm tình cờ đứng trước”.
3. **Warmup**: `bo_hai_ca` chạy một lượt `vis test` không cờ (bỏ kết quả) trước khi đo, giảm
   nhiễu nạp trang/binary.
4. **Trung vị 3 lần** cho ca `--repeat 2`: mỗi lần phải đủ 4 đơn vị `[k/N]`, stdout y hệt;
   đòi **≥ 2/3 lần** hai dòng đầu là `cham` (nhiễu lẻ không che regression); trung vị `cham`
   ≥ 800 ms và > trung vị `nhanh` **+300 ms**.
5. **Không `#[ignore]`, không xoá ca, không nới lỏng kiểu**: vẫn kiểm đơn vị `[k/N]`, stdout
   không đổi, giảm dần, `--filter`/`--shard`/`--shuffle`/`--jobs` compose. `--shard 2/2` đổi kỳ
   vọng sang `cham` vì `a_nhanh` đứng trước (`b_cham` là mục thứ 2).

## 3. Chạy dưới tải

- **10/10 pass** với load 1 phút **35,81 → 60,23** (60 tiến trình `yes`, 12 core):
  `dem_dung=3/3` mỗi lần; trung vị `cham` **1.025–1.064 s**, trung vị `nhanh` **15–57 ms**;
  khoảng cách trung vị > **0,96 s** (ngưỡng đòi > 0,3 s).
  Log: `/home/ps/tmp/st296-load30b.log`, sha256 `51b9dc3e…cd5d06` (4.511 B, ngoài repo — R7).
- Thêm **10/10 pass** ở lần ramp load 15 → 40: `/home/ps/tmp/st296-load30.log`,
  sha256 `1565ede9…c61b3e`.
- Test đơn lẻ (không tải) sau sửa: 7/7 ca của `vd270_test_slowest`, 25 s.

## 4. Cổng

- `./vis-dev gate --fast`: **PASS**, `Summary [65.285s] 3638 tests run: 3638 passed, 4 skipped`;
  luật **R1..R7 OK** (gồm R7 log >50 KB). Log: `/home/ps/tmp/st296-fix-gate.log`,
  sha256 `ca3caf02…e5901c`.
- `cargo fmt --all -- --check`: sạch; `cargo clippy --workspace --all-targets -- -D warnings`:
  sạch.
- `vis --version`: **0.3.6**; đĩa cuối: **17 G** trống; không push/merge/tag, không GitHub Actions.

**Kết luận**: test chịu được tải ≥30 (10/10, tới load 60) mà vẫn bắt regression “không sắp theo
thời gian / mất đơn vị `[k/N]` / đổi stdout”. Không có blocker.
