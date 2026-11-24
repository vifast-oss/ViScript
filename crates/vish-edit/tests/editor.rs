// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — test editor/render/history; PTY giả = chuỗi byte qua `read_key`.

use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU32, Ordering};

use vish_edit::{parse_keys, read_key, Action, Editor, History, Key};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> std::path::PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vish-edit-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn go(ed: &mut Editor, s: &str) -> Action {
    let mut last = Action::Continue;
    for c in s.chars() {
        last = ed.feed(Key::Char(c));
    }
    last
}

fn bo_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        if esc {
            if c == 'm' {
                esc = false;
            }
        } else if c == '\x1b' {
            esc = true;
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn parse_keys_arrows_utf8_ctrlc() {
    assert_eq!(
        parse_keys(b"a\x1b[A\xc3\xa9\x03\r\n"),
        vec![
            Key::Char('a'),
            Key::Up,
            Key::Char('é'),
            Key::CtrlC,
            Key::Enter,
            Key::Newline
        ]
    );
}

#[test]
fn pty_gia_doc_byte_qua_read_key() {
    let mut c = Cursor::new(b"ab\x1b[D\r".to_vec());
    assert_eq!(read_key(&mut c).unwrap(), Some(Key::Char('a')));
    assert_eq!(read_key(&mut c).unwrap(), Some(Key::Char('b')));
    assert_eq!(read_key(&mut c).unwrap(), Some(Key::Left));
    assert_eq!(read_key(&mut c).unwrap(), Some(Key::Enter));
    assert_eq!(read_key(&mut c).unwrap(), None);
}

#[test]
fn nhieu_dong_va_thut_le() {
    let mut ed = Editor::new(History::default());
    go(&mut ed, "fn main():");
    ed.feed(Key::Newline);
    assert_eq!(ed.text(), "fn main():\n    ");
    ed.feed(Key::Tab);
    go(&mut ed, "print(1)");
    assert_eq!(ed.text(), "fn main():\n        print(1)");
    assert_eq!(ed.feed(Key::Enter), Action::Submit(ed.text().to_string()));
}

#[test]
fn ctrl_c_huy() {
    let mut ed = Editor::new(History::default());
    go(&mut ed, "abc");
    assert_eq!(ed.feed(Key::CtrlC), Action::Cancel);
}

#[test]
fn history_append_only_va_fuzzy() {
    let d = tam("history");
    let p = d.join("h");
    let mut h = History::new(Some(p.clone())).unwrap();
    h.add("print(1)").unwrap();
    h.add("let x = 2").unwrap();
    h.add("print(2)").unwrap();
    let raw = std::fs::read_to_string(&p).unwrap();
    assert_eq!(raw.lines().count(), 3);
    let h2 = History::new(Some(p)).unwrap();
    assert_eq!(h2.entries().len(), 3);
    let f = h2.fuzzy("pr2");
    assert_eq!(f.first(), Some(&"print(2)"));
    assert!(h2.fuzzy("xyz").is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn fuzzy_chon_bang_ctrl_r() {
    let mut h = History::default();
    h.add("print(1)").unwrap();
    h.add("let x = 2").unwrap();
    let mut ed = Editor::new(h);
    ed.feed(Key::CtrlR);
    go(&mut ed, "pri");
    assert!(ed.render().contains("tìm: pri"));
    ed.feed(Key::Enter);
    assert_eq!(ed.text(), "print(1)");
}

#[test]
fn render_cst_co_mau_va_giu_byte() {
    let src = "fn main():\n    print(\"x\") # c\n";
    let r = vish_edit::render::render_cst(src);
    assert!(r.contains("\x1b[34m"), "keyword phải xanh: {r:?}");
    assert!(r.contains("\x1b[33m"), "string phải vàng: {r:?}");
    assert!(r.contains("\x1b[32m"), "comment phải xanh lá: {r:?}");
    assert_eq!(bo_ansi(&r), src);
}

#[test]
fn panic_giu_editor_va_history() {
    let d = tam("panic");
    let p = d.join("h");
    let mut ed = Editor::new(History::new(Some(p)).unwrap());
    ed.history_mut().add("truoc").unwrap();
    let r = catch_unwind(AssertUnwindSafe(|| {
        go(&mut ed, "abc");
        panic!("boom");
    }));
    assert!(r.is_err());
    ed.feed(Key::Char('d'));
    assert_eq!(ed.text(), "abcd");
    assert_eq!(ed.history().entries(), ["truoc"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(windows)]
#[test]
fn windows_parse_ctrl_c_va_arrow() {
    assert_eq!(parse_keys(b"\x03\x1b[A"), vec![Key::CtrlC, Key::Up]);
}

#[path = "edit_it/completion_job.rs"]
mod completion_job;
