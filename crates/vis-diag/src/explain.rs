// SPDX-License-Identifier: MIT OR Apache-2.0
//! Trợ giúp i18n: [`ExplainHelp`]/[`explain_help`] cho `vis explain` + [`severity_label`].
//! Tách nguyên văn từ `lib.rs` (ST-318d).

use super::books::{all_codes, BLOCKS_RESERVED};
use super::codes::{Lang, Severity};

/// **ST-316b (`#47`)** — trợ giúp `vis explain` theo ngôn ngữ: **một nguồn** trong `vis-diag`
/// (`vis-cli` chỉ in ra, không giữ bản sao chuỗi). Số khối/mã **đếm động**, không viết cứng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainHelp {
    /// Dòng cú pháp (`dùng:`/`usage:`/`使い方:`).
    pub usage: String,
    /// Dòng số khối/mã + gợi ý `--list`/`--json`.
    pub book: String,
}

/// Trợ giúp `vis explain` cho `lang` (vi/en/ja) — nguồn duy nhất của 3 ngôn ngữ.
pub fn explain_help(lang: Lang) -> ExplainHelp {
    let khoi = BLOCKS_RESERVED.len();
    let ma = all_codes().count();
    let (usage, book) = match lang {
        Lang::Vi => (
            "dùng: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]".to_string(),
            format!(
                "sổ mã: {khoi} khối, {ma} mã — `vis explain --list` in toàn bộ (mã + câu ngắn); `--json` in một dòng máy đọc"
            ),
        ),
        Lang::En => (
            "usage: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]".to_string(),
            format!(
                "code book: {khoi} blocks, {ma} codes — `vis explain --list` prints all (code + short text); `--json` prints one machine-readable line"
            ),
        ),
        Lang::Ja => (
            "使い方: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]".to_string(),
            format!(
                "コード帳: {khoi} ブロック、{ma} コード — `vis explain --list` で全件表示（コード + 短い説明）、`--json` は機械可読な1行"
            ),
        ),
    };
    ExplainHelp { usage, book }
}

/// Nhãn mức theo ngôn ngữ.
pub fn severity_label(sev: Severity, lang: Lang) -> &'static str {
    match (sev, lang) {
        (Severity::Error, Lang::Vi) => "lỗi",
        (Severity::Warning, Lang::Vi) => "cảnh báo",
        (Severity::Error, Lang::En) => "error",
        (Severity::Warning, Lang::En) => "warning",
        (Severity::Error, Lang::Ja) => "エラー",
        (Severity::Warning, Lang::Ja) => "警告",
    }
}
