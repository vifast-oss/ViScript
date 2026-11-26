// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-300b (VD-283)** — LSP test phần 2 (module-aware/completion/hover); tách khỏi `lsp.rs` (R1).

use super::*;
use std::path::Path;
use vis_core::value::Value;

fn mo_va_lay(d: &Path, f: &Path) -> Vec<(String, u32, u32)> {
    let uri = uri_f(f);
    let text = std::fs::read_to_string(f).unwrap_or_default();
    let vao = format!("{}{}", khung(&init()), khung(&did_open(&uri, &text)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "LSP exit: {err}");
    let _ = d;
    ds_lsp(&out)
}

const STD_OK: &str =
    "use std.str\n\nfn main():\n    let xs = str.split(\"a,b\", \",\")\n    print(xs)\n";
const STD_SAI: &str =
    "use std.str\n\nfn main():\n    let xs = str.khong_co(\"a,b\")\n    print(xs)\n";
const UTIL_OK: &str = "use ./util\n\nfn main():\n    print(f())\n";
const UTIL_THIEU: &str = "use ./khong_co\n\nfn main():\n    print(1)\n";

/// (a) `use std.str` + `str.split` hợp lệ ⇒ **không** chẩn đoán giả.
#[test]
fn lsp_module_std_hop_le_khong_bao_gia() {
    let d = thu_muc("std-ok");
    let f = ghi(&d, "a.vis", STD_OK);
    let ds = mo_va_lay(&d, &f);
    assert!(ds.is_empty(), "LSP báo giả: {ds:?}");
    let (c, dc) = ds_check(&f);
    assert_eq!(c, 0, "vis check phải xanh: {dc:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) `str.khong_co` ⇒ chẩn đoán **thật** giống `vis check`.
#[test]
fn lsp_module_std_sai_giong_vis_check() {
    let d = thu_muc("std-sai");
    let f = ghi(&d, "a.vis", STD_SAI);
    let ds = mo_va_lay(&d, &f);
    let (c, dc) = ds_check(&f);
    assert_eq!(c, 1, "vis check phải đỏ: {dc:?}");
    assert!(!dc.is_empty(), "vis check phải có chẩn đoán");
    assert_eq!(ds, dc, "LSP khác vis check");
    assert_eq!(ds[0].0, "VS0301", "{ds:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) `use ./util` hợp lệ ⇒ `[]`.
#[test]
fn lsp_module_util_hop_le_sach() {
    let d = thu_muc("util-ok");
    ghi(&d, "util.vis", "pub fn f() -> int:\n    return 1\n");
    let f = ghi(&d, "b.vis", UTIL_OK);
    let ds = mo_va_lay(&d, &f);
    assert!(ds.is_empty(), "LSP báo giả: {ds:?}");
    let (c, dc) = ds_check(&f);
    assert_eq!(c, 0, "vis check phải xanh: {dc:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) `use ./util` **không tồn tại** ⇒ `VS0701` ở **tệp chính**, giống `vis check`.
#[test]
fn lsp_module_thieu_giong_vis_check() {
    let d = thu_muc("util-thieu");
    let f = ghi(&d, "c.vis", UTIL_THIEU);
    let ds = mo_va_lay(&d, &f);
    let (c, dc) = ds_check(&f);
    assert_eq!(c, 1, "vis check phải đỏ: {dc:?}");
    assert!(
        ds.iter().any(|x| x.0 == "VS0701"),
        "LSP thiếu VS0701: {ds:?}"
    );
    assert_eq!(ds, dc, "LSP khác vis check");
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) Tệp **không** có trên đĩa ⇒ đường một tệp như trước, **không** bịa lỗi module.
#[test]
fn lsp_khong_co_tren_dia_khong_bia_loi_module() {
    let uri = "file:///tmp/vis-khong-ton-tai-st80/a.vis";
    let vao = format!(
        "{}{}",
        khung(&init()),
        khung(&did_open(
            uri,
            "use ./khong_co\n\nfn main():\n    print(1)\n"
        ))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "LSP exit: {err}");
    let ds = ds_lsp(&out);
    assert!(ds.is_empty(), "không được bịa lỗi module: {ds:?}");
}

/// (f) **So trực tiếp** 3 tệp mẫu: bộ `(mã, dòng, cột-1)` LSP **bằng** `vis check` (bỏ thứ tự).
#[test]
fn lsp_ba_tep_bang_vis_check() {
    let d = thu_muc("so");
    ghi(&d, "util.vis", "pub fn f() -> int:\n    return 1\n");
    let a = ghi(&d, "a.vis", STD_SAI);
    let b = ghi(&d, "b.vis", "use ./util\n\nfn main():\n    print(f())\n");
    let c = ghi(&d, "c.vis", UTIL_THIEU);
    for f in [&a, &b, &c] {
        let ds = mo_va_lay(&d, f);
        let (_, dc) = ds_check(f);
        assert_eq!(ds, dc, "lệch ở {}", f.display());
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// 5 tệp nhỏ, mỗi tệp `didOpen` (module-aware): mọi publish đều rỗng, exit 0, đủ **5** frame.
/// (Đo thời gian nạp module nằm ở unit test `lsp::module::tests::hieu_nang_nap_module_5_tep` —
/// không tính spawn tiến trình nên ổn định khi harness chạy //.)
#[test]
fn lsp_nam_tep_module_hoat_dong() {
    let d = thu_muc("perf");
    let mut vao = khung(&init());
    for i in 0..5 {
        let f = ghi(&d, &format!("t{i}.vis"), STD_OK);
        let uri = uri_f(&f);
        vao.push_str(&khung(&did_open(&uri, STD_OK)));
    }
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "LSP exit: {err}");
    assert_eq!(
        out.matches("textDocument/publishDiagnostics").count(),
        5,
        "phải publish 5 lần"
    );
    assert!(!out.contains("VS"), "không được có chẩn đoán: {out}");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------------------
// **ST-84 (VD-109)** — completion/hover **module-aware** (tệp chính có trên đĩa).
// ---------------------------------------------------------------------------

const UTIL: &str = "pub fn tinh(x: int) -> int:\n    return x + 1\n";
const MAIN_USE: &str = "use ./util\n\nfn main():\n    ti\n";

fn mo_uri(f: &Path, text: &str) -> String {
    let uri = uri_f(f);
    format!("{}{}", khung(&init()), khung(&did_open(&uri, text)))
}

/// (a) Completion thấy `tinh` từ `./util` — đúng 1 item, kind 3, detail có `util`.
#[test]
fn lsp_completion_thay_ham_module() {
    let d = thu_muc("mod-completion");
    ghi(&d, "util.vis", UTIL);
    let f = ghi(&d, "main.vis", MAIN_USE);
    let uri = uri_f(&f);
    let vao = format!(
        "{}{}",
        mo_uri(&f, MAIN_USE),
        khung(&completion(2, &uri, 3, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "{err}");
    let body = frame_cuoi(&out);
    assert_eq!(body.matches("\"label\":\"tinh\"").count(), 1, "{body}");
    assert!(
        body.contains("\"kind\":3") && body.contains("util: fn tinh(x: int) -> int"),
        "{body}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) Hover `tinh` ⇒ markdown chữ ký (có nguồn `util`).
#[test]
fn lsp_hover_ham_module() {
    let d = thu_muc("mod-hover");
    ghi(&d, "util.vis", UTIL);
    let src = "use ./util\n\nfn main():\n    print(tinh(1))\n";
    let f = ghi(&d, "main.vis", src);
    let uri = uri_f(&f);
    let vao = format!("{}{}", mo_uri(&f, src), khung(&hover(2, &uri, 3, 10)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "{err}");
    let body = frame_cuoi(&out);
    assert!(body.contains("util: fn tinh(x: int) -> int"), "{body}");
    assert!(body.contains("markdown"), "{body}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) `use std.str` ⇒ completion tiền tố `sp` thấy `split` (detail `std.str:`).
#[test]
fn lsp_completion_thay_std_da_use() {
    let d = thu_muc("mod-std");
    let src = "use std.str\n\nfn main():\n    sp\n";
    let f = ghi(&d, "main.vis", src);
    let uri = uri_f(&f);
    let vao = format!("{}{}", mo_uri(&f, src), khung(&completion(2, &uri, 3, 6)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "{err}");
    let body = frame_cuoi(&out);
    assert!(body.contains("\"label\":\"split\""), "{body}");
    assert!(body.contains("std.str: fn split"), "{body}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) Tệp **không** có trên đĩa ⇒ như cũ: **không** bịa ký hiệu module.
#[test]
fn lsp_completion_khong_co_dia_khong_bia_module() {
    let uri = "file:///tmp/vis-st84-khong-ton-tai/main.vis";
    let vao = format!("{}{}", khung(&init()), khung(&did_open(uri, MAIN_USE)));
    let vao = format!("{}{}", vao, khung(&completion(2, uri, 3, 6)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "{err}");
    let body = frame_cuoi(&out);
    assert!(!body.contains("tinh"), "không được bịa: {body}");
}

/// (e) Khử trùng: tệp chính có `fn tinh` + `./util` cũng có ⇒ **1** item (của tệp chính).
#[test]
fn lsp_completion_khu_trung_uu_tien_tep_chinh() {
    let d = thu_muc("mod-dedup");
    ghi(&d, "util.vis", UTIL);
    let src = "use ./util\n\nfn tinh(x: int) -> int:\n    return x + 2\n\nfn main():\n    ti\n";
    let f = ghi(&d, "main.vis", src);
    let uri = uri_f(&f);
    let vao = format!("{}{}", mo_uri(&f, src), khung(&completion(2, &uri, 6, 6)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "{err}");
    let body = frame_cuoi(&out);
    assert_eq!(body.matches("\"label\":\"tinh\"").count(), 1, "{body}");
    assert!(
        body.contains("\"detail\":\"fn tinh(x: int) -> int\""),
        "{body}"
    );
    assert!(!body.contains("util:"), "phải ưu tiên tệp chính: {body}");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------------------
// **ST-91 (VD-110)** — completion sau `.` theo kiểu tĩnh
// ---------------------------------------------------------------------------

/// Chạy một completion và trả `(label, kind, detail)` của `result`.
fn goi_y(text: &str, line: i64, ch: i64) -> Vec<(String, u8, Option<String>)> {
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&completion(2, URI, line, ch))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    let v = vis_core::json::decode(&body).expect("JSON frame cuối");
    let Value::Map(kv) = &v else {
        panic!("frame không phải object")
    };
    let Some(Value::List(items)) = lay(kv, "result") else {
        panic!("thiếu result: {body}")
    };
    items
        .iter()
        .map(|it| {
            let Value::Map(k) = it else {
                panic!("item không phải object")
            };
            let label = match lay(k, "label") {
                Some(Value::Str(s)) => s.clone(),
                _ => panic!("thiếu label"),
            };
            let kind = match lay(k, "kind") {
                Some(Value::Int(n)) => *n as u8,
                _ => panic!("thiếu kind"),
            };
            let detail = match lay(k, "detail") {
                Some(Value::Str(s)) => Some(s.clone()),
                _ => None,
            };
            (label, kind, detail)
        })
        .collect()
}

/// (a) `p.` với `p: P` ⇒ **đúng** trường `ten`, `x` (kind 5), không lẫn fn/từ khoá.
#[test]
fn lsp_completion_sau_cham_dung_truong() {
    let text = "type P:\n    x: int\n    ten: str\nfn main():\n    let p: P = {x: 1, ten: \"a\"}\n    p.\n";
    let ds = goi_y(text, 5, 6);
    let labels: Vec<&str> = ds.iter().map(|(l, _, _)| l.as_str()).collect();
    assert_eq!(labels, ["ten", "x"], "chỉ trường của P: {ds:?}");
    assert!(ds.iter().all(|(_, k, _)| *k == 5), "kind field 5: {ds:?}");
    assert!(
        !labels
            .iter()
            .any(|l| ["print", "let", "fn", "match"].contains(l)),
        "không lẫn từ khoá/hàm: {ds:?}"
    );
    assert!(
        ds.iter()
            .any(|(l, _, d)| l == "x" && d.as_deref() == Some("x: int")),
        "detail kiểu trường: {ds:?}"
    );
}

/// (b) `impl Show for P` ⇒ `p.` có thêm `show` (kind 2), vẫn có trường.
#[test]
fn lsp_completion_sau_cham_co_phuong_thuc() {
    let text = "type P:\n    x: int\n    ten: str\nimpl Show for P:\n    fn show(self) -> str:\n        \"P\"\nfn main():\n    let p: P = {x: 1, ten: \"a\"}\n    p.\n";
    let ds = goi_y(text, 8, 6);
    assert!(
        ds.iter().any(|(l, k, _)| l == "show" && *k == 2),
        "phải có phương thức show kind 2: {ds:?}"
    );
    assert!(ds.iter().any(|(l, _, _)| l == "x"), "vẫn có trường: {ds:?}");
}

/// (c) `s: str` (kiểu biết nhưng **không** có bảng thành viên) ⇒ **rỗng**, không fallback.
#[test]
fn lsp_completion_sau_cham_kieu_khong_ho_tro() {
    let text = "fn f(s: str):\n    s.\n";
    let ds = goi_y(text, 1, 6);
    assert!(ds.is_empty(), "str không có thành viên ⇒ rỗng: {ds:?}");
}

/// (d) biến **không rõ kiểu** (`foo()` chưa biết) ⇒ fallback **toàn bộ** như cũ (51 mục — VD-267).
#[test]
fn lsp_completion_sau_cham_khong_ro_kieu_fallback() {
    let text = "fn f():\n    let q = foo()\n    q.\n";
    let ds = goi_y(text, 2, 6);
    assert_eq!(ds.len(), 51, "fallback 51 mục: {:?}", ds.len());
    assert!(ds.iter().any(|(l, _, _)| l == "print"));
    assert!(ds.iter().any(|(l, _, _)| l == "let"));
}

/// (e) **tất định**: 5 lần cùng đầu vào ⇒ frame completion **byte y hệt**.
#[test]
fn lsp_completion_sau_cham_tat_dinh() {
    let text = "type P:\n    x: int\n    ten: str\nfn main():\n    let p: P = {x: 1, ten: \"a\"}\n    p.\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&completion(2, URI, 5, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    for _ in 0..4 {
        let (_, out2, _) = chay(&vao);
        assert_eq!(body, frame_cuoi(&out2), "5 lần phải cùng byte");
    }
}

// ===== ST-100: hover trường + didClose/cache =====

/// (a) hover `p.x` ⇒ `**x**: int` + “trường của `P`”.
#[test]
fn lsp_hover_truong_int() {
    let text = "type P:\n    x: int\n    ten: str\n\nfn f(p: P) -> int:\n    p.x\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&hover(2, URI, 5, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(body.contains(r#"**x**: int"#), "{body}");
    assert!(body.contains("trường của `P`"), "{body}");
    assert!(body.contains(r#""line":5,"character":6"#), "{body}");
}

/// (b) hover `p.ten` ⇒ `**ten**: str`.
#[test]
fn lsp_hover_truong_str() {
    let text = "type P:\n    x: int\n    ten: str\n\nfn f(p: P) -> int:\n    p.ten\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&hover(2, URI, 5, 6))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(body.contains(r#"**ten**: str"#), "{body}");
    assert!(body.contains("trường của `P`"), "{body}");
}

/// (c) `didClose` rồi `didOpen` lại ⇒ chẩn đoán **vẫn đúng** (cache không làm sai/không rò URI khác).
#[test]
fn lsp_close_roi_mo_lai_chan_doan() {
    let text = "fn main():\n    let x: int = \"s\"\n";
    let vao = format!(
        "{}{}{}{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&did_close(URI)),
        khung(&did_open(URI, text)),
        khung(&req(3, "shutdown")),
        khung(r#"{"jsonrpc":"2.0","method":"exit"}"#)
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        out.matches("VS0412").count(),
        2,
        "phải có 2 lần VS0412: {out}"
    );
    assert_eq!(out.matches(r#""diagnostics":[]"#).count(), 1, "{out}");
    assert!(out.contains(r#""line":1,"character":17"#), "{out}");
}

/// (d) **5 `didChange`** liên tiếp ⇒ hover trả **byte y hệt** lần đầu (cùng id).
#[test]
fn lsp_5_did_change_ket_qua_nhu_lan_dau() {
    let goc = "type P:\n    x: int\n\nfn f(p: P) -> int:\n    p.x\n";
    let vao0 = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, goc)),
        khung(&hover(2, URI, 4, 6))
    );
    let (_, out0, _) = chay(&vao0);
    let chuan = frame_cuoi(&out0);
    for k in 1..=5 {
        let text = format!("{goc}{}", "# doi\n".repeat(k));
        let vao = format!(
            "{}{}{}{}",
            khung(&init()),
            khung(&did_open(URI, goc)),
            khung(&did_change(URI, &text)),
            khung(&hover(2, URI, 4, 6))
        );
        let (code, out, err) = chay(&vao);
        assert_eq!(code, 0, "k={k}, stderr: {err}");
        assert_eq!(chuan, frame_cuoi(&out), "k={k}: hover phải y hệt lần đầu");
    }
}

/// Ca 23 — hover biến `x: int?` ⇒ markdown chứa `int?`, **không** `Option[int]` (VD-208 (4)).
#[test]
fn lsp_hover_option_in_int() {
    let text = "let x: int? = Some(5)\n\nx\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&hover(2, URI, 2, 1))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"contents\":{\"kind\":\"markdown\",\"value\":\"```vis\\nx: int?\\n```\"},\"range\":{\"start\":{\"line\":2,\"character\":0},\"end\":{\"line\":2,\"character\":1}}}}";
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
    assert!(
        !out.contains("Option[int]"),
        "hover không được in `Option[int]`: {out}"
    );
}

// ===== ST-269: go-to-definition =====

fn definition(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/definition", uri, line, ch)
}

/// Ca 24 — gọi hàm ⇒ nhảy tới **tên** trong khai báo `fn` (không phải cả khối).
#[test]
fn lsp_definition_ham() {
    let text = "fn add(a: int, b: int) -> int:\n    a + b\n\nfn main():\n    print(add(1, 2))\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&definition(2, URI, 4, 10))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"uri":"file:///a.vis","range":{"start":{"line":0,"character":3},"end":{"line":0,"character":6}}}]}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

/// Ca 25 — biến cục bộ ⇒ đúng span binder `let`.
#[test]
fn lsp_definition_bien_cuc_bo() {
    let text = "fn main():\n    let total = 1\n    print(total)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&definition(2, URI, 2, 10))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"uri":"file:///a.vis","range":{"start":{"line":1,"character":8},"end":{"line":1,"character":13}}}]}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

/// Ca 26 — trường bản ghi `u.f` ⇒ khai báo `f: int` trong `type`.
#[test]
fn lsp_definition_truong() {
    let text = "type U:\n    f: int\n\nfn main():\n    let u = U(f: 1)\n    print(u.f)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&definition(2, URI, 5, 12))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":[{"uri":"file:///a.vis","range":{"start":{"line":1,"character":4},"end":{"line":1,"character":5}}}]}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

/// Ca 27 — builtin `print` không có khai báo trong tệp ⇒ `result:null`.
#[test]
fn lsp_definition_khong_co_tra_null() {
    let text = "fn main():\n    print(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&definition(2, URI, 1, 5))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

// ===== ST-270: documentSymbol =====

fn document_symbol(id: i64, uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/documentSymbol")),
        (
            "params",
            m(vec![("textDocument", m(vec![("uri", s(uri))]))]),
        ),
    ]))
}

/// Ca 28 — cây ký hiệu: type + trường, enum + variant + trường, const, fn.
#[test]
fn lsp_document_symbol_cay() {
    let text = "type U:\n    f: int\n\nenum E:\n    A\n    B(x: int)\n\nconst C: int = 1\n\nfn add(a: int) -> int:\n    a\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&document_symbol(2, URI))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "frame documentSymbol: {body}"
    );
    for needle in [
        r#""name":"U","kind":23"#,
        r#""name":"f","kind":8"#,
        r#""name":"E","kind":10"#,
        r#""name":"A","kind":22"#,
        r#""name":"B","kind":22"#,
        r#""name":"x","kind":8"#,
        r#""name":"C","kind":14"#,
        r#""name":"add","kind":12"#,
    ] {
        assert!(body.contains(needle), "thiếu {needle} trong {body}");
    }
    assert!(
        body.contains(r#""children":[{"name":"f""#),
        "type U phải có con f: {body}"
    );
}

// ===== ST-271: references =====

fn references(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/references", uri, line, ch)
}

/// Ca 29 — `add` xuất hiện 2 lần (khai báo + gọi) ⇒ 2 `Location`.
#[test]
fn lsp_references_dem_hai() {
    let text = "fn add(a: int) -> int:\n    a\n\nfn main():\n    print(add(1))\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&references(2, URI, 4, 10))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "frame references: {body}"
    );
    assert_eq!(
        body.matches(r#""uri":"file:///a.vis""#).count(),
        2,
        "phải có đúng 2 tham chiếu: {body}"
    );
}

// ===== ST-272: rename =====

fn rename(id: i64, uri: &str, line: i64, ch: i64, ten_moi: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("textDocument/rename")),
        (
            "params",
            m(vec![
                ("textDocument", m(vec![("uri", s(uri))])),
                ("position", pos(line, ch)),
                ("newName", s(ten_moi)),
            ]),
        ),
    ]))
}

/// Ca 30 — đổi `add` → `sum`: 2 TextEdit (khai báo + gọi), không chạm chuỗi/chú thích.
#[test]
fn lsp_rename_hai_vi_tri() {
    let text = "fn add(a: int) -> int:\n    a\n\nfn main():\n    print(add(1))\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&rename(2, URI, 4, 10, "sum"))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert!(
        body.starts_with(r#"{"jsonrpc":"2.0","id":2,"result":{"changes":{"file:///a.vis":["#),
        "frame rename: {body}"
    );
    assert_eq!(
        body.matches(r#""newText":"sum""#).count(),
        2,
        "phải có đúng 2 edit: {body}"
    );
}

/// Ca 31 — tên mới không hợp lệ (`1x`) ⇒ `result:null`, không sửa gì.
#[test]
fn lsp_rename_ten_moi_sai_null() {
    let text = "fn add(a: int) -> int:\n    a\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&rename(2, URI, 0, 3, "1x"))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

// ===== ST-273: signatureHelp =====

fn signature(id: i64, uri: &str, line: i64, ch: i64) -> String {
    req_vi_tri(id, "textDocument/signatureHelp", uri, line, ch)
}

/// Ca 32 — trong lời gọi `add(1, ` ⇒ chữ ký `fn add(...)`, tham số hoạt động 1.
#[test]
fn lsp_signature_help_tham_so_thu_hai() {
    let text = "fn add(a: int, b: int) -> int:\n    a + b\n\nfn main():\n    let x = add(1, 2)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&signature(2, URI, 4, 18))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":{"signatures":[{"label":"fn add(a: int, b: int) -> int","parameters":[{"label":"a: int"},{"label":"b: int"}]}],"activeSignature":0,"activeParameter":1}}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

/// Ca 33 — builtin `print(` không có chữ ký trong tệp ⇒ `result:null`.
#[test]
fn lsp_signature_help_builtin_null() {
    let text = "fn main():\n    print(1)\n";
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(URI, text)),
        khung(&signature(2, URI, 1, 10))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let tra = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
    assert_eq!(
        out,
        format!("{}{}{}", khung(INIT), khung(&publish_rong(URI)), khung(tra))
    );
}

// ---------------------------------------------------------------------------
// **ST-316b (`#48`)** — completion module chỉ gợi ý item `pub` (mọi loại: fn/type/enum/alias/
// const/trait), không gợi ý item nội bộ của module khác.
// ---------------------------------------------------------------------------

const UTIL_PUB: &str = "\
pub fn cong(x: int) -> int:
    return x + 1

fn bi_mat_fn(x: int) -> int:
    return x + 2

pub type P:
    x: int

type Q:
    y: int

pub enum E:
    A
    B

enum F:
    C

pub trait Hien:
    fn hien(self) -> int

trait An:
    fn an(self) -> int

pub const HANG_PUB: int = 1

const HANG_KIN: int = 2

pub type TAPUB = int

type TAKIN = int
";

/// Chỉ item `pub` của `./util` xuất hiện trong completion; item không `pub` **không** lọt.
#[test]
fn lsp_completion_module_chi_item_pub() {
    let d = thu_muc("mod-pub");
    ghi(&d, "util.vis", UTIL_PUB);
    let src = "use ./util\n\nfn main():\n    print(1)\n";
    let f = ghi(&d, "main.vis", src);
    let uri = uri_f(&f);
    let vao = format!("{}{}", mo_uri(&f, src), khung(&completion(2, &uri, 3, 4)));
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "{err}");
    let body = frame_cuoi(&out);
    for ten in ["cong", "P", "E", "Hien", "HANG_PUB", "TAPUB"] {
        assert!(
            body.contains(&format!("\"label\":\"{ten}\"")),
            "thiếu item pub `{ten}`: {body}"
        );
    }
    for ten in ["bi_mat_fn", "Q", "F", "An", "HANG_KIN", "TAKIN"] {
        assert!(
            !body.contains(&format!("\"label\":\"{ten}\"")),
            "item không `pub` `{ten}` không được gợi ý: {body}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}
