// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267** — `vis fmt --stdin [--check]`: định dạng luồng, không đụng đĩa.
//!
//! Đỏ trước (HEAD 5405489): `printf … | vis fmt --stdin` bị coi là cờ lạ
//! (`vis fmt: không hỗ trợ cờ --stdin`) + usage, exit 2; chưa có đường stdin nên mọi ca dưới
//! đây đỏ. Sau sửa: đọc UTF-8, NFC/CRLF→LF như đường tệp, in stdout; `--check` không in, mã
//! thoát 0/1/2; tổ hợp cờ sai ⇒ usage + exit 2.
//!
//! Hợp đồng:
//! - `vis fmt --stdin`: stdout = bản đã định dạng, exit 0; đầu vào rỗng ⇒ stdout rỗng, exit 0.
//! - `vis fmt --stdin --check`: stdout rỗng; 0 đã đúng, 1 cần sửa, 2 lỗi cú pháp.
//! - tổ hợp cấm: `--stdin --write`, `--stdin --indent-report`, `--stdin <tệp>` ⇒ 2.

use std::io::Write;
use std::process::{Command, Stdio};

fn chay(args: &[&str], stdin: &[u8]) -> (i32, Vec<u8>, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("phải chạy được binary `vis`");
    con.stdin
        .take()
        .expect("stdin đã pipe")
        .write_all(stdin)
        .expect("ghi stdin");
    let o = con.wait_with_output().expect("chờ `vis`");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn chay_text(args: &[&str], stdin: &str) -> (i32, String, String) {
    let (ma, out, err) = chay(args, stdin.as_bytes());
    (
        ma,
        String::from_utf8(out).expect("stdout phải là UTF-8"),
        err,
    )
}

/// `vis fmt --stdin` chạy đúng formatter hiện có: sửa thụt lề, in stdout, không ghi đĩa; cho
/// đầu ra quay lại làm đầu vào phải là điểm bất động (c).
#[test]
fn stdin_in_ban_dinh_dang_va_idempotent() {
    let vao = "fn f():\n  let x = 1\n  x\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin"], vao);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(err, "", "không được báo lỗi");
    assert_eq!(ra, "fn f():\n    let x = 1\n    x\n");

    let (ma2, ra2, err2) = chay_text(&["fmt", "--stdin"], &ra);
    assert_eq!(ma2, 0, "stderr: {err2}");
    assert_eq!(ra2, ra, "idempotent: fmt(fmt(x)) == fmt(x)");
}

/// Cùng đường chuẩn hoá như tệp: tab → 4 dấu cách, CRLF → LF, bỏ khoảng trắng cuối dòng,
/// NFC cho chuỗi; độ sâu khối lồng nhau vẫn đúng bội số 4.
#[test]
fn stdin_chuan_hoa_tab_crlf_va_nfc() {
    let vao = "fn f():\r\n\tlet x = 1  \r\n\tif x > 0:\r\n\t\tx\r\n\telse:\r\n\t\t0 - x\r\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin"], vao);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        ra,
        "fn f():\n    let x = 1\n    if x > 0:\n        x\n    else:\n        0 - x\n"
    );
    assert!(!ra.contains('\t'), "không còn tab: {ra:?}");
    assert!(!ra.contains('\r'), "không còn CR: {ra:?}");

    // `e` + U+0301 (dấu sắc tổ hợp) ⇒ NFC `é` (cùng hành vi đường tệp).
    let (ma, ra, err) = chay_text(&["fmt", "--stdin"], "let s = \"e\u{0301}\"\n");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, "let s = \"é\"\n");
}

/// `--check`: không in stdout; 0 khi đầu vào **đã đúng**, 1 khi fmt sẽ đổi. Nhận cả hai thứ tự
/// cờ (`--stdin --check` và `--check --stdin`).
#[test]
fn check_stdin_0_1_khong_stdout() {
    let dung = "fn f():\n    1\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--check"], dung);
    assert_eq!(ma, 0, "đã đúng ⇒ 0; stderr: {err}");
    assert_eq!(ra, "", "--check không in stdout");

    let can_sua = "fn f():\n  1\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--check"], can_sua);
    assert_eq!(ma, 1, "cần sửa ⇒ 1; stderr: {err}");
    assert_eq!(ra, "", "--check không in stdout");

    let (ma, ra, err) = chay_text(&["fmt", "--check", "--stdin"], can_sua);
    assert_eq!(ma, 1, "thứ tự cờ ngược vẫn nhận; stderr: {err}");
    assert_eq!(ra, "", "--check không in stdout");
}

/// Lỗi cú pháp: stderr nêu mã, stdout rỗng, exit 2 — cả chế độ in lẫn `--check`.
#[test]
fn loi_cu_phap_stdin_exit_2() {
    let loi = "let x =\n";
    let thuong = ["fmt", "--stdin"];
    let check = ["fmt", "--stdin", "--check"];
    for args in [&thuong[..], &check[..]] {
        let (ma, ra, err) = chay_text(args, loi);
        assert_eq!(ma, 2, "args {args:?}; stderr: {err}");
        assert_eq!(ra, "", "không in gì khi lỗi (args {args:?})");
        assert!(
            err.contains("# VS0201"),
            "stderr phải nêu lỗi đầu (args {args:?}): {err}"
        );
    }
}

/// Đầu vào rỗng ⇒ đầu ra rỗng, exit 0 (cả `--check`, vì rỗng là điểm bất động).
#[test]
fn stdin_rong_stdout_rong_exit_0() {
    let thuong = ["fmt", "--stdin"];
    let check = ["fmt", "--stdin", "--check"];
    for args in [&thuong[..], &check[..]] {
        let (ma, ra, err) = chay_text(args, "");
        assert_eq!(ma, 0, "args {args:?}; stderr: {err}");
        assert_eq!(ra, "", "args {args:?}");
        assert_eq!(err, "", "args {args:?}");
    }
}

/// Stdin không phải UTF-8 hợp lệ ⇒ báo lỗi rõ, exit 2, không in stdout (cả `--check`).
#[test]
fn stdin_khong_utf8_exit_2() {
    let thuong = ["fmt", "--stdin"];
    let check = ["fmt", "--stdin", "--check"];
    for args in [&thuong[..], &check[..]] {
        let (ma, ra, err) = chay(args, &[0xff, 0xfe]);
        assert_eq!(ma, 2, "args {args:?}; stderr: {err}");
        assert_eq!(ra, b"", "không in gì khi stdin không UTF-8 (args {args:?})");
        assert!(
            err.contains("UTF-8"),
            "stderr phải nói rõ cần UTF-8 (args {args:?}): {err}"
        );
    }
}

fn phai_tu_choi(args: &[&str], manh: &str) {
    let (ma, ra, err) = chay_text(args, "");
    assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
    assert_eq!(ra, "", "usage không in stdout (args {args:?})");
    assert!(
        err.contains("dùng: vis fmt"),
        "phải in usage (args {args:?}): {err}"
    );
    assert!(
        err.contains(manh),
        "stderr phải nêu lý do chứa `{manh}` (args {args:?}): {err}"
    );
}

/// Tổ hợp vô nghĩa với `--stdin` bị từ chối **trước khi** đọc stdin, exit 2 kèm usage.
#[test]
fn tu_choi_to_hop_co_stdin_khong_hop_le() {
    phai_tu_choi(&["fmt", "--stdin", "--write"], "--write");
    phai_tu_choi(&["fmt", "--write", "--stdin"], "--write");
    phai_tu_choi(&["fmt", "--stdin", "--indent-report"], "--indent-report");
    phai_tu_choi(&["fmt", "--stdin", "a.vis"], "không nhận tệp");
    phai_tu_choi(&["fmt", "--stdin", "--json"], "không hỗ trợ cờ");
}
