// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tra cứu mã: [`lookup`], [`lookup_str`], [`next_block`]. Tách nguyên văn từ `lib.rs` (ST-318d).

use super::books::{all_codes, BLOCKS_RESERVED};
use super::codes::Code;

/// Tra một mã theo số (ví dụ `103` ⇒ `VS0103`). `None` nếu chưa cấp.
///
/// Tra **cả bốn sổ** đã cấp (VD-42): từ vựng `01xx`, cú pháp `02xx`, tên `03xx`, kiểu `04xx`.
/// Nay đủ **sáu sổ** (`01xx`..`06xx`); `lookup(701)` mới là `None`.
/// [`CODES`](crate::CODES)/[`SYNTAX_CODES`](crate::SYNTAX_CODES) vẫn nguyên vẹn nên test "khớp SPEC-lex §7" không đổi.
pub fn lookup(id: u16) -> Option<&'static Code> {
    all_codes().find(|c| c.id == id)
}

/// Tra theo chuỗi `VSxxxx` (chấp nhận `vs0103`); `None` nếu sai định dạng hoặc chưa cấp.
pub fn lookup_str(s: &str) -> Option<&'static Code> {
    let t = s.trim();
    let digits = t.strip_prefix("VS").or_else(|| t.strip_prefix("vs"))?;
    if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    lookup(digits.parse().ok()?)
}

/// Khối mã kế tiếp còn trống (quy tắc cấp): trả số đầu của khối mới, không tái dùng.
///
/// Suy ra từ **khối** đã cấp ([`all_codes`]: `01xx`..`05xx`) **và** khối đã **dành trước**
/// ([`BLOCKS_RESERVED`]: `01xx`..`06xx`) ⇒ `max(3, 6) + 1 = 07` ⇒ **`701`** (VD-42, ST-18). Không
/// hardcode `701`; thêm sổ mới thì số tự nhảy.
pub fn next_block() -> u16 {
    // Suy ra từ **khối** đã cấp (mã lớn nhất / 100) và khối đã dành trước — không hardcode số.
    let issued = all_codes().map(|c| c.id / 100).max().unwrap_or(0);
    let reserved = BLOCKS_RESERVED.iter().copied().max().unwrap_or(0);
    let block = issued.max(reserved) + 1;
    block * 100 + 1
}
