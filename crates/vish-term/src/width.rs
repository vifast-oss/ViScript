// SPDX-License-Identifier: MIT OR Apache-2.0
//! Độ rộng hiển thị: **số scalar sau NFC** — cùng quy tắc `Render` ST-41 (`vis-interp`).

use unicode_normalization::UnicodeNormalization;

/// Độ rộng cột của `s`: số `char` sau chuẩn hoá NFC.
///
/// NFC để NFD và NFC của cùng chữ tiếng Việt cho **cùng** độ rộng (ca `nfd_va_nfc` của ST-41).
/// Không đếm grapheme/độ rộng Đông Á — đúng chốt "cột theo Unicode scalar" của thiết kế.
pub fn display_width(s: &str) -> usize {
    s.nfc().count()
}
