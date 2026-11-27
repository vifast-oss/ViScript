<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-321b — `vis test`: hỗ trợ fixture `.trap` / `.out_prefix` / `.exit`

- **Ngày**: 2026-11-27 · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st321b`
- **Nền**: `main` @ `ff585af8aad8156e30b288b93f2de321305f83ea`
  (`[ST-00] docs: dong bo HEAD/VD/WASI/R38 truoc peak`)
- **Worktree**: `/home/ps/viswt/st321b` — **không chạm repo chính**.
- **Trạng thái**: mã + test + cổng **PASS**; **chưa merge / chưa push**, chờ Gatekeeper audit.
- **Commit mã**: `501964cc7a9344ca2caecbb7fac6f7d3536e05f0` — `[ST-321b] vistest: ho tro .trap/.out_prefix/.exit`
- **Commit journal**: (cùng nhánh, commit ngay sau).

## 0. TL;DR

Audit `2026-11-26T22:30Z` chỉ ra CLI `vis test test/conformance` đỏ **538 ok / 30 not ok** trong khi
harness `vis-conformance` xanh **568/568** — lỗi **runner**, không phải engine. Nguyên nhân:
`crates/vis-cli/src/vistest/ca.rs` chỉ đọc `.err` / `.out`, không đọc `.trap` (28 ca),
`.out_prefix` (2 ca) và `.exit` (10 ca). Áp draft patch (apply-check rc 0), giữ nguyên golden / TAP /
JSON / VBC1 / cache. Kết quả: **conformance 568/568**, **sem 525/525**, cả hai engine
**byte-identical**; `cargo test -p vis-cli`, `vis-dev gate --fast`, `vis-diff` 100k (`lech=0`) đều PASS.

| Hạng mục | Kết quả |
|---|---|
| `vis test test/conformance` (interp / VM, `--jobs 2`) | **568/568**, exit **0**, log **byte-identical** |
| `vis test test/sem` (interp / VM, `--jobs 2`) | **525/525**, exit **0**, log **byte-identical** |
| `cargo test -p vis-cli` | **1812 passed / 0 failed** (12 binary + doc-tests) |
| `./vis-dev gate --fast` | **PASS** (fmt, clippy `-D warnings`, R1..R10, DAG, self-test, build, doc-examples, L0) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`**, exit **0**, 7 passed |
| R8 | workspace top-level test `.rs` **39/40**; `vis-cli/tests` **8/15**; **0** tệp `.rs` cấp 1 mới |

## 1. Đầu vào & kiểm chứng

- Audit: `/home/ps/vislogs/audit-vistest-runner-20261126T2230Z.md`,
  sha256 `3599c6b3599618c0fae715ed2db55d69cc5ee6933b178c1f739bcabea1482274`.
- Draft patch: `/home/ps/vislogs/vistest-runner-fix-draft.patch`,
  sha256 `b0c19930d5e182e149a176d45f1ada024e6eda33d843d01a8e8017bb0c051fe6`;
  `git apply --check` rc **0**, `git apply` rc **0** (sửa đúng 1 tệp `ca.rs`, +59/−10).
- Đếm fixture tại nền: **28** `.trap`, **2** `.out_prefix`, **10** `.exit` — khớp đúng 30 ca `not ok`.

## 2. Thay đổi

Chỉ `crates/vis-cli/src/vistest/ca.rs` — thêm **hai nested fn** trong `chay_mot_lan` (không đụng module
khác, không đổi chữ ký/không gọi ra ngoài):

- `doc_exit(p) -> Option<i32>`: đọc dòng đầu không phải comment/SPDX của `<ca>.exit`, parse `i32`.
- `dong_dau(p, ext) -> String`: đọc dòng kỳ vọng đầu của fixture bất kỳ (dùng lại cho `.err`).

Ba nhánh kỳ vọng mới, đặt **trước** nhánh `.err`/`.out` để không làm đổi hành vi cũ:

1. **`.trap`** (28 ca): lấy mã `VSxxxx` bằng `chan_doan(stderr)` rồi so **exact** với dòng đầu
   `.trap`; đồng thời yêu cầu `exit != 0`. Có `.exit` thì override thành `code == e`.
   Khi red vẫn in cùng cấu trúc `ten:dong:cot: MA msg` nên TAP/JSON không đổi dạng.
2. **`.out_prefix`** (2 ca): `bo_spdx(stdout).starts_with(bo_spdx(prefix))`, exit mặc định `0`
   (override bởi `.exit` nếu có).
3. **`.exit`**: helper override hardcode `101/2` ở nhánh `.err` và `0` ở nhánh `.out`;
   `.err` vẫn giữ nguyên `stderr.contains(mong)` và fallback mã `VS05… ⇒ 101`.

**Không đổi**: golden, định dạng TAP/JSON, `vis-interp`, `vis-vm`, VBC1, cache, `Cargo.toml`.

## 3. Test & cổng

- Toolchain: phải đưa `/home/ps/.cargo/bin` vào `PATH` để rustup proxy chọn **1.100.0** theo
  `rust-toolchain.toml` (`rustc` hệ thống là 1.93.1 ⇒ bị từ chối `rust-version`).
- `TMPDIR=/home/ps/vislogs/st321b-tmp` (đĩa, không phải tmpfs), `CARGO_BUILD_JOBS=2`.
- `test-child` được build riêng (`cargo build -p vis-cli --bin test-child`) — nếu thiếu, 4 ca
  `vish/ngoai-*` trong `test/sem` đỏ **VS0501** do `No such file or directory`, không liên quan patch.
- Log:
  - `/home/ps/vislogs/st321b-conformance-interp.log` = `...-vm.log`, sha256
    `6b6c6568c4799a2463bd4bf773b6f851802ef00ae61449f2d64cc40ec542de06` (568 dòng `ok`, 0 `not ok`).
  - `/home/ps/vislogs/st321b-sem-interp.log` = `...-vm.log`, sha256
    `0c8f158baf57e8bb2b8f8bb52c295b2386f06d3e43ad8279a33d8c4e9018d6ea` (525 dòng `ok`, 0 `not ok`).
  - `/home/ps/vislogs/st321b-cargo-test-vis-cli.log`, `...-gate-fast.log`, `...-diff-100k.log`, `...-build.log`.
- `gate --fast` chạy đúng lệnh clippy bắt buộc: `cargo clippy --manifest-path … --workspace
  --all-targets -- -D warnings` → `--- clippy: OK`; tổng `vis-dev check: PASS`, `L0 test: OK`.
- `vis-diff`: `DIFF tong=100000 loi_tinh=5575 chay=94425 lech=0 giay=168.7 toc_do=593/s
  seed=3523158054`, `test result: ok. 7 passed`.

## 4. R8 & đĩa

- `find crates/vis-cli/tests -maxdepth 1 -name '*.rs'` → **8** (trần 15);
  `find crates/*/tests vis-host-linux/tests -maxdepth 1 -name '*.rs'` → **39** (trần 40).
  `git status` trước commit chỉ có **1** tệp `M crates/vis-cli/src/vistest/ca.rs`.
- Đĩa: đầu **13 GB** available; sau `gate --fast` tụt **4.9 GB** (dưới ngưỡng 6 GB) nên **tạm dừng**
  mọi việc nặng; sau khi hệ thống giải phóng, đo lại **9.1 GB** mới chạy `vis-diff`; kết thúc **7.2 GB**.
  Không kill tiến trình nền nào.

## 5. Đề xuất

Gatekeeper audit + merge `plan/st321b` vào `main` khi có mốc phù hợp; **không** tự merge/push ở đây.
