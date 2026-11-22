// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-34** — `| complete` + **pipe→stdin** cho chương trình ngoài.
//!
//! Khoá: `a | b` ghi **stdout của `a`** vào **stdin** của `b` (đóng stdin sau khi ghi); `complete` cho
//! record `{stdout: list[str], stderr: list[str], code: int}` và **exit≠0 không còn là lỗi**;
//! `complete` trên stage **Vis** ⇒ `VS0602`; pipe vẫn là **giá trị** (không đổi HIR/codegen).
//! Hermetic: chỉ dùng `{TC} echo`, `{TC} printf`, `{TC} cat`, `{TC} grep` — **assert tồn tại**.

use std::path::PathBuf;
use std::process::Command;

const TC: &str = env!("CARGO_BIN_EXE_test-child");

/// PATH có thêm thư mục chứa `test-child` — cho fixture `.vish` gọi `test-child` bare.
fn path_voi_tc() -> std::ffi::OsString {
    let tc = std::path::Path::new(env!("CARGO_BIN_EXE_test-child"));
    let mut muc: Vec<std::path::PathBuf> =
        vec![tc.parent().expect("test-child parent").to_path_buf()];
    let cur = std::env::var_os("PATH").unwrap_or_default();
    for p in std::env::split_paths(&cur) {
        muc.push(p);
    }
    std::env::join_paths(muc).expect("join PATH")
}

fn goc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../")
}

/// **Assert tồn tại** chương trình ngoài dùng trong test (như ST-27) — thiếu thì đỏ ngay, không flake.
fn co(ten: &str) {
    assert!(
        std::path::Path::new(ten).exists(),
        "thiếu chương trình ngoài `{ten}` (test giả định POSIX có nó)"
    );
}

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-pipe-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn chay(d: &std::path::Path, tag: &str, src: &str, env: &[(&str, &str)]) -> (i32, String, String) {
    let p = d.join(format!("{tag}.vish"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.env("PATH", path_voi_tc());
    c.arg("run").arg(&p);
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

// ---------- hermetic: assert tồn tại ----------

#[test]
fn chuong_trinh_ngoai_co_that() {
    co(TC);
}

// ---------- pipe → stdin ----------

#[test]
fn echo_qua_cat_nhan_stdin() {
    let d = tam("cat");
    let (c, o, e) = chay(&d, "cat", &format!("{} echo a | {} cat\n", TC, TC), &[]);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "[a]\n", "cat phải nhận stdin từ echo");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn echo_qua_grep_tim_thay() {
    let d = tam("grep");
    let (c, o, e) = chay(&d, "grep", &format!("{} echo a | {} grep a\n", TC, TC), &[]);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn grep_khong_khop_van_la_loi_khi_khong_complete() {
    let d = tam("grep0");
    let (c, _, e) = chay(&d, "grep0", &format!("{TC} echo x | {TC} grep z\n"), &[]);
    // **R9 (VD-178, C1)**: lỗi chạy có mã ⇒ exit **101** (trước: 1).
    assert_eq!(c, 101, "exit≠0 ⇒ VS0501 như VD-60");
    assert!(
        e.contains("VS0501") && e.contains("thoát mã 1"),
        "stderr: {e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- complete ----------

#[test]
fn complete_grep_khong_khop_het_la_loi() {
    let d = tam("cmp");
    let (c, o, e) = chay(
        &d,
        "cmp",
        &format!("{TC} echo x | {TC} grep z | complete\n"),
        &[],
    );
    assert_eq!(c, 0, "complete ⇒ **không** còn VS0501: {e}");
    // **SỬA KÈM LÝ DO (ST-41)**: `complete` trả **một bản ghi** ⇒ khối khoá–giá trị (Render, §9.6).
    assert_eq!(o, "stdout  []\nstderr  []\ncode    1\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn complete_grep_khop_code_0() {
    let d = tam("cmp2");
    let (c, o, _) = chay(
        &d,
        "cmp2",
        &format!("{TC} echo x | {TC} grep x | complete\n"),
        &[],
    );
    assert_eq!(c, 0);
    // **SỬA KÈM LÝ DO (ST-41)**: `complete` trả **một bản ghi** ⇒ khối khoá–giá trị (Render, §9.6).
    assert_eq!(o, "stdout  [x]\nstderr  []\ncode    0\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn complete_giu_stderr_va_code_khac_0() {
    let d = tam("cmp3");
    // `grep` với cờ lạ ⇒ exit 2 + stderr; `complete` giữ **cả hai**, không thành VS0501.
    let (c, o, e) = chay(
        &d,
        "cmp3",
        &format!("{TC} grep --khong-co | complete\n"),
        &[],
    );
    assert_eq!(c, 0, "stderr: {e}");
    // **SỬA KÈM LÝ DO (ST-41)**: `complete` trả **một bản ghi** ⇒ giá trị cuối pipeline in **khối
    // khoá–giá trị** (`Render`, §9.6): `stderr  […]` (hai dấu cách), không còn `stderr: […]`.
    //
    // **VD-132 (ST-134) — ca phụ thuộc nền**: thông điệp lỗi của `grep` in **tên chương trình** theo
    // chuẩn của nền: GNU grep (Linux) in `argv[0]` nguyên văn — ta gọi `{TC} grep` nên là
    // `{TC} grep:`; BSD grep (macOS) in tên chương trình `grep:`. Mỗi nhánh khẳng định **giá trị
    // cụ thể**; record vẫn phải mang đủ stderr + `code 2` (sản phẩm không đổi).
    let tien_to = "stderr  [test-child grep: unrecognized option '--khong-co'";
    assert!(
        o.contains(tien_to),
        "stderr phải có trong record với tiền tố {tien_to:?}: {o}"
    );
    assert!(o.contains("code    2"), "code 2: {o}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn complete_tren_stage_vis_la_vs0602() {
    let d = tam("vis");
    let (c, _, e) = chay(&d, "vis", "echo x | complete\n", &[]);
    assert_eq!(c, 2);
    assert!(e.contains("VS0602"), "stderr: {e}");
    assert!(
        e.contains("complete chỉ dùng sau chương trình ngoài"),
        "stderr: {e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn complete_co_doi_so_la_vs0602() {
    let d = tam("vis2");
    let (c, _, e) = chay(&d, "vis2", &format!("{} echo a | complete 1\n", TC), &[]);
    assert_eq!(c, 2, "stderr: {e}");
    assert!(e.contains("VS0602"), "stderr: {e}");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quy tắc nối stdin: str nguyên văn vs list[str] ----------

#[test]
fn printf_khong_newline_qua_cat_khong_them_dong_rong() {
    let d = tam("printf");
    let (c, o, e) = chay(
        &d,
        "printf",
        &format!("{TC} printf \"x\" | {TC} cat\n"),
        &[],
    );
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "[x]\n", "**một** dòng, không thừa dòng rỗng");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn printf_hai_dong_qua_cat_ra_hai_dong() {
    let d = tam("printf2");
    let (c, o, _) = chay(
        &d,
        "printf2",
        &format!("{TC} printf \"a\\nb\" | {TC} cat\n"),
        &[],
    );
    assert_eq!(c, 0);
    assert_eq!(o, "[a, b]\n", "hai dòng, không dòng rỗng");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn list_nhieu_dong_qua_grep_loc_dung_dong() {
    let d = tam("list");
    let (c, o, _) = chay(
        &d,
        "list",
        &format!("{TC} printf \"a\\nb\" | {TC} grep b\n"),
        &[],
    );
    assert_eq!(c, 0);
    assert_eq!(o, "[b]\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn raw_str_di_nguyen_van_khong_them_newline() {
    let d = tam("raw");
    // `--raw` ⇒ `str` nguyên văn ⇒ **không** nối thêm `\n`; `cat` in đúng một dòng.
    let (c, o, e) = chay(&d, "raw", &format!("{TC} printf x --raw | {TC} cat\n"), &[]);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "[x]\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- timeout nêu **stage** ----------

#[test]
fn timeout_neu_ro_stage() {
    let d = tam("to");
    let (c, _, e) = chay(
        &d,
        "to",
        &format!("{TC} sleep 5 | {TC} cat\n"),
        &[("VIS_CMD_TIMEOUT_MS", "50")],
    );
    // **R9 (VD-178, C1)**: lỗi chạy có mã ⇒ exit **101** (trước: 1).
    assert_eq!(c, 101, "stderr: {e}");
    assert!(e.contains("VS0501"), "stderr: {e}");
    assert!(
        e.contains("stage `") && e.contains(TC),
        "thông điệp timeout phải nêu stage: {e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- fixture `.vish` trong `test/cmd/pipe/**` ----------

#[test]
fn fixture_vish_trong_test_cmd_pipe() {
    let mut ca = 0;
    for (ten, muon) in [
        ("cat", "[a]\n"),
        ("grep", "[a]\n"),
        ("printf", "[x]\n"),
        // **SỬA KÈM LÝ DO (ST-41)**: `complete` trả **một bản ghi** ⇒ khối khoá–giá trị (Render, §9.6).
        ("complete", "stdout  []\nstderr  []\ncode    1\n"),
    ] {
        let p = goc().join("test/cmd/pipe").join(format!("{ten}.vish"));
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .env("PATH", path_voi_tc())
            .arg("run")
            .arg(&p)
            .output()
            .expect("chạy fixture");
        assert_eq!(
            o.status.code().unwrap_or(-1),
            0,
            "fixture {ten}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&o.stdout), muon, "fixture {ten}");
        ca += 1;
    }
    assert_eq!(ca, 4, "phải chạy đủ 4 fixture `.vish`");
}

// ---------- VM: **ST-89** có host Ngoai ⇒ chạy như interp ----------

/// **SỬA KÈM LÝ DO (ST-89, VD-110)**: trước đây test khoá “VM không host ⇒ `VS0604`”; nay VM
/// **đã có host `Ngoai`** (cùng trait với interp) nên lệnh ngoài phải chạy **cùng byte** — test
/// đổi thành khẳng định **hai đường cùng stdout/exit**, không còn kỳ vọng `VS0604`.
#[test]
fn vm_co_host_chay_cung_interp() {
    let d = tam("vm");
    let p = d.join("vm.vish");
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{TC} echo a | {TC} cat\n"),
    )
    .expect("ghi");
    let interp = Command::new(env!("CARGO_BIN_EXE_vis"))
        .env("PATH", path_voi_tc())
        .arg("run")
        .arg(&p)
        .output()
        .expect("chạy interp");
    let vm = Command::new(env!("CARGO_BIN_EXE_vis"))
        .env("PATH", path_voi_tc())
        .args(["run", "--vm", "--no-cache"])
        .arg(&p)
        .output()
        .expect("chạy --vm");
    assert_eq!(
        (interp.status.code(), &interp.stdout),
        (vm.status.code(), &vm.stdout),
        "hai đường phải cùng byte: interp {:?} / vm {:?}",
        String::from_utf8_lossy(&interp.stderr),
        String::from_utf8_lossy(&vm.stderr)
    );
    assert_eq!(interp.status.code().unwrap_or(-1), 0);
    assert_eq!(String::from_utf8_lossy(&interp.stdout), "[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}
