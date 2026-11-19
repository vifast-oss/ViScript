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

fn did_change(u: &str, text: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{u}","version":2}},"contentChanges":[{{"text":"{}"}}]}}}}"#,
        esc(text)
    )
}

/// Chạy `vis lsp --stdio`, bơm frame, deadline 10 s.
fn chay(dau_vao: &str) -> String {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
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
