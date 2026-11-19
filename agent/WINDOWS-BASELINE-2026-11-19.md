# WINDOWS-BASELINE-2026-11-19 — Windows/psvn baseline trước VD portability

Ghi theo điều kiện A0 đóng đợt Windows (2026-11-19, câu trả lời (iii)): kết quả chạy gắn với
**code commit `349af67`** (tách biệt với HEAD tài liệu `a4602cb` — `349af67..a4602cb` chỉ gồm tài liệu),
và danh sách baseline đỏ được lưu bền vững.

- **HEAD code**: `349af67b15ae7ad6c69c2a98a0127a4aea7a1eff` (nhánh `main`, origin/main).
- **HEAD sau đợt này**: `a4602cb` (docs), `.vis`-level toàn bộ 349af67 giữ nguyên.
- **Máy**: psvn (Windows 11 build 26300, x86_64, 6 lõi; Git Bash 5.2.37; cargo/rustc 1.100.0; repo `/c/Users/ps/viscript-moi`, `core.autocrlf=false`, dirty=0, `LC_ALL=C.UTF-8`).
- **Điều kiện**: Tier 2 — không phải cổng bắt buộc L0; workspace L0 đỏ **theo thiết kế VS3 của bộ test**, không di chuyển pha1-rc1, không dùng Windows/Linux thay bằng chứng mac.

## 1. Kết quả gắn với `349af67`

### PASS (bằng chứng sản phẩm)

| Hạng mục | Kết quả |
|---|---|
| fmt / clippy / luật R1..R6 / self-test 13/13 | OK |
| vis explain | OK |
| report-check | PASS (30 báo cáo, 243 hàng QUEUE) |
| build | OK |
| doc-examples | OK (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3) |
| `vis-dev check` tổng | **PASS** (vis-dev tự chọn cargo 1.100.0) |
| `vis test test/sem` (interp) | **483/483** (4 ca đỏ duy nhất = `vish/ngoai-{echo,printf,raw,true}`) |
| `vis test --vm test/sem` | **483/483** (cùng 4 ca) |
| `cargo test -p vis-cli --test crlf` | **3/3** |
| LSP `file:///c:/...` | URI chấp nhận; module-aware publishDiagnostics (test thủ công AGENT-3.08/5 ca: derive-eq, std-method, xin-chao, trung-ten VS0302, test/fmt/crlf) |

### FAIL — baseline đỏ (chưa port; VD portability sau đóng pha 1)

**25 suite, 101 ca đỏ** (90 tên test duy nhất). Đây là **baseline**: VD portability phân loại từng ca theo hợp đồng thật.

```
cap_proc (4)  cap_proc_grant (≥6)  chan_doan_da_tep  diag_cmp  extern
grant_proc_canon  host_conformance  json_ra  net  net_server  ngoai_vm
pipeline  quyen  sem (4)  siet_caps_pkg  soak_script  span_tep  st83_no_nho
std_fs_json (7)  strict_caps (2)  sync_list (4)  vis_test (3)
vish_main (1)  vm_main_argv (1)  vsh1b (1)
```

List 90 tên duy nhất:

```
argv_dau_cap_sau            b_dai_van_dung_a          cac_dong_ok_dung_thu_tu_khong_lap
cat_doc_stdin_rong_la_rong  cat_pipe_vm_cung_byte     ca_xanh_ba_duong_khop_stdout
check_json_ba_tep_dung_thu_tu     check_json_loi_hai_tep     check_json_loi_trong_tep_phu
check_json_severity_warning_tab   check_json_ten_tep_dac_biet_parse_duoc  check_json_tep_sach
check_sync_du_het_thi_exit_0_trong_cay_gia           check_sync_tai_repo_that_in_du_8_muc_va_exit_0
check_sync_thieu_mot_muc_thi_exit_2                  chuong_trinh_ngoai_co_that
complete_giu_stderr_va_code_khac_0  complete_grep_khong_khop_het_la_loi  complete_grep_khop_code_0
complete_ma_thoat_vm_cung_byte   co_pkg_va_khai_du_cap_thi_chay
differential_fs_va_json_khop_hai_duong  doc_ngoai_quyen  doc_trong_quyen  duong_dan_hermetic_co_that
echo_qua_cat_nhan_stdin  echo_qua_grep_tim_thay  echo_ra_list_str  echo_raw_giu_newline
exists_ngoai_quyen_trap  exists_trong_quyen_tra_bool
explain_perms_in_fs_nhat_ky  explain_perms_in_nhat_ky  false_thoat_ma_1_la_vs0501
fixture_vish_trong_test_cmd_pipe  fs_duong_dan_tuyet_doi_duoc_phep
fs_exists_va_read_thieu_la_err    fs_ghi_vao_duong_dan_sai_la_err
fs_list_sap_xep_khong_de_quy      fs_write_read_round_trip
ghi_co_co  ghi_thieu_co
grant_basename_echo_chay  grant_doi_duong_dan  grant_duong_dan_tuong_doi  grant_echo_chan_grep
grant_proc_cho_cap_dir_la_loi_kieu  grant_ten_tran_phan_giai_path  grant_tuyet_doi_echo_chay
grep_khong_khop_van_la_loi_khi_khong_complete
hai_cap_dir  hai_grant_pipeline_hai_stage  help_co_check_sync  host_conformance_suite
json_bao_dung_tep_va_dong_cot_moi_ca  khong_grant_khong_gioi_han  khong_khai_cap_thi_khong_han_che
khong_strict_khong_cap_van_chay   list_nhieu_dong_qua_grep_loc_dung_dong
loi_ca_hai_tep  loi_cap_tep_a  loi_trong_b_dong_sau  loi_trong_fn_a  long_tren_thu_muc_that
main_rong_va_pipeline_hop_le
printf_hai_dong_qua_cat_ra_hai_dong  printf_khong_newline_qua_cat_khong_them_dong_rong
printf_nhieu_dong_bo_newline_cuoi    raw_str_di_nguyen_van_khong_them_newline
sao_chep_trung_ten_bi_chan  sem_323_ca_qua_binary
server_hai_ket_noi_lien_tiep  server_hai_tien_trinh  server_mot_lan  sh_exit3_vm_cung_byte
stderr_khong_tron_stdout
strict_cap_dir_ngoai_root  strict_cap_dir_trong_root  strict_cap_proc_pipeline_hai_stage
strict_cap_proc_sh_exit3   strict_co_cap_proc_thi_chay_hai_duong
tap_hop_le_va_du_so_ca  tcp_tu_choi  timeout_la_vs0501  timeout_neu_ro_stage
true_khong_in_gi  tuyet_doi_khop_thi_chay  vis_soak_hop_le_va_in_du_10_vong
vm_co_host_chay_cung_interp  vm_khop_interp_va_khong_con_skip
```

Phân loại sơ bộ (chưa port; VD portability chịu trách nhiệm xác nhận độc lập từng ca):

| Lớp | (tiền ước lượng) | Ghi chú |
|---|---|---|
| A — POSIX binaries/WSL stub | ~48 | `/bin/echo`, `/bin/sh`, `/usr/bin/grep`, `printf`, `cat`, `true/false`, `execvpe(/bin/bash)` (WSL stub, sync_list, 4 ca) |
| B — Windows path/CRLF/JSON trong fixture & expectation | ~45 | `check_json_*`, `json_ra`, `span_tep`, `chan_doan_da_tep` (CRLF), `quyen` (8), `std_fs_json` (7), `strict_caps` (2), `vm_main_argv`, `host_conformance case 13` |
| D — WinSock/loopback semantics | 5 | `tcp_tu_choi`, `server_mot_lan`, `server_hai_tien_trinh`, `server_hai_ket_noi_lien_tiep` |
| E — Unix file mode | 1 | `vsh1b.long_tren_thu_muc_that` |

## 2. Log (bền vững — không chỉ /tmp)

psvn (`/c/Users/ps/vislogs/`, không nằm trong repo):
- `windows-349af67-ws.log` — full `cargo test --workspace --no-fail-fast` (stdout+stderr từng suite).
- `windows-349af67-gate.log` — `./vis-dev gate --fast` đầy đủ.
- `windows-349af67-run.sh` — **đã xóa** (rỗng do lỗi piping; không liên quan tới kết quả).

Máy ps (bản sao/đọc trực tiếp):
- `/tmp/opencode/psvn-windows-verify-stream.log` — concat output (3272 dòng; nguồn trích cho file này).
- `/tmp/opencode/psvn-windows-verify-*.log`, `win-msvc-check.log`, `gate-winfix.log`, `ws-winfix.log`, `gate-fix.log`, `ws-fix.log`.

Backup hạ tầng (không commit): `/c/Users/ps/vislogs/administrators_authorized_keys.bak-2026-11-19`.

## 3. Lưu ý / không tuyên bố

- **Không** tuyên bố `cargo test --workspace` hỗ trợ Windows; **không** "full Windows gate PASS";
  **không** dùng Windows/Linux thay bằng chứng mac; `pha1-rc1` không di chuyển.
- Mọi ca đỏ là **chưa được port & xác nhận độc lập**, không khẳng định chúng sai mã sản phẩm.
- VD portability (sau khi pha 1 đóng chính thức) phân loại từng ca theo hợp đồng thật; Unix-only mới
  được `#[cfg(unix)]`, process đa nền tảng phải có fixture executable native, coverage không được giảm.
