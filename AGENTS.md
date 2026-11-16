# AGENTS.md — dòng ViScript (tóm tắt)

Bản đầy đủ: [`../AGENTS.md`](../AGENTS.md). Tệp này chỉ thêm phần riêng của `viscript/`.
Nguồn sự thật của vòng vá: [`../docs/QUYET-DINH-sau-review.md`](../docs/QUYET-DINH-sau-review.md) (**QD**), nhất là §7.

## 1. Thứ tự nguồn sự thật (áp cho ViScript)

1. **Code + test** trong `viscript/`: crate biên dịch được, golden, fuzz, TAP.
2. **`SPEC`** — [`../docs/viscript/SPEC-lex.md`](../docs/viscript/SPEC-lex.md) (từ vựng + EBNF tự chứa) và `SPEC.md` (do ST-02a/ST-02b sinh); sửa **chỉ** qua Agent 0.
3. [`agent/DECISIONS.md`](agent/DECISIONS.md) — VD-01 trở đi; số mới nhất: `grep -o 'VD-[0-9]\+' viscript/agent/DECISIONS.md | sort -t- -k2 -n | tail -1` (QD §7, các câu Q1..Q5 của SPEC-lex, và các chỗ văn phạm chế độ lệnh/vis-fmt/vis-diag cần chốt).
4. Tài liệu thiết kế [`../docs/viscript/ViScript-thiet-ke.md`](../docs/viscript/ViScript-thiet-ke.md) — tham chiếu.

QD thắng tài liệu thiết kế.

## 2. Tài liệu và điều khiển

| Tệp | Vai trò |
|---|---|
| [`../docs/viscript/ViScript-thiet-ke.md`](../docs/viscript/ViScript-thiet-ke.md) | thiết kế đầy đủ (9.4 bảng xung đột chế độ lệnh, 14 cấu trúc crate, 20 quy trình, 22 quyết định) |
| [`../docs/viscript/SPEC-lex.md`](../docs/viscript/SPEC-lex.md) | đặc tả từ vựng + EBNF tự chứa — sản phẩm của **ST-02a** |
| [`agent/QUEUE.md`](agent/QUEUE.md) | mọi vòng; trạng thái từng task là nguồn duy nhất về tiến độ |
| [`agent/DECISIONS.md`](agent/DECISIONS.md) | VD-01 trở đi; số mới nhất: `grep -o 'VD-[0-9]\+' viscript/agent/DECISIONS.md | sort -t- -k2 -n | tail -1` |
| [`agent/STATE.md`](agent/STATE.md), [`agent/BLOCKED.md`](agent/BLOCKED.md) | trạng thái + chặn (Agent 2 ghi) |
| `agent/JOURNAL/`, `agent/reports/` | nhật ký ngày, `R<n>-viscript.md` + `.json` |

## 3. Trạng thái duyệt hiện tại

**Trạng thái dòng ViScript**: không ghi ở tệp này. Đọc theo thứ tự: viscript/agent/ROADMAP.md (8 hạng mục × mức thật), báo cáo mới nhất `ls viscript/agent/reports/ | sort -V | tail -1`, rồi viscript/agent/QUEUE.md. Tệp luật không chứa số đếm (số vòng, số test, VD mới nhất) để không lỗi thời.
**Quy trình hiện hành**: VD-119 (Agent 0 soạn, Agent 2 dán nguyên văn, commit [ST-00]).
