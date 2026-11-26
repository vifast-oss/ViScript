// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — API vào của parser: [`parse`], [`parse_file`], [`parse_voi`],
//! [`parse_voi_file`] và [`dat_file_cst`].
//!
//! Tách **nguyên văn** khỏi `parse/mod.rs` để giữ mỗi tệp ≤ 1.150 dòng; đường dẫn công khai
//! `vis_syntax::parse::*` không đổi (re-export ở `parse/mod.rs`).

use super::{ParseOutput, Parser};
use crate::cst::{Cst, Elem, Node};
use crate::lex::token::Token;
use crate::lex::{self, LexOutput, Mode};

/// **ST-48**: parse với **token đã lex sẵn** (span tuyệt đối) — dùng cho thân `cmd` lệnh: lex
/// phần thân đã bỏ thụt rồi **dịch span** về vị trí thật trong tệp, giữ `text` = văn bản tệp.
pub fn parse_voi(text: &str, tokens: Vec<Token>, mode: Mode) -> ParseOutput {
    let mut diags = Vec::new();
    let mut cmd_sexp = None;
    let root = if mode == Mode::Command {
        let mut p = Parser::new(&tokens, text);
        match p.cmd_file() {
            Ok((n, s)) => {
                cmd_sexp = Some(s);
                n
            }
            Err(d) => {
                diags.push(d.0);
                p.raw_program_from_current()
            }
        }
    } else {
        let mut p = Parser::new(&tokens, text);
        match p.file() {
            Ok(n) => n,
            Err(d) => {
                diags.push(d.0);
                p.raw_program_from_current()
            }
        }
    };
    let mut cst = Cst::new(root, tokens);
    if let Some(s) = cmd_sexp {
        cst.set_cmd_sexp(s);
    }
    cst.attach_comments();
    ParseOutput {
        text: text.to_string(),
        cst,
        diagnostics: diags,
    }
}

/// **ST-114a**: gắn `file` vào **mọi** span của cây CST (node + token) — đệ quy.
pub fn dat_file_cst(cst: &mut Cst, file: u16) {
    fn node(n: &mut Node, file: u16) {
        n.span.file = file;
        for e in &mut n.elems {
            if let Elem::Node(c) = e {
                node(c, file);
            }
        }
    }
    node(&mut cst.root, file);
    for t in &mut cst.tokens {
        t.span.file = file;
    }
}

/// **ST-114a**: như [`parse_voi`] nhưng mọi span (CST + chẩn đoán) mang `file`.
pub fn parse_voi_file(text: &str, tokens: Vec<Token>, mode: Mode, file: u16) -> ParseOutput {
    let mut out = parse_voi(text, tokens, mode);
    dat_file_cst(&mut out.cst, file);
    for d in &mut out.diagnostics {
        d.span.file = file;
    }
    out
}

/// **ST-114a**: parse tệp `file` (lex bằng [`lex::lex_file`], giữ nguyên hành vi `parse` cũ với file 0).
pub fn parse_file(src: &str, mode: Mode, file: u16) -> ParseOutput {
    let LexOutput {
        text,
        tokens,
        diagnostics,
    } = lex::lex_file(src, mode, file);
    let mut out = parse_voi_file(&text, tokens, mode, file);
    let mut ds = diagnostics;
    ds.extend(out.diagnostics);
    out.diagnostics = ds;
    out
}

/// Parse một văn bản nguồn theo chế độ.
pub fn parse(src: &str, mode: Mode) -> ParseOutput {
    let LexOutput {
        text,
        tokens,
        diagnostics,
    } = lex::lex(src, mode);
    let mut diags = diagnostics;
    let mut cmd_sexp = None;
    let root = if mode == Mode::Command {
        let mut p = Parser::new(&tokens, &text);
        match p.cmd_file() {
            Ok((n, s)) => {
                cmd_sexp = Some(s);
                n
            }
            Err(d) => {
                // Lỗi **tầng**: đã có chẩn đoán từ vựng thì không thêm lỗi cú pháp (mỗi tệp đúng
                // **1** lỗi đầu, VD-35) — chế độ lệnh giữ quy tắc này cho golden ST-06.
                if diags.is_empty() {
                    diags.push(d.0);
                }
                p.raw_program_from_current()
            }
        }
    } else {
        let mut p = Parser::new(&tokens, &text);
        match p.file() {
            Ok(n) => n,
            Err(d) => {
                diags.push(d.0);
                p.raw_program_from_current()
            }
        }
    };
    let mut cst = Cst::new(root, tokens);
    if let Some(s) = cmd_sexp {
        cst.set_cmd_sexp(s);
    }
    cst.attach_comments();
    ParseOutput {
        text,
        cst,
        diagnostics: diags,
    }
}
