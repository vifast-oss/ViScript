// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — `vis check --stdin`: kiểm bộ đệm UTF-8 từ stdin như một tệp script tên `-`.
//!
//! Đỏ trước (HEAD 55756dd): `printf 'let x = 1\n' | vis check --stdin` bị coi là cờ lạ —
//! stderr `vis check: không hỗ trợ cờ \`--stdin\`` + usage, exit 2; chưa có đường stdin nên mọi
//! ca dưới đây đỏ. Sau sửa: cùng pipeline tệp (lex → parse → lower → tyck trừ `--syntax`), cùng
//! luật lọc/mức chẩn đoán; chỉ tên hiển thị đổi thành `-`.
//!
//! Hợp đồng:
//! - TAP `1..1` + `ok 1 - -` (sạch) hoặc `not ok 1 - -` + `# <mã> <dòng>:<cột> <thông điệp>`;
//!   exit 0 sạch / 1 có chẩn đoán / 2 dùng sai, lỗi đọc hoặc stdin không phải UTF-8.
//! - `--json`: một entry `{"file":"-", …}`, `rel`/`hien` đều `-`.
//! - `--stdin` không đi với tệp/đối số khác (exit 2 + usage); `--syntax`/`--no-extern` tôn trọng.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const DUNG: &str = "dùng: vis check [--syntax] [--no-extern] [--json] <tệp…> | vis check --stdin [--syntax] [--no-extern] [--json]\n  --stdin: đọc UTF-8 từ stdin, kiểm như một tệp script tên `-`; không nhận tệp; thoát 0 (sạch) / 1 (có chẩn đoán) / 2 (dùng sai, không đọc được hoặc không phải UTF-8)\n";

/// Chạy `vis <args…>` với stdin cho trước (byte thô — để thử cả UTF-8 không hợp lệ).
fn chay(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
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
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn chay_text(args: &[&str], stdin: &str) -> (i32, String, String) {
    chay(args, stdin.as_bytes())
}

/// Như [`chay`] nhưng đặt `current_dir` (để thử `use ./x` tương đối theo cwd).
fn chay_tai(dir: &Path, args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .current_dir(dir)
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
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Nguồn sạch ⇒ TAP đúng `1..1` + `ok 1 - -`, stderr rỗng, exit 0. Đầu vào **rỗng** là chương
/// trình rỗng (hợp lệ) — cùng hành vi tệp 0 byte của `vis check`.
#[test]
fn sach_in_tap_1_1_va_exit_0() {
    let (ma, out, err) = chay_text(&["check", "--stdin"], "let x = 1\n");
    assert_eq!(ma, 0, "sạch ⇒ 0; stderr: {err}");
    assert_eq!(out, "TAP version 13\n1..1\nok 1 - -\n");
    assert_eq!(err, "");

    let (ma, out, err) = chay_text(&["check", "--stdin"], "");
    assert_eq!(ma, 0, "rỗng ⇒ 0; stderr: {err}");
    assert_eq!(out, "TAP version 13\n1..1\nok 1 - -\n");
    assert_eq!(err, "");
}

/// Lỗi kiểu (`VS0412`) — exit 1, `not ok 1 - -`, **đúng một** dòng `# …` với vị trí 1-based.
#[test]
fn loi_kieu_not_ok_dung_mot_chan_doan_va_exit_1() {
    let (ma, out, err) = chay_text(&["check", "--stdin"], "let x: int = \"s\"\n");
    assert_eq!(ma, 1, "có chẩn đoán ⇒ 1; stderr: {err}");
    assert_eq!(
        out,
        "TAP version 13\n1..1\nnot ok 1 - -\n# VS0412 1:14 khai báo `let`: cần `int`, được `str`\n"
    );
    assert_eq!(err, "");
}

/// `--syntax` bỏ **tyck** (lỗi kiểu xanh) nhưng vẫn giữ lỗi cú pháp — cùng bộ lọc QD §7.4.
#[test]
fn syntax_bo_qua_tyck_nhung_giu_loi_cu_phap() {
    // Cùng nguồn lỗi kiểu ở trên: chỉ cú pháp/hạ ⇒ sạch.
    let (ma, out, err) = chay_text(&["check", "--syntax", "--stdin"], "let x: int = \"s\"\n");
    assert_eq!(ma, 0, "--syntax bỏ tyck; stderr: {err}");
    assert_eq!(out, "TAP version 13\n1..1\nok 1 - -\n");
    assert_eq!(err, "");

    // Lỗi cú pháp thật vẫn đỏ, vị trí giữ nguyên như đường tệp.
    let (ma, out, err) = chay_text(&["check", "--syntax", "--stdin"], "let x =\n");
    assert_eq!(ma, 1, "lỗi cú pháp ⇒ 1; stderr: {err}");
    assert_eq!(
        out,
        "TAP version 13\n1..1\nnot ok 1 - -\n# VS0201 1:8 mong biểu thức, thật `\\n` (NEWLINE)\n"
    );
    assert_eq!(err, "");
}

/// UTF-8 không hợp lệ: **không** in TAP, báo stderr, exit 2 (khác tệp non-UTF8 — đường tệp là
/// lỗi I/O `not ok` exit 1; stdin là lỗi đầu vào, như `vis fmt --stdin`).
#[test]
fn utf8_khong_hop_le_exit_2() {
    for co in [
        &["check", "--stdin"][..],
        &["check", "--json", "--stdin"][..],
    ] {
        let (ma, out, err) = chay(co, &[0xff, 0xfe, 0xfd]);
        assert_eq!(ma, 2, "stdin non-UTF8 ⇒ 2; args={co:?}");
        assert_eq!(out, "", "không in gì ra stdout; args={co:?}");
        assert_eq!(err, "vis check: stdin không phải UTF-8 hợp lệ\n");
    }
}

/// `--stdin` không đi với tệp/đối số; cờ lạ cũng bị từ chối — usage + exit 2, stdout rỗng.
#[test]
fn stdin_khong_di_voi_tep_hoac_co_la() {
    let (ma, out, err) = chay_text(&["check", "--stdin", "a.vis"], "let x = 1\n");
    assert_eq!(ma, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!("vis check: --stdin không nhận đối số `a.vis` (không nhận tệp)\n{DUNG}")
    );

    let (ma, out, err) = chay_text(&["check", "--stdin", "--bogus"], "let x = 1\n");
    assert_eq!(ma, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!("vis check: không hỗ trợ cờ `--bogus` khi dùng --stdin\n{DUNG}")
    );
}

/// `--json`: **một** entry `file` = `-`; sạch và lỗi kiểu đúng hình dạng đường tệp, exit y hệt.
#[test]
fn json_mot_tep_ten_gach_ngang() {
    let (ma, out, err) = chay_text(&["check", "--json", "--stdin"], "let x = 1\n");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        "{\"ok\":true,\"files\":[{\"file\":\"-\",\"diagnostics\":[]}]}\n"
    );
    assert_eq!(err, "");

    let (ma, out, err) = chay_text(&["check", "--json", "--stdin"], "let x: int = \"s\"\n");
    assert_eq!(ma, 1, "có chẩn đoán ⇒ 1; stderr: {err}");
    assert_eq!(
        out,
        "{\"ok\":false,\"files\":[{\"file\":\"-\",\"diagnostics\":[{\"code\":\"VS0412\",\"line\":1,\"col\":14,\"severity\":\"error\",\"message\":\"khai báo `let`: cần `int`, được `str`\"}]}]}\n"
    );
    assert_eq!(err, "");

    // `--json --syntax`: bỏ tyck như TAP, vẫn một entry `-`.
    let (ma, out, err) = chay_text(
        &["check", "--json", "--syntax", "--stdin"],
        "let x: int = \"s\"\n",
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        "{\"ok\":true,\"files\":[{\"file\":\"-\",\"diagnostics\":[]}]}\n"
    );
    assert_eq!(err, "");
}

/// Cùng pipeline module như tệp: `use ./b` từ stdin giải theo **cwd**; stdin thắng tệp tên `-`
/// nếu có trên đĩa (không đọc/ghi đĩa cho tệp chính).
#[test]
fn use_tuong_doi_theo_cwd_va_khong_doc_tep_gach_ngang() {
    let d = std::env::temp_dir().join(format!("vis-check-stdin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    std::fs::write(d.join("b.vis"), "pub fn f() -> int:\n    1\n").expect("ghi module");
    // Tệp tên `-` có nội dung hỏng: nếu bị đọc nhầm thì TAP sẽ `not ok`.
    std::fs::write(d.join("-"), "not a program\n").expect("ghi tệp `-`");

    let (ma, out, err) = chay_tai(&d, &["check", "--stdin"], b"use ./b\nlet x: int = f()\n");
    assert_eq!(ma, 0, "dùng module theo cwd; stderr: {err}");
    assert_eq!(out, "TAP version 13\n1..1\nok 1 - -\n");
    assert_eq!(err, "");

    let _ = std::fs::remove_dir_all(&d);
}
