<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300m3 — race driver WASI (fix)

- **Phát hiện**: final Linux QA `v0.5.0` soak 10/10 dừng ở gate-3 với
  `vis-plugin-abi::vs3_plugin cap_dir_la_sandbox_tu_nhien` → `VS0501: manifest plugin thiếu dòng kết ok`.
- **Nguyên nhân**: `thu_muc_driver()` dùng chung `$TMPDIR/vis-plugin-wasi-driver` và ghi đè
  `driver.mjs`/`driver.py` mỗi lần gọi. Test plugin chạy song song trong cùng process ⇒ Node
  có thể đọc driver đang bị truncate, stdout rỗng. Không phải lỗi tải thuần.
- **Fix**: mỗi invocation một thư mục `vis-plugin-wasi-driver-<pid>-<dem>`; thêm unit test
  `wasi::tests_driver_rieng::moi_lan_goi_thu_muc_driver_rieng`.
- **Bằng chứng**: `cargo test -p vis-plugin-abi` **3 lần liên tiếp 6/6**, CLI `vs3_plugin` **4/4**,
  gate Linux **3748/4**; không đổi API/hành vi plugin.
- **Log**: `/home/ps/vislogs/st300m3-driver-race-*` (sha256 trong journal QA gốc).
