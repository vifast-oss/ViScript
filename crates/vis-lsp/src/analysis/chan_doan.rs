// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-68 (VD-102 (3))** — dựng notification `textDocument/publishDiagnostics`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use vis_core::value::Value;
use vis_diag::Severity;
use vis_syntax::Mode;

use super::jsonrpc::{m, s, thong_bao};
use super::phan_tich::{self, ChanDoan};
use super::vi_tri;

/// **ST-80**: chẩn đoán **module-aware** cho tệp **có trên đĩa** — nạp cả cây `use`, nhưng chỉ
/// giữ chẩn đoán của **tệp chính** (`rel` == tên tệp). Chẩn đoán tệp phụ **không** publish vào
/// tài liệu chính (LSP một tài liệu); lỗi thiếu module được `modules` gán cho tệp chính nên **vẫn**
/// hiện. Trả `None` nếu nạp lỗi ⇒ rơi về đường một tệp.
fn chan_doan_module(path: &Path, text: &str) -> Option<Vec<ChanDoan>> {
    let ten = path.file_name()?.to_string_lossy().to_string();
    let l = vis_loader::modules::load_full_voi_text(path.to_str()?, text, true, true).ok()?;
    let mut ds: Vec<ChanDoan> = l
        .diags
        .iter()
        .filter(|(rel, _)| rel == &ten)
        .map(|(_, d)| ChanDoan {
            span: d.span,
            ma: d.code.short,
            severity: match d.code.severity {
                Severity::Error => 1,
                Severity::Warning => 2,
            },
            message: d
                .message
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t"),
        })
        .collect();
    ds.sort_by(|a, b| a.span.start.cmp(&b.span.start).then(a.ma.cmp(b.ma)));
    Some(ds)
}

fn chuan(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Tài liệu `main_uri` có phụ thuộc (trực tiếp hoặc gián tiếp) vào `target` không.
///
/// Ưu tiên cây module thật (`Loaded.files`, đã canonical hoá, gồm cả phụ thuộc bắc cầu và buffer
/// đang mở ở overlay VS3-7). Nếu nạp lỗi (syntax/cycle/mất tệp) ⇒ quét `use ./…` **trực tiếp** để
/// vẫn phát hiện quan hệ tối thiểu — không publish nhầm tài liệu không liên quan.
fn phu_thuoc(main_uri: &str, text: &str, target: &Path) -> bool {
    let Some(main) = super::uri_thanh_duong(main_uri) else {
        return false;
    };
    if let Ok(l) = vis_loader::modules::load_full_voi_text(&main, text, true, true) {
        if l.files.iter().any(|f| chuan(&f.path) == *target) {
            return true;
        }
    }
    let dir = Path::new(&main).parent().unwrap_or_else(|| Path::new("."));
    super::module::use_dia_phuong(text, dir)
        .iter()
        .any(|p| chuan(p) == *target)
}

/// **VS3-7 (VD-271)**: URI các tài liệu **đang mở** phụ thuộc vào tài liệu `thay_doi_uri` (bị đổi/
/// mở/đóng) — cần publish lại chẩn đoán module-aware. Trả danh sách **sắp theo URI**, không gồm
/// chính `thay_doi_uri`, không trùng.
pub fn uri_lien_quan(thay_doi_uri: &str, van_ban: &HashMap<String, String>) -> Vec<String> {
    let Some(p) = super::uri_thanh_duong(thay_doi_uri) else {
        return Vec::new();
    };
    let target = chuan(Path::new(&p));
    let mut out: Vec<String> = Vec::new();
    for (uri, text) in van_ban {
        if uri == thay_doi_uri {
            continue;
        }
        if phu_thuoc(uri, text, &target) {
            out.push(uri.clone());
        }
    }
    out.sort();
    out
}

/// Notification chẩn đoán; `text = None` ⇒ `diagnostics:[]` (đóng tệp).
pub fn than(uri: &str, text: Option<&str>, mode: Mode) -> Vec<u8> {
    let ds = match text {
        None => Vec::new(),
        Some(t) => match super::uri_thanh_duong(uri)
            .filter(|p| Path::new(p).is_file())
            .and_then(|p| chan_doan_module(Path::new(&p), t))
        {
            // **ST-240 / VD-242 §4 (nợ #179)**: `span` của chẩn đoán trỏ vào văn bản đã chuẩn hoá
            // CRLF→LF. Dùng đúng bản đó để tính range — nếu dùng `t` thô, mỗi CRLF trước span làm
            // `line` bị lùi một dòng so với tệp LF cùng nội dung.
            Some(ds) => {
                let norm = vis_syntax::lex::normalize_source(t);
                ds.iter().map(|d| gia_tri(&norm, d)).collect()
            }
            None => {
                let kq = phan_tich::phan_tich(t, mode);
                kq.ds.iter().map(|d| gia_tri(&kq.text, d)).collect()
            }
        },
    };
    thong_bao(
        "textDocument/publishDiagnostics",
        m(vec![("uri", s(uri)), ("diagnostics", Value::list(ds))]),
    )
}

fn gia_tri(text: &str, d: &ChanDoan) -> Value {
    let ((l0, c0), (l1, c1)) = vi_tri::khoang(text, d.span);
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    m(vec![
        (
            "range",
            m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
        ),
        ("severity", Value::Int(d.severity as i64)),
        ("code", s(d.ma)),
        ("source", s("vis")),
        ("message", s(&d.message)),
    ])
}
