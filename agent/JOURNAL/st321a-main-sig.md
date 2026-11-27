<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-321a — tyck tĩnh chữ ký `main`: đóng `#200` (P2) + `#201` (P3)

- **Nhánh**: `plan/st321a`, nền `main` = `ff585af8aad8156e30b288b93f2de321305f83ea`;
  worktree `/home/ps/viswt/st321a`. **Không** sửa repo chính (agent khác đang merge vào `main`).
- **Thiết kế**: `/home/ps/vislogs/audit-200-201-20261126T225729Z.md` (sha `0ede8f7f…`),
  **Lane A** — một nguồn kiểm tĩnh trong `vis-types`, không sửa engine, không bump cache.
- **Quy trình**: VD-279 — commit `[ST-321a]` sản phẩm + journal; **không merge/push**, chờ Agent 1
  nghiệm thu. Đĩa lúc kết thúc: **6,5 GB** (≥ ngưỡng dừng 6 GB); `TMPDIR` ngoài repo
  `/home/ps/vislogs/st321a-tmp`, `CARGO_BUILD_JOBS=2`, không `cargo clean`.

## 1. Patch (tyck-only)

| Tệp | Thay đổi |
|---|---|
| `crates/vis-types/src/tyck/main_sig.rs` (mới, ~85 dòng) | `kiem(hir, &mut Checker)`: chọnentry `HItem::Fn` có `name == "main"`, `kind == FnKind::Fn`, `span.file == 0` (tránh `main` của module); đọc `Checker::sigs[ f.sym ]` **đã resolve** (không gọi lại `resolve_ann` ⇒ không trùng `VS0301`); chụp `(kiểu, has_default)` rồi mới ghi chẩn đoán (tránh borrow); span = `HParam.span`, thứ tự theo nguồn |
| `crates/vis-types/src/tyck/mod.rs` | `mod main_sig;` + gọi `main_sig::kiem(hir, &mut c)` **sau** `const_cycle::kiem`, trước `c.finish()` |

Luật (`cap[...]` = `TypeRef{name:"cap",..}` được bỏ qua):

| Trường hợp | Hợp lệ | Mã khi sai |
|---|---|---|
| không `cap` | `main()`; tham số 0 = `list[str]`; các tham số sau **có `default`** | `VS0412` / `VS0401` |
| có `cap` | tối đa **1** tham số non-`cap`, phải `list[str]` | `VS0412` (sai kiểu) / `VS0401` (thừa) |

- **Không mã mới**, **không** sửa `agent/API-CODES.txt` (R9 an toàn); dùng `MISMATCH`/`ARG_COUNT`
  đã `pub(crate)`. **Không** đụng `vis-loader/src/cache.rs` ⇒ **không bump `PHIEN_BAN`**: tyck chạy
  trước khi tra cache bytecode, nên cache cũ không chạy lọt chẩn đoán mới.
- Không chạm interp/VM/codegen/bytecode; path CLI (`vis check`/`vis run`) và harness conformance
  (gọi thẳng `vis_types::check`) cùng thấy luật này.

## 2. Test (R8 giữ **39/40**, không `.rs` cấp 1 mới)

- `test/conformance/lang/main_sig/` (5 ca, harness tự quét): `mot_int.err` (`VS0412`),
  `hai_int.err` (`VS0412` + `VS0401` theo thứ tự nguồn), `cap_int.err` (`VS0412`),
  `argv_hop_le.out` (`[]\n`), `default_tham_so.out` (`[]\n5\n` — hàng rào chống luật quá chặt).
- `crates/vis-cli/tests/lang_it/main_sig.rs` (mới trong thư mục con) + 1 dòng `#[path]` ở
  `lang_it.rs`: `main_mot_int_bi_tu_choi`, `main_hai_int_bi_tu_choi`. Mỗi ca: `vis check` exit 1,
  TAP `not ok` (chỉ **1** dòng `# VSxxxx` theo VD-38); interp và `--vm --no-cache` cùng exit 2,
  stdout rỗng, stderr byte-bằng nhau (`VS0412`, + `VS0401` cho ca hai tham số), kể cả có `-- a b`.
- Ca dương `vd259_frame2::default_tham_so_main_va_task_async` vẫn PASS `5\n6\n` (đã kiểm trong
  `gate --fast`).

## 3. Cổng (log `/home/ps/vislogs/st321a-*.log`)

| Cổng | Kết quả | sha256 log |
|---|---|---|
| `cargo fmt --all` | sạch | `st321a-fmt.log` |
| `cargo test -p vis-types -p vis-cli` | **0 fail**; `lang_it` 498/498 (gồm 2 ca mới); `cli_it` 125/125; `types_it` 287/287; tổng các binary ≈ 2039 pass | `08344ae9…` |
| `./vis-dev gate --fast` | **PASS**, `3936 passed, 4 skipped`; check + R1..R9 + self-test + doc-examples OK | `02e9dbe2…` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (`tong=100000`, `chay=94425`, 60,3 s, seed `3523158054`) | `93115cf1…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **OK** | `f10b2597…` |
| `cargo test -p vis-conformance --test corpus` | **573/573 ca xanh** (đã gồm 5 ca `main_sig/`) | `beab9e42…` |

sha256 đầy đủ: tests `08344ae9e42fa43163d5992fc2780e493703bf260d4768065a67d8d59f6112ff`;
gate `02e9dbe2ff0a51e12aea2d16a7efb90554134212b167a4367a0006ae6841ed74`;
diff `93115cf15f20530dd8def167ebd200aa88e15b2568c7b4c5d49e93e6a8a9f182`;
clippy `f10b2597e99c5946bdedaab0aa657713d1ccdcb2c9a4dfb559b602b37a5dbb28`;
corpus `beab9e420555cabe58f1ddbec5678aa7c334eb831a1a574ebaabc839ce7a02cc`;
repro `24efd3fd0380059eb2c4200b0d2549b2237e358f5fc7c35449454a7c7d5a173b`.

## 4. Repro `#200`/`#201` — trước / sau

Nguồn "trước": audit §3/§4 (binary `ee8f252d`). "Sau": `/home/ps/vislogs/st321a-repro-after.log`
(binary worktree, `vis 0.6.0`), lệnh có `-- a b` cho `run`.

| Ca | Lệnh | Trước | Sau |
|---|---|---|---|
| `#200` `fn main(a: int, b: int)` | `vis check` | exit 0, TAP `ok 1` | **exit 1**, `not ok 1`, `# VS0412 2:9` |
| | `vis run` (interp) | exit 101, `VS0501` thiếu `b` | **exit 2**, stdout rỗng, `VS0412 2:9` + `VS0401 2:17` |
| | `vis run --vm --no-cache` | exit 0, `[a, b]\n()\n` | **exit 2**, stdout rỗng, **stderr y hệt interp** |
| `#201` `fn main(a: int)` | `vis check` | exit 0, `ok 1` | **exit 1**, `not ok 1`, `# VS0412 2:9` |
| | `vis run` (interp) | exit 0, `[a, b]\n` | **exit 2**, stdout rỗng, `VS0412 2:9` |
| | `vis run --vm --no-cache` | exit 0, `[a, b]\n` | **exit 2**, stdout rỗng, stderr y hệt interp |
| dương `main(a: list[str], b: int = K + 2)` | `vis run` / `--vm --no-cache` | `[]\n5\n`, exit 0 | **`[]\n5\n`, exit 0** (không đổi) |

Thông điệp sau: `VS0412` — `` `main`: tham số `a` nhận `argv` phải là `list[str]` ``;
`VS0401` — `` `main` chỉ nhận một tham số `list[str]` (argv); tham số `b` thiếu giá trị mặc định ``.

## 5. Việc còn lại / đề nghị cho A0 + Agent 1

1. **Agent 1** nghiệm thu VD-279 bằng các log §3 (không cần chạy lại soak/fuzz — VD-289 đang ở chế
   độ Phát triển Kiến trúc).
2. **A0 cập nhật tài liệu** (Agent 2 không sửa SPEC): `docs/SPEC.md:1053` (§12 — thay dòng "không
   kiểm chặt…" bằng luật tĩnh mới), thêm 1 câu cạnh §9.2 `main(args: list[str])`, và sổ `SO-NO`
   đóng `#200`/`#201` (82/109 → 80/111). Release note: chữ ký `main` sai trước đây VM "chạy được"
   nay đỏ tĩnh (§12 vốn nói rõ không phải đặc tả ổn định).
3. **Lane B tuỳ chọn** (audit §6.4): parity engine cho artifact bỏ qua tyck — **hoãn**, không chặn
   đóng `#200`/`#201` trên đường CLI/`vis check`/`vis test`.
4. **Không merge/push**; nhánh `plan/st321a` bàn giao cho Agent 1. Dọn worktree theo VD-288 sau khi
   A0 duyệt `DONE`.
