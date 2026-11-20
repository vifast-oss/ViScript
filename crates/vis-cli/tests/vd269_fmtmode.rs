// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — `vis fmt --stdin --mode script|command`: chọn chế độ khi stdin không có đuôi tệp.
//!
//! Đỏ trước (HEAD `1d20f5b`): `printf … | vis fmt --stdin --mode command` bị coi là cờ lạ
//! (`vis fmt: không hỗ trợ cờ --mode khi dùng --stdin`) + usage, exit 2; chưa có cách nào định
//! dạng một **buffer `.vish`** qua stdin. Sau sửa: `--mode` (và `--mode=…`) chọn `Mode::Command`
//! cho đúng đường stdin; mặc định vẫn `Mode::Script` như VD-267.
//!
//! Hợp đồng:
//! - `vis fmt --stdin [--check] [--mode script|command]`; `--mode` thiếu/sai giá trị ⇒ 2 + usage.
//! - `--mode` **chỉ** hợp lệ với `--stdin`; đường tệp/thư mục có `--mode` ⇒ 2 + usage.
//! - `--check` giữ nguyên: stdout rỗng, 0 đã đúng / 1 cần sửa / 2 lỗi cú pháp.
//! - Chế độ lệnh giữ thụt lề gốc (formatter `.vish` hiện có), chế độ script chuẩn hoá 4 × độ sâu.

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

/// `cmd { … }` trong thân `fn`: formatter **script** dựng lại 4 × độ sâu (2 → 4 dấu cách);
/// formatter **command** (`.vish`) giữ nguyên thụt lề gốc — cùng một đầu vào, hai kết quả khác
/// nhau, chứng minh `--mode` thật sự đi vào `format_mode`.
const LENH: &str = "fn f():\n  let x = cmd { echo hi }\n  x\n";
const LENH_SCRIPT: &str = "fn f():\n    let x = cmd { echo hi }\n    x\n";

#[test]
fn command_mode_stdin_giu_thut_le_goc_khac_script() {
    // Mặc định = script (VD-267 giữ nguyên).
    let (ma, ra, err) = chay_text(&["fmt", "--stdin"], LENH);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, LENH_SCRIPT, "mặc định vẫn là chế độ script");

    // `--mode command` → đúng formatter chế độ lệnh: giữ 2 dấu cách.
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--mode", "command"], LENH);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, LENH, "chế độ lệnh giữ thụt lề gốc");

    // `--mode=command` (dạng dính) và thứ tự cờ khác đều nhận.
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--mode=command"], LENH);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, LENH);

    let (ma, ra, err) = chay_text(&["fmt", "--mode", "command", "--stdin"], LENH);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, LENH);

    // `--mode script` tường minh = mặc định.
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--mode=script"], LENH);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, LENH_SCRIPT);

    // Điểm bất động ở chế độ lệnh.
    let (ma2, ra2, err2) = chay_text(&["fmt", "--stdin", "--mode", "command"], &ra);
    assert_eq!(ma2, 0, "stderr: {err2}");
    assert_eq!(ra2, ra, "idempotent: fmt(fmt(x)) == fmt(x)");
}

/// Cú pháp **chỉ** đúng ở chế độ lệnh (`'…'` thô trong thân `cmd`) — đầu vào tương ứng một tệp
/// `.vish`; `--mode command` qua, mặc định script báo lỗi từ vựng và không in gì.
#[test]
fn command_mode_nhan_cu_phap_vish_con_script_thi_loi() {
    let vish = "cmd f():\n  echo 'a b'\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--mode", "command"], vish);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, vish, "giữ nguyên thụt lề 2 dấu cách");

    let (ma, ra, err) = chay_text(&["fmt", "--stdin"], vish);
    assert_eq!(ma, 2, "script không hiểu `'…'` thô; stderr: {err}");
    assert_eq!(ra, "", "lỗi ⇒ không in stdout");
    assert!(
        err.contains("# VS0105"),
        "script phải báo lỗi từ vựng: {err}"
    );
}

/// `--check` giữ đúng hợp đồng 0/1/2, không stdout — kể cả khi chế độ quyết định kết quả:
/// cùng đầu vào 2 dấu cách là **đã đúng** ở chế độ lệnh nhưng **cần sửa** ở chế độ script.
#[test]
fn check_stdin_theo_mode_0_1_khong_stdout() {
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--check", "--mode", "command"], LENH);
    assert_eq!(ma, 0, "chế độ lệnh đã đúng ⇒ 0; stderr: {err}");
    assert_eq!(ra, "", "--check không in stdout");

    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--check"], LENH);
    assert_eq!(ma, 1, "script cần sửa ⇒ 1; stderr: {err}");
    assert_eq!(ra, "", "--check không in stdout");

    // Chế độ lệnh vẫn sửa được khoảng trắng cuối dòng / CRLF ⇒ `--check` trả 1.
    let can_sua = "cmd f():\r\n  echo hi  \r\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--mode=command", "--check"], can_sua);
    assert_eq!(ma, 1, "cần sửa ⇒ 1; stderr: {err}");
    assert_eq!(ra, "", "--check không in stdout");

    // Lỗi cú pháp vẫn 2, cả hai chế độ.
    let loi = "let x =\n";
    for args in [
        &["fmt", "--stdin", "--mode", "command", "--check"][..],
        &["fmt", "--stdin", "--mode=script", "--check"][..],
    ] {
        let (ma, ra, err) = chay_text(args, loi);
        assert_eq!(ma, 2, "args {args:?}; stderr: {err}");
        assert_eq!(ra, "", "args {args:?}");
        assert!(err.contains("# VS0201"), "args {args:?}: {err}");
    }
}

/// Chế độ lệnh dùng cùng đường chuẩn hoá: CRLF → LF, NFC, đúng một newline cuối; thụt lề tab
/// thành 4 dấu cách.
#[test]
fn command_mode_crlf_nfc_va_tab() {
    let vao = "cmd f():\r\n\techo 'e\u{0301}'\r\n";
    let (ma, ra, err) = chay_text(&["fmt", "--stdin", "--mode", "command"], vao);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(ra, "cmd f():\n    echo 'é'\n");
    assert!(!ra.contains('\r'), "không còn CR: {ra:?}");
    assert!(!ra.contains('\t'), "không còn tab: {ra:?}");

    // Rỗng là điểm bất động ở mọi chế độ.
    for args in [
        &["fmt", "--stdin", "--mode", "script"][..],
        &["fmt", "--stdin", "--mode=command"][..],
    ] {
        let (ma, ra, err) = chay_text(args, "");
        assert_eq!(ma, 0, "args {args:?}; stderr: {err}");
        assert_eq!(ra, "", "args {args:?}");
        assert_eq!(err, "", "args {args:?}");
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

/// `--mode` thiếu giá trị / giá trị lạ / lặp lại ⇒ exit 2 + usage, không đọc stdin.
#[test]
fn mode_sai_gia_tri_hoac_lap_exit_2() {
    phai_tu_choi(&["fmt", "--stdin", "--mode"], "--mode");
    phai_tu_choi(&["fmt", "--stdin", "--mode", "mau"], "--mode");
    phai_tu_choi(&["fmt", "--stdin", "--mode=Command"], "--mode");
    phai_tu_choi(&["fmt", "--stdin", "--mode="], "--mode");
    phai_tu_choi(
        &["fmt", "--stdin", "--mode", "script", "--mode", "command"],
        "--mode",
    );
}

/// `--mode` chỉ có nghĩa với `--stdin`: đường tệp tự suy `.vish`/`.vis` ⇒ từ chối trước khi đọc
/// tệp, không đổi hành vi cũ nào.
#[test]
fn mode_khong_co_stdin_exit_2() {
    phai_tu_choi(&["fmt", "--mode", "script", "a.vis"], "--stdin");
    phai_tu_choi(&["fmt", "--mode=command", "a.vish"], "--stdin");
    phai_tu_choi(&["fmt", "--check", "--mode", "command", "a.vis"], "--stdin");
    phai_tu_choi(&["fmt", "--mode", "script"], "--stdin");
}
