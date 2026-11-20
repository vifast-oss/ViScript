<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258 — kế hoạch 4 luồng song song (A0 authority, 2026-11-20 → 2026-11-21 09:00 JST)

> **Căn cứ**: review 2 ngày 18–20/11 (VD-257 và trước) + quyết định A0 trong phiên này.
> **Mục tiêu**: sửa lỗi tính đúng/soundness và các blocker của runtime mới; cải thiện
> tooling/test; thêm một tính năng nhỏ có test. **Không** chuyển runtime mặc định, không
> tag/release, không sửa SPEC.

## 1. Bốn luồng (mỗi luồng một worktree riêng)

| Luồng | Worktree | Phạm vi chính | Ưu tiên |
|---|---|---|---|
| A — checker/type soundness | `vd258-checker` | `vis-types`, `lift.rs`, Option lift trong `interp.rs`/codegen | `Result?`; record ctor/default; `None`; `?.` lồng; lift ở ctor/`with`/`const`/container; multi-file trait/ctor |
| B — frame/async resume | `vd258-frame` | `frame/**`, `rt/sched.rs`, `vm_frame.rs` | `Redispatch` stack; `CallMethod` Pending; `RedispatchMethod`; deadlock exit 0; closure env; const env; fake-source final-join; exit code |
| C — fee/budget + tooling | `vd258-fee-tools` | `vis-bytecode/fee.rs`, fee lines, toolchain/infra, core test gating | `verify_fee` OOB/panic; `for` E_LOOP; call/method fee parity; MSRV 1.100.0; core test xanh |
| D — SQLite/stdlib/feature | `vd258-sqlite-stdlib` | `vis-cli/db/**`, `stdlib.rs`, `builtin_ls.rs`, `lsp/**` | cell cap/pre-materialization; non-UTF-8 panic; tombstone; `json.encode` Float; **feature `std.path` Windows** |

## 2. Hợp đồng chung

- Mỗi luồng chỉ sửa worktree của mình; commit local `[VD-258][...]`, không push/merge.
- Không xoá/nới test, không đổi golden trừ khi có lý do và ghi rõ.
- Mỗi fix phải có test đỏ-trước (hoặc test chứng minh hành vi cũ sai) + chạy targeted tests.
- Default runtime gating (`frame.enabled = false`, budget/quota tắt) phải giữ nguyên.
- `./vis-dev gate --fast` trên cây tích hợp là cổng cuối; không chấp nhận “xanh một phần”.

## 3. Thứ tự tích hợp (dự kiến)

1. A (soundness) — vì B/C/D có thể dựa vào kiểu/coercion đúng.
2. B (frame) — độc lập file, nhưng test dùng `lift.rs` của A.
3. C (fee/tooling) — sửa toolchain/core test để cổng cuối có ý nghĩa.
4. D (SQLite/feature) — cuối, ít phụ thuộc.
5. Chạy `./vis-dev gate --fast`, `cargo test -p vis-cli --no-default-features --test sem`
   (kỳ vọng xanh sau C), `./vis-dev check`, kiểm tra cây sạch, cập nhật QUEUE/report.

## 4. Quyết định A0 trong phiên này

- **Không** đổi runtime mặc định hoặc bật frame/budget/quota; mọi sửa frame chỉ trên đường test.
- **Không** đổi `SPEC-lex.md`; nếu phát hiện lệch spec, ghi report cho A0 sau.
- **Không** mở rộng API công khai mới ngoài `std.path` (feature nhỏ, có test).
- `verify_fee` panic là blocker của cổng budget, không được coi là “test-only”.
- `test/toolchain` đỏ và core test đỏ phải được sửa trong luồng C, không bỏ qua.

## 5. Bằng chứng cần nộp

- Commit hash từng luồng, file thay đổi, lệnh test + số pass/fail.
- Trước/sau cho các repro chính (record ctor, `None`, `?.`, `Result?`, `Redispatch`,
  `CallMethod`, deadlock, SQLite RSS, `json.encode`, toolchain).
- Cây tích hợp sạch + gate PASS + danh sách gap còn lại.
