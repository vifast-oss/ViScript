// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304/ST-310b** — tô màu buffer theo token của **CST `vis-syntax`** (không regex trên text).
//!
//! **ST-310b**: mã SGR không còn viết tay ở đây; mọi màu đi qua [`vish_term::AnsiPosix`] +
//! [`vish_term::Style`] — `vish-term` là tầng Terminal/Render duy nhất.

use vis_syntax::{Kind, Mode};
use vish_term::{AnsiPosix, Color, ColorMode, Style, Terminal};

/// Trả `Some(style)` cho token cần tô; `None` ⇒ ghi nguyên văn.
fn kieu(k: Kind, text: &str) -> Option<Style> {
    let mau = match k {
        Kind::Kw => Some(Color::Blue),
        Kind::Str | Kind::RawStr | Kind::MultiStr | Kind::RawMulti | Kind::Char => {
            Some(Color::Yellow)
        }
        Kind::Int | Kind::Float | Kind::Size | Kind::Duration | Kind::Percent | Kind::Bytes => {
            Some(Color::Magenta)
        }
        Kind::Comment | Kind::DocComment => Some(Color::Green),
        Kind::Ident if text.chars().next().is_some_and(|c| c.is_uppercase()) => Some(Color::Cyan),
        _ => None,
    };
    mau.map(|fg| Style::new().fg(fg))
}

/// Terminal ANSI16 trên bộ đệm bộ nhớ — dùng cho `render_cst`/`dim` và test.
fn ansi() -> AnsiPosix<Vec<u8>> {
    AnsiPosix::with_writer(Vec::new()).with_color(ColorMode::Ansi16)
}

fn ra_string(term: AnsiPosix<Vec<u8>>) -> String {
    String::from_utf8(term.into_inner()).expect("render ANSI là UTF-8")
}

/// Render buffer với ANSI màu; vẫn giữ nguyên byte văn bản (chỉ chèn escape).
pub fn render_cst(text: &str) -> String {
    let parsed = vis_syntax::parse(text, Mode::Script);
    let src = &parsed.text;
    let mut term = ansi();
    let mut last = 0usize;
    for t in &parsed.cst.tokens {
        if t.kind == Kind::Eof || t.span.start < last || t.span.end > src.len() {
            continue;
        }
        if t.span.start > last {
            let _ = term.write(&src[last..t.span.start]);
        }
        let token = &src[t.span.start..t.span.end];
        match kieu(t.kind, token) {
            Some(style) => {
                let _ = term.write_styled(style, token);
            }
            None => {
                let _ = term.write(token);
            }
        }
        last = t.span.end;
    }
    if last < src.len() {
        let _ = term.write(&src[last..]);
    }
    let _ = term.flush();
    ra_string(term)
}

/// Chuỗi mờ (dùng cho chỉ báo tìm `Ctrl-R`) — cùng đường SGR của `vish-term`.
pub fn dim(text: &str) -> String {
    let mut term = ansi();
    let _ = term.write_styled(Style::DIM, text);
    let _ = term.flush();
    ra_string(term)
}
