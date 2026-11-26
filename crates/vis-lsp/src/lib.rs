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
//!
//! **ST-317b**: tách `lib.rs` (1.056 dòng) — vòng lặp/dispatch sang `server/run.rs`, kết quả
//! `initialize` sang `server/khoi_tao.rs`, chuyển đổi URI + `mode_cua` sang `server/uri.rs`.
//! `lib.rs` chỉ còn public surface (`run`, `AnalysisHost`, `ThongKeCache`) + mod + test URI.

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

// `server/` — vòng lặp stdio + dispatch JSON-RPC + URI/initialize (ST-317b).
mod server;

pub use host::{AnalysisHost, ThongKeCache};
pub use server::run;

// Giữ đường `super::uri_thanh_duong`/`super::duong_thanh_uri`/`super::mode_cua` của `analysis/*`
// (ST-310f) — chỉ đổi nơi định nghĩa, không đổi tên.
pub(crate) use server::uri::{duong_thanh_uri, mode_cua, uri_thanh_duong};

#[cfg(test)]
mod vd258_tests {
    use crate::server::uri::{duong_thanh_uri, percent_giai_ma, uri_thanh_duong_voi};
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
