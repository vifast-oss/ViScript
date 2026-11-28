<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-325d — repro-first nợ `#2`: `match` giá trị rơi nhánh đẩy `false` — **TÁI LẬP & ĐÓNG**

- **Nhánh**: `plan/st325d`, nền `main` = `37ae9c35`; worktree `/home/ps/viswt/st325d`.
  **Không** đụng repo chính, **không** merge/push (VD-279).
- **Lane sửa**: `crates/vis-bytecode/src/codegen/expr.rs` (codegen **giá trị** `HExpr::Match` của VM)
  + ca test trong thư mục con `crates/vis-cli/tests/lang_it/st325d.rs`, khai `#[path]` trong
  aggregator `lang_it.rs` có sẵn — **R8 giữ 39/40**, không tệp `.rs` cấp 1 mới. **Không** đụng
  parser/lower/tyck/interp/VM runtime. Blast radius nhỏ: 1 điểm phát lệnh + 1 builtin giả có sẵn
  (`trap_no_arm`, dùng từ ST-238 ở nhánh câu lệnh).
- **Môi trường**: cargo/rustc **1.100.0** (`rust-toolchain.toml`, `RUSTUP_TOOLCHAIN=1.100.0`),
  `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/vislogs/tmp-st325d`
  (đĩa, không tmpfs). Đĩa đầu **29 G**, cuối **23 G** (> ngưỡng dừng 5 G).

## 1. Repro (trước sửa — binary dựng ngay trong worktree)

`cargo build -p vis-cli --bin vis` ⇒ exit 0 (`st325d-build.log`, sha `b033796a…a203e`);
binary `/home/ps/viswt/st325d/target/debug/vis`, sha256
`4d1439845170a11146afba7b894a0036bb191f36cf5e9ab3c2f0faea88ea6cf2`.
Ca tối thiểu `d.vis` nằm **ngoài repo**: `/home/ps/vislogs/st325d-repro/d.vis`.

```vis
fn main():
    let x: int? = Some(2)
    let v = match x:
        Some(1): 10
        None: 20
    print(v)
```

| Lệnh | Exit | stdout | stderr |
|---|---|---|---|
| `vis check d.vis` | **0** | `TAP version 13\n1..1\nok 1 - d.vis\n` | rỗng |
| `vis run d.vis` (interp) | **101** | rỗng | `d.vis:3:13: error VS0501: \`match\` không khớp nhánh nào (không xét hết bị lọt)\n` |
| `vis run --vm --no-cache d.vis` | **0** | `false\n` | rỗng |

Log đầy đủ `/home/ps/vislogs/st325d-repro/baseline.log` (sha `55cd162d…7e10`).
**Kết luận**: tái lập đúng triệu chứng sổ — VM in `false`/exit 0, interp trap `VS0501`/exit 101,
trong khi `check` xanh. Các thử nghiệm loại trừ: `_ if false` và enum có guard bị tyck chặn
`VS0402` đúng luật (guard không chứng minh phủ) — nên không phải ca tối thiểu; cần lợi dụng khe
tyck chỉ so **tên ctor** (`Some(1)` + `None` ⇒ tưởng phủ) để runtime có đường rơi.

Sau sửa, cùng binary (sha mới `55b19bc6619ea379f6668d18543cb85c4c21341f6651e7072d6a25a106ab1673`,
log `postfix.log` sha `9b095ad6…0204`): `check 0`; interp **và** VM đều exit **101**, stderr
**byte-bằng** nhau, cùng `VS0501` + cùng thông điệp + cùng vị trí `3:13`.

## 2. Xác định tầng & bản vá

- **Nguyên nhân lệch**: `crates/vis-bytecode/src/codegen/expr.rs` nhánh `HExpr::Match` — sau khi
  mọi nhánh trượt (`JumpIfFalse` tới `lpop`), phát `Instr::Const(Bool "false")` (+1 stack) thay vì
  bẫy. Trong khi đó:
  - interp giá trị `Interp::eval_match` (`interp/goi.rs`) đã `Err(Abort::Trap(RT, "match không
    khớp nhánh nào…", span))` ⇒ exit 101;
  - interp câu lệnh (`interp/stmt.rs`) và **codegen câu lệnh** (`codegen/cau.rs::trap_no_arm`,
    ST-238) cũng đã trap; VM runtime `exec.rs` đã chặn builtin giả `$vis_match_no_arm` ⇒ `VS0501`.
  ⇒ Lỗi nằm ở **một điểm codegen giá trị**, không phải tyck/interp/VM runtime. Sửa tyck để từ chối
  `Some(1)` là đổi ngữ nghĩa lớn hơn và trái kỳ vọng nợ (runtime `VS0501`), nên **không** làm.
- **Patch**: thay khối `Const(false)` bằng `self.trap_no_arm(*span, true)?`. `keep_value = true`
  giữ nguyên delta ngăn xếp +1 để verifier thấy một giá trị như mỗi nhánh khác; runtime trap trước
  khi đẩy giá trị nên không có tác dụng phụ. Không thêm opcode/tag, không đổi VERSION.
- **Khe tyck còn lại (ghi cho A0, không tự cấp số nợ)**: `xet_het` (`vis-types/.../xet_het.rs`)
  chỉ gom **tên ctor/field** (`ctor_names`/`record_cov`) nên `Some(1)` được coi là phủ dù chỉ khớp
  một giá trị; `check` vì thế xanh. Đây là nợ riêng về **độ chính xác phủ**, nằm ngoài repro-first
  #2 (vốn đòi parity lúc chạy). Hai engine giờ cùng trap `VS0501` làm hàng rào an toàn.

## 3. Test mới (`lang_it/st325d.rs`, 3 ca)

1. `value_match_fallthrough_hai_engine_cung_vs0501` — ca §1: `check 0` (tiền điều kiện lọt tyck),
   interp = VM = exit **101**, stdout rỗng, stderr cùng `VS0501` + thông điệp.
2. `value_match_khop_van_tra_gia_tri` — `Some(2)` → đổi thành `Some(1)`: cả hai in `10\n`, exit 0
   (đối chứng dương, chặn use-case hợp lệ bị hỏng).
3. `statement_match_fallthrough_hai_engine_cung_vs0501` — cùng dữ liệu ở **ngữ cảnh câu lệnh**:
   cả hai trap `VS0501` (giữ parity đã có từ ST-238).

`cargo test -p vis-cli --test lang_it st325d` ⇒ **3 passed / 0 failed**
(`st325d-test-targeted.log` sha `d103f97d…2be`).

## 4. Cổng (log `/home/ps/vislogs/st325d-*.log`)

| Cổng | Kết quả | sha256 log |
|---|---|---|
| Build binary trước/sau | exit 0; sha `4d143984…` → `55b19bc6…` | `b033796a…a203e` / `85696d79…281c` |
| Repro trước / sau | `interp 101 / VM 0 false` → `interp 101 / VM 101 VS0501` byte-bằng | `55cd162d…7e10` / `9b095ad6…0204` |
| Test đích danh | **3 passed / 0 failed** | `d103f97d…2be` |
| `./vis-dev gate --fast` | **PASS exit 0**; fmt/clippy/R1..R10/DAG/self-test/build/doc-examples OK; nextest **3952 passed / 4 skipped**; L0 OK | `b856f5da…0441` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (`tong=100000`, `chay=94425`, `loi_tinh=5575`, 72,7 s, seed `3523158054`) | `f33bf4d1…c46c` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **OK** | `3fe980a0…e527` |

Sha256 đầy đủ: `st325d-build.log` `b033796af5cfeb3593cd2f5c62d96aad8e406fae1b11c735ac9db517ac2e203e`;
`st325d-rebuild.log` `85696d79f7de6b611e9ed092a3bac125b3e5566a48ba5859382626fab810281c`;
`st325d-repro/baseline.log` `55cd162d273872827e233632ee1cfa134e54fd9ac768db717747091435ed7e10`;
`st325d-repro/postfix.log` `9b095ad69126f3af59ea53a499fc55731ca285fced87b0d87b2cace26ee30204`;
`st325d-test-targeted.log` `d103f97d2d193885de41fce468900b4729be7c9d09d2a3878cb5ea86bd6262be`;
`st325d-gate-fast.log` `b856f5dab1c90becde5a4669ad55a9d9c445a6da97dd7182037642ddd9990441`;
`st325d-vis-diff-100k.log` `f33bf4d1d4829944096fe014f7564a68fd570e09cc5bdf0a1bdfaeae0a04c46c`;
`st325d-clippy.log` `3fe980a0f9709b0a2d093ea2a65202e1e589d4893ac71be4272c165923b9e527`.
Lần đầu `gate --fast` bị **fmt FAIL** do dòng `assert_eq!` dài trong test mới; đã `cargo fmt --all`
rồi chạy lại ⇒ PASS (log đầu ghi đè bởi log cuối).

## 5. Đề xuất Agent 0

- Đóng nợ **`#2` → `đóng ở ST-325d`** trong `agent/SO-NO.md` (Builder **không** tự sửa sổ, §3 QD):
  căn cứ là repro §1 (VM in `false`/exit 0) → sau vá VM trap `VS0501`/exit 101 byte-bằng interp;
  test §3 + toàn bộ cổng §4 xanh. Không tuyên bố tyck đã phủ chặt — xem lưu ý §2.
- Nếu A0 muốn siết thêm khe phủ (`Some(1)` bị coi là phủ), đề nghị **mở nợ mới** riêng cho
  `xet_het`; Builder không tự cấp số nợ (VD-279).
- Không merge/push; chờ Agent 1 nghiệm thu rồi A0 merge theo VD-279. Không tag phát hành;
  VD-289 tiếp diễn. Không tạo nhánh/worktree mới.
