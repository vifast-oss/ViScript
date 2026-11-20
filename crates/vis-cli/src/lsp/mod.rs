// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-67/68/69 (VD-102)** — `vis lsp --stdio`: khung LSP + JSON-RPC 2.0 + dispatch.
//!
//! `initialize` (capabilities cố định), `initialized` (notification, không frame), `shutdown`
//! (`result:null`), `exit` (0 nếu đã shutdown, 1 nếu chưa), method lạ `-32601`, body JSON hỏng
//! `-32700`, request trước `initialize` `-32002`; `didOpen/didChange/didClose` ⇒ publishDiagnostics
//! (ST-68); `textDocument/completion` + `textDocument/hover` (ST-69, dùng văn bản đã lưu).
//!
//! stdout **tuyệt đối sạch**: chỉ frame; mọi lỗi giao thức ghi ra stderr.

mod chan_doan;
mod chon;
mod chu_ky;
mod codelens;
mod dinh_dang;
mod dinh_nghia;
mod doi_ten;
mod gap;
mod goi_y;
mod inlay_hint;
mod jsonrpc;
mod khung;
mod ky_hieu;
mod module;
mod phan_tich;
mod phuong_loi;
mod thanh_vien;
mod thuc_thi;
mod type_definition;
mod vi_tri;
mod workspace;

use std::collections::HashMap;
use std::io::{self, BufReader};
use std::process::ExitCode;
use std::rc::Rc;

use vis_interp::json;
use vis_interp::value::Value;
use vis_syntax::Mode;

use phan_tich::KetQua;

/// **ST-283**: cache kết quả phân tích theo `(uri, text)` trong một phiên LSP. Mọi thay đổi
/// văn bản (didOpen/didChange/didClose) **xoá cả cache** để không phục vụ module/dependency cũ.
fn phan_tich_cache(
    cache: &mut HashMap<String, (String, Rc<KetQua>)>,
    uri: &str,
    text: &str,
    mode: Mode,
) -> Rc<KetQua> {
    if let Some((cu, kq)) = cache.get(uri) {
        if cu == text {
            return kq.clone();
        }
    }
    let kq = Rc::new(module::phan_tich_uri(uri, text, mode));
    cache.insert(uri.to_string(), (text.to_string(), kq.clone()));
    kq
}

/// Chạy server LSP tới khi `exit` hoặc **EOF stdin**. Chỉ nhận `--stdio` (mặc định cũng là stdio).
pub fn run(args: &[String]) -> ExitCode {
    for a in args {
        match a.as_str() {
            "--stdio" => {}
            khac => {
                eprintln!("vis lsp: cờ lạ `{khac}` (dùng --stdio)");
                return ExitCode::from(2);
            }
        }
    }
    let stdin = io::stdin();
    let mut doc = khung::Doc::moi(BufReader::new(stdin.lock()));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut da_khoi_tao = false;
    let mut da_shutdown = false;
    // **ST-69**: văn bản gần nhất theo URI (didOpen/didChange) — request completion/hover đọc lại.
    let mut van_ban: HashMap<String, String> = HashMap::new();
    // **ST-277**: thư mục gốc từ `initialize.params.rootUri` (workspace/symbol).
    let mut root: Option<std::path::PathBuf> = None;
    // **ST-283**: cache phân tích của tệp đang mở (xoá khi văn bản đổi).
    let mut kq_cache: HashMap<String, (String, Rc<KetQua>)> = HashMap::new();

    // Ghi frame; lỗi ghi (stdout đóng) ⇒ dừng với mã 1 (không panic).
    macro_rules! gui {
        ($body:expr) => {
            if let Err(e) = khung::ghi(&mut out, &$body) {
                eprintln!("vis lsp: {e}");
                return ExitCode::from(1);
            }
        };
    }

    loop {
        let body = match doc.doc_frame() {
            Ok(Some(b)) => b,
            Ok(None) => return ExitCode::SUCCESS, // EOF stdin — thoát sạch
            Err(e) => {
                eprintln!("vis lsp: {e}");
                return ExitCode::from(1);
            }
        };
        let text = match std::str::from_utf8(&body) {
            Ok(t) => t,
            Err(_) => {
                gui!(jsonrpc::loi(
                    &jsonrpc::id_null(),
                    -32700,
                    "JSON không hợp lệ",
                    Value::Str("body không phải UTF-8".to_string()),
                ));
                continue;
            }
        };
        let v = match json::decode(text) {
            Ok(v) => v,
            Err(e) => {
                gui!(jsonrpc::loi(
                    &jsonrpc::id_null(),
                    -32700,
                    "JSON không hợp lệ",
                    Value::Str(e),
                ));
                continue;
            }
        };
        let Value::Map(kv) = &v else {
            gui!(jsonrpc::loi(
                &jsonrpc::id_null(),
                -32600,
                "request không phải object",
                Value::none(),
            ));
            continue;
        };
        let id = jsonrpc::lay(kv, "id").cloned();
        let method = jsonrpc::lay(kv, "method")
            .and_then(jsonrpc::chuoi)
            .map(str::to_string);
        let Some(method) = method else {
            // Object nhưng thiếu `method`: request hỏng (notification cũng không hợp lệ).
            if let Some(id) = id {
                gui!(jsonrpc::loi(&id, -32600, "thiếu `method`", Value::none()));
            }
            continue;
        };
        match method.as_str() {
            "initialize" => {
                da_khoi_tao = true;
                root = jsonrpc::lay_params(kv)
                    .and_then(|p| match p {
                        Value::Map(p) => jsonrpc::lay(p, "rootUri"),
                        _ => None,
                    })
                    .and_then(jsonrpc::chuoi)
                    .and_then(uri_thanh_duong)
                    .map(std::path::PathBuf::from);
                if let Some(id) = id {
                    gui!(jsonrpc::phan_hoi(&id, ket_qua_khoi_tao()));
                }
            }
            // Notification: KHÔNG frame (kể cả trước initialize).
            "initialized" => {}
            // **ST-68**: văn bản mở/đổi (full sync) ⇒ tính lại chẩn đoán + `publishDiagnostics`.
            "textDocument/didOpen" => {
                if let Some((uri, text)) = jsonrpc::van_ban_mo(kv) {
                    van_ban.insert(uri.clone(), text.clone());
                    kq_cache.clear();
                    gui!(chan_doan::than(&uri, Some(&text), mode_cua(&uri)));
                }
            }
            "textDocument/didChange" => {
                if let Some((uri, text)) = jsonrpc::van_ban_doi(kv) {
                    van_ban.insert(uri.clone(), text.clone());
                    kq_cache.clear();
                    gui!(chan_doan::than(&uri, Some(&text), mode_cua(&uri)));
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = jsonrpc::uri_cua(kv) {
                    van_ban.remove(&uri);
                    kq_cache.clear();
                    // **ST-100**: dọn cache nạp module; URI khác vẫn dùng lại bình thường.
                    crate::modules::xoa_cache_phu();
                    module::xoa_cache_module();
                    gui!(chan_doan::than(&uri, None, mode_cua(&uri)));
                }
            }
            // **ST-69**: completion + hover (dùng văn bản đã lưu; chưa mở ⇒ rỗng).
            "textDocument/completion" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            // Chưa mở ⇒ nội dung rỗng ⇒ `[]` (không lỗi).
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(goi_y::tra_loi_completion(&kq, byte, &id));
                            }
                        }
                    }
                }
            }
            "textDocument/hover" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(goi_y::tra_loi_hover(&kq, byte, &id));
                            }
                        }
                    }
                }
            }
            // **ST-269**: go-to-definition trong tệp đang mở.
            "textDocument/definition" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(dinh_nghia::tra_loi_definition(
                                    &kq,
                                    &uri,
                                    byte,
                                    root.as_deref(),
                                    &id
                                ));
                            }
                        }
                    }
                }
            }
            // **VD-270**: nhảy tới khai báo kiểu (bản ghi/enum) của từ tại con trỏ.
            "textDocument/typeDefinition" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(type_definition::tra_loi(&kq, &uri, byte, &id));
                            }
                        }
                    }
                }
            }
            // **VD-270**: vị trí các `impl` của phương thức tại con trỏ (ghép tên trait/impl).
            "textDocument/implementation" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(thuc_thi::tra_loi_implementation(&kq, &uri, byte, &id));
                            }
                        }
                    }
                }
            }
            // **ST-273**: gợi ý chữ ký hàm cấp tệp tại lời gọi.
            "textDocument/signatureHelp" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(chu_ky::tra_loi_signature(&kq, byte, mode_cua(&uri), &id));
                            }
                        }
                    }
                }
            }
            // **ST-280**: chuẩn bị đổi tên (range + placeholder) trước khi client hỏi tên mới.
            "textDocument/prepareRename" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(doi_ten::tra_loi_prepare(&kq, byte, &id));
                            }
                        }
                    }
                }
            }
            // **ST-272**: đổi tên định danh trong tệp (token Ident, không chạm chuỗi/chú thích).
            "textDocument/rename" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        let ten_moi = jsonrpc::lay_params(kv).and_then(|p| match p {
                            Value::Map(p) => jsonrpc::lay(p, "newName"),
                            _ => None,
                        });
                        let ten_moi = ten_moi.and_then(jsonrpc::chuoi);
                        match (ten_moi, van_ban.get(&uri)) {
                            (Some(ten_moi), Some(text)) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(doi_ten::tra_loi_rename(
                                    &kq,
                                    &uri,
                                    byte,
                                    ten_moi,
                                    mode_cua(&uri),
                                    &id
                                ));
                            }
                            _ => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                        }
                    }
                }
            }
            // **VD-270**: chuỗi vùng chọn tại từng vị trí (token → dòng → khai báo → tệp).
            "textDocument/selectionRange" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, vi_tris)) = jsonrpc::vi_tri_nhieu(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                gui!(chon::tra_loi_selection(&kq, mode_cua(&uri), &vi_tris, &id));
                            }
                        }
                    } else {
                        gui!(jsonrpc::phan_hoi(&id, Value::none()));
                    }
                }
            }
            // **ST-282**: highlight mọi lần xuất hiện định danh trong tệp.
            "textDocument/documentHighlight" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(dinh_nghia::tra_loi_highlights(
                                    &kq,
                                    byte,
                                    mode_cua(&uri),
                                    &id
                                ));
                            }
                        }
                    }
                }
            }
            // **ST-271**: tìm mọi tham chiếu của từ tại con trỏ trong tệp.
            "textDocument/references" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some((uri, line, ch)) = jsonrpc::vi_tri_van_ban(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(dinh_nghia::tra_loi_references(&kq, &uri, byte, &id));
                            }
                        }
                    }
                }
            }
            // **ST-270**: cây ký hiệu của tệp đang mở.
            "textDocument/documentSymbol" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                gui!(ky_hieu::tra_loi_document_symbol(&kq, &id));
                            }
                        }
                    }
                }
            }
            // **VD-270**: code lens chạy test cho từng `test fn` cấp tệp.
            "textDocument/codeLens" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                gui!(codelens::tra_loi_code_lens(&kq, &uri, &id));
                            }
                        }
                    }
                }
            }
            // **VD-270**: gợi ý kiểu (inlay hint) cho `let`/`var` không có nhãn kiểu.
            "textDocument/inlayHint" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match van_ban.get(&uri) {
                            // Tệp chưa mở ⇒ `null` (khác các method theo vị trí trả `[]`).
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                gui!(inlay_hint::tra_loi(&kq, &id));
                            }
                        }
                    }
                }
            }
            // **ST-275**: vùng gấp theo khai báo nhiều dòng.
            "textDocument/foldingRange" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                gui!(gap::tra_loi_folding(&kq, &id));
                            }
                        }
                    }
                }
            }
            // **ST-277**: tìm ký hiệu trong thư mục gốc (workspace/symbol).
            "workspace/symbol" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else {
                        let query = jsonrpc::lay_params(kv)
                            .and_then(|p| match p {
                                Value::Map(p) => jsonrpc::lay(p, "query"),
                                _ => None,
                            })
                            .and_then(jsonrpc::chuoi)
                            .unwrap_or("");
                        gui!(workspace::tra_loi_symbol(root.as_deref(), query, &id));
                    }
                }
            }
            // **ST-279**: format toàn tệp bằng formatter CLI.
            "textDocument/formatting" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match van_ban.get(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::List(Vec::new()))),
                            Some(text) => {
                                let kq = phan_tich_cache(&mut kq_cache, &uri, text, mode_cua(&uri));
                                gui!(dinh_dang::tra_loi_format(&kq, mode_cua(&uri), &id));
                            }
                        }
                    }
                }
            }
            "shutdown" => {
                if !da_khoi_tao {
                    if let Some(id) = id {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    }
                    continue;
                }
                da_shutdown = true;
                if let Some(id) = id {
                    gui!(jsonrpc::phan_hoi(&id, Value::none()));
                }
            }
            "exit" => {
                return if da_shutdown {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                };
            }
            khac => {
                if !da_khoi_tao {
                    if let Some(id) = id {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    }
                    continue;
                }
                if let Some(id) = id {
                    gui!(jsonrpc::loi(
                        &id,
                        -32601,
                        "method không tồn tại",
                        Value::Str(khac.to_string())
                    ));
                }
            }
        }
    }
}

/// Kết quả `initialize` — **cố định** (không version/rootUri) để snapshot tất định.
fn ket_qua_khoi_tao() -> Value {
    let completion = Value::Map(vec![jsonrpc::kv(
        "triggerCharacters",
        Value::List(vec![Value::Str(".".to_string())]),
    )]);
    let capabilities = Value::Map(vec![
        jsonrpc::kv("textDocumentSync", Value::Int(1)),
        jsonrpc::kv("positionEncoding", Value::Str("utf-16".to_string())),
        jsonrpc::kv("hoverProvider", Value::Bool(true)),
        jsonrpc::kv("definitionProvider", Value::Bool(true)),
        jsonrpc::kv("implementationProvider", Value::Bool(true)),
        jsonrpc::kv("documentSymbolProvider", Value::Bool(true)),
        jsonrpc::kv(
            "codeLensProvider",
            jsonrpc::m(vec![("resolveProvider", Value::Bool(false))]),
        ),
        jsonrpc::kv("foldingRangeProvider", Value::Bool(true)),
        jsonrpc::kv("workspaceSymbolProvider", Value::Bool(true)),
        jsonrpc::kv("documentFormattingProvider", Value::Bool(true)),
        jsonrpc::kv("referencesProvider", Value::Bool(true)),
        jsonrpc::kv("documentHighlightProvider", Value::Bool(true)),
        jsonrpc::kv("selectionRangeProvider", Value::Bool(true)),
        jsonrpc::kv("inlayHintProvider", Value::Bool(true)),
        jsonrpc::kv("typeDefinitionProvider", Value::Bool(true)),
        jsonrpc::kv(
            "renameProvider",
            jsonrpc::m(vec![("prepareProvider", Value::Bool(true))]),
        ),
        jsonrpc::kv(
            "signatureHelpProvider",
            jsonrpc::m(vec![(
                "triggerCharacters",
                Value::List(vec![Value::Str("(".into()), Value::Str(",".into())]),
            )]),
        ),
        jsonrpc::kv("completionProvider", completion),
    ]);
    let server_info = Value::Map(vec![jsonrpc::kv("name", Value::Str("vis".to_string()))]);
    Value::Map(vec![
        jsonrpc::kv("capabilities", capabilities),
        jsonrpc::kv("serverInfo", server_info),
    ])
}

/// **ST-80**: `textDocument.uri` (`file://…`) → đường dẫn tệp; `None` cho URI không phải `file:`,
/// percent-decode cơ bản (`%20`, UTF-8), chấp nhận đường dẫn tuyệt đối (`file:///…`).
pub(crate) fn uri_thanh_duong(uri: &str) -> Option<String> {
    uri_thanh_duong_voi(uri, cfg!(windows))
}

/// Như [`uri_thanh_duong`] nhưng nhận cờ nền tảng (test thuần trên Linux vẫn kiểm được nhánh
/// Windows). **VD-258**: percent-decode **trước** khi nhận diện drive, nên `file:///C%3A/path`
/// (VS Code gửi drive đã mã hoá) hoạt động; trước đây `%3A` chưa được giải mã nên path còn
/// `/C%3A/path` và bị coi là tương đối/hỏng trên Windows.
fn uri_thanh_duong_voi(uri: &str, windows: bool) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    // Percent-decode trước (UTF-8), rồi mới chuẩn hoá đường dẫn theo nền tảng.
    let rest = percent_giai_ma(rest)?;
    // **VD-241 § fix(Windows/CRLF #psvn-A5)**: LSP trên Windows gửi `file:///c:/path` (3 slash)
    // hoặc `file:///c:\path`. Trên đường dẫn tồn tại `/c:/...` không phải path hợp lệ trên Windows
    // (Rust Path coi `/c:/...` là tương đối); chuẩn hoá về `c:/...` khi phát hiện drive letter.
    fn trim_win_drive(r: &str) -> Option<String> {
        let b = r.as_bytes();
        // `/c:/path` or `/C:\path` → `c:/path` (lowercase drive)
        if b.len() >= 3
            && b[0] == b'/'
            && b[1].is_ascii_alphabetic()
            && b[2] == b':'
            && (b.len() == 3 || b[3] == b'/' || b[3] == b'\\')
        {
            return Some(r[1..].replace('\\', "/"));
        }
        // `c:/path` or `C:\path` (no leading slash) → `c:/path`
        if b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && (b[2] == b'/' || b[2] == b'\\')
        {
            return Some(r.replace('\\', "/"));
        }
        None
    }
    if windows {
        match trim_win_drive(&rest) {
            Some(r) => Some(r),
            None if rest.starts_with('/') => Some(rest),
            None => None,
        }
    } else {
        // Non-Windows: only accept absolute paths starting with `/`
        rest.starts_with('/').then_some(rest)
    }
}

/// Percent-decode cơ bản (UTF-8); `%` không theo sau 2 hex hợp lệ được giữ nguyên (hành vi cũ).
fn percent_giai_ma(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let h = std::str::from_utf8(&b[i + 1..i + 3]).ok()?;
            if let Ok(v) = u8::from_str_radix(h, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

/// `didOpen`: `params.textDocument.{uri,text}`.
/// `didChange` full sync: lấy `contentChanges[last].text`.
/// Chế độ phân tích theo đuôi URI (`.vish` ⇒ Command).
fn mode_cua(uri: &str) -> Mode {
    if uri.ends_with(".vish") {
        Mode::Command
    } else {
        Mode::Script
    }
}

#[cfg(test)]
mod vd258_tests {
    use super::{percent_giai_ma, phan_tich_cache, uri_thanh_duong_voi};
    use std::collections::HashMap;
    use std::rc::Rc;
    use vis_syntax::Mode;

    #[test]
    fn cache_phan_tich_theo_text() {
        let mut c: HashMap<String, (String, Rc<crate::lsp::phan_tich::KetQua>)> = HashMap::new();
        let a = phan_tich_cache(
            &mut c,
            "file:///x.vis",
            "fn main():\n    print(1)\n",
            Mode::Script,
        );
        let b = phan_tich_cache(
            &mut c,
            "file:///x.vis",
            "fn main():\n    print(1)\n",
            Mode::Script,
        );
        assert!(Rc::ptr_eq(&a, &b), "cùng text phải dùng lại cache");
        let c2 = phan_tich_cache(
            &mut c,
            "file:///x.vis",
            "fn main():\n    print(2)\n",
            Mode::Script,
        );
        assert!(!Rc::ptr_eq(&a, &c2), "text đổi phải phân tích lại");
    }

    #[test]
    fn uri_percent_drive_windows() {
        // VS Code trên Windows có thể gửi `C%3A` (drive đã percent-encode).
        assert_eq!(
            uri_thanh_duong_voi("file:///C%3A/a/b.vis", true).as_deref(),
            Some("C:/a/b.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///c%3a/a%20b.vis", true).as_deref(),
            Some("c:/a b.vis")
        );
        // Dạng cũ (không mã hoá) vẫn chạy; `%20` vẫn decode.
        assert_eq!(
            uri_thanh_duong_voi("file:///C:/a%20b.vis", true).as_deref(),
            Some("C:/a b.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///c:\\a\\b.vis", true).as_deref(),
            Some("c:/a/b.vis")
        );
        // **VD-259** — drive mã hoá cả chữ cái, backslash `%5C`, UTF-8 percent-encoded, `%`
        // hỏng: decode **trước** khi nhận diện drive; `%` hỏng giữ nguyên (không panic).
        assert_eq!(
            uri_thanh_duong_voi("file:///C%3A%5Ca%5Cb.vis", true).as_deref(),
            Some("C:/a/b.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///%43%3a/a.vis", true).as_deref(),
            Some("C:/a.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///C%3A/a%20b/%C3%A9.vis", true).as_deref(),
            Some("C:/a b/é.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///C%3A/a%zz.vis", true).as_deref(),
            Some("C:/a%zz.vis")
        );
        // Tương đối/không drive ⇒ None (trừ đường dẫn bắt đầu `/`).
        assert_eq!(uri_thanh_duong_voi("file://relative/x", true), None);
        assert_eq!(
            uri_thanh_duong_voi("file:///etc/x", true).as_deref(),
            Some("/etc/x")
        );
    }

    #[test]
    fn uri_posix_giu_nguyen() {
        assert_eq!(
            uri_thanh_duong_voi("file:///a%20b/c.vis", false).as_deref(),
            Some("/a b/c.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///C:/a", false).as_deref(),
            Some("/C:/a")
        );
        assert_eq!(uri_thanh_duong_voi("file://relative/x", false), None);
        assert_eq!(uri_thanh_duong_voi("http://x/y", false), None);
        // `%` không hợp lệ được giữ nguyên (hành vi cũ), không panic.
        assert_eq!(percent_giai_ma("a%zzb").as_deref(), Some("a%zzb"));
        assert_eq!(percent_giai_ma("%").as_deref(), Some("%"));
    }
}
