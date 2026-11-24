// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (check-json)** — khoá hợp đồng `vis check --json` **hiện có** (ST-101/ST-110/VD-269).
//!
//! Hình dạng (một object **nén**, đúng **một dòng** + newline cuối, stdout):
//! `{"ok":true|false,"files":[{"file":"…","diagnostics":[{"code":"VS0xxx","line":L,"col":C,
//! "severity":"error|warning","message":"…"}]}]}`.
//! - `files` theo **thứ tự đối số** (mỗi đối số một mục; module theo `use` nằm cùng mục tệp chính);
//! - `diagnostics` theo **thứ tự phát hiện**, `line`/`col` **1-based**;
//! - exit 0 (sạch) / 1 (có chẩn đoán không mềm) / 2 (dùng sai, IO);
//! - `--stdin` dùng tên hiển thị `-`; stdin rỗng là chương trình hợp lệ.
//!
//! File này **chỉ khoá hành vi**, không đổi hình dạng JSON/TAP. Ngoại lệ duy nhất: cờ lạ đi với
//! `--json` trước đây bị coi là **tên tệp** (`VS0201`, exit 1) trong khi TAP/`--stdin` trả lỗi
//! dùng (exit 2) — nay `--json` cũng trả 2, stdout rỗng. Ca `co_la_voi_json_la_dung_sai` là ca
//! đỏ-trước của sửa nhỏ đó.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vd270-check-json-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi tệp");
}

/// Chạy `vis <args…>` tại `d` (đường dẫn tương đối ⇒ JSON ổn định giữa các nền tảng).
fn vis_tai(d: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .current_dir(d)
        .stdin(Stdio::null())
        .output()
        .expect("chạy được `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Như [`vis_tai`] nhưng ghi byte thô vào stdin (thử cả UTF-8 không hợp lệ).
fn vis_stdin(d: &Path, args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .current_dir(d)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("chạy được `vis`");
    {
        let mut vao = con.stdin.take().expect("stdin đã pipe");
        if let Err(e) = vao.write_all(stdin) {
            // CLI có thể từ chối tham số trước khi đọc stdin ⇒ ống đóng sớm là hợp lệ.
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("ghi stdin: {e}");
            }
        }
    }
    let o = con.wait_with_output().expect("chờ `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Hợp đồng chung: **một dòng** + newline cuối, và **parse lại được** bằng `json.decode` của
/// ViScript (không thêm dev-dependency `serde_json`; crate chưa có).
fn kiem_json(out: &str) {
    assert!(
        out.ends_with('\n'),
        "JSON phải kết thúc bằng newline: {out:?}"
    );
    assert_eq!(
        out.matches('\n').count(),
        1,
        "JSON phải đúng một dòng (mọi newline trong chuỗi đã escape): {out:?}"
    );
    vis_core::json::decode(out.trim_end_matches('\n')).expect("JSON phải parse được");
}

// Các mẩu chẩn đoán **nguyên văn** (raw string giữ đúng dấu `\` của JSON đã escape).
const CP_NEWLINE: &str = r#"{"code":"VS0201","line":1,"col":8,"severity":"error","message":"mong biểu thức, thật `\n` (NEWLINE)"}"#;
const KIEU_DONG2: &str = r#"{"code":"VS0412","line":2,"col":18,"severity":"error","message":"khai báo `let`: cần `int`, được `str`"}"#;
const KIEU_DONG3: &str = r#"{"code":"VS0412","line":3,"col":18,"severity":"error","message":"khai báo `let`: cần `str`, được `int`"}"#;
const KIEU_STDIN: &str = r#"{"code":"VS0412","line":1,"col":14,"severity":"error","message":"khai báo `let`: cần `int`, được `str`"}"#;
const TAB_WARNING: &str = r#"{"code":"VS0107","line":2,"col":1,"severity":"warning","message":"tab (cảnh báo-lỗi; tab = 4)"}"#;
const KHONG_UTF8: &str = r#"{"code":"VS0201","line":1,"col":1,"severity":"error","message":"không đọc được: stream did not contain valid UTF-8"}"#;

/// Tệp sạch: `ok:true`, `files` một mục, `diagnostics` rỗng, exit 0, stderr rỗng.
#[test]
fn sach_mot_tep_json_chinh_xac() {
    let d = tam("sach");
    ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "a.vis"]);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(e, "");
    assert_eq!(
        o,
        "{\"ok\":true,\"files\":[{\"file\":\"a.vis\",\"diagnostics\":[]}]}\n"
    );
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// Lỗi cú pháp: mã/dòng/cột/mức/thông điệp **nguyên văn**, `ok:false`, exit 1.
#[test]
fn loi_cu_phap_json_chinh_xac() {
    let d = tam("cu-phap");
    ghi(&d, "a.vis", "let x =\n");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "a.vis"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong = format!(
        "{{\"ok\":false,\"files\":[{{\"file\":\"a.vis\",\"diagnostics\":[{CP_NEWLINE}]}}]}}\n"
    );
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// Nhiều tệp (bẩn, sạch, bẩn): `files` đúng **thứ tự đối số**, chẩn đoán nằm đúng mục, exit 1.
#[test]
fn nhieu_tep_dung_thu_tu_doi_so() {
    let d = tam("nhieu");
    ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    ghi(&d, "b.vis", "let x =\n");
    ghi(&d, "c.vis", "fn main():\n    let x: int = \"s\"\n");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "b.vis", "a.vis", "c.vis"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong = format!(
        "{{\"ok\":false,\"files\":[{{\"file\":\"b.vis\",\"diagnostics\":[{CP_NEWLINE}]}},{{\"file\":\"a.vis\",\"diagnostics\":[]}},{{\"file\":\"c.vis\",\"diagnostics\":[{KIEU_DONG2}]}}]}}\n"
    );
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// Một tệp **hai** chẩn đoán: giữ **thứ tự phát hiện**, cả hai đều `severity:"error"`.
#[test]
fn mot_tep_hai_chan_doan_giu_thu_tu() {
    let d = tam("hai-chan-doan");
    ghi(
        &d,
        "a.vis",
        "fn main():\n    let x: int = \"s\"\n    let y: str = 1\n",
    );
    let (c, o, e) = vis_tai(&d, &["check", "--json", "a.vis"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong = format!(
        "{{\"ok\":false,\"files\":[{{\"file\":\"a.vis\",\"diagnostics\":[{KIEU_DONG2},{KIEU_DONG3}]}}]}}\n"
    );
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// `--stdin --json`: stdin **rỗng** và nguồn sạch đều là chương trình hợp lệ, tệp `-`, exit 0.
#[test]
fn stdin_sach_va_rong() {
    let d = tam("stdin-sach");
    for src in ["", "let x = 1\n"] {
        let (c, o, e) = vis_stdin(&d, &["check", "--json", "--stdin"], src.as_bytes());
        assert_eq!(c, 0, "nguồn {src:?}; stderr: {e}");
        assert_eq!(e, "");
        assert_eq!(
            o,
            "{\"ok\":true,\"files\":[{\"file\":\"-\",\"diagnostics\":[]}]}\n"
        );
        kiem_json(&o);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// `--stdin --json` có lỗi: `file:"-"`, đúng dòng/cột/mức, exit 1.
#[test]
fn stdin_loi_json_chinh_xac() {
    let d = tam("stdin-loi");
    let (c, o, e) = vis_stdin(&d, &["check", "--json", "--stdin"], b"let x: int = \"s\"\n");
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong =
        format!("{{\"ok\":false,\"files\":[{{\"file\":\"-\",\"diagnostics\":[{KIEU_STDIN}]}}]}}\n");
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// `--stdin` không đi với tệp: lỗi dùng, exit 2, **stdout rỗng**, stderr nêu đối số.
#[test]
fn stdin_khong_di_voi_tep() {
    let d = tam("stdin-tep");
    ghi(&d, "a.vis", "let x = 1\n");
    let (c, o, e) = vis_stdin(&d, &["check", "--json", "--stdin", "a.vis"], b"");
    assert_eq!(c, 2);
    assert_eq!(o, "", "lỗi dùng không in JSON");
    assert!(
        e.starts_with("vis check: --stdin không nhận đối số `a.vis` (không nhận tệp)\n"),
        "stderr: {e:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Cờ lạ + `--json`: lỗi dùng (exit 2, stdout rỗng) như TAP/`--stdin` — **không** coi cờ là tệp.
///
/// Đỏ-trước: trước sửa nhỏ VD-270, lệnh này in JSON với `"file":"--khong-co"` và exit 1.
#[test]
fn co_la_voi_json_la_dung_sai() {
    let d = tam("co-la");
    ghi(&d, "a.vis", "let x = 1\n");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "--khong-co", "a.vis"]);
    assert_eq!(c, 2, "stderr: {e}");
    assert_eq!(o, "", "lỗi dùng không in JSON");
    assert!(
        e.starts_with("vis check: không hỗ trợ cờ `--khong-co`\n"),
        "stderr: {e:?}"
    );
    // Đường TAP đã đúng từ trước — khoá luôn để hai đường không lệch.
    let (c2, o2, _) = vis_tai(&d, &["check", "--khong-co", "a.vis"]);
    assert_eq!(c2, 2);
    assert_eq!(o2, "");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--json` không có tệp: lỗi dùng, exit 2, stdout rỗng.
#[test]
fn json_khong_co_tep_la_dung_sai() {
    let d = tam("json-rong");
    let (c, o, e) = vis_tai(&d, &["check", "--json"]);
    assert_eq!(c, 2);
    assert_eq!(o, "");
    assert!(e.starts_with("dùng: vis check"), "stderr: {e:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--syntax` / `--no-extern` vẫn đi cùng `--json` (không phải cờ hình dạng xung đột):
/// `--syntax` bỏ tyck ⇒ sạch; `--no-extern` không đổi chương trình sạch.
#[test]
fn syntax_va_no_extern_van_di_voi_json() {
    let d = tam("co-phu");
    ghi(&d, "loi.vis", "fn main():\n    let x: int = \"s\"\n");
    ghi(&d, "sach.vis", "fn main():\n    print(1)\n");
    let ca = [
        (&["check", "--json", "--syntax", "loi.vis"][..], "loi.vis"),
        (
            &["check", "--json", "--no-extern", "sach.vis"][..],
            "sach.vis",
        ),
    ];
    for (args, f) in ca {
        let (c, o, e) = vis_tai(&d, args);
        assert_eq!(c, 0, "args={args:?}; stderr: {e}");
        assert_eq!(e, "");
        assert_eq!(
            o,
            format!("{{\"ok\":true,\"files\":[{{\"file\":\"{f}\",\"diagnostics\":[]}}]}}\n"),
            "args={args:?}"
        );
        kiem_json(&o);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Tệp không phải UTF-8: vẫn là chẩn đoán `VS0201` trong JSON, exit 1, stderr rỗng.
#[test]
fn tep_khong_phai_utf8_giu_json() {
    let d = tam("khong-utf8");
    std::fs::write(d.join("bad.bin"), [0xff, 0xfe, 0xfd]).expect("ghi tệp");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "bad.bin"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong = format!(
        "{{\"ok\":false,\"files\":[{{\"file\":\"bad.bin\",\"diagnostics\":[{KHONG_UTF8}]}}]}}\n"
    );
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// Tệp không đọc được (không tồn tại): `VS0201` 1:1 `error`, `message` bắt đầu `không đọc được:`,
/// exit 1, stderr rỗng (thông điệp hệ điều hành khác nhau nên chỉ khoá phần ổn định).
#[test]
fn tep_khong_doc_duoc_giu_json() {
    let d = tam("khong-doc");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "khong_ton_tai.vis"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    assert!(o.contains(r#""file":"khong_ton_tai.vis""#), "{o}");
    assert!(
        o.contains(
            r#""code":"VS0201","line":1,"col":1,"severity":"error","message":"không đọc được:"#
        ),
        "{o}"
    );
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// stdin không phải UTF-8: **không** in JSON, stderr báo lỗi, exit 2 (đường tệp là `VS0201`).
#[test]
fn stdin_khong_phai_utf8_exit_2() {
    let d = tam("stdin-khong-utf8");
    let (c, o, e) = vis_stdin(&d, &["check", "--json", "--stdin"], &[0xff, 0xfe, 0xfd]);
    assert_eq!(c, 2);
    assert_eq!(o, "");
    assert_eq!(e, "vis check: stdin không phải UTF-8 hợp lệ\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `VS0107` (tab) là warning-lỗi: JSON ghi `severity:"warning"`, vẫn làm `ok:false`/exit 1.
#[test]
fn tab_la_warning_trong_json() {
    let d = tam("tab");
    ghi(&d, "t.vish", "cmd x:\n\tprint(1)\n");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "t.vish"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong = format!(
        "{{\"ok\":false,\"files\":[{{\"file\":\"t.vish\",\"diagnostics\":[{TAB_WARNING}]}}]}}\n"
    );
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}

/// Không có `--json` ⇒ TAP **byte y hệt** (không thêm trường/làm đổi dòng chẩn đoán).
#[test]
fn tap_khong_doi_khi_khong_co_json() {
    let d = tam("tap");
    ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    ghi(&d, "b.vis", "let x =\n");
    let (c, o, e) = vis_tai(&d, &["check", "a.vis", "b.vis"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    assert_eq!(
        o,
        "TAP version 13\n1..2\nok 1 - a.vis\nnot ok 2 - b.vis\n# VS0201 1:8 mong biểu thức, thật `\\n` (NEWLINE)\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Tên tệp chứa `"` và `\`: JSON escape đúng, vẫn parse được và khôi phục tên gốc.
#[cfg(unix)]
#[test]
fn ten_tep_escape_json_va_khoi_phuc() {
    let d = tam("ten-escape");
    ghi(&d, "a\"b\\c.vis", "let x: int = \"s\"\n");
    let (c, o, e) = vis_tai(&d, &["check", "--json", "a\"b\\c.vis"]);
    assert_eq!(c, 1, "stderr: {e}");
    assert_eq!(e, "");
    let mong = format!(
        "{{\"ok\":false,\"files\":[{{\"file\":\"a\\\"b\\\\c.vis\",\"diagnostics\":[{KIEU_STDIN}]}}]}}\n"
    );
    assert_eq!(o, mong);
    kiem_json(&o);
    let _ = std::fs::remove_dir_all(&d);
}
