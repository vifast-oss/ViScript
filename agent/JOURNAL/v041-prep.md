# v0.4.1 prep — version/cache/CHANGELOG/R34

**Nhánh**: `plan/v041-prep` từ `main` `a1730c5`. **Loại**: docs/version-only; không sửa
`QUEUE.md`/`SO-NO.md`/`DECISIONS.md`/R33 JSON. Không push/merge/tag; không GitHub Actions.

## Đã làm

- Workspace **0.4.1** (`version.workspace`); `vis --version` = `vis 0.4.1`.
- `PHIEN_BAN` thêm `+v041-release-2026-11-23`; khoá cache mới
  **`42f5389364ae126f8b9dea4c3bf107c983c3b92d7b1743a5293d547f3d4d7c0f`**;
  `assert_ne` khoá 0.4.0 `22572d0b…3a8b`; cache 0.4.0 tự vô hiệu.
- `CHANGELOG.md`: `## v0.4.1` — ST-296a/b/c/d + số đo; tag `v0.4.0 @ c2accdf`; v0.4.1 chưa tag,
  tag trên commit R34 sau final QA Linux/Windows.
- `agent/reports/R34-viscript.md`: báo cáo đầy đủ 6 mục (không phải draft), **không tạo R34 JSON**
  (R33 JSON giữ baseline `90cfc39`, A0 chốt JSON sau).
- `agent/ROADMAP.md` + `agent/ACTIVE.md`: v0.4.0 đã tag `c2accdf`; ST-296a/b/c/d DONE/merged;
  ST-297 READY `plan/w2-st297-paths` @ `a188f15` (gate 3660/4, 4 ca mới) chờ sau v0.4.1;
  ST-298 BLOCKED.

## Cổng

- `./vis-dev gate --fast`: **PASS — 3656 passed / 4 skipped** (R1..R7 OK; `vis-dev check` PASS;
  L0 214,6 s).
- `python3 test/report/check_report.py --all`: **PASS (33 báo cáo, 329 hàng QUEUE)**.
- fmt/clippy sạch; `vis --version` **0.4.1**; đĩa còn **15 G** (>10 G).

## Blocker

Không. Việc còn lại ngoài commit: final QA Linux/Windows để tag `v0.4.1` trên commit R34; sau tag
xét merge ST-297 và xử lý ST-298.
