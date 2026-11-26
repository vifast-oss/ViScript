// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-67/68/69 (VD-102)** — `vis lsp --stdio`: khung LSP + JSON-RPC 2.0 + dispatch.
//!
//! `initialize` (capabilities cố định), `initialized` (notification, không frame), `shutdown`
//! (`result:null`), `exit` (0 nếu đã shutdown, 1 nếu chưa), method lạ `-32601`, body JSON hỏng
//! `-32700`, request trước `initialize` `-32002`; `didOpen/didChange/didClose` ⇒ publishDiagnostics
//! (ST-68); **VS3-7 (VD-271)**: tệp module phụ đổi/mở/đóng ⇒ publish lại cho **mọi tài liệu đang mở
//! phụ thuộc** (buffer mở làm overlay cho module), mỗi URI tối đa một frame/sự kiện;
//! `textDocumentSync = {openClose:true, change:2}` — `didChange` áp **incremental** (range UTF-16,
//! nhiều thay đổi tuần tự, giữ CRLF) hoặc full khi không có range.
//! `textDocument/completion` + `textDocument/hover` (ST-69).
//!
//! stdout **tuyệt đối sạch**: chỉ frame; mọi lỗi giao thức ghi ra stderr.

#![forbid(unsafe_code)]

// **ST-310f**: gom tệp theo miền nhưng giữ **nguyên tên module phẳng** (`super::jsonrpc`, …)
// để không đổi hành vi/đường dẫn API nội bộ; chỉ đổi vị trí tệp bằng `#[path]`.
// `protocol/` — JSON-RPC, khung LSP, vị trí/UTF-16, sửa văn bản, URI.
#[path = "protocol/jsonrpc.rs"]
mod jsonrpc;
#[path = "protocol/khung.rs"]
mod khung;
#[path = "protocol/phuong_loi.rs"]
mod phuong_loi;
#[path = "protocol/sua.rs"]
mod sua;
#[path = "protocol/vi_tri.rs"]
mod vi_tri;

// `analysis/` — mô hình phân tích, module/overlay, chẩn đoán, thành viên, workspace.
#[path = "analysis/chan_doan.rs"]
mod chan_doan;
/// **ST-317c (#81/#66)** — trạng thái phiên LSP tiêm được (`AnalysisHost`), không `thread_local!`.
#[path = "analysis/host.rs"]
mod host;
#[path = "analysis/module.rs"]
mod module;
#[path = "analysis/phan_tich.rs"]
mod phan_tich;
#[path = "analysis/thanh_vien.rs"]
mod thanh_vien;
#[path = "analysis/workspace.rs"]
mod workspace;

// `features/` — từng tính năng LSP (completion, hover, rename, format, …).
#[path = "features/chon.rs"]
mod chon;
#[path = "features/chu_ky.rs"]
mod chu_ky;
#[path = "features/code_action.rs"]
mod code_action;
#[path = "features/codelens.rs"]
mod codelens;
#[path = "features/dinh_dang.rs"]
mod dinh_dang;
#[path = "features/dinh_nghia.rs"]
mod dinh_nghia;
#[path = "features/doi_ten.rs"]
mod doi_ten;
#[path = "features/gap.rs"]
mod gap;
#[path = "features/goi_y.rs"]
mod goi_y;
#[path = "features/inlay_hint.rs"]
mod inlay_hint;
#[path = "features/ky_hieu.rs"]
mod ky_hieu;
#[path = "features/semantic_tokens.rs"]
mod semantic_tokens;
#[path = "features/thuc_thi.rs"]
mod thuc_thi;
#[path = "features/type_definition.rs"]
mod type_definition;

use std::io::{self, BufReader};
use std::process::ExitCode;

use vis_core::json;
use vis_core::value::Value;
use vis_syntax::Mode;

pub use host::{AnalysisHost, ThongKeCache};

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
                    let f = host.dong(&uri).map(|_| host.chan_doan_dong(&uri));
                    if let Some(f) = f {
                        gui!(f);
                    }
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

/// Kết quả `initialize` — **cố định** (không version/rootUri) để snapshot tất định.
fn ket_qua_khoi_tao() -> Value {
    let completion = Value::map(vec![jsonrpc::kv(
        "triggerCharacters",
        Value::list(vec![Value::Str(".".to_string())]),
    )]);
    let capabilities = Value::map(vec![
        jsonrpc::kv(
            "textDocumentSync",
            jsonrpc::m(vec![
                ("openClose", Value::Bool(true)),
                ("change", Value::Int(2)),
            ]),
        ),
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
        jsonrpc::kv("codeActionProvider", code_action::provider()),
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
                Value::list(vec![Value::Str("(".into()), Value::Str(",".into())]),
            )]),
        ),
        jsonrpc::kv("completionProvider", completion),
        jsonrpc::kv("semanticTokensProvider", semantic_tokens::provider()),
    ]);
    let server_info = Value::map(vec![jsonrpc::kv("name", Value::Str("vis".to_string()))]);
    Value::map(vec![
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
    // **ST-297**: trên Windows `\` là dấu phân cách; chuẩn hoá **trước** khi nhận diện
    // drive/UNC để mixed slash (`C:/a\b`, `%5C`) về cùng một dạng. Trên POSIX `\` là ký tự
    // thường nên KHÔNG đổi.
    let rest = if windows {
        rest.replace('\\', "/")
    } else {
        rest
    };
    // **VD-241 § fix(Windows/CRLF #psvn-A5)**: LSP trên Windows gửi `file:///c:/path` (3 slash)
    // hoặc `file:///c:\path`; chuẩn hoá về `c:/...` khi phát hiện drive letter.
    fn trim_win_drive(r: &str) -> Option<String> {
        let b = r.as_bytes();
        // `/c:/path` → `c:/path`
        if b.len() >= 3
            && b[0] == b'/'
            && b[1].is_ascii_alphabetic()
            && b[2] == b':'
            && (b.len() == 3 || b[3] == b'/')
        {
            return Some(r[1..].to_string());
        }
        // `c:/path` (no leading slash) → `c:/path`
        if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/' {
            return Some(r.to_string());
        }
        None
    }
    if windows {
        if let Some(r) = trim_win_drive(&rest) {
            return Some(r);
        }
        // `file:////server/share` (path `//server/share`) giữ nguyên.
        if rest.starts_with("//") {
            return Some(rest);
        }
        // `file://server/share/a` — authority là UNC server; LSP file URI không có
        // authority nghĩa là path tương đối, nhưng trên Windows đây là UNC hợp lệ.
        if !rest.starts_with('/') {
            if rest.is_empty() || !rest.contains('/') {
                return None;
            }
            return Some(format!("//{rest}"));
        }
        // `file:///etc/x` — đường dẫn tuyệt đối kiểu POSIX/Windows rooted.
        Some(rest)
    } else {
        // Non-Windows: authority (`file://server/...`) không phải path cục bộ; `file:////…`
        // cho path `//…` vẫn giữ.
        rest.starts_with('/').then_some(rest)
    }
}

/// **ST-297** — path → URI `file:` chuẩn (percent-encode). `windows` chỉ đổi cách nhận
/// drive/UNC, không phụ thuộc nền tảng host nên test được trên Linux.
pub(crate) fn duong_thanh_uri(p: &std::path::Path, windows: bool) -> String {
    fn ma_hoa(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for &b in s.as_bytes() {
            match b {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'/'
                | b':' => out.push(b as char),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    let s = p.to_string_lossy();
    if windows {
        let s = s.replace('\\', "/");
        if let Some(rest) = s.strip_prefix("//") {
            // UNC: `file://server/share/...` (authority), không phải `file:////`.
            return format!("file://{}", ma_hoa(rest));
        }
        if let Some(rest) = s.strip_prefix('/') {
            return format!("file:///{}", ma_hoa(rest));
        }
        // Drive `C:/...` hoặc path tương đối (LSP luôn gửi absolute; giữ an toàn).
        return format!("file:///{}", ma_hoa(&s));
    }
    if let Some(rest) = s.strip_prefix('/') {
        format!("file:///{}", ma_hoa(rest))
    } else {
        format!("file:///{}", ma_hoa(&s))
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
    use super::{percent_giai_ma, uri_thanh_duong_voi};

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
        // **ST-297**: authority `file://server/share` là UNC trên Windows (trước đây None).
        assert_eq!(
            uri_thanh_duong_voi("file://relative/x", true).as_deref(),
            Some("//relative/x")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///etc/x", true).as_deref(),
            Some("/etc/x")
        );
    }

    #[test]
    fn uri_st297_unc_mixed_percent_roundtrip() {
        use super::{duong_thanh_uri, uri_thanh_duong_voi};
        use std::path::Path;

        // URI → path: UNC authority, `file:////…`, mixed slash, percent UTF-8.
        assert_eq!(
            uri_thanh_duong_voi("file://server/share/a.vis", true).as_deref(),
            Some("//server/share/a.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:////server/share/a.vis", true).as_deref(),
            Some("//server/share/a.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file://server/share/a%20b/%C3%A9.vis", true).as_deref(),
            Some("//server/share/a b/é.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///C:/a\\b/c.vis", true).as_deref(),
            Some("C:/a/b/c.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///%43%3A%5Ca%5Cb.vis", true).as_deref(),
            Some("C:/a/b.vis")
        );
        // `%25`/`%23` giải mã đúng; `%` hỏng giữ nguyên.
        assert_eq!(
            uri_thanh_duong_voi("file:///C:/a%25b%23c.vis", true).as_deref(),
            Some("C:/a%b#c.vis")
        );
        // Non-Windows: authority không phải path cục bộ; `//…` vẫn giữ; `\` là ký tự thường.
        assert_eq!(
            uri_thanh_duong_voi("file://server/share/a.vis", false),
            None
        );
        assert_eq!(
            uri_thanh_duong_voi("file:////server/share/a.vis", false).as_deref(),
            Some("//server/share/a.vis")
        );
        assert_eq!(
            uri_thanh_duong_voi("file:///a%5Cb", false).as_deref(),
            Some("/a\\b")
        );

        // Path → URI: percent-encode space/UTF-8/#/%, giữ `/` và `:` drive.
        assert_eq!(
            duong_thanh_uri(Path::new("/tmp/a b/é.vis"), false),
            "file:///tmp/a%20b/%C3%A9.vis"
        );
        assert_eq!(
            duong_thanh_uri(Path::new("C:\\a b\\é.vis"), true),
            "file:///C:/a%20b/%C3%A9.vis"
        );
        assert_eq!(
            duong_thanh_uri(Path::new("\\\\server\\share\\a b.vis"), true),
            "file://server/share/a%20b.vis"
        );
        assert_eq!(
            duong_thanh_uri(Path::new("/tmp/a#b%c?.vis"), false),
            "file:///tmp/a%23b%25c%3F.vis"
        );

        // Round-trip các dạng chính.
        for (p, w) in [
            (Path::new("/tmp/a b/é.vis"), false),
            (Path::new("C:/a b/é.vis"), true),
            (Path::new("//server/share/a b.vis"), true),
        ] {
            let uri = duong_thanh_uri(p, w);
            let lai = uri_thanh_duong_voi(&uri, w);
            assert_eq!(
                lai.as_deref(),
                Some(p.to_string_lossy().as_ref()),
                "uri={uri}"
            );
        }
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
