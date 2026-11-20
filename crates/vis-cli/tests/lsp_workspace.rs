// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-277** — `workspace/symbol` qua `initialize.rootUri`.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use vis_interp::value::Value;

static DEM: AtomicU32 = AtomicU32::new(0);

fn khung(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

fn chay(dau_vao: &str) -> (i32, String, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["lsp", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `vis lsp`");
    con.stdin
        .take()
        .expect("stdin")
        .write_all(dau_vao.as_bytes())
        .expect("bơm stdin");
    let han = Instant::now() + Duration::from_secs(5);
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
    let o = con.wait_with_output().expect("đọc output");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}
fn m(kv: Vec<(&str, Value)>) -> Value {
    Value::Map(
        kv.into_iter()
            .map(|(k, v)| (Value::Str(k.to_string()), v))
            .collect(),
    )
}
fn so(v: &Value) -> String {
    let mut o = String::new();
    vis_interp::stdlib::json(v, false, &mut o);
    o
}

fn init_root(uri: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(1)),
        ("method", s("initialize")),
        ("params", m(vec![("rootUri", s(uri))])),
    ]))
}

fn symbol(id: i64, query: &str) -> String {
    so(&m(vec![
        ("jsonrpc", s("2.0")),
        ("id", Value::Int(id)),
        ("method", s("workspace/symbol")),
        ("params", m(vec![("query", s(query))])),
    ]))
}

fn frame_cuoi(out: &str) -> String {
    let i = out.rfind("Content-Length: ").expect("có frame");
    let j = out[i..].find("\r\n\r\n").expect("header") + i + 4;
    out[j..].to_string()
}

#[test]
fn workspace_symbol_loc_theo_root() {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-lsp-ws-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("sub")).expect("tmp");
    std::fs::write(d.join("a.vis"), "fn alpha():\n    print(1)\n").expect("a");
    std::fs::write(d.join("sub/b.vis"), "fn beta():\n    print(2)\n").expect("b");
    let uri = format!("file://{}", d.display());
    let vao = format!("{}{}", khung(&init_root(&uri)), khung(&symbol(2, "alph")));
    let (code, out, err) = chay(&vao);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(!frame_cuoi(&out).is_empty());
    assert!(
        frame_cuoi(&out).starts_with(r#"{"jsonrpc":"2.0","id":2,"result":["#),
        "frame: {}",
        frame_cuoi(&out)
    );
    let body = frame_cuoi(&out);
    assert!(body.contains(r#""name":"alpha""#), "thiếu alpha: {body}");
    assert!(body.contains("a.vis"), "thiếu uri a.vis: {body}");
    assert!(
        !body.contains(r#""name":"beta""#),
        "không được có beta: {body}"
    );
}

#[test]
fn workspace_symbol_khong_root_rong() {
    let vao = format!(
        "{}{}",
        khung(&so(&m(vec![
            ("jsonrpc", s("2.0")),
            ("id", Value::Int(1)),
            ("method", s("initialize")),
            ("params", m(vec![])),
        ]))),
        khung(&symbol(2, ""))
    );
    let (code, out, err) = chay(&vao);
    assert_eq!(code, 0, "stderr: {err}");
    let body = frame_cuoi(&out);
    assert_eq!(
        body, r#"{"jsonrpc":"2.0","id":2,"result":[]}"#,
        "không root ⇒ rỗng"
    );
}
