// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-7 (VD-271)** — `textDocument/didChange` **incremental** (`change:2`): áp nhiều
//! `contentChanges` có `range` theo UTF-16, giữ CRLF, rồi publish chẩn đoán/hover đúng.
//!
//! Ca: chèn/xoá/thay nhiều dòng tuần tự; emoji ngoài BMP (2 code unit UTF-16); CRLF; hover sau khi
//! đổi kiểu. Mỗi ca chạy `vis lsp --stdio` thật, deadline 10 s.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-lsp-inc-{tag}-{}-{}",
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
    let p = d.join(ten).to_string_lossy().to_string();
    if std::env::consts::OS == "windows" {
        format!("file:///{}", p.replace('\\', "/"))
    } else {
        format!("file://{p}")
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

/// Một `contentChanges[]` có range (0-based UTF-16).
fn doi_range(sl: u32, sc: u32, el: u32, ec: u32, text: &str) -> String {
    format!(
        r#"{{"range":{{"start":{{"line":{sl},"character":{sc}}},"end":{{"line":{el},"character":{ec}}}}},"text":"{}"}}"#,
        esc(text)
    )
}

fn did_change(u: &str, version: i64, changes: &[String]) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{u}","version":{version}}},"contentChanges":[{}]}}}}"#,
        changes.join(",")
    )
}

fn hover(id: i64, u: &str, line: u32, ch: u32) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{u}"}},"position":{{"line":{line},"character":{ch}}}}}}}"#
    )
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

/// URI của các frame `publishDiagnostics`, theo thứ tự.
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

/// Body phản hồi có `id` — `""` nếu không thấy.
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

/// Ca 1 — nhiều `contentChanges` tuần tự: xoá dòng, chèn **nhiều** dòng, thay ký tự ⇒ vị trí lỗi
/// dời đúng; rồi sửa lỗi ⇒ publish rỗng.
#[test]
fn nhieu_change_tuan_tu_chen_xoa_thay() {
    let d = tam("nhieu");
    let text = "let a = 1\nlet b = 2\nlet c: int = \"s\"\n";
    ghi(&d, "a.vis", text);
    let u = uri(&d, "a.vis");
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(&u, text)),
        khung(&did_change(
            &u,
            2,
            &[
                // Xoá dòng 0.
                doi_range(0, 0, 1, 0, ""),
                // Chèn 2 dòng ở đầu.
                doi_range(0, 0, 0, 0, "let z = 0\nlet y = 9\n"),
                // Thay `2` → `3` trên dòng `let b = ...` (dời thành dòng 2).
                doi_range(2, 8, 2, 9, "3"),
            ]
        )),
        khung(&did_change(&u, 3, &[doi_range(3, 13, 3, 16, "1")]))
    );
    let out = chay(&vao);
    assert_eq!(
        cac_publish(&out),
        vec![u.clone(), u.clone(), u.clone()],
        "mở + 2 lần đổi, mỗi lần một publish: {out}"
    );
    // Lần đổi 2: `let c: int = "s"` nằm dòng 3 (0-based) sau 2 dòng chèn.
    let sau2 = {
        // Frame publish thứ hai (sau didChange v2).
        let mut ds: Vec<&str> = out
            .split("Content-Length: ")
            .filter(|p| p.contains("publishDiagnostics"))
            .collect();
        ds.remove(1)
    };
    assert!(sau2.contains(r#""line":3,"character":13"#), "{out}");
    let cuoi = publish_cuoi(&out, &u);
    assert!(
        cuoi.contains(r#""diagnostics":[]"#),
        "sửa `\"s\"`→`1` phải hết lỗi: {cuoi}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 2 — UTF-16: chèn dòng trước, rồi xoá **emoji ngoài BMP** (2 code unit) bằng range; vị trí lỗi
/// sau emoji phải tính đúng code unit.
#[test]
fn utf16_emoji_chen_va_xoa() {
    let d = tam("emoji");
    let text = "let s = \"😀\" 1\n";
    ghi(&d, "a.vis", text);
    let u = uri(&d, "a.vis");
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(&u, text)),
        khung(&did_change(
            &u,
            2,
            &[
                doi_range(0, 0, 0, 0, "let z = 0\n"),
                // Emoji ở code unit 9..11 trên dòng 1.
                doi_range(1, 9, 1, 11, ""),
            ]
        )),
        // Chèn emoji lại (1 lần chèn, 2 code unit UTF-16) trước quote đóng.
        khung(&did_change(&u, 3, &[doi_range(1, 9, 1, 9, "😀")]))
    );
    let out = chay(&vao);
    let publish: Vec<&str> = out
        .split("Content-Length: ")
        .filter(|p| p.contains("publishDiagnostics"))
        .collect();
    assert_eq!(publish.len(), 3, "{out}");
    // `let s = "" 1` trên dòng 1 ⇒ `1` ở 11..12 (UTF-16).
    assert!(
        publish[1].contains(r#""line":1,"character":11"#),
        "range sau khi xoá emoji phải 1:11: {}",
        publish[1]
    );
    assert!(
        publish[1].contains("VS0201"),
        "`let s = \"\" 1` vẫn là lỗi biểu thức: {}",
        publish[1]
    );
    // Chèn lại emoji ⇒ `1` dời về 13..14 (2 code unit).
    assert!(
        publish[2].contains(r#""line":1,"character":13"#),
        "range sau khi chèn lại emoji phải 1:13: {}",
        publish[2]
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — CRLF: xoá dòng qua `range` kết thúc ở `(dòng kế, 0)`, rồi chèn dòng CRLF; chẩn đoán dời
/// đúng dòng.
#[test]
fn crlf_range_dong() {
    let d = tam("crlf");
    let text = "let a = 1\r\nlet x: int = \"s\"\r\n";
    ghi(&d, "a.vis", text);
    let u = uri(&d, "a.vis");
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(&u, text)),
        khung(&did_change(&u, 2, &[doi_range(0, 0, 1, 0, "")])),
        khung(&did_change(
            &u,
            3,
            &[doi_range(0, 0, 0, 0, "let z = 0\r\n")]
        ))
    );
    let out = chay(&vao);
    let publish: Vec<&str> = out
        .split("Content-Length: ")
        .filter(|p| p.contains("publishDiagnostics"))
        .collect();
    assert_eq!(publish.len(), 3, "{out}");
    assert!(
        publish[1].contains(r#""line":0,"character":13"#),
        "sau xoá dòng CRLF, lỗi ở dòng 0: {}",
        publish[1]
    );
    assert!(
        publish[2].contains(r#""line":1,"character":13"#),
        "chèn dòng CRLF ⇒ lỗi dời xuống dòng 1: {}",
        publish[2]
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4 — hover sau `didChange` incremental thấy kiểu mới (`int` → `str`), chẩn đoán cũng cập nhật.
#[test]
fn hover_sau_change() {
    let d = tam("hover");
    let text = "let v: int = 1\nv\n";
    ghi(&d, "a.vis", text);
    let u = uri(&d, "a.vis");
    let vao = format!(
        "{}{}{}{}",
        khung(&init()),
        khung(&did_open(&u, text)),
        khung(&did_change(&u, 2, &[doi_range(0, 7, 0, 10, "str")])),
        khung(&hover(2, &u, 1, 0))
    );
    let out = chay(&vao);
    let h = frame_id(&out, 2);
    assert!(h.contains("v: str"), "hover phải thấy kiểu mới: {h}");
    let cuoi = publish_cuoi(&out, &u);
    assert!(
        cuoi.contains(r#""line":0,"character":13"#),
        "gán int cho str phải báo lỗi ở `1`: {cuoi}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
