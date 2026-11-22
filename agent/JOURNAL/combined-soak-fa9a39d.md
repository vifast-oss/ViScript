# Soak kết hợp 4h trên `main` `fa9a39d` — ST-296 (W4)

**Nhánh**: `plan/w3b-budget` (fast-forward `main` `fa9a39d`). **Chạy**: 2026-11-22 (JST). **Không sửa mã.**
**HEAD**: `fa9a39d` (merge ST-290 budget + ST-294 async-h2 default + ST-295 openat). `vis --version`: **0.3.6**.

## 1. Môi trường (đúng yêu cầu A0)

- Worktree riêng `plan-w3b-budget`; `CARGO_TARGET_DIR=$PWD/target` (không dùng target main/worktree khác).
- `TMPDIR=/home/ps/soak-tmp` (trên `/`, **không** dùng `/tmp` tmpfs).
- `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`.
- `./bin/vis-soak --log <tệp> --append`; mỗi lần = 10 vòng: **gate ×3, diff 100k ×3, host ×2, cache ×2**.
- Không export `VIS_TASK_ID` (xem §2).

## 2. T-INFRA lần chạy đầu (harness, không phải sản phẩm)

Lần phát đầu (20:51 JST, log `st296-soak-harness-env-fail.log`) đỏ ngay **gate-1 87 s**:
`soak_script::vis_soak_hop_le_va_in_du_10_vong` gọi `bin/vis-soak --dry-run`; biến `VIS_TASK_ID=296`
do runner **export** rò vào tiến trình con ⇒ script con chọn đúng log đang ghi (khác rỗng) ⇒ từ chối
ghi đè (exit 2) ⇒ test đỏ. **Nguyên nhân môi trường harness**, không phải sản phẩm. Đã bỏ export
`VIS_TASK_ID` (vẫn truyền `--log` tường minh) và chạy lại.

## 3. Diễn biến

### 3.1 Lần 1 (`st296-soak-lan1-4x10-fail-load36.log`)

- **4 lần × 10/10 vòng OK** (40 vòng): gate **3613/4** ×12, diff 100k **0 lệch** ×12, host **54/54** ×8,
  cache **×8**. Tổng thời gian 4 lần: **3902 s** (65 phút).
- **Lần 5 dừng ở gate-2** (13:02 UTC): `vis-cli::vd270_test_slowest::slowest_repeat_2_ten_k_tren_n`
  đỏ vì **thứ tự thời gian** dưới tải: ca “nhanh” đo **74 ms** > ca “chậm” **66/60 ms**; test kỳ vọng
  hai dòng đầu là ca `cham`. Log ghi load **36.06** (ba tác nhân khác đang build/fuzz cùng máy).
  Chạy **riêng** ca đó 5/5 lần sau đó: **pass** (0,28–0,45 s). ⇒ **T-INFRA** (test đo thời gian
  tường, không phải hồi quy sản phẩm). Dừng ngay theo luật soak; **không retry** trong cùng lần.
- Đĩa lúc dừng: còn ~6,6 G (dao động do build của tác nhân khác); đã dọn **test binary cũ**
  (`target/debug/build/vis-cli/*/out/*` >5 MB, cũ hơn 20:30) — **không** đụng cache/nguồn.

### 3.2 Lần 2 (`st296-soak-lan2.log`) — sau khi tải giảm

Bắt đầu **23:00:40 JST** khi load về **4,66**; deadline **01:10 JST**. Kết quả: **9 lần × 10/10 vòng OK**
(90 vòng), dừng theo time-box sau lần 9 (elapsed 7.924 s ≈ 132 phút). Mọi gate **3613/4**, diff 100k
**0 lệch**, host **54/54**, cache **×2**; không flake.

## 4. Tổng hợp

| Hạng mục | Số |
|---|---|
| **Lần 10/10 vòng** | **13** (lần 1: 4; lần 2: 9) |
| **Tổng vòng** | **130** (13 × 10) |
| Gate `3613 passed / 4 skipped` | **39/39** lần gate (12 + 27); thêm 1 gate-1 xanh của lần 5 (không tính 10/10) |
| Diff 100k `lech=0` | **39/39** lần (12 + 27) |
| Host `54/54` | **26/26** lần (8 + 18) |
| `cache_khong_che` | **26/26** lần (8 + 18) |
| Thời gian chạy xanh | **11.825 s ≈ 3 h 17 phút** (lần 1: 3.902 s; lần 2: 7.923 s) |
| Tổng thời gian bao gồm chờ/tải | **≈ 4 h 30 phút** (20:44 → 01:14 JST) |
| Flake/T-INFRA | **1 flake** `vd270_test_slowest::slowest_repeat_2_ten_k_tren_n` (lần 1, gate-2, load 36) — riêng ca đó **5/5 pass** sau đó; **1 lỗi harness** `VIS_TASK_ID` rò vào test `soak_script` (đã sửa env). **0 hồi quy sản phẩm** |
| Đĩa cuối | **15 G** trên `/` |
| `vis --version` | **0.3.6** |
| HEAD | **`fa9a39d`** |

**Kết luận**: trên `main` `fa9a39d`, 130 vòng soak kết hợp (gate + diff 100k + host + cache) đều xanh;
không có lỗi sản phẩm. Hai sự cố đều **T-INFRA/harness** (tải máy 36 và biến môi trường), đã ghi rõ
và không tính là flake sản phẩm. Không sửa mã.

## Log ngoài repo (R7)

Các log soak >50 KB đã chuyển khỏi `agent/JOURNAL/` sang `/home/ps/vislogs/st296/` (R7); sha256:
```
8725c94ff5d110cd3a0e76177f17e23887d9287b444b95eab6b00aa324918165  st296-soak-harness-env-fail.log
8a5539b49657aece4419f828fd72a37e998b22b880250619a368dee65d7f956b  st296-soak-lan1-4x10-fail-load36.log
2600475b362728f2346ac7be2a43d4a78a1c6f0de557e23a5f6e567b98989bfc  st296-soak-lan2.log
```
