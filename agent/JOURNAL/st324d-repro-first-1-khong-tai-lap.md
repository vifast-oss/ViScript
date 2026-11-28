<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-324d — repro-first nợ `#1`: `let out = ls -l` — **KHÔNG tái lập** (triệu chứng đã hết hạn)

- **Nhánh**: `plan/st324d`, nền `main` = `d1077075`; worktree `/home/ps/viswt/st324d`.
  **Không** đụng repo chính, **không** merge/push (VD-279).
- **Môi trường**: cargo/rustc **1.100.0** (`rust-toolchain.toml`, `RUSTUP_TOOLCHAIN=1.100.0`),
  `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/vislogs/tmp-st324d`
  (đĩa, không tmpfs). Đĩa đầu/cuối **29 G** (> ngưỡng dừng 5 G).
- **Binary repro**: dựng **trong worktree** (`cargo build -p vis-cli --bin vis`, `BUILD_EXIT=0`,
  `/home/ps/vislogs/st324d-build.log`), sha256
  `1602e56d820828b49301ab492a125e75daa5a52868cb4c0b03f2a1cbad1f7bcc`.
  `/home/ps/viswt/st324d/target/debug/vis`.

## 1. Repro trên HEAD `d1077075` (trước khi sửa — thực tế **không** sửa mã)

Log đầy đủ: `/home/ps/vislogs/st324d-repro-head.log` (sha xem §3). Hai tệp nằm **ngoài repo**
(`${TMPDIR}/repro1-min.vish`, `${TMPDIR}/repro2-use.vish`), chạy trực tiếp binary debug ở trên.

| Ca | Lệnh | Exit | stdout | stderr |
|---|---|---|---|---|
| A — tối thiểu (`# SPDX…` + `let out = ls -l`, **không** dùng `out`) | `vis check` | **0** | `TAP version 13\n1..1\nok 1 - repro1-min.vish\n` (43 B) | rỗng |
| A | `vis run` | **0** | rỗng | rỗng |
| A | `vis run --vm --no-cache` | **0** | rỗng | rỗng |
| B — có dùng `out` (`# SPDX…` + `let out = ls -l` + `print(out)`) | `vis check` | **1** | `TAP version 13\n1..1\nnot ok 1 - repro2-use.vish\n# VS0301 3:7 tên không tồn tại: \`out\` chưa được khai báo (VD-260)\n` (125 B) | rỗng |
| B | `vis run` | **2** | rỗng | `repro2-use.vish:3:7: error VS0301: tên không tồn tại: \`out\` chưa được khai báo (VD-260)\n` (100 B) |
| B | `vis run --vm --no-cache` | **2** | rỗng | **byte-bằng** interp (100 B, cùng `VS0301`) |

**Kết luận repro**: triệu chứng được ghi trong sổ — *"`check` exit 0 nhưng `run` báo `out` chưa bind"*
— **không tái lập**. Ca A: `check` xanh và `run` cũng xanh (không có lỗi "out chưa bind" ở đường chạy).
Ca B: nơi duy nhất `out` được dùng, `check` **cũng đỏ** `VS0301` (exit 1) và `run` từ chối **tĩnh**
(exit 2) với **cùng** chẩn đoán `VS0301 ... (VD-260)` trên **cả hai engine** — hết dạng "check xanh /
run đỏ" của nợ cũ.

## 2. Phân tích ngắn (không sửa mã)

- `let out = ls -l` vẫn được phân loại là **shell-let**, không phải khai báo: luật một nguồn
  `vis_syntax::let_la_khai_voi` (`crates/vis-syntax/src/parse/cmd/phan_loai.rs`) thấy định danh `l`
  **chưa bind** ⇒ `dinh_danh_chua_bind` trả `l` ⇒ `la_khai_bao_voi` /
  `khai_sau_stage` (`vis-types/src/lower/khai.rs`) đẩy dòng này về **stage**. `out` vì thế **không**
  được bind — đúng như ghi chú "shell-let chưa hạ".
- Điểm **đổi** so với thời R10/R11/R12 là chẩn đoán: `print(out)` nay bị chặn **tĩnh** `VS0301`
  (gắn nhãn `VD-260`), nên `check` không còn "ngó lơ" như cũ. Hành vi này đã được test khoá sẵn:
  `crates/vis-host-native/tests/security_it/gan_hai_duong.rs::vish_shell_let_khong_thanh_khai_bao`
  (`check` phải đỏ `VS0301`, `run` exit 2 `VS0301`, **`l` không bị báo oan**), cùng
  `cli_it::vish_decl::let_shell_van_la_stage`, `cli_it::vsh1b::shell_let_st44_khong_doi`,
  `lang_it::stage_dau::golden_ca24_va_ca_a_or_b_khong_doi`, `fixture_cmd_long`.
- Vì triệu chứng **không tái lập**, theo lệnh ST-324d mục 2: **KHÔNG sửa mã**, không đụng lane
  `vis-syntax`/`vis-types`, không thêm test/cờ mới, không đụng `golden.cmd` (giữ 23/24).

## 3. Cổng xác nhận (log `/home/ps/vislogs/st324d-*.log`)

| Cổng | Kết quả | sha256 log |
|---|---|---|
| Repro HEAD (bảng §1) | `check 0 / run 0` (ca A); `check 1 / run 2` **cùng `VS0301`** (ca B), interp ≡ VM | `92ca6b90…993bead` |
| Build binary repro | `BUILD_EXIT=0`, toolchain **1.100.0** | `3e8b8911…b74f` |
| `cargo test -p vis-cli` | **exit 0**, **1816 passed / 0 failed** (12 binary test + doc-test) | `57c68f2f…6985` |
| `./vis-dev gate --fast` | **PASS exit 0**, nextest **3948 passed / 4 skipped**; R1..R10 OK, DAG self-test OK, 20 case âm OK, L0 OK | `95482424…7146` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **exit 0, `lech=0`** (`tong=100000`, `chay=94425`, `loi_tinh=5575`, 169,4 s, seed `3523158054`) | `eb26bc62…f9` |

Sha256 đầy đủ: `st324d-repro-head.log` `92ca6b90bb64494d205b216f97ad16efa1c01e24ca8655bcf573f796f993bead`;
`st324d-build.log` `3e8b8911b6ef77858a536728d9fc481d61a288752f7fb91299f1f937dc29b74f`;
`st324d-test-vis-cli.log` `57c68f2f1a50ea762de040da7b17a26b63e19ec50ebd3a8b77bf00a90316985b`;
`st324d-gate-fast.log` `95482424fe95ca34769bca2e79d062b7bd229e9f0926eb083d3f4c152e8f7146`;
`st324d-vis-diff-100k.log` `eb26bc62d32250e004c0be92d1efab31c27600ec84fdeb43f5512357a7c5a5f9`.
**Không** chạy clippy riêng: gate `--fast` đã gồm `cargo clippy -- -D warnings` và PASS; không có dòng mã sản
phẩm nào bị đụng. `golden.cmd` **23/24 giữ nguyên** (không đổi tệp test/fixture nào).

## 4. Đề xuất Agent 0

- Đóng nợ **`#1` dạng "đã hết hạn"** trong `agent/SO-NO.md` (Builder **không** tự sửa sổ): căn cứ là
  repro §1 — dạng "`check` xanh / `run` báo `out` chưa bind" không còn xảy ra ở HEAD `d1077075`;
  từ chối tĩnh `VS0301` nhất quán ở `check` lẫn `run`, interp ≡ VM byte-khớp.
- **Lưu ý cho A0 (quan trọng)**: đây là đóng **triệu chứng**, không phải tuyên bố "shell-let đã hạ".
  Tính năng `let out = <lệnh>` **tự bind kết quả lệnh vào `out`** vẫn **chưa** có; `out` vẫn bị từ
  chối khi dùng. Nếu A0 muốn có tính năng đó, đề nghị **mở nợ mới** (feature) thay vì giữ `#1` mở
  dưới dạng bug "check/run lệch" —Builder không tự cấp số nợ (VD-279).
- Không merge/push; chờ Gatekeeper/A0. Không tag phát hành; VD-289 tiếp diễn. Không nợ mới từ lát này.
