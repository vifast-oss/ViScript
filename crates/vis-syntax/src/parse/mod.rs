// SPDX-License-Identifier: MIT OR Apache-2.0
//! Parser + CST lossless — ST-05 (tài liệu thiết kế Phụ lục A, A.1–A.3/A.5).
//!
//! API công khai: [`parse`] → [`ParseOutput`] (văn bản đã chuẩn hoá + CST + chẩn đoán).
//!
//! # Hợp đồng
//!
//! - **Lossless**: nguồn parse thành công ⇒ `cst.render(&out.text) == out.text` (khoảng trắng là
//!   gap giữa hai lá, chú thích được gắn tường minh — xem [`crate::cst`]).
//! - **Mỗi tệp lỗi đúng 1 lỗi đầu** (VD-35): parser dừng ngay chẩn đoán cú pháp đầu tiên ở cấp
//!   tệp, không báo dồn, không panic; CST vẫn chứa toàn bộ token để render lại được.
//! - Mã lỗi cú pháp lấy từ **sổ riêng** `vis_diag::SYNTAX_CODES` (`VS0201`, `VS0202`) — không
//!   sửa sổ từ vựng `CODES` (VD-35).
//!
//! Chế độ lệnh (`Mode::Command`) là ST-06: vòng này parser trả `Program` chứa nguyên token
//! (lossless) và **không** phát chẩn đoán cú pháp.
//!
//! **ST-318f** — tách **nguyên văn** theo nhóm cú pháp (R1 mềm): `co_so` (trợ giúp token/node),
//! `khai_bao` (A.1), `kieu` (A.5), `pattern` (A.4), `lenh` (A.2), `expr` (A.3), `api`; tệp này
//! chỉ còn entry + `pub use`. Token/AST/CST/API không đổi.

mod api;
pub mod cmd;
mod co_so;
pub mod expr;
mod khai_bao;
mod kieu;
mod lenh;
mod pattern;

pub use api::*;

use crate::cst::Cst;
use crate::lex::token::Token;
use crate::lex::Diagnostic;

/// Kết quả một lượt parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    /// Văn bản đã chuẩn hoá (mốc của mọi `span`).
    pub text: String,
    pub cst: Cst,
    /// Chẩn đoán từ vựng + **tối đa một** chẩn đoán cú pháp (lỗi đầu).
    pub diagnostics: Vec<Diagnostic>,
}

/// Bộ parse đệ quy xuống trên luồng token (bỏ chú thích khỏi luồng cú pháp; chúng được gắn vào
/// CST bằng [`Cst::attach_comments`]).
pub struct Parser<'a> {
    toks: &'a [Token],
    /// Văn bản nguồn đã NFC — cần cho chế độ lệnh (cắt biểu thức trong `( … )`, thân closure).
    text: &'a str,
    /// Chỉ số token **không phải chú thích** (kể cả `EOF` ở cuối).
    sig: Vec<u32>,
    pos: usize,
    /// **ST-60b**: tên **đã bind** ở vùng khai báo (dựng sẵn + dòng khai báo trước đó) — dùng để
    /// phân loại `let` khai báo ↔ shell-let (`let out = ls -l` là shell-let vì `l` chưa bind).
    da_bind: std::collections::HashSet<String>,
    /// **VD-266**: độ sâu ngoặc `() [] {}` của các token **đã ăn**. Lexer không sinh
    /// NEWLINE/INDENT/DEDENT khi `depth > 0` (SPEC-lex §4.3), nên `suite()` dùng bộ đếm này để
    /// nhận **kết thúc khối ngầm** cho `if`/`match` dạng biểu thức nằm trong đối số/danh sách/
    /// ngoặc — chỉ khi thật sự trong ngoặc, không nới lỏng cấp câu lệnh.
    delim: usize,
}

/// Lỗi cú pháp đầu tiên — chuyển thành [`Diagnostic`] ở [`parse`].
pub struct PErr(pub Diagnostic);

type PResult<T> = Result<T, PErr>;
