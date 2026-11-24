// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-263 (mức thư viện) — interp đơn lẻ phải truyền `break`/`continue` từ biểu thức
//! `if`/`match` ra vòng lặp gần nhất**, không đổi chúng thành `()`.
//!
//! Đây là test hành vi trực tiếp của `vis_interp::run` (không qua CLI/VM), giữ lại làm
//! "mốc đỏ" cho hồi quy: trước bản sửa các ca `if`/`match` dưới đây in `3`/`5` thay vì
//! `1`/`4` vì `exec_body` nuốt `Ctrl`.
//!
//! Mọi ca phải **check sạch** (đúng luật VD-259: `break`/`continue` không phải câu cuối
//! của nhánh cần giá trị), chạy sạch, `steps > 0`, và stdout đúng.

use vis_core::RunOutput;
use vis_interp::run;
use vis_syntax::{lex, parse, Mode};
use vis_types::{check, lower};

fn chay(src: &str) -> RunOutput {
    let p = parse(src, Mode::Script);
    assert!(
        p.diagnostics.is_empty(),
        "ca phải parse sạch: {:?}\nnguồn: {src}",
        p.diagnostics
            .iter()
            .map(|d| d.code_str())
            .collect::<Vec<_>>()
    );
    let h = lower(&p.cst, &lex(src, Mode::Script));
    assert!(h.is_clean(), "ca phải hạ sạch: {:?}", h.codes());
    let t = check(&h.hir, &h.symbols, &h.types);
    assert!(
        t.is_clean(),
        "ca phải kiểm kiểu sạch: {:?}",
        t.diags
            .iter()
            .map(|d| format!("{} {}:{} {}", d.code.short, d.line, d.col, d.message))
            .collect::<Vec<_>>()
    );
    let out = run(src, &h.hir, &h.symbols, &h.types, &[]);
    assert!(
        out.is_clean(),
        "ca phải chạy sạch: {:?}",
        out.diagnostics
            .iter()
            .map(|d| format!("{} {}:{} {}", d.code.short, d.line, d.col, d.message))
            .collect::<Vec<_>>()
    );
    out
}

fn kiem(tag: &str, src: &str, muon: &str) {
    let out = chay(src);
    assert_eq!(out.stdout, muon, "{tag}: stdout sai");
    assert!(out.steps > 0, "{tag}: chưa chạy bước nào (xanh giả)");
    assert_eq!(out.exit, 0, "{tag}: exit");
}

/// Ca gốc: `let x = if true: break; 0 else: 0` phải thoát vòng ngay (in `1`, không phải `3`).
#[test]
fn if_break_trong_let() {
    kiem(
        "if-break",
        r#"fn main():
    var i = 0
    loop:
        i = i + 1
        if i == 1:
            let x = if true:
                break
                0
            else:
                0
        if i >= 3:
            break
    print(i)
"#,
        "1\n",
    );
}

/// `continue` trong nhánh `if` giá trị phải bỏ phần còn lại của vòng.
#[test]
fn if_continue_trong_let() {
    kiem(
        "if-continue",
        r#"fn main():
    var dem = 0
    var i = 0
    loop:
        i = i + 1
        let x = if i == 2:
            continue
            0
        else:
            0
        dem = dem + 1
        if i >= 5:
            break
    print(dem)
    print(i)
"#,
        "4\n5\n",
    );
}

/// `break` trong nhánh `match` giá trị.
#[test]
fn match_break_trong_let() {
    kiem(
        "match-break",
        r#"fn main():
    var i = 0
    loop:
        i = i + 1
        let x = match i:
            1:
                break
                0
            2:
                7
            _:
                0
        if i >= 3:
            break
    print(i)
"#,
        "1\n",
    );
}

/// `continue` trong nhánh `match` giá trị.
#[test]
fn match_continue_trong_let() {
    kiem(
        "match-continue",
        r#"fn main():
    var dem = 0
    var i = 0
    loop:
        i = i + 1
        let x = match i:
            2:
                continue
                0
            _:
                0
        dem = dem + 1
        if i >= 4:
            break
    print(dem)
    print(i)
"#,
        "3\n4\n",
    );
}

/// `return` trong biểu thức `if` vẫn thoát hàm (không bị đổi thành trap/`()`).
#[test]
fn return_trong_if_van_thoat_ham() {
    kiem(
        "ret-if",
        r#"fn f(n: int) -> int:
    var i = 0
    loop:
        i = i + 1
        let x = if i == n:
            return i
        else:
            0
        if i >= 10:
            break
    return 0

fn main():
    print(f(3))
"#,
        "3\n",
    );
}
