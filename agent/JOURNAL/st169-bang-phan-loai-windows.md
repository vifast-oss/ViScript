<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-169 · Bảng phân loại **105 ca đỏ** `cargo test --workspace` trên Windows (`psvn`)

**Nguồn**: `JOURNAL/st161-cargo-test-psvn-nff.log` @ `0df923c2` (131 suite, 1068 đạt / 105 đỏ).
**Vai trò**: đầu vào cho **ST-168 mục 5** (Windows "hỗ trợ thật", VD-162). **Chỉ đọc log + mã**, không sửa.
**Quy ước đường dẫn (VD-165 §2)**: `file:dòng` tính từ **gốc ViScript** (sau khi tách repo), tức không có tiền tố thư mục hiện tại.

## Cách phân loại (theo VD-162(3))

| Nhóm | Nghĩa | Cách xử lý dự kiến |
|---|---|---|
| **a** | Phụ thuộc chương trình POSIX `/bin/*` (hoặc `bash`, `sh`, `python3` ngoài repo) | Thay bằng **chương trình trợ giúp do repo tự build** (VD-162(2)); **cùng kỳ vọng trên cả 3 nền** |
| **b** | **Nghi lỗi thật** của ViScript trên Windows (hoặc hành vi cần quyết định) | Sửa sản phẩm; nếu là khác biệt bản chất ⇒ chuyển **c**; **cần Agent 0 chốt từng ca** |
| **c** | Khác biệt **bản chất nền** (dấu `\`, tên tệp đặc biệt, TAP/golden có đường dẫn tuyệt đối, POSIX bit, signal, symlink…) | Kỳ vọng **theo nền** trong **MỘT tệp danh sách duy nhất** có lý do từng ca; **Agent 0 duyệt danh sách** |

**Đếm**: **a = 48** · **b = 15** · **c = 42** (tổng **105**). Các ca **b** là **ứng viên**, cần Agent 0 xác nhận từng ca
(đúng theo VD-162(3): danh sách kỳ vọng theo nền phải được duyệt).

## Bảng (105 ca)

| # | Ca | Nhóm | `file:dòng` | Bằng chứng (rút từ log) |
|---|---|---|---|---|
| 1 | `cap_proc::khong_strict_khong_cap_van_chay` | **a** | `crates/vis-cli/tests/cap_proc.rs:98:5` | assertion `left == right` failed: interp stderr: c.vish:1:1: error VS0501: không chạy được |
| 2 | `cap_proc::strict_cap_proc_pipeline_hai_stage` | **a** | `crates/vis-cli/tests/cap_proc.rs:133:5` | assertion `left == right` failed: stderr: e.vish:4:1: error VS0501: không chạy được chương |
| 3 | `cap_proc::strict_cap_proc_sh_exit3` | **a** | `crates/vis-cli/tests/cap_proc.rs:115:5` | assertion `left == right` failed: stderr: d.vish:4:1: error VS0501: không chạy được chương |
| 4 | `cap_proc::strict_co_cap_proc_thi_chay_hai_duong` | **a** | `crates/vis-cli/tests/cap_proc.rs:85:5` | assertion `left == right` failed: interp stderr: main.vish:4:1: error VS0501: không chạy đ |
| 5 | `cap_proc_grant::grant_basename_echo_chay` | **a** | `crates/vis-cli/tests/cap_proc_grant.rs:105:5` | assertion `left == right` failed: (2, [], "vis run: `--grant p=echo`: không thấy lệnh `ech |
| 6 | `cap_proc_grant::grant_echo_chan_grep` | **a** | `crates/vis-cli/tests/cap_proc_grant.rs:86:5` | vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot find |
| 7 | `cap_proc_grant::grant_proc_cho_cap_dir_la_loi_kieu` | **a** | `crates/vis-cli/tests/cap_proc_grant.rs:156:5` | assertion `left == right` failed: (0, [49, 10], "") |
| 8 | `cap_proc_grant::grant_tuyet_doi_echo_chay` | **a** | `crates/vis-cli/tests/cap_proc_grant.rs:69:5` | assertion `left == right` failed: (2, [], "vis run: `--grant p=/bin/echo`: không phân giải |
| 9 | `cap_proc_grant::hai_grant_pipeline_hai_stage` | **a** | `crates/vis-cli/tests/cap_proc_grant.rs:141:5` | assertion `left == right` failed: (2, [], "vis run: `--grant p=/bin/echo`: không phân giải |
| 10 | `cap_proc_grant::khong_grant_khong_gioi_han` | **a** | `crates/vis-cli/tests/cap_proc_grant.rs:120:5` | assertion `left == right` failed: (1, [], "d.vish:5:1: error VS0501: không chạy được chươn |
| 11 | `extern::cat_doc_stdin_rong_la_rong` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/cat` — test phải đỏ, không fallback PATH |
| 12 | `extern::duong_dan_hermetic_co_that` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH |
| 13 | `extern::echo_ra_list_str` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH |
| 14 | `extern::echo_raw_giu_newline` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH |
| 15 | `extern::explain_perms_in_nhat_ky` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH |
| 16 | `extern::false_thoat_ma_1_la_vs0501` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/usr/bin/false` — test phải đỏ, không fallback PATH |
| 17 | `extern::printf_nhieu_dong_bo_newline_cuoi` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/usr/bin/printf` — test phải đỏ, không fallback PATH |
| 18 | `extern::stderr_khong_tron_stdout` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/cat` — test phải đỏ, không fallback PATH |
| 19 | `extern::timeout_la_vs0501` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/bin/sleep` — test phải đỏ, không fallback PATH |
| 20 | `extern::true_khong_in_gi` | **a** | `crates/vis-cli/tests/extern.rs:35:5` | thiếu chương trình hermetic `/usr/bin/true` — test phải đỏ, không fallback PATH |
| 21 | `grant_proc_canon::grant_duong_dan_tuong_doi` | **a** | `crates/vis-cli/tests/grant_proc_canon.rs:227:5` | assertion `left == right` failed: (1, [], "e.vish:5:1: error VS0501: không chạy được chươn |
| 22 | `grant_proc_canon::grant_ten_tran_phan_giai_path` | **a** | `crates/vis-cli/tests/grant_proc_canon.rs:174:5` | assertion `left == right` failed: (2, [], "vis run: `--grant p=echo`: không thấy lệnh `ech |
| 23 | `grant_proc_canon::khong_grant_khong_gioi_han` | **a** | `crates/vis-cli/tests/grant_proc_canon.rs:287:5` | assertion `left == right` failed: (1, [], "g.vish:5:1: error VS0501: không chạy được chươn |
| 24 | `grant_proc_canon::sao_chep_trung_ten_bi_chan` | **a** | `crates/vis-cli/tests/grant_proc_canon.rs:109:5` | vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot find |
| 25 | `grant_proc_canon::tuyet_doi_khop_thi_chay` | **a** | `crates/vis-cli/tests/grant_proc_canon.rs:91:5` | assertion `left == right` failed: (2, [], "vis run: `--grant p=/bin/echo`: không phân giải |
| 26 | `ngoai_vm::cat_pipe_vm_cung_byte` | **a** | `crates/vis-cli/tests/ngoai_vm.rs:56:5` | assertion `left == right` failed: [67, 58, 92, 85, 115, 101, 114, 115, 92, 112, 115, 92, 6 |
| 27 | `ngoai_vm::complete_ma_thoat_vm_cung_byte` | **a** | `crates/vis-cli/tests/ngoai_vm.rs:64:5` | assertion `left == right` failed: [67, 58, 92, 85, 115, 101, 114, 115, 92, 112, 115, 92, 6 |
| 28 | `ngoai_vm::sh_exit3_vm_cung_byte` | **a** | `crates/vis-cli/tests/ngoai_vm.rs:88:5` | assertion `left == right` failed: [67, 58, 92, 85, 115, 101, 114, 115, 92, 112, 115, 92, 6 |
| 29 | `pipeline::chuong_trinh_ngoai_co_that` | **a** | `crates/vis-cli/tests/pipeline.rs:18:5` | thiếu chương trình ngoài `/bin/echo` (test giả định POSIX có nó) |
| 30 | `pipeline::complete_giu_stderr_va_code_khac_0` | **a** | `crates/vis-cli/tests/pipeline.rs:130:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-cmp3-169 |
| 31 | `pipeline::complete_grep_khong_khop_het_la_loi` | **a** | `crates/vis-cli/tests/pipeline.rs:104:5` | assertion `left == right` failed: complete ⇒ **không** còn VS0501: C:\Users\ps\AppData\Loc |
| 32 | `pipeline::complete_grep_khop_code_0` | **a** | `crates/vis-cli/tests/pipeline.rs:119:5` | assertion `left == right` failed |
| 33 | `pipeline::echo_qua_cat_nhan_stdin` | **a** | `crates/vis-cli/tests/pipeline.rs:72:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-cat-1691 |
| 34 | `pipeline::echo_qua_grep_tim_thay` | **a** | `crates/vis-cli/tests/pipeline.rs:81:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-grep-169 |
| 35 | `pipeline::fixture_vish_trong_test_cmd_pipe` | **a** | `crates/vis-cli/tests/pipeline.rs:255:9` | assertion `left == right` failed: fixture cat: C:\Users\ps\vifast-windows\repo\viscript\cr |
| 36 | `pipeline::grep_khong_khop_van_la_loi_khi_khong_complete` | **a** | `crates/vis-cli/tests/pipeline.rs:91:5` | stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-grep0-16912\grep0.vish:2:1: error VS0501:  |
| 37 | `pipeline::list_nhieu_dong_qua_grep_loc_dung_dong` | **a** | `crates/vis-cli/tests/pipeline.rs:202:5` | assertion `left == right` failed |
| 38 | `pipeline::printf_hai_dong_qua_cat_ra_hai_dong` | **a** | `crates/vis-cli/tests/pipeline.rs:188:5` | assertion `left == right` failed |
| 39 | `pipeline::printf_khong_newline_qua_cat_khong_them_dong_rong` | **a** | `crates/vis-cli/tests/pipeline.rs:179:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-printf-1 |
| 40 | `pipeline::raw_str_di_nguyen_van_khong_them_newline` | **a** | `crates/vis-cli/tests/pipeline.rs:212:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-raw-1691 |
| 41 | `pipeline::timeout_neu_ro_stage` | **a** | `crates/vis-cli/tests/pipeline.rs:230:5` | thông điệp timeout phải nêu stage: C:\Users\ps\AppData\Local\Temp\vis-pipe-to-16912\to.vis |
| 42 | `pipeline::vm_co_host_chay_cung_interp` | **a** | `crates/vis-cli/tests/pipeline.rs:298:5` | assertion `left == right` failed |
| 43 | `soak_script::vis_soak_hop_le_va_in_du_10_vong` | **a** | `crates/vis-cli/tests/soak_script.rs:49:10` | chạy --dry-run: Os { code: 193, kind: Uncategorized, message: "%1 is not a valid Win32 app |
| 44 | `sync_list::check_sync_du_het_thi_exit_0_trong_cay_gia` | **a** | `crates/vis-cli/tests/sync_list.rs:338:5` | assertion `left == right` failed: đủ mục ⇒ exit 0. stdout: "" |
| 45 | `sync_list::check_sync_tai_repo_that_in_du_8_muc_va_exit_0` | **a** | `crates/vis-cli/tests/sync_list.rs:288:5` | assertion `left == right` failed: stdout: "" |
| 46 | `sync_list::check_sync_thieu_mot_muc_thi_exit_2` | **a** | `crates/vis-cli/tests/sync_list.rs:309:5` | assertion `left == right` failed: thiếu mục ⇒ exit 2. stdout: "" |
| 47 | `sync_list::help_co_check_sync` | **a** | `crates/vis-cli/tests/sync_list.rs:359:5` | assertion `left == right` failed: stderr: <3>WSL (9 - Relay) ERROR: CreateProcessCommon:88 |
| 48 | `vish_main::main_rong_va_pipeline_hop_le` | **a** | `crates/vis-cli/tests/vish_main.rs:61:5` | assertion `left == right` failed: interp stderr: a.vish:4:1: error VS0501: không chạy được |
| 49 | `?::host_conformance_suite` | **b** | `` | ok 1 - fs_read_thuong |
| 50 | `json_ra::check_json_ba_tep_dung_thu_tu` | **b** | `crates/vis-cli/tests/json_ra.rs:97:52` | thiếu f1 |
| 51 | `json_ra::check_json_loi_hai_tep` | **b** | `crates/vis-cli/tests/json_ra.rs:272:5` | assertion `left == right` failed |
| 52 | `json_ra::check_json_loi_trong_tep_phu` | **b** | `crates/vis-cli/tests/json_ra.rs:248:5` | assertion `left == right` failed: {"ok":false,"files":[{"file":"C:\\Users\\ps\\AppData\\Lo |
| 53 | `json_ra::check_json_severity_warning_tab` | **b** | `crates/vis-cli/tests/json_ra.rs:169:5` | python3 lỗi: Traceback (most recent call last): |
| 54 | `json_ra::check_json_tep_sach` | **b** | `crates/vis-cli/tests/json_ra.rs:55:5` | assertion `left == right` failed |
| 55 | `lsp::lsp_completion_thay_ham_module` | **b** | `crates/vis-cli/tests/lsp.rs:875:5` | assertion `left == right` failed: {"jsonrpc":"2.0","id":2,"result":[]} |
| 56 | `lsp::lsp_completion_thay_std_da_use` | **b** | `crates/vis-cli/tests/lsp.rs:911:5` | {"jsonrpc":"2.0","id":2,"result":[]} |
| 57 | `lsp::lsp_hover_ham_module` | **b** | `crates/vis-cli/tests/lsp.rs:895:5` | {"jsonrpc":"2.0","id":2,"result":null} |
| 58 | `lsp::lsp_module_std_sai_giong_vis_check` | **b** | `crates/vis-cli/tests/lsp.rs:753:5` | assertion `left == right` failed: LSP khác vis check |
| 59 | `lsp::lsp_module_thieu_giong_vis_check` | **b** | `crates/vis-cli/tests/lsp.rs:779:5` | LSP thiếu VS0701: [] |
| 60 | `net::tcp_tu_choi` | **b** | `crates/vis-cli/tests/net.rs:153:5` | stdout: "lỗi: quá thời gian chờ nối tới `127.0.0.1:50267`\n" |
| 61 | `net_server::server_hai_ket_noi_lien_tiep` | **b** | `crates/vis-cli/tests/net_server.rs:67:9` | quá 5 s chưa thấy địa chỉ trong C:\Users\ps\AppData\Local\Temp\vis-srv-hai-ket-noi-5124\ad |
| 62 | `net_server::server_hai_tien_trinh` | **b** | `crates/vis-cli/tests/net_server.rs:67:9` | quá 5 s chưa thấy địa chỉ trong C:\Users\ps\AppData\Local\Temp\vis-srv-hai-5124\addr-0.txt |
| 63 | `net_server::server_mot_lan` | **b** | `crates/vis-cli/tests/net_server.rs:67:9` | quá 5 s chưa thấy địa chỉ trong C:\Users\ps\AppData\Local\Temp\vis-srv-mot-5124\addr.txt |
| 64 | `chan_doan_da_tep::b_dai_van_dung_a` | **c** | `crates/vis-cli/tests/chan_doan_da_tep.rs:184:5` | assertion `left == right` failed: C:\Users\ps\AppData\Local\Temp\vis-da-tep-dai-14424-1\a. |
| 65 | `chan_doan_da_tep::loi_ca_hai_tep` | **c** | `crates/vis-cli/tests/chan_doan_da_tep.rs:117:5` | assertion `left == right` failed: C:\Users\ps\AppData\Local\Temp\vis-da-tep-hai-14424-0\a. |
| 66 | `chan_doan_da_tep::loi_cap_tep_a` | **c** | `crates/vis-cli/tests/chan_doan_da_tep.rs:84:5` | assertion `left == right` failed: C:\Users\ps\AppData\Local\Temp\vis-da-tep-a-14424-2\a.vi |
| 67 | `chan_doan_da_tep::loi_trong_b_dong_sau` | **c** | `crates/vis-cli/tests/chan_doan_da_tep.rs:106:5` | assertion `left == right` failed: C:\Users\ps\AppData\Local\Temp\vis-da-tep-b-14424-3\a.vi |
| 68 | `chan_doan_da_tep::loi_trong_fn_a` | **c** | `crates/vis-cli/tests/chan_doan_da_tep.rs:95:5` | assertion `left == right` failed: C:\Users\ps\AppData\Local\Temp\vis-da-tep-fn-14424-4\a.v |
| 69 | `diag_cmp::ca_xanh_ba_duong_khop_stdout` | **c** | `crates/vis-cli/tests/diag_cmp.rs:494:5` | 4 ca lệch: |
| 70 | `json_ra::check_json_ten_tep_dac_biet_parse_duoc` | **c** | `crates/vis-cli/tests/json_ra.rs:24:29` | ghi: Os { code: 123, kind: InvalidFilename, message: "The filename, directory name, or vol |
| 71 | `ls_open::khong_mock_thi_dung_fs_that` | **c** | `crates/vis-cli/tests/ls_open.rs:250:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-ls-real-17292 |
| 72 | `lsp::lsp_ba_tep_bang_vis_check` | **c** | `crates/vis-cli/tests/lsp.rs:820:9` | assertion `left == right` failed: lệch ở C:\Users\ps\AppData\Local\Temp\vis-lsp-so-16892-0 |
| 73 | `lsp_da_tep::a_sach_rong_b_co_loi` | **c** | `crates/vis-cli/tests/lsp_da_tep.rs:154:5` | Content-Length: 199 |
| 74 | `lsp_da_tep::did_change_van_dung_a` | **c** | `crates/vis-cli/tests/lsp_da_tep.rs:182:5` | Content-Length: 199 |
| 75 | `lsp_da_tep::mo_a_chi_chan_doan_a` | **c** | `crates/vis-cli/tests/lsp_da_tep.rs:114:5` | Content-Length: 199 |
| 76 | `lsp_da_tep::mo_b_chan_doan_b` | **c** | `crates/vis-cli/tests/lsp_da_tep.rs:134:5` | Content-Length: 199 |
| 77 | `quyen::doc_ngoai_quyen` | **c** | `crates/vis-cli/tests/quyen.rs:92:5` | stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 78 | `quyen::doc_trong_quyen` | **c** | `crates/vis-cli/tests/quyen.rs:70:5` | assertion `left == right` failed: stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 79 | `quyen::ghi_co_co` | **c** | `crates/vis-cli/tests/quyen.rs:135:5` | assertion `left == right` failed: stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 80 | `quyen::ghi_thieu_co` | **c** | `crates/vis-cli/tests/quyen.rs:114:5` | stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 81 | `quyen::grant_doi_duong_dan` | **c** | `crates/vis-cli/tests/quyen.rs:180:5` | stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 82 | `quyen::hai_cap_dir` | **c** | `crates/vis-cli/tests/quyen.rs:227:5` | stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 83 | `quyen::khong_khai_cap_thi_khong_han_che` | **c** | `crates/vis-cli/tests/quyen.rs:154:5` | assertion `left == right` failed: stderr: f.vis:4:19: error VS0102: escape không hợp lệ |
| 84 | `quyen::net_thieu_cap` | **c** | `crates/vis-cli/tests/quyen.rs:200:5` | stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 85 | `sem::sem_323_ca_qua_binary` | **c** | `crates/vis-cli/tests/sem.rs:230:5` | 4/447 ca sai: |
| 86 | `siet_caps_pkg::co_pkg_va_khai_du_cap_thi_chay` | **c** | `crates/vis-cli/tests/siet_caps_pkg.rs:209:5` | assertion `left == right` failed: stderr: f.vis:4:23: error VS0102: escape không hợp lệ |
| 87 | `span_tep::json_bao_dung_tep_va_dong_cot_moi_ca` | **c** | `crates/vis-cli/tests/span_tep.rs:93:5` | 5 ca sai: |
| 88 | `st83_no_nho::exists_ngoai_quyen_trap` | **c** | `crates/vis-cli/tests/st83_no_nho.rs:72:5` | f.vis:3:23: error VS0102: escape không hợp lệ |
| 89 | `st83_no_nho::exists_trong_quyen_tra_bool` | **c** | `crates/vis-cli/tests/st83_no_nho.rs:98:5` | assertion `left == right` failed: stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 90 | `std_fs_json::differential_fs_va_json_khop_hai_duong` | **c** | `crates/vis-cli/tests/std_fs_json.rs:52:5` | assertion `left == right` failed: [diff] interp exit 2: diff.vis:5:20: error VS0102: escap |
| 91 | `std_fs_json::explain_perms_in_fs_nhat_ky` | **c** | `crates/vis-cli/tests/std_fs_json.rs:377:5` | assertion `left == right` failed: stderr: log.vis:4:20: error VS0102: escape không hợp lệ |
| 92 | `std_fs_json::fs_duong_dan_tuyet_doi_duoc_phep` | **c** | `crates/vis-cli/tests/std_fs_json.rs:52:5` | assertion `left == right` failed: [abs] interp exit 2: abs.vis:4:20: error VS0102: escape  |
| 93 | `std_fs_json::fs_exists_va_read_thieu_la_err` | **c** | `crates/vis-cli/tests/std_fs_json.rs:52:5` | assertion `left == right` failed: [ex] interp exit 2: ex.vis:4:21: error VS0102: escape kh |
| 94 | `std_fs_json::fs_ghi_vao_duong_dan_sai_la_err` | **c** | `crates/vis-cli/tests/std_fs_json.rs:52:5` | assertion `left == right` failed: [bad] interp exit 2: bad.vis:4:20: error VS0102: escape  |
| 95 | `std_fs_json::fs_list_sap_xep_khong_de_quy` | **c** | `crates/vis-cli/tests/std_fs_json.rs:52:5` | assertion `left == right` failed: [ls] interp exit 2: ls.vis:4:19: error VS0102: escape kh |
| 96 | `std_fs_json::fs_write_read_round_trip` | **c** | `crates/vis-cli/tests/std_fs_json.rs:52:5` | assertion `left == right` failed: [rw] interp exit 2: rw.vis:4:20: error VS0102: escape kh |
| 97 | `strict_caps::strict_cap_dir_ngoai_root` | **c** | `crates/vis-cli/tests/strict_caps.rs:122:5` | f.vis:3:23: error VS0102: escape không hợp lệ |
| 98 | `strict_caps::strict_cap_dir_trong_root` | **c** | `crates/vis-cli/tests/strict_caps.rs:99:5` | assertion `left == right` failed: stderr: f.vis:3:23: error VS0102: escape không hợp lệ |
| 99 | `vis_test::cac_dong_ok_dung_thu_tu_khong_lap` | **c** | `crates/vis-cli/tests/vis_test.rs:62:9` | dòng thứ 423 sai thứ tự: ok 426 - vish\ngoai-thoat-ma.vish |
| 100 | `vis_test::tap_hop_le_va_du_so_ca` | **c** | `crates/vis-cli/tests/vis_test.rs:49:5` | assertion `left == right` failed: exit phải 0 khi toàn xanh: |
| 101 | `vis_test::vm_khop_interp_va_khong_con_skip` | **c** | `crates/vis-cli/tests/vis_test.rs:76:5` | assertion `left == right` failed: VM: exit phải 0 (ca cần host thì SKIP): |
| 102 | `vish_cmd_body::khong_bind_van_la_lenh` | **c** | `crates/vis-cli/tests/vish_cmd_body.rs:356:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-cmdbody-khong |
| 103 | `vish_cmd_body::than_lenh_tren_thu_muc_that` | **c** | `crates/vis-cli/tests/vish_cmd_body.rs:312:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-cmdbody-that- |
| 104 | `vm_main_argv::argv_dau_cap_sau` | **c** | `crates/vis-cli/tests/vm_main_argv.rs:112:5` | assertion `left == right` failed: (2, [], "h.vis:4:40: error VS0102: escape không hợp lệ\n |
| 105 | `vsh1b::long_tren_thu_muc_that` | **c** | `crates/vis-cli/tests/vsh1b.rs:161:5` | assertion `left == right` failed: stderr: C:\Users\ps\AppData\Local\Temp\vis-vsh1b-that-75 |

## Ghi chú

1. **a (48)**: chia 3 nhóm con — `/bin/*` trực tiếp (`cap_proc*`, `grant_proc_canon`, `extern`, `pipeline`, `ngoai_vm`, `vish_main`),
   `bash`/`sh` thiếu (`sync_list`), và `python3`/tệp thực thi Windows (`soak_script`, `json_ra::check_json_severity_warning_tab`).
   Cả 3 đều xử được bằng helper repo-built + biến môi trường, **không** sửa kỳ vọng.
2. **b (15)**: `lsp` 6 (completion/hover rỗng, "LSP khác vis check", thiếu `VS0701`) · `net_server` 3 (không thấy tệp `addr`)
   · `net` 1 (timeout nối) · `host_conformance` 1 (TAP) · `json_ra` 4 (tên tệp đặc biệt/`thiếu f1`/JSON đường dẫn). Cần Agent 0 chốt.
3. **c (42)**: chủ yếu là **dấu `\`** trong đường dẫn đưa vào mã `.vis`/golden ⇒ lỗi `escape không hợp lệ` (quyen 8, std_fs_json 7,
   strict_caps 2, st83 2, siet_caps 1, vm_main_argv 1) hoặc đường dẫn tuyệt đối trong TAP/golden (chan_doan_da_tep 5, vis_test 3,
   vish_cmd_body 2, vsh1b 1, ls_open 1, sem 1, span_tep 1, diag_cmp 1, json_ra 4). Cách xử lý **không** phải "sửa sản phẩm":
   test nên dùng `/` hoặc escape, và golden nên so **tương đối**; khi đó nhiều ca sẽ xanh **mà không cần** kỳ vọng theo nền.
4. **Đề nghị**: trước khi mở danh sách kỳ vọng theo nền, thử **sửa 42 ca (c)** theo hướng "test dùng `/`/tương đối" — nếu xanh hết thì
   danh sách theo nền thu về **0**, đúng tinh thần VD-162(1) "chạy native, cùng kỳ vọng". Việc này thuộc **ST-168 mục 5** (tài liệu), **chưa** làm trong ST-169.
5. Bảng này **không** thay đổi số đo: baseline Windows vẫn **1068/105 @0df923c2**; cổng Windows vẫn dừng **L0** (150/4 fail-fast).
