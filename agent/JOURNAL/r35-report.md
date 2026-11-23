# R35 — md+json cho `v0.4.5` (prep)

**Nhánh**: `plan/r35-report` từ `9af3976`. **Loại**: docs/tài liệu báo cáo; không code.
Không merge/push/tag; không GitHub Actions.

## Đã làm

- `agent/reports/R35-viscript.md`: báo cáo đầy đủ GĐ2 `v0.4.5` (ST-297/298/299a/299b/300),
  bằng chứng Linux/Windows/WASI, `#149` đóng/`#193` mở, gate/release, Tier 1 chưa công bố;
  tag trước `v0.4.0 @ c2accdf`, `v0.4.1 @ 90844bc`.
- `agent/reports/R35-viscript.json` + `.json.license`: 11 khoá, `round=35`, `head=9af3976`,
  **330 task** phủ đủ 330 hàng QUEUE@head, 10 blocked, commits có thật; không placeholder
  `[SOAK_*]`.
- `test/report/gen_report_json.py --round 35`: sinh lại JSON từ `QUEUE@HEAD_SHA`; đổi `HEAD_SHA` khi A0
  chốt soak/release rồi chạy lại.
- Placeholder soak/Tier 1 **chỉ** trong R35 md: `[SOAK_ROUNDS]`, `[SOAK_FAIL]`, `[SOAK_FD_RSS]`.

## Kiểm

- `python3 test/report/check_report.py --all`: **PASS (34 báo cáo, 330 hàng QUEUE)**.
- `./vis-dev check`: **PASS** — R1..R7, fmt/clippy/doc-examples/fixtures.
- Đĩa còn **21 G**; 0 unsafe; không sửa `QUEUE.md`/`SO-NO.md`/`DECISIONS.md`.

## Blocker

Không. A0 cập nhật `head` (sau soak/release), điền `[SOAK_*]` trong R35 md, chạy lại
`gen_report_json.py --round 35` + `check_report.py --all` rồi merge/tag `v0.4.5`.
