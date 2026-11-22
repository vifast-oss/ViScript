# Audit tài liệu kho ViScript — 2026-11-19 ~21:00

> Agent 2 → A0/Agent 1. Survey-line: sau đợt Gate 2 + narrow freeze + commit snapshot revamp.
> Tính đến HEAD `8d68a0c`. **Không còn "backlog stale" ngoài hàng đợi sau đây.**

## 1. Trạng thái hiện tại (máy đo top tại HEAD)

| Hạng mục | Trạng thái |
|---|---|
| **`main` HEAD** | `8d68a0c` (pushed) |
| **`pha1-rc1`** | `05e42d0…` (nguyên) |
| **SQLite Gate 2** | **Merged** `main` `8355022`; **narrow freeze** `+9 std.sqlite.*` vào `agent/API-STD.txt` (CODES/VBC1 không đổi) |
| **windows (psvn)** | build OK; sem 525/525 ×2; sqlite 15/15; CRLF 3/3; smoke CSV 22/22 |
| **macOS (mps)** | KHÔNG verified (máy bận costante, load ~5–7) |
| **Windows Tier 2** | giữ nguyên (không Tuyên bố Tier nào ngoài đã verify) |
| **`sem` count** | **475 `.vis` + 50 `.vish` = 525** |
| **macOS VM-54 / host-guard** | **chưa verify** sau khi đổi code (Linux VM 54/54 + macOS đã chạy trước đó ở commit cũ); giữ nguyên |

## 2. Những gì đã xong trong đợt này (liệt kê)

1. **`agent/SNAPSHOT-2026-11-19.md`** — thêm banner SUPERSEDED + pointer `SNAPSHOT-2026-11-20.md`.
2. **`agent/SNAPSHOT-2026-11-20.md`** — refresh: HEAD `c23c013` → `c333e8f` → `adc6a18` → `8d68a0c`; bảng Gate 2 executable; typo `Mic:` → `macOS:`; `path-bindingfixtures` → `path-bindings`.
3. **`agent/LICH-SANG-2026-11-20.md`** — thêm banner SUPERSEDED + addendum Gate 2; nội dung cũ giữ lịch sử.
4. **`agent/QUEUE.md`** — nhánh `[HOÀN TẤT 2026-11-19] SQLite Gate 2` với verified evidence.
5. **`CHEAT-SHEET-ViScript.md`** — `derives` chỉ 3 (`Eq`/`Show`/`Json`); keyword list bỏ alias/newtype (khớp code); bảng `match` fix lỗi `\\n` thô ở line 100; mục 9 trùng số ở §12 đã re-number; `target/debug/vis @ f4005aed` → `vis test test/cheatsheet` (hiện tại, không commit-version cứng).
6. **`docs/ViScript-thiet-ke.md`** — §2.1 từ khoá bỏ alias/newtype; §248 "Eq/Ord/Hash/Show/Json/Default" → "Eq, Show, Json"; §3.6 drop `Ord`/`Hash` reference.
7. **`README.md`** — mục "Cấu trúc repo" liệt kê đúng `bin/vis, vis-dev, vis-remote, vis-linux, vis-soak, vis-targets, vis-diff` + note `infra/` + correct count (8 crate).
8. **`agent/RFC-TYPED-SERIALIZATION-2026-11-19.md`** — banner SUPERSEDED (HEAD iệt cập; phase order sau Gate 2; path `crates/vis-interp/`; refs §3/§11 đã chỉnh; link trực tiếp tới `agent/SO-NO.md`).
9. **`agent/BAO-CAO-VD-SQLITE-MVP-2026-11-19.md`** — missing `agent/` prefix (`CHEAT-SHEET/VISCRIPT-ERGONOMICS-FACTS.md` → `CHEAT-SHEET-ViScript.md`/`agent/VISCRIPT-ERGONOMICS-FACTS.md`); ký tự CJK 狭 → `hẹp`.
10. **`agent/SO-NO.md`** — "HEAD khi cập nhật: `d6111c1`" → `c333e8f`.

## 3. Trạng thái các "mục còn treo" (đọc file gốc, không lặp lại nội dung wiki)

- **Windows L0 portability VD** — chờ pha 1 đóng chính thức (đã đóng @ `05e42d0`) + A0 ratify (điều kiện & bước đã có). Không có điều kiện ẩn ngoài việc A0 mở VD riêng.
- **Phase B-3 impl xuyên module + bound generic** — chờ A0; đây là task lớn nhất còn mở.
- **#147** (macOS performance, VD-197) — chỉ thực hiện khi macOS rảnh.
- **#152** (VM vs interp cùng HEAD, ngưỡng 1,10) — chỉ thực hiện khi macOS rảnh.
- **Typed serialization RFC** — đã soạn, chưa phê duyệt; implementation chưa duyệt.
- **PO-Windows portability ~101 ca** — chờ pha 1 đóng chính thức + A0 ratify mở VD.
- **CSV workflow MVP** — đã ship `96451f1`; không mở thêm trường hợp mới (CSV pack, output flags, .vish integration) trừ khi A0 mở.

## 4. Những điều KHÔNG phải làm lặp lại trong đợt sau

- **Không tự `VIS_FREEZE_REGEN=1`** ở gate/nightly; chỉ khi A0 ratify explicit.
- **Không** ghi đè/sửa `agent/API-STD.txt` trừ khi A0.
- **Không** mở API/spec (`SPEC-lex.md`, `docs/ViScript-thiet-ke.md`, `SPEC.md` không có).
- **Không** tuyên bố "macOS/Windows verified" mà chưa có máy rảnh + đo thật.
- **Không** tự mở phase 2 VD mới (B-3, typed serialization impl, Windows portability, Tier 1) khi chưa có lệnh A0 — dù có thể thấy "đứng yên" vẫn ahead kham.

## 5. Những task được phép (không cần A0 duyệt riêng)

- **Fix docs stale/logice** — cần làm khi (i) thực tế thay đổi (ví dụ test count), hoặc (ii) một file tự mô tả lỗi thời (ví dụ: "Giờ chỉ 3 derive").
- **Cập nhật snapshot** — mỗi Gate push, mỗi batch docs.
- **Naming consistency** (ví dụ: `agent/VISCRIPT-ERGONOMICS-*.md` root → `agent/`) — đã xong đợt này.

## 6. Đường đi nếu A0 muốn mở phase 2

- Xem `agent/ROADMAP-PHA2-NHAP.md` (ứng viên). Tùy A0 chọn, thứ tự sau:
  1. `#151` (RULE-EXPORT; #/.spec pragma)
  2. `#149` (split-path — audit liệt kê)
  3. `lambda annotation` (cần trước #155)
  4. `#155` lift closure-params (phụ thuộc A0-3)
  5. `T??` lift params return
  6. `#154` parse `?` + length-units
  7. `#152` VM perf
  8. `#147` macOS soak

## 7. Ràng buộc duy trì (sau đợt này)

- **Windows Tier 2** giữnguyên @ `349af67`; không mở rộng tuyên bố.
- **pha1-rc1** không di chuyển; **`./vis-dev gate --fast`** phải xanh trước mỗi merge.
- **fresh-clone** gate @ HEAD phải PASS trước và sau mỗi docs commit (chỉ cần làm khi push lên main).
- **Windows test-suite portability** **không** trường hợp "phần thưởng" — đợi A0 duyệt chính thức sau pha 1.

---

**Trạng thái tài liệu A0 đang đọc**: bản này thay thế `agent/SNAPSHOT-2026-11-20.md` làm điểm vào cho pha 2; nội dung bổ sung cho `SNAPSHOT-2026-11-19.md` (nay có banner superseded) và `LICH-SANG-2026-11-20.md`.
