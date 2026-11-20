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
**Quy trình hiện hành**: VD-119 (Agent 0 soạn, Agent 2 dán nguyên văn, commit [ST-00]).

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
