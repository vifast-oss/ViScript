<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-239 — review Agent 1 trước merge (VD-242 / Đợt B)

**Nguồn**: worktree `vd242-st239-option`, commit Agent 2 `72f5ea6`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Từng finding

| Finding | Kết quả đúng / căn cứ | Bằng chứng Agent 2 | Verdict |
|---|---|---|---|
| **#161** `?.` base optional | VM đúng, interp thiếu `lift_opt(1)`; `None⇒None`, `Some(u)⇒Some(u.field)`, không flatten; field `str?` ⇒ `str??` — VD-242 §3.2 | interp gọi chung `$lift-opt`; cả hai `Some(An) Some(An) An None`; **không đụng VM** | **SHIP / đóng #161** |
| **#162** `?.` base thường | Chương trình **không hợp lệ** ⇒ `VS0412`, rc 2 — VD-242 Q2=A | `tyck/truong.rs` từ chối base không `Opt`; cả hai cùng lỗi | **SHIP / đóng #162** |
| **#165** or-pattern bind không đầy đủ | Tên dùng trong guard/thân mà không bind ở **mọi** alternative ⇒ từ chối; **không sửa VM trả `2`** — VD-242 §3.1 | `tyck/pat.rs::kiem_or_bind`, cả nhánh `Match` statement/expression; 1 ca hợp lệ không bị từ chối oan | **SHIP / đóng #165** |
| **#166** `set{}` dedup | interp đúng ⇒ VM dedup theo `value::eq`, giữ phần tử đầu | `MakeSet` tách riêng trong `vm.rs`; cả hai `[1,2,3]` | **SHIP / đóng #166** |
| **#170** map khoá trùng | literal tĩnh trùng ⇒ lỗi checker; runtime last-wins; `json.decode` last-wins (lựa chọn ViScript, không phải luật JSON); equality so ánh xạ — VD-242 Q4a/§3.4 | `static_key` chỉ nhận literal, **không gọi mã người dùng**, bắt cả `1` vs `1.0`; `MakeMap` last-wins; `eq_value` gộp map rồi so; doc ghi rõ lựa chọn | **SHIP / đóng #170** |

## 2. Điểm còn mở (không chặn merge, nhưng phải ghi sổ)

1. **Option coercion — field/index assignment chưa nâng.** Checker chấp nhận `u.f = 5` (`f: int?`) và `m["a"] = 7` (`map[str,int?]`), nhưng **cả hai engine lưu giá trị thô** (`5`, `7`) thay vì `Some(5)`/`Some(7)`. VD-242 §3.3 yêu cầu áp ở field/index khi kiểu đích đã biết ⇒ đây là **khoảng trống triển khai**, không phải ngữ nghĩa mới. Cần task riêng ST-248 (`codegen.rs`/`than.rs` + `interp.rs`), nằm ngoài quyền của ST-239/ST-240 hiện tại. **Không được coi là đã xong hợp đồng Option** cho tới khi ST-248 đóng.
2. **Fixture `test/sem/**`: ST-239 chuẩn bị 8 fixture rồi **xoá và revert count**, vì thêm làm đổi số cứng trong `cli/tests/diag_cmp.rs` (525→533, 437→442, 53→56) và `vis-bytecode/tests/{roundtrip,tin_tyck}.rs`, mà `cli/**` nằm trong danh sách cấm của task. Regression Rust `st239_parity.rs` **8/8** đã phủ 5 finding; không mở rộng corpus trong đợt này để tránh churn số cứng.
3. **Docs `T??`**: đã được sửa sẵn (`docs/ViScript-thiet-ke.md` §2, §4.4, §7.3, VD-19, grammar, bảng tóm tắt); không còn chỗ nào nói `T??` là lỗi kiểu. Không cần hành động.

## 3. Gate

Agent 2 báo tại worktree `72f5ea6`: `./vis-dev check` PASS · `gate --fast` PASS · `vis test` 525/525 · `--vm` 525/525 · `st239_parity.rs` 8/8. Agent 1 sẽ chạy lại trên commit tích hợp `main`, không dùng kết quả worktree thay kết quả main.

## 4. Kết luận

**SHIP 5/5 finding.** Không có ngữ nghĩa chưa có luật phát sinh; mọi từ chối/đầu ra bám VD-242 §3.1–3.4. Merge `72f5ea6`, đóng #161/#162/#165/#166/#170, mở ST-248 cho phần field/index còn thiếu.
