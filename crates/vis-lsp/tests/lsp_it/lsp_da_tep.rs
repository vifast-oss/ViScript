// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-117** — LSP `publishDiagnostics` đa tệp: chỉ công bố chẩn đoán của **tài liệu đang mở**,
//! đúng tệp/dòng/cột. Hành vi đã đúng từ ST-114b (file-id + map `rel`); bộ này **chỉ thêm ca**.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-lsp-tep-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi");
}

fn uri(d: &Path, ten: &str) -> String {
    // **VD-241 § fix(Windows/CRLF #psvn-A5)**: on Windows, LSP URIs use forward slashes:
    //   `C:\Users\ps\foo.vis` → `file:///C:/Users/ps/foo.vis`
    // Written WITHOUT `#[cfg]` so every platform type-checks the same code (tránh lỗi
    // "biên dịch được trên Linux nhưng gãy trên Windows" — psvn bắt được ở ba11a90).
    let p = d.join(ten).to_string_lossy().to_string();
    if std::env::consts::OS == "windows" {
        format!("file:///{}", p.replace('\\', "/"))
    } else {
        format!("file://{}", p)
    }
}

fn khung(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn init() -> String {
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string()
}

fn did_open(u: &str, text: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{u}","languageId":"viscript","version":1,"text":"{}"}}}}}}"#,
        esc(text)
    )
}

fn did_change_v(u: &str, text: &str, version: i64) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{u}","version":{version}}},"contentChanges":[{{"text":"{}"}}]}}}}"#,
        esc(text)
    )
}

fn did_change(u: &str, text: &str) -> String {
    did_change_v(u, text, 2)
}

#[allow(clippy::too_many_arguments)]
fn did_change_range(
    u: &str,
    version: i64,
    line: u32,
    ch: u32,
    end_line: u32,
    end_ch: u32,
    text: &str,
) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{u}","version":{version}}},"contentChanges":[{{"range":{{"start":{{"line":{line},"character":{ch}}},"end":{{"line":{end_line},"character":{end_ch}}}}},"text":"{}"}}]}}}}"#,
        esc(text)
    )
}

fn did_close(u: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{u}"}}}}}}"#
    )
}

fn hover(id: i64, u: &str, line: u32, ch: u32) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{u}"}},"position":{{"line":{line},"character":{ch}}}}}}}"#
    )
}

/// URI của các frame `publishDiagnostics`, theo đúng thứ tự xuất hiện.
fn cac_publish(out: &str) -> Vec<String> {
    let mut res = Vec::new();
    let mut i = 0usize;
    while let Some(j) = out[i..].find(r#""method":"textDocument/publishDiagnostics""#) {
        let k = i + j;
        let Some(a) = out[k..].find(r#""uri":""#) else {
            break;
        };
        let start = k + a + r#""uri":""#.len();
        let Some(b) = out[start..].find('"') else {
            break;
        };
        res.push(out[start..start + b].to_string());
        i = start + b;
    }
    res
}

/// Body (bỏ header) của frame phản hồi có `id` — `""` nếu không thấy.
fn frame_id(out: &str, id: i64) -> String {
    for phan in out.split("Content-Length: ").skip(1) {
        let Some(i) = phan.find("\r\n\r\n") else {
            continue;
        };
        let body = &phan[i + 4..];
        if body.starts_with(&format!(r#"{{"jsonrpc":"2.0","id":{id},"#)) {
            return body.to_string();
        }
    }
    String::new()
}

/// Frame publish **cuối** cho `uri` (body JSON).
fn publish_cuoi(out: &str, uri: &str) -> String {
    let mut cuoi = String::new();
    for phan in out.split("Content-Length: ").skip(1) {
        let Some(i) = phan.find("\r\n\r\n") else {
            continue;
        };
        let body = &phan[i + 4..];
        if body.contains("publishDiagnostics") && body.contains(&format!(r#""uri":"{uri}""#)) {
            cuoi = body.to_string();
        }
    }
    cuoi
}

/// Chạy `vis lsp --stdio`, bơm frame, deadline 10 s.
fn chay(dau_vao: &str) -> String {
    let mut con = Command::new(crate::common::vis_bin())
        .args(["lsp", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lsp");
    con.stdin
        .take()
        .expect("stdin")
        .write_all(dau_vao.as_bytes())
        .expect("bơm stdin");
    let han = Instant::now() + Duration::from_secs(10);
    loop {
        if con.try_wait().expect("try_wait").is_some() {
            break;
        }
        if Instant::now() > han {
            let _ = con.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let o = con.wait_with_output().expect("output");
    assert_eq!(
        o.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.stderr.is_empty());
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn dem_ca(out: &str) -> usize {
    out.matches(r#""code":"VS0412""#).count()
}

/// (a) mở a.vis, **cả hai** có lỗi ⇒ chỉ publish lỗi **a** (không lẫn b, không rỗng).
#[test]
fn mo_a_chi_chan_doan_a() {
    let d = tam("a");
    ghi(&d, "a.vis", "use ./b\nlet x: int = \"s\"\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n    return \"s\"\n");
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let out = chay(&format!(
        "{}{}",
        khung(&init()),
        khung(&did_open(&ua, "use ./b\nlet x: int = \"s\"\n"))
    ));
    assert!(out.contains(&format!(r#""uri":"{ua}""#)), "{out}");
    assert!(!out.contains(&ub), "không được publish b: {out}");
    assert_eq!(dem_ca(&out), 1, "đúng 1 lỗi a: {out}");
    assert!(out.contains(r#""line":1,"character":13"#), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) mở b.vis (module) ⇒ publish lỗi **chính b**, đúng dòng/cột; không lỗi a.
#[test]
fn mo_b_chan_doan_b() {
    let d = tam("b");
    ghi(&d, "a.vis", "use ./b\nlet x: int = \"s\"\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n    return \"s\"\n");
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let out = chay(&format!(
        "{}{}",
        khung(&init()),
        khung(&did_open(&ub, "pub fn f() -> int:\n\n\n    return \"s\"\n"))
    ));
    assert!(out.contains(&format!(r#""uri":"{ub}""#)), "{out}");
    assert!(!out.contains(&ua), "không được publish a: {out}");
    assert_eq!(dem_ca(&out), 1, "đúng 1 lỗi b: {out}");
    assert!(out.contains(r#""line":3,"character":4"#), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) a sạch, b lỗi ⇒ mở a publish `[]`; mở b có lỗi.
#[test]
fn a_sach_rong_b_co_loi() {
    let d = tam("c");
    ghi(&d, "a.vis", "use ./b\nprint(1)\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n    return \"s\"\n");
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let out_a = chay(&format!(
        "{}{}",
        khung(&init()),
        khung(&did_open(&ua, "use ./b\nprint(1)\n"))
    ));
    assert!(
        out_a.contains(&format!(r#""uri":"{ua}","diagnostics":[]"#)),
        "{out_a}"
    );
    assert_eq!(dem_ca(&out_a), 0, "{out_a}");
    let out_b = chay(&format!(
        "{}{}",
        khung(&init()),
        khung(&did_open(&ub, "pub fn f() -> int:\n\n\n    return \"s\"\n"))
    ));
    assert_eq!(dem_ca(&out_b), 1, "{out_b}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-7** — mở a + b, sửa **b** (buffer chưa ghi đĩa) ⇒ chẩn đoán **a** cập nhật đúng vị trí;
/// mỗi sự kiện publish mỗi URI đúng **một** lần (không trùng); hover sau khi đổi b thấy chữ ký mới
/// (cache module bị xoá đúng lúc).
#[test]
fn sua_b_cap_nhat_chan_doan_a_va_cache() {
    let d = tam("sua-b");
    let a_text = "use ./b\nlet x: int = f()\n";
    let b_str = "pub fn f() -> str:\n    return \"s\"\n";
    let b_int = "pub fn f() -> int:\n    return 1\n";
    ghi(&d, "a.vis", a_text);
    // Đĩa giữ bản `str`; didChange chỉ đổi **buffer** (overlay VS3-7).
    ghi(&d, "b.vis", b_str);
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let vao = format!(
        "{}{}{}{}{}{}{}",
        khung(&init()),
        khung(&did_open(&ua, a_text)),
        khung(&did_open(&ub, b_str)),
        khung(&did_change_v(&ub, b_int, 2)),
        khung(&hover(2, &ua, 1, 13)),
        khung(&did_change_v(&ub, b_str, 3)),
        khung(&hover(3, &ua, 1, 13))
    );
    let out = chay(&vao);

    let ds = cac_publish(&out);
    assert_eq!(
        ds,
        vec![
            ua.clone(),
            ub.clone(),
            ua.clone(),
            ub.clone(),
            ua.clone(),
            ub.clone(),
            ua.clone()
        ],
        "mỗi sự kiện: tài liệu đổi trước, tài liệu liên quan sau, không trùng: {out}"
    );
    assert_eq!(dem_ca(&out), 3, "lỗi a: mở a, mở b, đổi b về str: {out}");
    assert!(
        publish_cuoi(&out, &ua).contains(r#""line":1,"character":13"#),
        "vị trí lỗi a phải 1:13: {out}"
    );

    let h2 = frame_id(&out, 2);
    assert!(
        h2.contains("b: fn f() -> int"),
        "hover sau đổi b phải thấy chữ ký mới: {h2}"
    );
    let h3 = frame_id(&out, 3);
    assert!(
        h3.contains("b: fn f() -> str"),
        "hover sau đổi lại phải thấy str (cache đã xoá): {h3}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-7** — mở b với buffer **khác đĩa**, rồi `didClose` ⇒ b publish rỗng; a quay lại đọc đĩa
/// và publish lại lỗi đúng vị trí.
#[test]
fn dong_b_cap_nhat_a() {
    let d = tam("dong-b");
    let a_text = "use ./b\nlet x: int = f()\n";
    let b_str = "pub fn f() -> str:\n    return \"s\"\n";
    let b_int = "pub fn f() -> int:\n    return 1\n";
    ghi(&d, "a.vis", a_text);
    ghi(&d, "b.vis", b_str);
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(&ua, a_text)),
        khung(&did_open(&ub, b_int)),
        khung(&did_close(&ub))
    );
    let out = chay(&vao);
    assert_eq!(
        cac_publish(&out),
        vec![ua.clone(), ub.clone(), ua.clone(), ub.clone(), ua.clone()],
        "mở a, mở b, đóng b: mỗi URI một lần mỗi sự kiện: {out}"
    );
    assert_eq!(dem_ca(&out), 2, "a lỗi lúc mở a và sau khi đóng b: {out}");
    assert!(
        publish_cuoi(&out, &ub).contains(r#""diagnostics":[]"#),
        "đóng b phải xoá chẩn đoán b: {out}"
    );
    let a_cuoi = publish_cuoi(&out, &ua);
    assert!(
        a_cuoi.contains("VS0412") && a_cuoi.contains(r#""line":1,"character":13"#),
        "sau đóng b, a phải đọc đĩa (str) và báo lỗi 1:13: {a_cuoi}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-7** — sửa tệp **không liên quan** không publish lại a (không bắn chẩn đoán thừa).
#[test]
fn sua_tep_khong_lien_quan_khong_publish_a() {
    let d = tam("khong-lien-quan");
    let a_text = "use ./b\nprint(1)\n";
    let b_text = "pub fn f() -> int:\n    return 1\n";
    let c_text = "let y = 1\n";
    let c_moi = "let y = 2\n";
    ghi(&d, "a.vis", a_text);
    ghi(&d, "b.vis", b_text);
    ghi(&d, "c.vis", c_text);
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let uc = uri(&d, "c.vis");
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(&ua, a_text)),
        khung(&did_open(&uc, c_text)),
        khung(&did_change_v(&uc, c_moi, 2))
    );
    let out = chay(&vao);
    assert_eq!(
        cac_publish(&out),
        vec![ua, uc.clone(), uc],
        "chỉ a lúc mở a, rồi c (mở + đổi); b không mở, a không phụ thuộc c: {out}"
    );
    assert!(!out.contains(&ub), "b không mở thì không publish: {out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **VS3-7** — `didChange` có `range` được **áp incremental**, không lấy `text` làm toàn tài liệu.
#[test]
fn did_change_co_range_ap_dung() {
    let d = tam("range");
    let a_text = "let x: int = 1\n";
    ghi(&d, "a.vis", a_text);
    let ua = uri(&d, "a.vis");
    let vao = format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(&ua, a_text)),
        khung(&did_change_range(&ua, 2, 0, 13, 0, 14, "\"s\""))
    );
    let out = chay(&vao);
    assert_eq!(
        cac_publish(&out),
        vec![ua.clone(), ua],
        "mở a, rồi đổi range ⇒ publish lại đúng một lần: {out}"
    );
    assert_eq!(dem_ca(&out), 1, "range tạo lỗi kiểu: {out}");
    assert!(
        publish_cuoi(&out, &uri(&d, "a.vis")).contains(r#""line":0,"character":13"#),
        "vị trí lỗi sau range: {out}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) `didChange` a (thêm lỗi) khi b có lỗi ⇒ publish lại vẫn đúng a, không lẫn b.
#[test]
fn did_change_van_dung_a() {
    let d = tam("d");
    ghi(&d, "a.vis", "use ./b\nprint(1)\n");
    ghi(&d, "b.vis", "pub fn f() -> int:\n\n\n    return \"s\"\n");
    let ua = uri(&d, "a.vis");
    let ub = uri(&d, "b.vis");
    let out = chay(&format!(
        "{}{}{}",
        khung(&init()),
        khung(&did_open(&ua, "use ./b\nprint(1)\n")),
        khung(&did_change(&ua, "use ./b\nlet x: int = \"s\"\n"))
    ));
    assert!(
        out.contains(&format!(r#""uri":"{ua}","diagnostics":[]"#)),
        "{out}"
    );
    assert!(!out.contains(&ub), "{out}");
    assert_eq!(dem_ca(&out), 1, "chỉ lỗi a sau didChange: {out}");
    assert!(out.contains(r#""line":1,"character":13"#), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}
