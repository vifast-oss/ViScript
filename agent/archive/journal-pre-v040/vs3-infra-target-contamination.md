# Infra: target dùng chung làm đỏ check trên main (22/11/2026)

- **Hiện tượng**: sau merge `b77689c`, `./vis-dev check` báo
  `cannot find function run_workers_local in crate vis_interp` ở
  `crates/vis-cli/tests/vs3_worker_eval_local.rs`, dù `cargo test -p vis-cli --test
  vs3_worker_eval_local` trực tiếp **8/8 PASS** và `run_workers_local` có trong
  `crates/vis-interp/src/lib.rs`.
- **Nguyên nhân**: worktree W2 (`vs3-w2-tls`, source cũ hơn P1) build với
  `CARGO_TARGET_DIR=/home/ps/ViScript/target`, ghi đè artifact/rmeta `vis-interp` của main.
  Không phải lỗi mã P1.
- **Xử lý**: W2 chuyển sang target riêng trong worktree; main chạy
  `cargo clean -p vis-interp` (1,3 GiB) rồi `./vis-dev check` + `gate --fast`:
  **PASS — 3547 passed / 3 skipped**, R1 OK.
- **Luật từ nay**: mỗi worktree/subagent dùng `CARGO_TARGET_DIR` riêng; không build vào
  `target/` của main khi source khác commit.
