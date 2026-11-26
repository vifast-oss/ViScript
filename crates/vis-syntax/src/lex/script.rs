// SPDX-License-Identifier: MIT OR Apache-2.0
//! Quét token chế độ **script**: định danh, chú thích, ký tự, dấu câu, số + hậu tố.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `lex/mod.rs` (R1 mềm): chỉ đổi vị trí,
//! token/span/chẩn đoán/hành vi không đổi.

use super::token::{is_keyword, longest_punct};
use super::*;

impl<'a> Lexer<'a> {
    pub(super) fn token_script(&mut self) {
        let c = match self.byte() {
            Some(c) => c,
            None => return,
        };
        match c {
            b'0'..=b'9' => self.scan_number(),
            b'"' => {
                if self.starts_with("\"\"\"") {
                    self.scan_quoted(Kind::MultiStr);
                } else {
                    self.scan_quoted(Kind::Str);
                }
            }
            b'\'' => self.scan_char(),
            b'#' => self.scan_comment(),
            b'r' if self.starts_with("r\"\"\"") => self.scan_quoted(Kind::RawMulti),
            b'r' if self.starts_with("r\"") => self.scan_quoted(Kind::RawStr),
            b'b' if self.starts_with("b\"") => self.scan_quoted(Kind::Bytes),
            b'p' if self.starts_with("p\"") => self.scan_quoted(Kind::Path),
            _ if self.is_ident_start() => self.scan_ident(),
            _ => self.scan_punct(),
        }
    }

    pub(super) fn is_ident_start(&self) -> bool {
        match self.cur_char() {
            Some(c) => c == '_' || unicode_ident::is_xid_start(c),
            None => false,
        }
    }

    pub(super) fn scan_ident(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        self.bump_char();
        while let Some(c) = self.cur_char() {
            if c == '_' || unicode_ident::is_xid_continue(c) {
                self.bump_char();
            } else {
                break;
            }
        }
        let text = &self.src[start..self.i];
        // `set{` dính liền là set literal (§5); có khoảng trắng thì `set` là định danh.
        if text == "set" && self.byte() == Some(b'{') {
            self.bump_ascii();
            self.depth += 1;
            self.push_at(Kind::SetLbrace, start, line, col);
            return;
        }
        let kind = if is_keyword(text) {
            Kind::Kw
        } else {
            Kind::Ident
        };
        self.push_at(kind, start, line, col);
    }

    pub(super) fn scan_comment(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        // `##` chỉ là tài liệu khi đứng đầu dòng (sau thụt lề) — tài liệu thiết kế §2.3.
        let doc = !self.line_has_code && self.starts_with("##");
        while let Some(c) = self.cur_char() {
            if c == '\n' || c == '\r' {
                break;
            }
            self.bump_char();
        }
        self.push_at(
            if doc { Kind::DocComment } else { Kind::Comment },
            start,
            line,
            col,
        );
    }

    pub(super) fn scan_char(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        self.bump_ascii(); // '
        let mut count = 0usize;
        while let Some(c) = self.cur_char() {
            if c == '\'' || c == '\n' || c == '\r' {
                break;
            }
            count += 1;
            self.bump_char();
        }
        let closed = self.byte() == Some(b'\'');
        if closed {
            self.bump_ascii();
        }
        if count != 1 || !closed {
            // §1: `char` là **đúng một** scalar; nhiều/ít hơn là VS0105.
            self.error(105, Span::new(start, self.i), line, col);
            return;
        }
        self.push_at(Kind::Char, start, line, col);
    }

    pub(super) fn scan_punct(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);
        if let Some(p) = longest_punct(self.rest()) {
            for _ in 0..p.len() {
                self.bump_ascii();
            }
            match p {
                "(" | "[" | "{" => self.depth += 1,
                ")" | "]" | "}" => self.depth = self.depth.saturating_sub(1),
                _ => {}
            }
            self.push_at(Kind::Punct, start, line, col);
            return;
        }
        self.bump_char();
        self.error(103, Span::new(start, self.i), line, col);
    }

    // ---------------------------------------------------------------- số

    /// `true` nếu `.` tại `self.i` là **truy cập method** chứ không phải dấu thập phân: ngay sau
    /// nó là ký tự mở đầu định danh, **trừ** dạng số mũ `[eE][+-]?[0-9]+` — `1.e3` vẫn là literal
    /// số sai (`VS0104`), không phải method `.e3` (R19c, VD-173#2/VD-178).
    pub(super) fn cham_mo_method(&self) -> bool {
        // `.` là ASCII 1 byte ⇒ `self.i + 1` luôn là biên ký tự.
        let Some(c) = self.src[self.i + 1..].chars().next() else {
            return false;
        };
        if !(c == '_' || unicode_ident::is_xid_start(c)) {
            return false;
        }
        let mut j = 1usize; // bỏ dấu `.`
        if matches!(self.peek(j), Some(b'e' | b'E')) {
            j += 1;
            if matches!(self.peek(j), Some(b'+' | b'-')) {
                j += 1;
            }
            if self.peek(j).is_some_and(|b| b.is_ascii_digit()) {
                return false; // `1.e3` / `1.e+3` — float viết sai, không phải method
            }
        }
        true
    }

    pub(super) fn scan_number(&mut self) {
        let start = self.i;
        let (line, col) = (self.line, self.col);

        // Cơ số 16/2 **chỉ khi** theo sau là chữ số hợp lệ (§1: `0b` đơn lẻ là SIZE(0,b)).
        let mut radix: u64 = 10;
        let mut prefix = 0usize;
        if self.starts_with("0x") && self.peek(2).is_some_and(|b| b.is_ascii_hexdigit()) {
            radix = 16;
            prefix = 2;
        } else if self.starts_with("0b") && matches!(self.peek(2), Some(b'0' | b'1')) {
            radix = 2;
            prefix = 2;
        }
        for _ in 0..prefix {
            self.bump_ascii();
        }
        while let Some(b) = self.byte() {
            let ok = b == b'_' || (b as char).is_digit(radix as u32);
            if !ok {
                break;
            }
            self.bump_ascii();
        }

        // Phần thập phân / số mũ → FLOAT.
        let mut is_float = false;
        if radix == 10 {
            if self.byte() == Some(b'.') {
                match self.peek(1) {
                    Some(b'.') => {} // `1..2` là INT RANGE INT (§2)
                    Some(d) if d.is_ascii_digit() => {
                        self.bump_ascii(); // '.'
                                           // Phần lẻ là `[0-9]+` (§1); `_` chỉ được **giữa hai chữ số** (§2), nên
                                           // `1.2_` / `1.2__3` là literal sai (VS0104) — không nuốt `_` vào token.
                        loop {
                            match self.byte() {
                                Some(b) if b.is_ascii_digit() => self.bump_ascii(),
                                Some(b'_') if self.peek(1).is_some_and(|n| n.is_ascii_digit()) => {
                                    self.bump_ascii();
                                    self.bump_ascii();
                                }
                                Some(b'_') => {
                                    let off = self.i;
                                    self.error_at(104, off);
                                    break;
                                }
                                _ => break,
                            }
                        }
                        is_float = true;
                    }
                    // R19c (VD-173#2, VD-178): `.` **chỉ** là dấu thập phân khi ngay sau là chữ
                    // số. `.` mở định danh là truy cập method ⇒ **không** ăn `.`, để token sau
                    // xử lý: `5.bit_or(3)` = `INT(5)` `.` `IDENT(bit_or)` `(` `INT(3)` `)`.
                    _ if self.cham_mo_method() => {}
                    _ => {
                        // §1: `1.` / `1.e3` sai ⇒ VS0104 (dấu `.` để token sau xử lý).
                        let off = self.i;
                        self.error_at(104, off);
                    }
                }
            }
            if !is_float && self.scan_exponent() {
                is_float = true;
            } else if is_float {
                self.scan_exponent();
            }
        }

        let text = &self.src[start..self.i];
        // `pct` áp cho cả `INT` và `FLOAT` (§1: `(INT | FLOAT)` + `pct`).
        if self.starts_with("pct") {
            for _ in 0..3 {
                self.bump_ascii();
            }
            self.push_at(Kind::Percent, start, line, col);
            return;
        }
        if is_float {
            self.push_at(Kind::Float, start, line, col);
            return;
        }
        // **VD-266**: tầng từ vựng nhận **độ lớn** tới `i64::MAX + 1` (2^63) để `-2^63` gộp
        // thành `i64::MIN`; literal dương 2^63 bị tầng kiểm kiểu từ chối (VS0104).
        let magnitude = match lit::decode_int_magnitude(text) {
            Ok(v) => v,
            Err(e) => {
                self.error_at(e.code, start + e.at);
                return;
            }
        };

        // Hậu tố đơn vị đo → SIZE; đơn vị thời gian → DURATION (§1, chỉ cho `INT`).
        // **VD-266**: size/duration vẫn phải nằm trong `i64` — chỉ `INT` thuần mới được chạm
        // độ lớn 2^63 (và chỉ khi có dấu `-`).
        if let Some(n) = scan_size_suffix(self.rest()) {
            if magnitude > i64::MAX as u64 {
                self.error_at(104, start + text.len());
                return;
            }
            for _ in 0..n {
                self.bump_ascii();
            }
            self.push_at(Kind::Size, start, line, col);
            return;
        }
        if let Some(n) = scan_duration_suffix(self.rest()) {
            if magnitude > i64::MAX as u64 {
                self.error_at(104, start + text.len());
                return;
            }
            for _ in 0..n {
                self.bump_ascii();
            }
            self.push_at(Kind::Duration, start, line, col);
            return;
        }
        self.push_at(Kind::Int, start, line, col);
    }

    /// `e`/`E` [+-]? chữ số → số mũ. Trả `true` nếu đã ăn.
    pub(super) fn scan_exponent(&mut self) -> bool {
        let mut j = 0usize;
        match self.peek(j) {
            Some(b'e' | b'E') => j += 1,
            _ => return false,
        }
        if matches!(self.peek(j), Some(b'+' | b'-')) {
            j += 1;
        }
        if !self.peek(j).is_some_and(|b| b.is_ascii_digit()) {
            return false;
        }
        j += 1;
        while self.peek(j).is_some_and(|b| b.is_ascii_digit()) {
            j += 1;
        }
        for _ in 0..j {
            self.bump_ascii();
        }
        true
    }
}

/// Độ dài hậu tố đơn vị kích thước khớp đầu `s` (`kb`, `kib`, `b`…), dài nhất trước.
fn scan_size_suffix(s: &str) -> Option<usize> {
    lit::SIZE_UNITS
        .iter()
        .find(|u| s.starts_with(*u))
        .map(|u| u.len())
}

/// Độ dài chuỗi đơn vị thời gian ghép **giảm dần** (`2h30m` → 5). Đơn vị đứng sau phải nhỏ
/// hơn đơn vị trước, nếu không chuỗi dừng ở đoạn đã đọc (SPEC-lex §1, §2).
fn scan_duration_suffix(s: &str) -> Option<usize> {
    let mut i = 0usize;
    let mut last_rank: Option<u8> = None;
    loop {
        let unit = lit::DURATION_UNITS
            .iter()
            .find(|(u, _)| s[i..].starts_with(u))?;
        if let Some(prev) = last_rank {
            if unit.1 >= prev {
                return Some(i);
            }
        }
        i += unit.0.len();
        last_rank = Some(unit.1);
        let start_digits = i;
        let digits = s[i..]
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(s[i..].len());
        if digits == 0 {
            return Some(i);
        }
        i += digits;
        // Chữ số chỉ thuộc token nếu theo sau là đơn vị **nhỏ hơn**; ngược lại nhả chữ số ra
        // cho token INT kế tiếp (`30m2h` ⇒ DURATION(30m) INT(2)…, không phải DURATION(30m2)).
        let next_ok = lit::DURATION_UNITS
            .iter()
            .any(|(u, r)| s[i..].starts_with(u) && Some(*r) < last_rank);
        if !next_ok {
            return Some(start_digits);
        }
    }
}
