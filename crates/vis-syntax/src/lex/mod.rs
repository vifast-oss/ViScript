// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lexer ViScript — `docs/SPEC-lex.md` (APPROVED 2026-11-12), ST-04.
//!
//! Trình tự một lượt lex ([`lex`]):
//! 1. **Chuẩn hoá NFC toàn tệp** trước khi lex (SPEC-lex §0.1, VD-11).
//! 2. Theo chế độ ([`Mode`]): script `.vis` hay lệnh `.vish`/REPL (SPEC-lex §6).
//! 3. Trả [`LexOutput`] gồm văn bản đã NFC (mốc của mọi offset), token và chẩn đoán.
//!
//! Lỗi từ vựng **không** dừng lex (trừ chuỗi chưa đóng): token vẫn ra, chẩn đoán đi kèm, để
//! `vis check` báo nhiều lỗi một lượt.

pub mod lit;
pub mod token;
pub mod verify;

mod cmd;
mod quoted;
mod scan;
mod script;

pub use token::{Kind, Span, Token};

/// Chế độ cú pháp — SPEC-lex §6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `.vis` — script: `#` luôn là chú thích, `'a'` là `char`, có thụt lề.
    Script,
    /// `.vish`/REPL — lệnh: `'…'` thô, `#` chỉ ở đầu từ, `$x`/`^`, từ trần `WORD`.
    Command,
}

/// Một chẩn đoán từ vựng.
///
/// Mã **lấy từ sổ `vis-diag`** (không tự đặt chuỗi mã): `VS0101..VS0110` của SPEC-lex §7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static vis_diag::Code,
    pub message: String,
    pub span: Span,
    pub line: u32,
    pub col: u32,
}

impl Diagnostic {
    /// Mã dạng `VS01xx` (tiện cho log/so khớp).
    pub fn code_str(&self) -> &'static str {
        self.code.short
    }
}

/// Kết quả một lượt lex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexOutput {
    /// Văn bản đã **chuẩn hoá CRLF→LF rồi NFC** — mọi `span` của token/chẩn đoán trỏ vào chuỗi này.
    pub text: String,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Chuẩn hoá NFC toàn văn bản — bước bắt buộc trước khi lex (SPEC-lex §0.1, VD-11).
pub fn normalize_nfc(src: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    src.nfc().collect()
}

/// Chuẩn hoá đầu vào của lexer: **CRLF → LF** (SPEC-lex §1, hàng `NEWLINE`) rồi **NFC** toàn tệp
/// (§0.1). Mọi offset của token/chẩn đoán trỏ vào kết quả của hàm này ([`LexOutput::text`]).
///
/// Hệ quả: một `\r` **đứng một mình** (không có `\n` ngay sau) không phải newline — nó là ký tự
/// lạ ⇒ `VS0103`. Phép thay chạy **một lượt** như mọi chuẩn hoá CRLF: `"\r\r\n"` ⇒ `"\r\n"`
/// (một `\r` lạ + một NEWLINE), không nuốt thêm.
///
/// Ngoài ra `LexOutput::text` là mốc của **mọi** `span`, nên vị trí byte/dòng/cột đều tính trên
/// văn bản đã chuẩn hoá — không phải trên byte gốc của tệp.
pub fn normalize_source(src: &str) -> String {
    if src.contains("\r\n") {
        normalize_nfc(&src.replace("\r\n", "\n"))
    } else {
        normalize_nfc(src)
    }
}

/// Lex một văn bản nguồn theo chế độ (hàm thuần, SPEC-lex §0.2).
pub fn lex(src: &str, mode: Mode) -> LexOutput {
    lex_file(src, mode, 0)
}

/// **ST-114a**: lex cho tệp `file` — token/chẩn đoán mang `Span.file = file`; `lex` cũ giữ file 0.
pub fn lex_file(src: &str, mode: Mode, file: u16) -> LexOutput {
    let text = normalize_source(src);
    let (mut tokens, mut diagnostics) = {
        let mut lx = Lexer::new(&text, mode);
        lx.run();
        (lx.tokens, lx.diags)
    };
    for t in &mut tokens {
        t.span.file = file;
    }
    for d in &mut diagnostics {
        d.span.file = file;
    }
    LexOutput {
        text,
        tokens,
        diagnostics,
    }
}
/// Trạng thái chạy của lexer.
struct Lexer<'a> {
    src: &'a str,
    /// Offset byte hiện tại trong `src` (đã NFC).
    i: usize,
    line: u32,
    col: u32,
    mode: Mode,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
    /// Ngăn xếp độ sâu thụt lề (theo **cột**, tab = 4) — luôn có phần tử `0`.
    stack: Vec<usize>,
    /// Đơn vị thụt lề = mức thụt đầu tiên > 0 gặp trong tệp (SPEC-lex §4.1).
    unit: Option<usize>,
    /// Độ sâu ngoặc `() [] {}` — trong ngoặc không sinh NEWLINE/INDENT/DEDENT (§4.3).
    depth: usize,
    /// Dòng hiện tại đã có token nội dung chưa (dòng trống/chú thích thì không).
    line_has_code: bool,
    at_line_start: bool,
    /// Dòng này được nối với dòng trước bằng `|>` (§4.5): không tính thụt lề.
    joined: bool,
    /// Chế độ lệnh: đang ở đầu stage (`^` hợp lệ ở đây).
    stage_start: bool,
}

#[cfg(test)]
mod tests;
