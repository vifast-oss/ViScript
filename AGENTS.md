# AGENTS.md — repo ViScript (luật gốc)

**ST-171 (VD-165/VD-179 §4)**: repo này là **bản tách** của cây `viscript/` + `docs/` khỏi
`ViFastOs`. Gốc repo **là** cây ViScript (`crates/`, `bin/`, `test/`, `agent/`, `docs/`,
`Cargo.toml`), không còn tiền tố `viscript/`.
Luật chung của `ViFastOs` (`AGENTS.md` gốc, `docs/QUYET-DINH-sau-review.md` — **QD**) **vẫn áp** nhưng
nay nằm ở repo `ViFastOs`; tệp này giữ phần ViScript tự chứa.

## 1. Thứ tự nguồn sự thật (áp cho ViScript)

1. **Code + test** trong repo này: crate biên dịch được, golden, fuzz, TAP.
2. **`SPEC`** — [`docs/SPEC-lex.md`](docs/SPEC-lex.md) (từ vựng + EBNF tự chứa) và `SPEC.md` (do ST-02a/ST-02b sinh); sửa **chỉ** qua Agent 0.
3. [`agent/DECISIONS.md`](agent/DECISIONS.md) — VD-01 trở đi; số mới nhất: `grep -o 'VD-[0-9]\+' agent/DECISIONS.md | sort -t- -k2 -n | tail -1` (QD §7, các câu Q1..Q5 của SPEC-lex, và các chỗ văn phạm chế độ lệnh/vis-fmt/vis-diag cần chốt).
4. Tài liệu thiết kế [`docs/ViScript-thiet-ke.md`](docs/ViScript-thiet-ke.md) — tham chiếu.

QD thắng tài liệu thiết kế.

## 2. Tài liệu và điều khiển

| Tệp | Vai trò |
|---|---|
| [`docs/ViScript-thiet-ke.md`](docs/ViScript-thiet-ke.md) | thiết kế đầy đủ (9.4 bảng xung đột chế độ lệnh, 14 cấu trúc crate, 20 quy trình, 22 quyết định) |
| [`docs/SPEC-lex.md`](docs/SPEC-lex.md) | đặc tả từ vựng + EBNF tự chứa — sản phẩm của **ST-02a** |
| [`agent/QUEUE.md`](agent/QUEUE.md) | mọi vòng; trạng thái từng task là nguồn duy nhất về tiến độ |
| [`agent/DECISIONS.md`](agent/DECISIONS.md) | VD-01 trở đi; số mới nhất: `grep -o 'VD-[0-9]\+' agent/DECISIONS.md | sort -t- -k2 -n | tail -1` |
| [`agent/STATE.md`](agent/STATE.md), [`agent/BLOCKED.md`](agent/BLOCKED.md) | trạng thái + chặn (Agent 2 ghi) |
| `agent/JOURNAL/`, `agent/reports/` | nhật ký ngày, `R<n>-viscript.md` + `.json` |

## 3. Trạng thái duyệt hiện tại

**Trạng thái dòng ViScript**: không ghi ở tệp này. Đọc theo thứ tự: `agent/ROADMAP.md` (8 hạng mục × mức thật), báo cáo mới nhất `ls agent/reports/ | sort -V | tail -1`, rồi `agent/QUEUE.md`. Tệp luật không chứa số đếm (số vòng, số test, VD mới nhất) để không lỗi thời.
**Quy trình hiện hành**: **VD-279** (mở rộng VD-119/VD-137) — phân vai theo quyền hạn (`Architect`/`Agent 0`, `Builder`/`Agent 2`, `Gatekeeper`/`Agent 1`), độc lập môi trường: Agent 0 có repo ghi trực tiếp commit `[ST-00]` và đẩy lên `origin`; Agent 0 chỉ trò chuyện dùng khuôn dán nguyên văn `VD-119`; một tác nhân kiêm nhiệm cả 3 vai vẫn phải tách riêng commit `[ST-00]` khỏi `[ST-###]` và nghiệm thu bằng cổng máy tái lập được.

## 4. Lệnh

```bash
./vis-dev check             # fmt + clippy + luật R1..R4 + self-test + build + doc-examples
./vis-dev gate --fast       # cổng bắt buộc cuối mỗi task ViScript (check + L0 test)
./vis-dev nightly           # gate --fast + fuzz parser + vòng tròn fmt
./vis                   # chạy `vis` từ workspace
```

`bin/vis-linux` (đồng bộ + chạy trên host Linux) và `bin/vis-remote` (chạy một lệnh trên
`mps`/`psvn`; máy chủ đạo **`ps`** chạy tại chỗ — ST-259) vẫn dùng được; danh sách đồng bộ là
`SYNC_DIRS`/`SYNC_FILES` ở đầu `bin/vis-linux`.

## 5. Điều độ Công việc theo Khung giờ DeepSeek (`VD-287`)

Mục tiêu: **tối ưu 50% chi phí token** và giữ thông lượng máy 24/7 bằng cách tách tuyệt đối
**chạy nặng tự trị (0 token LLM)** vào giờ cao điểm và **phát triển / refactor** vào giờ thấp điểm.

### Khung giờ chuẩn (theo lịch chính thức DeepSeek API — nguồn gốc là **UTC**)

| Chế độ | Khung giờ **UTC** | Quy đổi **JST (UTC+9)** | Quy đổi **ICT / VN (UTC+7)** | Giá token | Loại việc bắt buộc |
|---|---|---|---|---|---|
| **Cao điểm 1** *(3h, T2–T6)* | `01:00–04:00` | `10:00–13:00` | `08:00–11:00` | **2×** | **Chỉ chạy nặng 0-LLM**: `vis-gate-macos`/`windows --full` ×3, VM watchdog 2h (`fuzz_mutate`), fuzz parser 3h, verifier 100M, `vis-diff` 5×100k |
| **Thấp điểm trưa** *(2h, T2–T6)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | **0,5×** | Đọc kết quả log Khung 1, vá lỗi ngắn / refactor gọn, chuẩn bị commit sạch & xếp job 4h cho Cao điểm 2 |
| **Cao điểm 2** *(4h, T2–T6)* | `06:00–10:00` | `15:00–19:00` | `13:00–17:00` | **2×** | **Chỉ chạy nặng 0-LLM**: soak **4h** `psvn` (Windows) & `mps` (macOS), fuzz parser **4h** (`ps`), duy trì soak **24h** nền trên `ps` |
| **Thấp điểm chính** *(15h, T2–T6)* | `10:00–01:00` | `19:00–10:00` | `17:00–08:00` | **0,5×** | **Phát triển tính năng, refactor kiến trúc đa luồng (`W1..W4`)**, viết test mới, code review, tổng hợp báo cáo `R<n>` |
| **Cuối tuần & lễ TQ** *(63h liên tục)* | T6 `10:00` → T2 `01:00` | T6 `19:00` → T2 `10:00` | T6 `17:00` → T2 `08:00` | **0,5×** | Toàn thời gian thấp điểm — ưu tiên phát triển tính năng & refactor lớn, kết hợp soak 24h chạy nền song song |

*(Lưu ý: ngày lễ công cộng Trung Quốc cả ngày là thấp điểm theo quy định DeepSeek API; mốc giờ trên tính theo UTC, không đổi theo mùa.)*

### Quy tắc vận hành bắt buộc cho Agent 2

1. **T−5 phút trước giờ Cao điểm** (`00:55` / `05:55 UTC`): hoàn tất chỉnh sửa hiện tại, commit gọn (không bỏ dở biên dịch, **không** kill tiến trình nền đang chạy), kích hoạt **một** chuỗi lệnh nặng tự trị (`nohup`/`tmux`, ghi log ra ngoài repo `/home/ps/vislogs/` kèm `sha256sum` ở cuối), rồi **dừng gọi LLM** cho đến đầu khung thấp điểm kế tiếp.
2. **Trong giờ Cao điểm**: **cấm** mở phát triển tính năng/refactor mới bằng LLM và **cấm** vòng lặp LLM polling trạng thái tiến trình nền (`sleep` + gọi lại đốt token). Nếu đang dở tác vụ code lúc chạm giờ cao điểm ⇒ commit checkpoint sạch vào nhánh làm việc, ghi ngắn vào `agent/JOURNAL/`, chuyển máy sang chạy tải nặng không LLM.
3. **Trong giờ Thấp điểm**: tập trung tối đa cho thiết kế, viết mã, sửa lỗi, refactor và tổng hợp báo cáo. Khi đọc log từ đợt chạy nặng, chỉ đọc dòng kết luận cuối + `sha256` để tiết kiệm ngữ cảnh.
4. **Tối ưu Prompt Cache & Chọn tầng mô hình**:
   - Chỉ nạp `agent/ACTIVE.md` + phần đuôi liên quan của `agent/QUEUE.md`/`agent/DECISIONS.md` (**không** đọc toàn tệp > 350 KB mỗi lượt) để giữ prefix ổn định và đạt tỷ lệ cache-hit tối đa.
   - Tác vụ cơ học (chia tệp, sửa import, `fmt`/clippy, gom bảng báo cáo, kiểm R1..R9) ưu tiên mô hình nhanh/rẻ (`deepseek-flash`); chỉ dùng mô hình suy luận sâu (`deepseek-v4-pro`) cho bài toán kiến trúc, hệ kiểu, VM/async và bảo mật (`openat2`/`statat`).

## 6. Quản trị Nhánh, Worktree & Đóng băng Mốc Nghiệm thu (`VD-288`)

1. **Vòng đời Nhánh & Worktree (chống phình nhánh/đĩa)**:
   - Ngay sau khi nhánh `plan/*` đã merge vào `main` (hoặc ngay sau khi Agent 0 duyệt `DONE`), **bắt buộc dọn trong vòng 24h**: kiểm tra không còn tiến trình chạy & `status` sạch → `git worktree remove <wt>` → `git branch -d <branch>` (dùng `-d` thường) → `git push origin --delete <branch>` → `git fetch --prune` & `git worktree prune`.
   - **Trần thường trực**: tối đa **≤ 8 worktree** hoạt động cùng lúc (`W1..W4` + QA/soak); **0 nhánh `plan/*` đã merge** tồn đọng trên `origin` quá 24h; chạy `bin/vis-gc-targets` định kỳ giữ đĩa trống > 10 GB.
   - **Không tạo/push nhánh dài ngày chỉ cho 1 tệp journal trung gian**: nhật ký QA sau khi merge/cherry-pick vào `main` thì xóa nhánh ngay; nháp chưa chốt lưu tại `/home/ps/vislogs/` (luật R7). Nhánh chưa merge chỉ được xóa sau khi kiểm `git cherry -v main` + `git diff --name-only main...<branch>` xác nhận không mất mã sản phẩm và không cắt đứt tham chiếu trong `agent/reports/R<n>-viscript.{md,json}`.
2. **Quy trình Pre-Freeze 3-OS trước khi đóng băng mốc `H` cho soak dài**:
   - **Cross-OS Clippy Gate**: bắt buộc chạy `cargo clippy --workspace --all-targets -- -D warnings` trên **cả Linux (`ps`), Windows (`psvn`) và macOS (`mps`)** trước khi mở soak 4h/24h (tránh lỗi OS-only lọt qua `vis-gate-windows --full`).
   - **Smoke Soak 1 chu kỳ (~10–15 phút) trên cả 3 OS**: chạy trước khi mở soak 4h/24h để phát hiện sớm race song song của harness/TMPDIR/lock.
   - **Tách rạch ròi `[ST-00]` docs-only khỏi `H_code`**: commit `[ST-00]` sau khi đóng băng `H_code` chỉ sửa `AGENTS.md`/`agent/**` (`git diff H_code..HEAD -- crates/ vis-host-*/ bin/ test/ Cargo.*` rỗng) nên **không vô hiệu hoá** bằng chứng soak/fuzz/full-gate đang chạy hoặc đã đo trên `H_code`.
