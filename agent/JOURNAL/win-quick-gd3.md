<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows — verify quick 3 nhánh GD3 (`ST-302` Pha 2b, `ST-304` vish-edit, `ST-305` LSP/VS Code)

> **Người đo**: subagent GD3, 2026-11-23 · máy `psvn` (Windows) · chạy **tuần tự**, không song song.
> **Mốc nền**: `main` **`c3b7290`** (`merge ST-300m1: Darwin errno + vis-gate-macos + #137`).
> **psvn**: `MINGW64_NT-10.0-26300`, nproc **6**, `rustc/cargo 1.100.0`; `TMPDIR=/c/Users/ps/tmp`;
> `CARGO_BUILD_JOBS=3` cho các lệnh `cargo test` trực tiếp.
> **Kết quả**: **3/3 nhánh PASS** trên Windows. Không merge / push / tag; không đụng `mps`.

## 0. TL;DR

| Nhánh | Tip | Lệnh | Kết quả |
|---|---|---|---|
| `plan/w1-st302-pha2b` | `d3ea467` | `cargo test -p vis-cli --test vd284_async_impl --test vd284_derives_json` | **PASS** 3 + 2 |
| | | `bin/vis-gate-windows --quick` | **PASS** pkg 25 / h2 33 / cancel 8 / select 25 |
| `plan/w3-st304-vish` | `6e0675a` | `cargo test -p vish-edit` | **PASS** 9 (crossterm Windows biên dịch + chạy) |
| | | `bin/vis-gate-windows --quick` | **PASS** pkg 25 / h2 33 / cancel 8 / select 25 |
| `plan/w4-st305-lsp-vscode` | `fa1a70b` | `cargo test -p vis-lsp` | **PASS** 29 |
| | | `cargo test -p vis-cli --test lsp_it` | **PASS** 155 |
| | | `bin/vis-gate-windows --quick` | **PASS** pkg 25 / h2 33 / cancel 8 / select 25 |

**Không có blocker sản phẩm.** Hai điểm lệch công cụ (không phải lỗi mã) ghi ở §4.

## 1. Chuyển mã sang `psvn` — nhánh **không** có trên GitHub

- `git ls-remote origin 'refs/heads/plan/*'` → **rỗng**; `origin` chỉ có `main`, `fix/macos-v045-tests`,
  `rev/vd-sqlite-mvp-2026-11-19`, `rev/vd241-t-2026-11-19`. Ba nhánh cần đo là **nhánh cục bộ trên `ps`**.
- Vì bị cấm push/tag, đã đóng gói **bundle tự chứa** rồi chép sang `psvn` (không qua GitHub):

```bash
git bundle create /tmp/win-quick-gd3.bundle \
  refs/heads/plan/w1-st302-pha2b refs/heads/plan/w3-st304-vish refs/heads/plan/w4-st305-lsp-vscode
# 14M · sha256 371fa54a8767d3f13c1130e1883fea3fc7fab60a1b81050c178bed197efa7637
scp /tmp/win-quick-gd3.bundle psvn:C:/Users/ps/tmp/win-quick-gd3.bundle
# sha256 trên psvn khớp tuyệt đối
git bundle verify   # "records a complete history" · "is okay"
git fetch <bundle> '+refs/heads/plan/*:refs/heads/winq/*'
```

- Ref nhận trên `psvn` (`/c/Users/ps/viscript-moi`): `winq/w1-st302-pha2b d3ea467c`,
  `winq/w3-st304-vish 6e0675af`, `winq/w4-st305-lsp-vscode fa1a70b0` — **khớp tip đo**.
- Lý do bundle tự chứa: repo gốc trên `psvn` (HEAD `8cc3588a`) **không có** `c3b7290`/`700bb66`,
  nên bundle có prerequisite sẽ bị `git bundle verify` từ chối.

## 2. Worktree riêng từng nhánh (detached, sạch)

| Worktree | HEAD sau khi đo | dirty |
|---|---|---|
| `/c/Users/ps/wq-gd3-w1` | `d3ea467cb2063bd908de7ea5a73644f372cc1257` | **0** |
| `/c/Users/ps/wq-gd3-w3` | `6e0675af566743361f152fbcd0da94ac5b9bd7b5` | **0** |
| `/c/Users/ps/wq-gd3-w4` | `fa1a70b04dca1043af85968c4f9c3a5eb77fe919` | **0** |

Tạo bằng `git worktree add --detach <đích> refs/heads/winq/<nhánh>`; kiểm trước khi chạy:
`crates/vis-cli/tests/vd284_async_impl.rs`, `vd284_derives_json.rs` (w1),
`crates/vish-edit/Cargo.toml` (w3), `crates/vis-lsp/Cargo.toml` + `crates/vis-cli/tests/lsp_it.rs` (w4) — đủ.

Lệnh đo (mỗi nhánh: cargo test trước, gate sau; xong nhánh này mới sang nhánh kế):

```bash
bin/vis-remote psvn --tag gd3-wN-... --log /home/ps/vislogs/... \
  -- bash -lc 'export TMPDIR=/c/Users/ps/tmp TMP=/c/Users/ps/tmp TEMP=/c/Users/ps/tmp CARGO_BUILD_JOBS=3; cd <worktree> && <cargo test>'
VIS_GATE_WINDOWS_REPO=<worktree> VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
  VIS_GATE_WINDOWS_LOG=/home/ps/vislogs/gd3-wN-gate.log bin/vis-gate-windows --quick
```

## 3. Chi tiết kết quả

### 3.1 `plan/w1-st302-pha2b` @ `d3ea467` — PASS

- Build test profile **1m 00s**; `vd284_async_impl` **3/3** (`async_cmd_await`, `async_method_self`,
  `async_method_var_self`); `vd284_derives_json` **2/2** (`vd284_derives_json_tu_choi_tinh`,
  `vd284_15_ca_derives_json_parity`). Tổng thời gian lệnh **1m 05s**.
- Gate quick **42.9 s**, exit **0**: `pkg::` **25** (138 filtered), `http2_it` **33**, `cancel_http_` **8**
  (155 filtered), `async_it vs3_select::` **25** (296 filtered). Không `FAILED`/`panicked`.

### 3.2 `plan/w3-st304-vish` @ `6e0675a` — PASS

- `cargo test -p vish-edit` **12.9 s**: unit **0**, `tests/editor.rs` **9/9**
  (`ctrl_c_huy`, `parse_keys_arrows_utf8_ctrlc`, `fuzzy_chon_bang_ctrl_r`, `nhieu_dong_va_thut_le`,
  `pty_gia_doc_byte_qua_read_key`, **`windows_parse_ctrl_c_va_arrow`**, `render_cst_co_mau_va_giu_byte`,
  `panic_giu_editor_va_history`, `history_append_only_va_fuzzy`), doc-test **0**.
  `crossterm 0.29.0` + `crossterm_winapi 0.9.1` biên dịch sạch trên Windows.
- Gate quick **1m 16s**, exit **0**: 25 / 33 / 8 / 25 như trên.

### 3.3 `plan/w4-st305-lsp-vscode` @ `fa1a70b` — PASS

- `cargo test -p vis-lsp` **29.8 s**: **29/29** (gồm `vd258_tests::uri_percent_drive_windows`,
  `uri_st297_unc_mixed_percent_roundtrip`, `vi_tri_*`, `semantic_tokens`, `sua`, `workspace`).
- `cargo test -p vis-cli --test lsp_it` **50.1 s**: **155/155** (selection / inlay hint / semantic tokens /
  type definition / workspace symbol, gồm `lsp_*` và `workspace_symbol_loc_theo_root`).
- Gate quick **40.4 s**, exit **0**: 25 / 33 / 8 / 25.

## 4. Bằng chứng log (ngoài repo) + sha256

Log cục bộ trên `ps`: `/home/ps/vislogs/`; bản trên máy đích: `/c/Users/ps/vislogs/` (đường dẫn và
sha256 lấy từ khối `# ---------- KET ----------` do `vis-remote` ghi).

| Lần đo | Log cục bộ (sha256) | Log trên `psvn` (sha256) |
|---|---|---|
| w1 cargo | `gd3-w1-cargo.log` `c20c3e88ce601d5bfed4c9da3f1d31d9ce17a9d4365eefc6a31ca26f6fcceec3` | `gd3-w1-cargo-20261123T044440Z.log` `8060f23325184d1eeb65f3beaa731e31f36afecdd0fe5a631dba882362c10c15` |
| w1 gate | `gd3-w1-gate.log` `ed0114335a76500990a6f436887a5c4bd1eecb5c803e8c08e8d824bc84b3318b` | `vis-remote-20261123T044605Z.log` `3e46cce914655148e45f95e0b828167afbebf83ecada592ac7a9fc0ea94b4caa` |
| w3 cargo | `gd3-w3-cargo.log` `6a9249ed0b59bf6678ef1e3552d427793869dbaa74aff981a9f5b64c57d0a17e` | `gd3-w3-cargo-20261123T044658Z.log` `8ca2606bf2592fe1b1a628938ecd2084dd246adabaa5958a53572c1705395070` |
| w3 gate | `gd3-w3-gate.log` `9b59fbaa378c0fa53e041e1607bc891c3e96f969ca14255d8bad4558077d3a27` | `vis-remote-20261123T044716Z.log` `811392fa153a6d344db04de04b39f6b7987a3bfa934f8d81eee880f9270c3aa2` |
| w4 vis-lsp | `gd3-w4-lsp.log` `f438ba6adbc31433d83ed297ab518ed2738d4a716eabe8e7ffd72f6313378386` | `gd3-w4-lsp-20261123T044838Z.log` `021addfc5f771a0de65df4d4646de3a8e9ffa62fb13ecf0135c031f43021e77d` |
| w4 lsp_it | `gd3-w4-lspit.log` `5307d7c18773b4488a24c84a723c1a482ab094cebddfbed423bdc90a4b794cd9` | `gd3-w4-lspit-20261123T044913Z.log` `d56780e8470b1bc6cc5e1e068e3cc864f39ed8d6688bd833123c9158ba12796b` |
| w4 gate | `gd3-w4-gate.log` `09714c7366425f8c7d14c9865111f5b29c51531e6fd10effb526166744340943` | `vis-remote-20261123T045010Z.log` `b80a9621aa814571dae855a5f445f5ff4340ff86fc5e8a395a80f0d71c7df467` |

Cửa sổ UTC (từ header/footer): w1 cargo `04:44:42→04:45:45` · w1 gate `04:46:07→04:46:47` ·
w3 cargo `04:47:00→04:47:10` · w3 gate `04:47:18→04:48:32` · w4 lsp `04:48:40→04:49:07` ·
w4 lsp_it `04:49:15→04:50:03` · w4 gate `04:50:12→04:50:50`. Không cửa sổ nào chồng nhau ⇒ đúng tuần tự.

## 5. Lệch công cụ / giới hạn bằng chứng

1. **Không fetch được từ GitHub**: ba nhánh chỉ tồn tại cục bộ trên `ps` (§1). Đã dùng bundle + `scp`;
   tip sau fetch khớp từng ký tự với tip đo ⇒ bằng chứng vẫn quy đúng commit.
2. **`CARGO_BUILD_JOBS=3` không tới được thân `vis-gate-windows`**: `vis-remote` không chuyển env
   sang script trên máy đích và `vis-gate-windows` chỉ có `VIS_GATE_WINDOWS_{REPO,TMPDIR,LOG,REMOTE}`.
   Các lệnh `cargo test` trực tiếp **có** `CARGO_BUILD_JOBS=3`; gate chạy với mặc định cargo
   (nproc 6). Vì chạy tuần tự, tại mỗi thời điểm chỉ một build ⇒ không có hai build chồng nhau.
   Muốn gate cũng ghim jobs thì phải sửa công cụ (đề xuất: thêm `VIS_GATE_WINDOWS_JOBS`), **không** làm trong lượt này.
3. **`--quick` không phủ nội dung ba nhánh**: quick chỉ là 4 suite lõi (pkg/h2/cancel/select);
   phần mới của từng nhánh được đo bằng `cargo test` đích danh ở §3. Kết luận "PASS Windows" cho
   từng nhánh là hợp của hai phần này.

## 6. Kết luận

- **`plan/w1-st302-pha2b` @ `d3ea467`: PASS** trên Windows (async impl + derives JSON + quick lõi).
- **`plan/w3-st304-vish` @ `6e0675a`: PASS** trên Windows (vish-edit crossterm + quick lõi).
- **`plan/w4-st305-lsp-vscode` @ `fa1a70b`: PASS** trên Windows (vis-lsp + lsp_it + quick lõi).
- Không có lỗi mã, không cần retry. Quyết định merge/push để **Agent 0**; lượt này không merge/push/tag.
- Dọn dẹp tuỳ chọn trên `psvn` (chưa làm, để giữ bằng chứng): `git worktree remove` ba `wq-gd3-*`,
  `git branch -D winq/*`, xoá `/c/Users/ps/tmp/win-quick-gd3.bundle`.
