// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kiểu **công khai** của kết quả nạp: [`FileUnit`], [`Loaded`], [`LoadError`].

use std::path::PathBuf;

use vis_hir::{Hir, Symbols, TypeTable};
use vis_syntax::Diagnostic;

/// Một tệp đã nạp.
#[derive(Debug, Clone)]
pub struct FileUnit {
    /// Đường dẫn tuyệt đối (đã `canonicalize`) — dùng cho `--dump-deps`/test.
    #[allow(dead_code)]
    pub path: PathBuf,
    /// Đường dẫn tương đối so với **tệp chính** (ổn định khi đổi `cwd`).
    pub rel: String,
    pub text: String,
}

/// Kết quả nạp một đơn vị biên dịch.
pub struct Loaded {
    /// Đường dẫn tuyệt đối của tệp chính.
    #[allow(dead_code)]
    pub main: PathBuf,
    /// Tệp theo **hậu duyệt** (phụ thuộc trước, tệp chính cuối).
    pub files: Vec<FileUnit>,
    pub hir: Hir,
    pub symbols: Symbols,
    pub types: TypeTable,
    /// Chẩn đoán **tĩnh** kèm tên tệp: `(rel, diag)`.
    pub diags: Vec<(String, Diagnostic)>,
    /// **ST-240 (#177)**: `file_id → rel` theo thứ tự cấp (`Nap::file_rel`, chính = 0) — để CLI
    /// ánh xạ `span.file` của trap runtime về đúng tệp/module.
    pub file_rel: Vec<String>,
    /// Cây phụ thuộc cho `--dump-deps`: `rel → [rel phụ thuộc trực tiếp]`.
    pub deps: Vec<(String, Vec<String>)>,
    /// **ST-84**: `rel` của từng item trong `hir.items` (cùng thứ tự) — để LSP biết item thuộc tệp nào.
    pub item_rel: Vec<String>,
}

impl Loaded {
    /// Tổng byte của cả đơn vị (dùng cho log/đo).
    #[allow(dead_code)]
    pub fn tong_byte(&self) -> usize {
        self.files.iter().map(|f| f.text.len()).sum()
    }

    /// Tệp chính (phần tử cuối của `files`, thứ tự hậu duyệt).
    #[allow(dead_code)]
    pub fn tep_chinh(&self) -> Option<&FileUnit> {
        self.files.last()
    }
}

/// Lỗi **cấp API** khi nạp (không phải chẩn đoán ngôn ngữ).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Không phải tệp.
    NotFile(String),
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadError::NotFile(p) => write!(f, "không phải tệp: `{p}`"),
        }
    }
}
