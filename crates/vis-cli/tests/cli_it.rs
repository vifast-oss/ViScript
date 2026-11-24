// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: CLI: check/fmt/run/test/explain/json/doctor + diag/security.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

#[path = "cli_it/cache_fastpath.rs"]
mod cache_fastpath;
#[path = "cli_it/chan_doan_da_tep.rs"]
mod chan_doan_da_tep;
#[path = "cli_it/cheatsheet.rs"]
mod cheatsheet;
#[path = "cli_it/check.rs"]
mod check;
#[path = "cli_it/cmd_run.rs"]
mod cmd_run;
#[path = "cli_it/crlf.rs"]
mod crlf;
#[path = "cli_it/diag_cmp.rs"]
mod diag_cmp;
#[path = "cli_it/diff_loi.rs"]
mod diff_loi;
#[path = "cli_it/explain.rs"]
mod explain;
#[path = "cli_it/explain_moi.rs"]
mod explain_moi;
#[path = "cli_it/extern.rs"]
mod r#extern;
#[path = "cli_it/fmt.rs"]
mod fmt;
#[path = "cli_it/fmt_indent.rs"]
mod fmt_indent;
#[path = "cli_it/fmt_moi.rs"]
mod fmt_moi;
#[path = "cli_it/json_ra.rs"]
mod json_ra;
#[path = "cli_it/json_sort.rs"]
mod json_sort;
#[path = "cli_it/ls_open.rs"]
mod ls_open;
#[path = "cli_it/modules.rs"]
mod modules;
#[path = "cli_it/nhom.rs"]
mod nhom;
#[path = "cli_it/pipeline.rs"]
mod pipeline;
#[path = "cli_it/render.rs"]
mod render;
#[path = "cli_it/repl_term.rs"]
mod repl_term;
#[path = "cli_it/soak_script.rs"]
mod soak_script;
#[path = "cli_it/span_tep.rs"]
mod span_tep;
#[path = "cli_it/st240_std_cli.rs"]
mod st240_std_cli;
#[path = "cli_it/st246_cache_compat.rs"]
mod st246_cache_compat;
#[path = "cli_it/st289_version.rs"]
mod st289_version;
#[path = "cli_it/sync_list.rs"]
mod sync_list;
#[path = "cli_it/targets.rs"]
mod targets;
#[path = "cli_it/vd270_check_json.rs"]
mod vd270_check_json;
#[path = "cli_it/vd270_doctor.rs"]
mod vd270_doctor;
#[path = "cli_it/vd270_explain_json.rs"]
mod vd270_explain_json;
#[path = "cli_it/vd270_fmt_diff.rs"]
mod vd270_fmt_diff;
#[path = "cli_it/vd270_fmt_json.rs"]
mod vd270_fmt_json;
#[path = "cli_it/vd270_fmt_list.rs"]
mod vd270_fmt_list;
#[path = "cli_it/vd270_fmt_stat.rs"]
mod vd270_fmt_stat;
#[path = "cli_it/vd270_run_time.rs"]
mod vd270_run_time;
#[path = "cli_it/vd270_test_jobs.rs"]
mod vd270_test_jobs;
#[path = "cli_it/vd270_test_shard.rs"]
mod vd270_test_shard;
#[path = "cli_it/vd270_test_slowest.rs"]
mod vd270_test_slowest;
#[path = "cli_it/vd270_version.rs"]
mod vd270_version;
#[path = "cli_it/vd270_vistest_jsonout.rs"]
mod vd270_vistest_jsonout;
#[path = "cli_it/vd270_vistest_repeat.rs"]
mod vd270_vistest_repeat;
#[path = "cli_it/vd270_vistest_shuffle.rs"]
mod vd270_vistest_shuffle;
#[path = "cli_it/vd270_vistest_timeout.rs"]
mod vd270_vistest_timeout;
#[path = "cli_it/vis_check_stdin.rs"]
mod vis_check_stdin;
#[path = "cli_it/vis_run_stdin.rs"]
mod vis_run_stdin;
#[path = "cli_it/vis_test.rs"]
mod vis_test;
#[path = "cli_it/vis_test_filter.rs"]
mod vis_test_filter;
#[path = "cli_it/vis_test_list.rs"]
mod vis_test_list;
#[path = "cli_it/vish_cmd_body.rs"]
mod vish_cmd_body;
#[path = "cli_it/vish_decl.rs"]
mod vish_decl;
#[path = "cli_it/vish_indent.rs"]
mod vish_indent;
#[path = "cli_it/vish_main.rs"]
mod vish_main;
#[path = "cli_it/vish_nhom_vi_tri.rs"]
mod vish_nhom_vi_tri;
#[path = "cli_it/vistest_async.rs"]
mod vistest_async;
#[path = "cli_it/vm_main_argv.rs"]
mod vm_main_argv;
#[path = "cli_it/vsh1b.rs"]
mod vsh1b;
#[path = "cli_it/where_tinh.rs"]
mod where_tinh;

#[path = "cli_it/st303_cli_dev.rs"]
mod st303_cli_dev;
