// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — tô màu buffer theo token của **CST `vis-syntax`** (không regex trên text).

use vis_syntax::{Kind, Mode};

pub const RESET: &str = "\x1b[0m";
pub const BLUE: &str = "\x1b[34m";
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const MAGENTA: &str = "\x1b[35m";
pub const DIM: &str = "\x1b[2m";

fn mau(k: Kind, text: &str) -> Option<&'static str> {
    match k {
        Kind::Kw => Some(BLUE),
        Kind::Str | Kind::RawStr | Kind::MultiStr | Kind::RawMulti | Kind::Char => Some(YELLOW),
        Kind::Int | Kind::Float | Kind::Size | Kind::Duration | Kind::Percent | Kind::Bytes => {
            Some(MAGENTA)
        }
        Kind::Comment | Kind::DocComment => Some(GREEN),
        Kind::Ident if text.chars().next().is_some_and(|c| c.is_uppercase()) => Some(CYAN),
        _ => None,
    }
}

/// Render buffer với ANSI màu; vẫn giữ nguyên byte văn bản (chỉ chèn escape).
pub fn render_cst(text: &str) -> String {
    let parsed = vis_syntax::parse(text, Mode::Script);
    let src = &parsed.text;
    let mut out = String::with_capacity(src.len() + 64);
    let mut last = 0usize;
    for t in &parsed.cst.tokens {
        if t.kind == Kind::Eof || t.span.start < last || t.span.end > src.len() {
            continue;
        }
        if t.span.start > last {
            out.push_str(&src[last..t.span.start]);
        }
        let token = &src[t.span.start..t.span.end];
        if let Some(c) = mau(t.kind, token) {
            out.push_str(c);
            out.push_str(token);
            out.push_str(RESET);
        } else {
            out.push_str(token);
        }
        last = t.span.end;
    }
    if last < src.len() {
        out.push_str(&src[last..]);
    }
    out
}
