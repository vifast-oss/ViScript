// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vòng lặp server LSP qua stdio + dispatch JSON-RPC (tách nguyên văn khỏi `lib.rs` ở `ST-317b`).
//!
//! stdout **tuyệt đối sạch**: chỉ frame; mọi lỗi giao thức ghi ra stderr. Hành vi/wire protocol
//! không đổi — chỉ đổi vị trí vật lý của hàm `run`.

use std::io::{self, BufReader};
use std::process::ExitCode;

use vis_core::json;
use vis_core::value::Value;

use super::khoi_tao::ket_qua_khoi_tao;
use super::uri::{mode_cua, uri_thanh_duong};
use crate::{
    chon, chu_ky, code_action, codelens, dinh_dang, dinh_nghia, doi_ten, gap, goi_y, inlay_hint,
    jsonrpc, khung, ky_hieu, semantic_tokens, sua, thuc_thi, type_definition, vi_tri, workspace,
    AnalysisHost,
};

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
    // **ST-317c (#81/#66)**: toàn bộ trạng thái phiên (buffer, cache phân tích/module, overlay
    // loader, rootUri) nằm trong **một** `AnalysisHost` tiêm được — không còn `thread_local!`.
    let mut host = AnalysisHost::new();

    // Ghi frame; lỗi ghi (stdout đóng) ⇒ dừng với mã 1 (không panic).
    macro_rules! gui {
        ($body:expr) => {
            if let Err(e) = khung::ghi(&mut out, &$body) {
                eprintln!("vis lsp: {e}");
                return ExitCode::from(1);
            }
        };
    }
    // **VS3-7**: publish lại chẩn đoán cho mọi tài liệu **đang mở** phụ thuộc tài liệu vừa đổi.
    // Dùng macro để mượn `gui!`/`out` của `run`; mỗi URI tối đa một frame mỗi sự kiện.
    macro_rules! publish_lien_quan {
        ($uri:expr) => {
            for u in host.uri_lien_quan($uri) {
                if let Some(f) = host.chan_doan_tai_lieu(&u) {
                    gui!(f);
                }
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
                let root = jsonrpc::lay_params(kv)
                    .and_then(|p| match p {
                        Value::Map(p) => jsonrpc::lay(p, "rootUri"),
                        _ => None,
                    })
                    .and_then(jsonrpc::chuoi)
                    .and_then(uri_thanh_duong)
                    .map(std::path::PathBuf::from);
                host.dat_root(root);
                if let Some(id) = id {
                    gui!(jsonrpc::phan_hoi(&id, ket_qua_khoi_tao()));
                }
            }
            // Notification: KHÔNG frame (kể cả trước initialize).
            "initialized" => {}
            // **ST-68**: văn bản mở/đổi (full sync) ⇒ tính lại chẩn đoán + `publishDiagnostics`.
            "textDocument/didOpen" => {
                if let Some((uri, text)) = jsonrpc::van_ban_mo(kv) {
                    host.mo(&uri, &text);
                    if let Some(f) = host.chan_doan_tai_lieu(&uri) {
                        gui!(f);
                    }
                    publish_lien_quan!(&uri);
                }
            }
            "textDocument/didChange" => {
                if let Some((uri, thay_doi)) = jsonrpc::thay_doi_van_ban(kv) {
                    // Tài liệu chưa mở: bỏ qua (không tự tạo buffer từ chênh lệch).
                    if let Some(cu) = host.van_ban(&uri).map(str::to_string) {
                        match sua::ap_dung(&cu, &thay_doi) {
                            // Không đổi byte (change rỗng) ⇒ không publish thừa.
                            Ok(moi) if moi == cu => {}
                            Ok(moi) => {
                                host.mo(&uri, &moi);
                                if let Some(f) = host.chan_doan_tai_lieu(&uri) {
                                    gui!(f);
                                }
                                publish_lien_quan!(&uri);
                            }
                            // Range hỏng/ngược: bỏ qua sự kiện, giữ buffer cũ (không panic).
                            Err(_) => {}
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = jsonrpc::uri_cua(kv) {
                    // **ST-100**: dọn cache phân tích/module/tệp phụ của host này; URI khác vẫn
                    // dùng lại bình thường; overlay buffer của tài liệu vừa đóng được bỏ.
                    host.dong(&uri);
                    gui!(host.chan_doan_dong(&uri));
                    // **VS3-7**: tài liệu khác đang mở phụ thuộc tệp vừa đóng ⇒ đọc lại từ đĩa.
                    publish_lien_quan!(&uri);
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
                        match host.phan_tich(&uri) {
                            // Chưa mở ⇒ nội dung rỗng ⇒ `[]` (không lỗi).
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
                                let byte = vi_tri::byte_tu_vi_tri(&kq.text, line, ch);
                                gui!(dinh_nghia::tra_loi_definition(
                                    &kq,
                                    &uri,
                                    byte,
                                    host.root(),
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
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
                        match (ten_moi, host.phan_tich(&uri)) {
                            (Some(ten_moi), Some(kq)) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
                                gui!(codelens::tra_loi_code_lens(&kq, &uri, &id));
                            }
                        }
                    }
                }
            }
            // **VD-270**: token ngữ nghĩa toàn tệp (legend tối thiểu, UTF-16).
            "textDocument/semanticTokens/full" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match host.phan_tich(&uri) {
                            // Không mở/URI lạ ⇒ lỗi chuẩn (InvalidParams), không đoán nội dung.
                            None => gui!(jsonrpc::loi(
                                &id,
                                -32602,
                                "tài liệu chưa mở",
                                Value::Str(uri),
                            )),
                            Some(kq) => {
                                gui!(semantic_tokens::tra_loi(&kq, &id));
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
                        match host.phan_tich(&uri) {
                            // Tệp chưa mở ⇒ `null` (khác các method theo vị trí trả `[]`).
                            None => gui!(jsonrpc::phan_hoi(&id, Value::none())),
                            Some(kq) => {
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
                        gui!(workspace::tra_loi_symbol(host.root(), query, &id));
                    }
                }
            }
            // **VD-270**: code action toàn tài liệu — hiện chỉ `source.fixAll` = bản đã format.
            // `range`/`context` bị bỏ qua có tài liệu hoá; URI chưa mở ⇒ `-32602` như semanticTokens.
            "textDocument/codeAction" => {
                if let Some(id) = id {
                    if !da_khoi_tao {
                        gui!(jsonrpc::loi(
                            &id,
                            -32002,
                            "chưa `initialize`",
                            Value::none()
                        ));
                    } else if let Some(uri) = jsonrpc::uri_cua(kv) {
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::loi(
                                &id,
                                -32602,
                                "tài liệu chưa mở",
                                Value::Str(uri),
                            )),
                            Some(kq) => {
                                gui!(code_action::tra_loi(&kq, &uri, mode_cua(&uri), &id));
                            }
                        }
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
                        match host.phan_tich(&uri) {
                            None => gui!(jsonrpc::phan_hoi(&id, Value::list(Vec::new()))),
                            Some(kq) => {
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
