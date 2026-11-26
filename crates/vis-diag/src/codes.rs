// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kiểu nền của sổ mã: [`code`] (`VS` + 4 chữ số), [`Severity`], [`Code`], [`Lang`]. Tách
//! nguyên văn từ `lib.rs` (ST-318d).

/// Định dạng một mã lỗi ViScript: `VS` + **đúng 4 chữ số** (VD: `VS0101`).
///
/// # Panics
/// `n > 9999` — SPEC hứa 4 chữ số; cấp quá là lỗi lập trình, không được lặng lẽ thành 5 chữ số.
pub fn code(n: u16) -> String {
    assert!(n <= 9999, "mã lỗi phải 4 chữ số (0..=9999), nhận {n}");
    format!("VS{n:04}")
}

/// Mức của một mã: lỗi hay cảnh báo-lỗi (tab vẫn lex được nhưng phải báo — SPEC-lex §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Một dòng trong sổ mã lỗi. Số **không tái dùng** (quy tắc cấp, xem `docs` của crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    pub id: u16,
    pub short: &'static str,
    /// Thông điệp tiếng Việt — **nguồn chuẩn** (SPEC-lex §7; test `so_khop_spec_lex_that` khoá).
    pub message_vi: &'static str,
    /// Thông điệp tiếng Anh (ST-03).
    pub message_en: &'static str,
    /// Thông điệp tiếng Nhật (ST-03).
    pub message_ja: &'static str,
    pub severity: Severity,
}

/// Ngôn ngữ thông điệp (ST-03): mặc định `Vi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Vi,
    En,
    Ja,
}

impl Lang {
    /// `vi`/`en`/`ja` (không phân biệt hoa thường); sai ⇒ `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "vi" => Some(Self::Vi),
            "en" => Some(Self::En),
            "ja" => Some(Self::Ja),
            _ => None,
        }
    }
}

impl Code {
    /// Thông điệp theo ngôn ngữ; `message_vi` vẫn là chuẩn của SPEC-lex §7.
    pub fn message(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Vi => self.message_vi,
            Lang::En => self.message_en,
            Lang::Ja => self.message_ja,
        }
    }
}
