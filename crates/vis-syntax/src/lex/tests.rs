// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — test đơn vị của lexer (`lex/mod.rs`), tách ra cùng thư mục.

use super::*;

fn kinds(src: &str) -> Vec<Kind> {
    lex(src, Mode::Script)
        .tokens
        .into_iter()
        .filter(|t| t.kind != Kind::Eof)
        .map(|t| t.kind)
        .collect()
}

fn codes(src: &str) -> Vec<&'static str> {
    lex(src, Mode::Script)
        .diagnostics
        .iter()
        .map(|d| d.code_str())
        .collect()
}

fn codes_cmd(src: &str) -> Vec<&'static str> {
    lex(src, Mode::Command)
        .diagnostics
        .iter()
        .map(|d| d.code_str())
        .collect()
}

#[test]
fn nfc_duoc_ap_truoc_khi_lex() {
    let nfd = "'o\u{302}\u{301}'";
    let out = lex(nfd, Mode::Script);
    let t = &out.tokens[0];
    assert_eq!(t.kind, Kind::Char);
    assert_eq!(t.text, "'ố'", "token phải là dạng NFC");
    assert!(out.diagnostics.is_empty());
}

#[test]
fn span_tro_dung_van_ban_da_nfc() {
    let out = lex("let số = 1_0", Mode::Script);
    for t in &out.tokens {
        assert_eq!(&out.text[t.span.start..t.span.end], t.text.as_str());
    }
}

#[test]
fn dedent_sinh_o_eof() {
    let k = kinds("if x:\n    y\n");
    assert_eq!(
        k,
        vec![
            Kind::Kw,
            Kind::Ident,
            Kind::Punct,
            Kind::Newline,
            Kind::Indent,
            Kind::Ident,
            Kind::Newline,
            Kind::Dedent
        ]
    );
}

#[test]
fn dong_trong_va_chu_thich_khong_sinh_token_ket_thuc_dong() {
    let k = kinds("a\n\n# c\nb\n");
    assert_eq!(
        k,
        vec![
            Kind::Ident,
            Kind::Newline,
            Kind::Comment,
            Kind::Ident,
            Kind::Newline
        ]
    );
}

#[test]
fn noi_dong_bang_pipe_arrow() {
    let k = kinds("x\n|> f\n");
    assert_eq!(
        k,
        vec![Kind::Ident, Kind::Punct, Kind::Ident, Kind::Newline]
    );
    assert!(lex("x\n|> f\n", Mode::Script).diagnostics.is_empty());
}

#[test]
fn ngoac_nuot_newline_va_indent() {
    let out = lex("(1 +\n   2)", Mode::Script);
    assert!(out.diagnostics.is_empty());
    assert_eq!(
        out.tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![
            Kind::Punct,
            Kind::Int,
            Kind::Punct,
            Kind::Int,
            Kind::Punct,
            Kind::Eof
        ]
    );
}

#[test]
fn tab_sinh_vs0107_va_van_lex_tiep() {
    let out = lex("\tif", Mode::Script);
    assert_eq!(codes("\tif"), vec!["VS0107"]);
    assert_eq!(
        out.tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![Kind::Indent, Kind::Kw, Kind::Dedent, Kind::Eof]
    );
    assert_eq!(out.tokens[0].col, 1);
}

#[test]
fn thut_le_sai_don_vi_la_vs0109() {
    assert_eq!(codes("if x:\n    y\n  z"), vec!["VS0109"]);
}

#[test]
fn che_do_lenh_tach_tu_tran() {
    let k: Vec<Kind> = lex("ls -la ~/x", Mode::Command)
        .tokens
        .iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(k, vec![Kind::Word, Kind::Word, Kind::Word, Kind::Eof]);
}

#[test]
fn the_khong_dau_stage_la_vs0110() {
    assert_eq!(codes_cmd("a^b"), vec!["VS0110"]);
    assert!(codes_cmd("^a").is_empty());
    assert!(codes_cmd("a | ^b").is_empty());
}

#[test]
fn duration_va_size_khong_nham_nhau() {
    let out = lex("5kb 5mib 2h30m 250ms 30m2h", Mode::Script);
    let got: Vec<(&str, String)> = out
        .tokens
        .iter()
        .filter(|t| t.kind != Kind::Eof)
        .map(|t| (t.kind.name(), t.text.clone()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("SIZE", "5kb".into()),
            ("SIZE", "5mib".into()),
            ("DURATION", "2h30m".into()),
            ("DURATION", "250ms".into()),
            ("DURATION", "30m".into()),
            ("DURATION", "2h".into()),
        ]
    );
}
#[test]
fn crlf_duoc_chuan_hoa_thanh_lf_truoc_khi_lex() {
    let src = "a\r\nb\r\n";
    let out = lex(src, Mode::Script);
    assert_eq!(out.text, "a\nb\n", "CRLF phải thành LF trong text");
    assert!(out.diagnostics.is_empty());
    let kinds: Vec<Kind> = out.tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            Kind::Ident,
            Kind::Newline,
            Kind::Ident,
            Kind::Newline,
            Kind::Eof
        ]
    );
    assert_eq!(out.tokens[1].text, "\n");
    // Toạ độ dòng 2 tính trên văn bản đã chuẩn hoá.
    assert_eq!((out.tokens[2].line, out.tokens[2].col), (2, 1));
}

#[test]
fn cr_don_doc_la_ky_tu_la() {
    let out = lex("a\rb", Mode::Script);
    assert_eq!(
        out.diagnostics
            .iter()
            .map(|d| d.code_str())
            .collect::<Vec<_>>(),
        vec!["VS0103"]
    );
}

#[test]
fn gach_duoi_trong_phan_le_cua_float_phai_giua_hai_chu_so() {
    assert_eq!(codes("1.2_"), vec!["VS0104"]);
    assert_eq!(codes("1.2__3"), vec!["VS0104"]);
    let ok = lex("1.2_3", Mode::Script);
    assert!(ok.diagnostics.is_empty());
    assert_eq!(ok.tokens[0].kind, Kind::Float);
    assert_eq!(ok.tokens[0].text, "1.2_3");
}

#[test]
fn bytes_chi_nhan_escape_xnn() {
    assert_eq!(codes("b\"a\\nb\""), vec!["VS0102"]);
    assert!(codes("b\"\\x41\"").is_empty());
}
#[test]
fn cr_dung_truoc_crlf_van_la_ky_tu_la() {
    // `replace("\r\n", "\n")` chạy một lượt: "\r\r\n" ⇒ "\r\n" (một `\r` lạ + LF).
    let out = lex("a\r\r\nb", Mode::Script);
    assert_eq!(out.text, "a\r\nb");
    assert_eq!(
        out.diagnostics
            .iter()
            .map(|d| d.code_str())
            .collect::<Vec<_>>(),
        vec!["VS0103"]
    );
    assert_eq!(
        out.tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![Kind::Ident, Kind::Newline, Kind::Ident, Kind::Eof]
    );
}
