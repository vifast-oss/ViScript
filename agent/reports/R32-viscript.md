# R32 — ViScript, vòng 32 (khung chuẩn bị, ST-289 · `v0.3.6`)

> **Trạng thái: KHUNG CHUẨN BỊ (2026-11-22)** — báo cáo này mới phủ phần **ST-289** làm được ngay
> (đồng bộ phiên bản, đóng `#111`/`#151`, docs hiện thực). Số đo đầy đủ của vòng 32 **chờ
> ST-284..ST-288** (R0 Refactor, hook huỷ HTTP, `VS0708`/`#142`) merge vào baseline; khi đó R32
> được chốt lại (workspace/fresh-clone/`psvn`/fuzz) và thay thế phần khung này.
>
> Baseline `head` của khung: **`1c58ba3`** (sau `VD-272`..`VD-279`, trước GĐ 0).
> Không tag, không push, không GitHub Actions.

## 1. Phạm vi

Vòng 32 mở chương trình dài hạn hậu `v0.3.5` theo `VD-272`..`VD-279`; Giai đoạn 0 (`v0.3.6`) gồm
sáu lát: R0a/R0b/R0c refactor 0 đổi hành vi (ST-284..ST-286), hook huỷ HTTP (ST-287), khép G3 gói
(ST-288) và đồng bộ sự thật (ST-289). Báo cáo này **chỉ** chốt phần ST-289; các lát còn lại đang
ở nhánh riêng, chưa merge — mọi số đo toàn workspace dưới đây là **số của baseline**, không phải
số nghiệm thu cuối vòng.

## 2. Baseline được đo

- `head` = `1c58ba3fed78534bf4d8eb12d04814433cc365d0` (tổ tiên HEAD; QUEUE `ST-284..309` đã có).
- Tag gần nhất: **`v0.3.5`** @ `03bb7b9` — giữ nguyên, không di chuyển.
- Linux (`ps`): `gate --fast` **PASS** tại baseline của khung (xem §5); `vis --version` = `vis 0.3.6`.
- Windows: bằng chứng hiện hành vẫn là tag `v0.3.5` @ `03bb7b9` — workspace **3528/0/3**
  (debug + release), P2 10/10, P3 10/10 (`JOURNAL/vs3-w4-tag-v035.md`); **Tier 2**, chưa tuyên bố
  hỗ trợ. Vòng này **không** đo lại Windows.
- macOS: không có máy Darwin thật — **không tuyên bố** (giữ nguyên `VD-242 Q7`). wasm32: chỉ `cargo check`.

## 3. Đã xong trong khung (ST-289, phần chuẩn bị)

1. **Phiên bản một nguồn**: `[workspace.package] version = "0.3.6"`; toàn bộ 8 crate thành viên dùng
   `version.workspace = true` (vis-bytecode/vis-interp/vis-vm chuyển từ số chép tay). `vis --version`,
   `vis version`, `vis version --json`, `vis doctor` đều in `0.3.6`. **Không** đổi `VERSION` VBC1 (2),
   `INSTRUMENTED_VERSION` (3), event-table (4), weight-table (1), opcode hay định dạng cache; khoá cache
   đổi **có chủ ý** vì `PHIEN_BAN` nhúng `CARGO_PKG_VERSION` (VD-129) — cache bản 0.1.0 tự vô hiệu,
   ghim lại trong `cache::tests::khoa_cache_ghim`.
2. **`#111` đóng**: gán `json.decode(...)` vào biến `cap[Dir]` là lỗi tĩnh **VS0412** (exit 2), khoá
   bằng `crates/vis-cli/tests/cap_khong_lo.rs` ca (1b) `gan_json_vao_bien_cap_khong_sinh_quyen`.
3. **`#151` đóng**: fixture `test/doc-examples/` đổi sang slug ổn định
   `doc-<slug-tài-liệu>-<slug-tiêu-đề>-<số-thứ-tự-trong-mục>.<ext>`; 24 tệp đổi tên **thuần**
   (0 đổi byte nội dung), `--regen` tái lập; allowlist vẫn theo `tệp:dòng` (ngoài phạm vi #151).
4. **Docs hiện thực**: `docs/ViScript-thiet-ke.md` §Mức hiện thực chỉ sửa phần đã kiểm (phiên bản,
   async chọn theo HIR, `http2` default-on, Windows `v0.3.5`); `agent/HOST-BAO-DAM.md` cập nhật bằng
   chứng Windows `v0.3.5`; `agent/SO-NO.md` đóng `#111`/`#151` (§8 **192 = 94 mở / 73 đóng / 22 VS3 /
   2 không làm / 1 điều hướng**).

## 4. Chờ ST-284..ST-288 (điều kiện chốt R32)

- **ST-284/ST-285/ST-286**: R0a/R0b/R0c — mọi tệp lõi ≤ 1.250 dòng, 0 đổi ABI/cache/hành vi,
  `gate --fast` + `diff 100k` = 0 lệch.
- **ST-287**: hook huỷ giữa HTTP request (h1 + `async-h2`), không rò fd, parity interp↔VM.
- **ST-288**: cấp `VS0708` (`vi/en/ja`) + khoá `pkg gc --root` + vá `#142`.
- Khi cả năm lát `DONE`: chốt R32 đầy đủ — `gate --fast`/đầy đủ, fresh-clone Linux, verify `psvn`,
  rà `SO-NO.md` các nợ đã đóng kèm commit hash, cập nhật `perf`/`coverage`/`sim` trong
  `R32-viscript.json`, và chỉ khi đó mới mở Giai đoạn 1 (`ST-290..`).

## 5. Cổng

- `fast`: **PASS** trên Linux (`ps`) tại khung này — `vis-dev gate --fast` gồm fmt, clippy, luật
  R1..R6, self-test, doc-examples (24 khối: 19 xanh + 5 ngoại lệ), L0 nextest
  **3573 passed / 0 failed / 3 skipped** (`CARGO_INCREMENTAL=0`, log phiên `/tmp/st289-gate.log`).
- `full`: **—** (không chạy trong khung; sẽ chạy khi chốt R32).
- `nightly`: **—** (không chạy trong khung).
- Kiểm báo cáo: `test/report/check_report.py --all` PASS (R32 phủ đủ 322 hàng QUEUE, 20 hàng
  `BLOCKED` có câu hỏi).

## 6. Ghi chú sổ sách

`head` là **baseline sản phẩm được đo**, không phải commit chứa báo cáo này (tránh tự tham chiếu
hash). `ST-289` ghi `status = DONE` với `queue = READY` vì QUEUE tại `head` chưa lật trạng thái
(nó phụ thuộc ST-284..288); khi R32 chốt, hàng QUEUE và `status` được cập nhật cùng lúc. Các hàng
`ST-212/ST-241/ST-243/ST-248` còn `READY` trong QUEUE là **hàng lịch sử** (đã có việc tương ứng
trong JOURNAL), không thuộc Giai đoạn 0. Không dùng các số xanh của khung để tuyên bố Giai đoạn 0
hoàn tất.
