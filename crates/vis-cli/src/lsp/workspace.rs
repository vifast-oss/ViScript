// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-277** — `workspace/symbol` cho LSP.
//!
//! Quét **thư mục gốc** (`initialize.params.rootUri`) tìm `*.vis`/`*.vish`, dùng cây ký hiệu
//! (`phan_tich`) để trả `SymbolInformation[]`. Giới hạn an toàn: 2.000 tệp, độ sâu 8, bỏ
//! `.git`/`target`/`node_modules`, không đi theo symlink; 200 ký hiệu.
//!
//! Không có `rootUri` ⇒ mảng rỗng (không đoán thư mục hiện tại).

use std::path::{Path, PathBuf};

use vis_interp::value::Value;
use vis_syntax::Mode;

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::phan_tich;
use super::vi_tri::khoang;

const MAX_FILES: usize = 2000;
const MAX_DEPTH: usize = 8;
const MAX_SYMS: usize = 200;

fn la_tep(p: &Path) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()),
        Some("vis") | Some("vish")
    )
}

fn gom(p: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if out.len() >= MAX_FILES || depth > MAX_DEPTH {
        return;
    }
    let Ok(rd) = std::fs::read_dir(p) else {
        return;
    };
    let mut ds: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    ds.sort();
    for x in ds {
        if out.len() >= MAX_FILES {
            break;
        }
        let Ok(ft) = x.symlink_metadata() else {
            continue;
        };
        if ft.file_type().is_symlink() {
            continue;
        }
        if ft.is_dir() {
            let ten = x.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(ten, ".git" | "target" | "node_modules") {
                continue;
            }
            gom(&x, out, depth + 1);
        } else if ft.is_file() && la_tep(&x) {
            out.push(x);
        }
    }
}

fn vi_tri(text: &str, start: usize, end: usize) -> Value {
    let ((l0, c0), (l1, c1)) = khoang(text, vis_syntax::Span::new(start, end));
    m(vec![
        (
            "start",
            m(vec![
                ("line", Value::Int(l0 as i64)),
                ("character", Value::Int(c0 as i64)),
            ]),
        ),
        (
            "end",
            m(vec![
                ("line", Value::Int(l1 as i64)),
                ("character", Value::Int(c1 as i64)),
            ]),
        ),
    ])
}

/// Phản hồi `workspace/symbol` (`SymbolInformation[]`).
pub fn tra_loi_symbol(root: Option<&Path>, query: &str, id: &Value) -> Vec<u8> {
    let Some(root) = root else {
        return phan_hoi(id, Value::List(Vec::new()));
    };
    let mut files = Vec::new();
    gom(root, &mut files, 0);
    let mut out: Vec<Value> = Vec::new();
    for p in files {
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let mode = if p.extension().and_then(|e| e.to_str()) == Some("vish") {
            Mode::Command
        } else {
            Mode::Script
        };
        let kq = phan_tich(&text, mode);
        let uri = format!("file://{}", p.display());
        for k in &kq.ky_hieu {
            if !query.is_empty() && !k.ten.contains(query) {
                continue;
            }
            out.push(m(vec![
                ("name", s(&k.ten)),
                ("kind", Value::Int(k.kind as i64)),
                (
                    "location",
                    m(vec![
                        ("uri", s(&uri)),
                        ("range", vi_tri(&kq.text, k.span.start, k.span.end)),
                    ]),
                ),
            ]));
            if out.len() >= MAX_SYMS {
                break;
            }
        }
        if out.len() >= MAX_SYMS {
            break;
        }
    }
    phan_hoi(id, Value::List(out))
}

/// **ST-278**: fallback định nghĩa xuyên tệp — quét `*.vis`/`*.vish` dưới `root` tìm item cấp tệp
/// trùng tên (tối đa 20 vị trí). Không có ⇒ `result:null`.
pub fn tra_loi_definition(root: Option<&Path>, ten: &str, id: &Value) -> Vec<u8> {
    let Some(root) = root else {
        return phan_hoi(id, Value::none());
    };
    let mut files = Vec::new();
    gom(root, &mut files, 0);
    let mut locs: Vec<Value> = Vec::new();
    for p in files {
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let mode = if p.extension().and_then(|e| e.to_str()) == Some("vish") {
            Mode::Command
        } else {
            Mode::Script
        };
        let kq = phan_tich(&text, mode);
        let uri = format!("file://{}", p.display());
        for k in &kq.ky_hieu {
            if k.ten != ten {
                continue;
            }
            locs.push(m(vec![
                ("uri", s(&uri)),
                ("range", vi_tri(&kq.text, k.span.start, k.span.end)),
            ]));
            if locs.len() >= 20 {
                break;
            }
        }
        if locs.len() >= 20 {
            break;
        }
    }
    if locs.is_empty() {
        phan_hoi(id, Value::none())
    } else {
        phan_hoi(id, Value::List(locs))
    }
}
