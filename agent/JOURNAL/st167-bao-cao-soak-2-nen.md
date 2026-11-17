# ST-167 — Soak 10 vòng **cùng một HEAD**: macOS vs `ps` (chỉ đo, không sửa)

Ngày đo: 2026-11-17 (JST). Người đo: Agent 2 (dòng ViScript). Công cụ: `viscript/bin/vis-soak` (10 vòng cố định:
`gate-1 gate-2 diff-1 diff-2 gate-3 diff-3 host-1 host-2 cache-1 cache-2`), log **không ghi đè**, chạy **tuần tự**
(không soak song song với build/cổng khác).

## 1. Kết luận ngắn

| nền | HEAD lúc bắt đầu | 10/10? | thời gian (wall-clock) | tỉ lệ mac/ps |
|---|---|---|---|---|
| macOS (Apple M6) | `795df046` | **10/10 OK** `[exit=0]` | **1696 s** (28 m 16 s) | **6,68×** |
| `ps` (Linux i5-1235U) | `795df046` | **10/10 OK** `[exit=0]` | **254 s** (4 m 14 s) | 1,00 |

**1696 / 254 = 6,68 lần > 3 lần** ⇒ **ĐỀ NGHỊ A0 MỞ NỢ ĐIỀU TRA HIỆU NĂNG** (số nợ do A0 cấp; đề xuất **#147**,
sổ hiện 147 dòng tới `#146`). **Không** tối ưu, **không** sửa mã, **không** đổi ngưỡng/ca trong đêm nay.

## 2. Cùng HEAD — bằng chứng

Đồng nhất **mã ViScript** được đo bằng hai khoá độc lập:

| khoá | mac | `ps` |
|---|---|---|
| `git rev-parse HEAD` (header do `vis-soak` tự ghi) | `795df046` | `795df046` |
| `git rev-parse HEAD:viscript` (sha cây thư mục `viscript/`) | `092fd5ab25897dc6380c3ff89571e86c58d8fe88` | `092fd5ab25897dc6380c3ff89571e86c58d8fe88` |
| load trước / sau | 8,11 5,89 5,50 → 5,46 6,62 6,03 | 0,09 0,09 0,09 → 1,79 0,93 0,44 |
| khung giờ | 13:01:30 → 13:29:47 | 13:30:45 → 13:34:59 |

**Đồng bộ `ps` trước khi đo** (cây lệch HEAD):
- Trước: `ps` HEAD `6f8f63a5` (đã fetch ⇒ `origin/main` = `c9f5ed45`, rồi `5d422b3b`). Đã `git fetch` + `git merge --ff-only`
  tới **đúng HEAD của lần đo** `795df046` (`st167-ps-fetch.log`, `st167-ps-merge2-lan1.log`: `Updating c9f5ed45..795df046`,
  `Fast-forward`, HEAD sau = `795df046`, cây `viscript` = `092fd5ab…`).
- Hai lần merge đầu **bị git từ chối** (`st167-ps-merge-lan1.log`): **14** đường dẫn tracked đang bị **xoá** trong worktree
  `ps` (`v2/agent/*`, `v2/test/rules/*`, `v2/tools/verify-numbers.py`, `vis-host-linux/tests/landlock.rs`,
  `viscript/crates/vis-cli/src/quyen.rs`) + **5** log untracked `st165-*.log` mà commit đích sẽ tạo. Cách xử lý **không xoá
  bằng chứng**: `git checkout <HEAD đích> -- <14 tệp>` (lấy nội dung từ git) và **chuyển** 5 log sang
  `/root/vislogs/ps-backup-st165/` (đủ 5 tệp) — rồi merge ff thành công. Không đổi lịch sử, không `reset --hard`,
  không đụng tệp chưa commit nào khác.
- Cây `ps` **vốn đã bị tỉa** từ trước (2159 dòng `git status`: 2158 tệp tracked bị xoá trong worktree, chỉ còn
  `AGENTS.md docs vis vis-dev vis-host-linux vis-host-wasm viscript` ở mức 1) — **hiện trạng có trước task này**, tôi
  **không** khôi phục và **không** ghi đè; chỉ khôi phục đúng 14 tệp bị merge chặn. Ghi ở `st167-ps-dieutra-cay3.log`.

**Biến động HEAD trong lúc đo (ghi rõ, không giấu):** dòng OS commit tiếp trong lúc mac soak (`795df046` → `12fbff3e`);
đối chiếu `git diff --stat 795df046 12fbff3e -- viscript/` = **8 tệp, +77/−2, toàn bộ là `viscript/agent/*.md`
(QUEUE/DECISIONS/SO-NO/journal/vs3-qd-4)**; `git diff` cho `viscript/crates`, `viscript/bin`, `viscript/test`,
`viscript/Cargo.*` **rỗng** ⇒ **0 dòng mã đổi** trong suốt 2 lần đo ⇒ 10/10 vòng của cả hai nền chạy **đúng một bộ mã**.

## 3. Toàn bộ 10 vòng (giây, wall-clock từng vòng)

| vòng | mac | `ps` | tỉ lệ |
|---|---|---|---|
| gate-1 | 350 | 40 | 8,75× |
| gate-2 | 382 | 39 | 9,79× |
| diff-1 | 180 | 40 | 4,50× |
| diff-2 | 179 | 41 | 4,37× |
| gate-3 | 346 | 39 | 8,87× |
| diff-3 | 189 | 42 | 4,50× |
| host-1 | 8 | **0** | (mac phải dựng lại binary; `ps` đã có sẵn target) |
| host-2 | 8 | 1 | 8,0× |
| cache-1 | 30 | 6 | 5,0× |
| cache-2 | 24 | 6 | 4,0× |
| **TỔNG** | **1696** | **254** | **6,68×** |

Không vòng nào đỏ; `vis-soak` chỉ in `TỔNG | … | 10/10 vòng OK` khi đủ 10 vòng, cả hai log đều có dòng này.

**Phân rã theo pha** (tổng 3 lượt gate, 3 lượt diff, 2 host, 2 cache):

| pha | mac (s) | `ps` (s) | tỉ lệ | phần chênh so tổng |
|---|---|---|---|---|
| `gate --fast` ×3 | **1078** | 118 | **9,14×** | 960 s ≈ **57 %** của 1696 s |
| `vis-diff` 100k seed ×3 | 548 | 123 | 4,46× | 425 s ≈ 25 % |
| `vis-host-suite` ×2 | 16 | 1 | — | 15 s |
| `cargo test -p vis-cli --test cache_khong_che` ×2 | 54 | 12 | 4,50× | 42 s |

⇒ Chênh lệch **tập trung ở pha cổng `gate --fast`** trên macOS, không riêng `vis-diff`.

Số ca đạt cộng dồn qua 3 lượt gate: mac **3577**, `ps` **3589** (+12 = **+4/lượt**, các ca
`cfg(target_os = "linux")` — cùng hướng với ST-161 `+3`; không có ca nào đỏ ở cả hai nền).

## 4. Nhiễu tải máy (bắt buộc đọc kèm)

- mac **không rảnh**: lúc bắt đầu có hoạt động build/QEMU của dòng OS trên **cùng máy** (11 tiến trình
  `cargo`/`qemu` tại 13:01, về 0 tiến trình lúc 13:01:18); load 1 phút 8,11 → 5,46. Vì vậy **1696 s là cận trên**
  (bất lợi cho mac), không phải "tốc độ máy rảnh".
- `ps` rảnh: load 0,09 → 1,79 (chính soak này).
- Dù vậy, mức chênh **6,68×** lớn hơn ngưỡng **3×** khá xa; kể cả giả định mac bị chậm oan toàn bộ phần vượt
  tải thì pha gate vẫn còn 9,14× (đo trên cùng bộ mã, cùng toolchain `rustc/cargo 1.99.1`).
- **Cần đo lại trên mac rảnh** (load < 2, không build song song) **trước khi kết luận nguyên nhân** — đó là nội dung
  của nợ đề nghị mở, không phải việc của đêm nay.

## 5. Phần cứng / môi trường

| | mac | `ps` |
|---|---|---|
| CPU | Apple M6, **12** nhân | 12th Gen Intel i5-1235U, **12** luồng |
| RAM | 32 GB | 14 GB |
| OS | macOS 27.0.1 (Darwin 27.0.0) | Linux 7.0.0-43-generic |
| rustc / cargo | 1.99.1 / 1.99.1 | 1.99.1 / 1.99.1 |

## 6. Đề nghị (chỉ đề nghị — **không** sửa gì)

1. **A0 cấp số nợ** cho "điều tra hiệu năng soak macOS vs `ps`" (đề xuất **#147**; `SO-NO.md` hiện tới `#146`).
   Nội dung tối thiểu: `1696 s vs 254 s = 6,68×` tại cùng HEAD `795df046`/cây `092fd5ab…`; 57 % chênh nằm ở
   `gate --fast`; **đo lại khi mac rảnh** (load < 2) trước khi tìm nguyên nhân (build/test, `fork/exec`, fsync,
   `std::fs::metadata` trên macOS — liên quan `#146`/`VD-145`).
2. Không đổi ngưỡng, không đổi ca, không tối ưu mã trong đợt này (đúng luật cứng của task).
3. Nếu sau này cần số "chuẩn" cho R29: dùng **1696 s / 254 s / 6,68×** kèm ghi chú tải máy ở §4.

## 7. Tệp bằng chứng (trong `viscript/agent/JOURNAL/`)

- `st167-soak-macos.log` (6220 dòng, header `HEAD 795df046`, `# load trước/sau`, `[exit=0]`, dòng `TỔNG`).
- `st167-soak-ps.log` (6234 dòng, chép từ `ps` bằng `scp`; `TỔNG | 254s | 10/10 vòng OK`); **bản sao khớp tuyệt đối**:
  sha256 `e2b49ef6cb62654b04c00d5342857e8e38362831c3a1513f1933d152a81d7ee0` ở **cả hai phía** (kiểm bằng
  `st167-ps-sha-kiem.log`), 6234 dòng cả hai.
- `st167-soak-ps-header.log` (header VD-136 + `[exit=0]` **đọc từ tệp `.exit`** trên `ps`, không lấy `$?` của ssh;
  sha256 `46cc9936…` trong tệp này là của **log `vis-remote` trên máy đích**, không phải của tệp soak — tệp soak là
  `e2b49ef6…` ở trên).
- `st167-ps-fetch.log`, `st167-ps-merge-lan1.log` (bị chặn: 14 tệp + 5 log), `st167-ps-merge2-lan1.log` (ff OK
  `c9f5ed45..795df046`).
- `st167-ps-head-khop.log`, `st167-ps-dieutra-cay3.log`, `st167-ps-phancung.log`, `st167-ps-diff-2tep.log`.
