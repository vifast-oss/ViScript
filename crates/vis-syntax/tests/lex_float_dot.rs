// SPDX-License-Identifier: MIT OR Apache-2.0
//! R19c (VD-173#2, VD-178): `.` **chỉ** là dấu thập phân khi ký tự **ngay sau** nó là chữ số
//! ASCII `0-9`.
//!
//! - `5.bit_or(3)` ⇒ `INT(5)` `.` `IDENT(bit_or)` `(` `INT(3)` `)` — **không** `FLOAT`, không
//!   chẩn đoán (method `bit_or` thuộc R19a pha 2; ở đây chỉ kiểm tầng token).
//! - `5.0` vẫn là `FLOAT`.
//! - `1.` và `1.e3` vẫn bị cấm (`VS0104`) — không nới lỏng trường hợp sai.
//! - `5 . bit_or(3)` (có khoảng trắng) cho **cùng** chuỗi token như `5.bit_or(3)`.

use vis_syntax::{lex, Kind, Mode};

/// `(kind, text)` của mọi token trừ `EOF` — khuôn so sánh của test lexer.
fn toks(src: &str) -> Vec<(Kind, String)> {
    lex(src, Mode::Script)
        .tokens
        .into_iter()
        .filter(|t| t.kind != Kind::Eof)
        .map(|t| (t.kind, t.text))
        .collect()
}

fn codes(src: &str) -> Vec<&'static str> {
    lex(src, Mode::Script)
        .diagnostics
        .iter()
        .map(|d| d.code_str())
        .collect()
}

/// Chuỗi token kỳ vọng của `5.bit_or(3)` — dùng chung cho ca có/không khoảng trắng.
fn want_chuoi_method() -> Vec<(Kind, String)> {
    vec![
        (Kind::Int, "5".to_string()),
        (Kind::Punct, ".".to_string()),
        (Kind::Ident, "bit_or".to_string()),
        (Kind::Punct, "(".to_string()),
        (Kind::Int, "3".to_string()),
        (Kind::Punct, ")".to_string()),
    ]
}

#[test]
fn cham_sau_so_khong_phai_thap_phan_khi_theo_sau_la_chu_cai() {
    let out = lex("5.bit_or(3)", Mode::Script);
    assert!(
        out.diagnostics.is_empty(),
        "`5.bit_or(3)` không được có chẩn đoán, có {:?}",
        codes("5.bit_or(3)")
    );
    assert!(
        !out.tokens.iter().any(|t| t.kind == Kind::Float),
        "`5.bit_or(3)` không được sinh FLOAT, có {:?}",
        toks("5.bit_or(3)")
    );
    assert_eq!(toks("5.bit_or(3)"), want_chuoi_method());
}

#[test]
fn khoang_trang_truoc_cham_khong_doi_chuoi_token() {
    assert_eq!(toks("5 . bit_or(3)"), want_chuoi_method());
    assert_eq!(toks("5 .bit_or(3)"), want_chuoi_method());
}

#[test]
fn cham_mo_method_voi_dinh_danh_viet_va_sau_float() {
    // Định danh tiếng Việt cũng mở method — cùng `is_xid_start` với `scan_ident`.
    assert_eq!(
        toks("5.cộng(3)"),
        vec![
            (Kind::Int, "5".to_string()),
            (Kind::Punct, ".".to_string()),
            (Kind::Ident, "cộng".to_string()),
            (Kind::Punct, "(".to_string()),
            (Kind::Int, "3".to_string()),
            (Kind::Punct, ")".to_string()),
        ]
    );
    // Sau một FLOAT, `.` vốn đã là dấu câu — nhánh mới không được làm hỏng ca này.
    assert_eq!(
        toks("5.0.bit_or(3)"),
        vec![
            (Kind::Float, "5.0".to_string()),
            (Kind::Punct, ".".to_string()),
            (Kind::Ident, "bit_or".to_string()),
            (Kind::Punct, "(".to_string()),
            (Kind::Int, "3".to_string()),
            (Kind::Punct, ")".to_string()),
        ]
    );
}

#[test]
fn cham_theo_sau_chu_so_van_la_float() {
    assert_eq!(toks("5.0"), vec![(Kind::Float, "5.0".to_string())]);
    assert!(codes("5.0").is_empty());
    // `1.2..3.4` vẫn là FLOAT RANGE FLOAT (SPEC-lex §1).
    assert_eq!(
        toks("1.2..3.4"),
        vec![
            (Kind::Float, "1.2".to_string()),
            (Kind::Punct, "..".to_string()),
            (Kind::Float, "3.4".to_string()),
        ]
    );
}

#[test]
fn cham_khong_mo_method_van_la_vs0104() {
    assert_eq!(codes("1."), vec!["VS0104"], "`1.` phải vẫn bị cấm");
    assert_eq!(codes("1.e3"), vec!["VS0104"], "`1.e3` phải vẫn bị cấm");
    assert_eq!(codes("1.e+3"), vec!["VS0104"], "`1.e+3` phải vẫn bị cấm");
    // `.` không mở định danh (sau là khoảng trắng) ⇒ vẫn là literal số sai, không nới lỏng.
    assert_eq!(codes("1. "), vec!["VS0104"], "`1. ` phải vẫn bị cấm");
}
