// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-101 (VD-113)** — đầu ra máy đọc: `vis check --json` + `vis test --json`.
//! Hợp đồng: **một** object JSON nén/dòng, khoá + thứ tự cố định; `check` dùng `line/col` **1-based**;
//! `test` message một dòng ≤200 ký tự; không cờ ⇒ TAP y hệt; stderr rỗng.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Đường dẫn **dùng `/`** — Windows chấp nhận cả hai. Trên Unix giữ nguyên `\`
/// (ký tự hợp lệ hợp đồng của tên tệp, vd. `a"b\c.vis`).
fn ps(p: &Path) -> String {
    let s = p.display().to_string();
    #[cfg(windows)]
    {
        s.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        s
    }
}

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-json-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    std::fs::write(&p, src).expect("ghi");
    p
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn ca_test(d: &Path, them: &[&str]) -> (i32, String, String) {
    let dp = ps(d);
    let mut args = vec!["test", "--json", dp.as_str()];
    args.extend_from_slice(them);
    vis(&args)
}

/// (a) check 1 tệp sạch ⇒ `ok:true`, diagnostics rỗng, exit 0, stderr rỗng.
#[test]
fn check_json_tep_sach() {
    let d = tam("sach");
    let p = ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    let f = ps(&p);
    let (c, o, e) = vis(&["check", "--json", &f]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(e, "");
    assert_eq!(
        o,
        format!("{{\"ok\":true,\"files\":[{{\"file\":\"{f}\",\"diagnostics\":[]}}]}}\n")
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) check 1 tệp **2 lỗi** ⇒ 2 chẩn đoán, đúng mã + line/col 1-based, exit 1, stderr rỗng.
#[test]
fn check_json_hai_loi() {
    let d = tam("loi");
    let p = ghi(
        &d,
        "b.vis",
        "fn main():\n    let x: int = \"s\"\n    let y: str = 1\n",
    );
    let f = ps(&p);
    let (c, o, e) = vis(&["check", "--json", &f]);
    assert_eq!(c, 1, "{e}");
    assert_eq!(e, "");
    assert_eq!(o.matches(r#""code":"VS0412""#).count(), 2, "{o}");
    assert!(o.contains(r#""line":2,"col":18"#), "{o}");
    assert!(o.contains(r#""line":3,"col":18"#), "{o}");
    assert_eq!(o.lines().count(), 1, "phải **một dòng** JSON: {o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) check **3 tệp** ⇒ `files` đúng thứ tự đối số (sạch, lỗi, sạch), `ok:false`.
#[test]
fn check_json_ba_tep_dung_thu_tu() {
    let d = tam("ba");
    let a = ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    let b = ghi(&d, "b.vis", "fn main():\n    let x: int = \"s\"\n");
    let c = ghi(&d, "c.vis", "fn main():\n    print(2)\n");
    let (f1, f2, f3) = (ps(&a), ps(&b), ps(&c));
    let (ma, o, e) = vis(&["check", "--json", &f1, &f2, &f3]);
    assert_eq!(ma, 1, "{e}");
    assert_eq!(e, "");
    let i1 = o.find(&format!("\"file\":\"{f1}\"")).expect("thiếu f1");
    let i2 = o.find(&format!("\"file\":\"{f2}\"")).expect("thiếu f2");
    let i3 = o.find(&format!("\"file\":\"{f3}\"")).expect("thiếu f3");
    assert!(i1 < i2 && i2 < i3, "sai thứ tự files: {o}");
    assert!(o.starts_with("{\"ok\":false,"), "{o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) test 2/3 ok ⇒ total/ok/failed + cases đúng thứ tự + message một dòng; exit 1; VM cùng JSON.
#[test]
fn test_json_hai_tren_ba() {
    let d = tam("test");
    ghi(&d, "a.vis", "test fn a():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "test fn b():\n    assert 1 == 2\n");
    ghi(&d, "c.vis", "test fn c():\n    assert 1 == 1\n");
    let mong = "{\"total\":3,\"ok\":2,\"failed\":1,\"cases\":[{\"name\":\"a\",\"ok\":true,\"message\":\"\"},{\"name\":\"b\",\"ok\":false,\"message\":\"VS0501 assert thất bại\"},{\"name\":\"c\",\"ok\":true,\"message\":\"\"}]}\n";
    let (c1, o1, e1) = ca_test(&d, &[]);
    assert_eq!(c1, 1, "{e1}");
    assert_eq!(e1, "");
    assert_eq!(o1, mong);
    let (c2, o2, e2) = ca_test(&d, &["--vm"]);
    assert_eq!(c2, 1, "{e2}");
    assert_eq!(o2, mong, "VM phải cùng JSON");
    assert_eq!(e2, "");
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) test **tất cả ok** ⇒ exit 0, `failed:0`; VM cùng JSON.
#[test]
fn test_json_tat_ca_ok() {
    let d = tam("ok");
    ghi(&d, "a.vis", "test fn a():\n    assert 1 == 1\n");
    let mong = "{\"total\":1,\"ok\":1,\"failed\":0,\"cases\":[{\"name\":\"a\",\"ok\":true,\"message\":\"\"}]}\n";
    for them in [&[][..], &["--vm"][..]] {
        let (ma, o, e) = ca_test(&d, them);
        assert_eq!(ma, 0, "{e}");
        assert_eq!(e, "");
        assert_eq!(o, mong, "them={them:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (f) **không** `--json` ⇒ TAP **byte y hệt** (không hồi quy), stderr rỗng.
#[test]
fn khong_json_tap_nguyen_ven() {
    let d = tam("tap");
    ghi(&d, "a.vis", "test fn a():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "test fn b():\n    assert 1 == 2\n");
    ghi(&d, "c.vis", "test fn c():\n    assert 1 == 1\n");
    let mong =
        "TAP version 13\n1..3\nok 1 - a\nnot ok 2 - b\n# b:5:1: VS0501 assert thất bại\nok 3 - c\n";
    let dp = ps(&d);
    let (ma, o, e) = vis(&["test", &dp]);
    assert_eq!(ma, 1, "{e}");
    assert_eq!(e, "");
    assert_eq!(o, mong);
    let _ = std::fs::remove_dir_all(&d);
}

/// Parse JSON bằng **python3** (kiểm độc lập, không dùng code ViScript đang kiểm).
fn py_json(code: &str, input: &str) -> String {
    use std::io::Write;
    use std::process::Stdio;
    let mut c = Command::new("python3")
        .arg("-c")
        .arg(code)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn python3");
    c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let o = c.wait_with_output().expect("python3");
    assert!(
        o.status.success(),
        "python3 lỗi: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n")
}

/// (g) `check --json` với **tên tệp chứa ký tự lạ** ⇒ JSON parse được, `file` khôi phục đúng,
/// đúng **2** tệp + **1** chẩn đoán; không phá `files` order.
/// Windows không cho `"`/`\` trong tên tệp ⇒ dùng bộ ký tự lạ hợp lệ trên mọi HĐH.
#[test]
fn check_json_ten_tep_dac_biet_parse_duoc() {
    let d = tam("ten-db");
    let sach = ghi(&d, "sach.vis", "fn main():\n    print(1)\n");
    // Unix: `"` + `\`. Windows: `&`, dấu cách, ngoặc — hợp lệ, vẫn cùng hợp đồng JSON escaping.
    #[cfg(unix)]
    let ten = "a\"b\\c.vis";
    #[cfg(windows)]
    let ten = "a&b c(d) - e$f.vis";
    let db = ghi(&d, ten, "fn main():\n    let x: int = \"s\"\n");
    let sp = ps(&sach);
    let bp = ps(&db);
    let (ma, o, e) = vis(&["check", "--json", &sp, &bp]);
    assert_eq!(ma, 1, "{e}");
    assert_eq!(e, "");
    let code = r#"import json,sys
d=json.load(sys.stdin)
print(d["files"][1]["file"])
print(len(d["files"]))
print(len(d["files"][1]["diagnostics"]))
print(d["files"][1]["diagnostics"][0]["code"])"#;
    let out = py_json(code, &o);
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(dong[0], bp, "file phải khôi phục đúng");
    assert_eq!(dong[1], "2");
    assert_eq!(dong[2], "1");
    assert_eq!(dong[3], "VS0412");
    let _ = std::fs::remove_dir_all(&d);
}

/// (h) `test --json` với **tên ca tiếng Việt** ⇒ parse được, `name` khôi phục UTF-8 thô.
#[test]
fn test_json_ten_tieng_viet() {
    let d = tam("ten-viet");
    ghi(&d, "v.vis", "test fn kiểm_tra():\n    assert 1 == 1\n");
    let dp = ps(&d);
    let (ma, o, e) = vis(&["test", "--json", &dp]);
    assert_eq!(ma, 0, "{e}");
    assert_eq!(e, "");
    let code = r#"import json,sys
d=json.load(sys.stdin)
print(d["cases"][0]["name"])
print(d["total"])"#;
    let out = py_json(code, &o);
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(dong[0], "kiểm_tra");
    assert_eq!(dong[1], "1");
    let _ = std::fs::remove_dir_all(&d);
}

// ===== ST-110: severity + `file` đúng tệp phụ =====

/// (i) lỗi trong tệp phụ ⇒ `files` gồm **cả** a.vis (rỗng) + b.vis (lỗi), đúng thứ tự nạp.
#[test]
fn check_json_loi_trong_tep_phu() {
    let d = tam("phu");
    let a = ghi(&d, "a.vis", "use ./b\nprint(1)\n");
    ghi(&d, "b.vis", "fn f() -> int:\n    return \"s\"\n");
    let ap = ps(&a);
    let (c, o, e) = vis(&["check", "--json", &ap]);
    assert_eq!(c, 1, "{e}");
    assert_eq!(e, "");
    let code = r#"import json,sys
d=json.load(sys.stdin)
print(len(d["files"]))
print(d["files"][0]["file"].replace("\\","/").rsplit("/",1)[-1])
print(len(d["files"][0]["diagnostics"]))
print(d["files"][1]["file"].replace("\\","/").rsplit("/",1)[-1])
print(d["files"][1]["diagnostics"][0]["code"])
print(d["files"][1]["diagnostics"][0]["severity"])
print(d["files"][1]["diagnostics"][0]["line"], d["files"][1]["diagnostics"][0]["col"])"#;
    let out = py_json(code, &o);
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["2", "a.vis", "0", "b.vis", "VS0412", "error", "2 5"],
        "{o}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (j) lỗi ở **cả hai** ⇒ 2 mục `files`, mỗi mục 1 chẩn đoán, `a` trước `b`.
#[test]
fn check_json_loi_hai_tep() {
    let d = tam("hai-phu");
    let a = ghi(&d, "a.vis", "use ./b\nfn main():\n    let x: int = \"s\"\n");
    ghi(&d, "b.vis", "fn f() -> int:\n    return \"t\"\n");
    let ap = ps(&a);
    let (c, o, e) = vis(&["check", "--json", &ap]);
    assert_eq!(c, 1, "{e}");
    let code = r#"import json,sys
d=json.load(sys.stdin)
fs=d["files"]
print(len(fs))
print(fs[0]["file"].replace("\\","/").rsplit("/",1)[-1], len(fs[0]["diagnostics"]))
print(fs[1]["file"].replace("\\","/").rsplit("/",1)[-1], len(fs[1]["diagnostics"]))
print(fs[0]["diagnostics"][0]["severity"], fs[1]["diagnostics"][0]["severity"])"#;
    let out = py_json(code, &o);
    assert_eq!(out, "2\na.vis 1\nb.vis 1\nerror error\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (k) mọi chẩn đoán thường có `severity:"error"`.
#[test]
fn check_json_severity_error() {
    let d = tam("severity");
    let a = ghi(&d, "a.vis", "fn main():\n    let x: int = \"s\"\n");
    let ap = ps(&a);
    let (c, o, e) = vis(&["check", "--json", &ap]);
    assert_eq!(c, 1, "{e}");
    assert!(o.contains(r#""severity":"error""#), "{o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (l) tab trong `.vish` (`VS0107`) ⇒ `severity:"warning"` (ánh xạ từ `vis_diag::Severity`).
#[test]
fn check_json_severity_warning_tab() {
    let d = tam("canh-bao");
    let a = ghi(&d, "t.vish", "cmd x:\n\tprint(1)\n");
    let ap = ps(&a);
    let (c, o, e) = vis(&["check", "--json", &ap]);
    assert_eq!(c, 1, "{e}");
    let code = r#"import json,sys
d=json.load(sys.stdin)
x=d["files"][0]["diagnostics"][0]
print(x["code"], x["severity"], x["line"], x["col"])"#;
    let out = py_json(code, &o);
    assert_eq!(out, "VS0107 warning 2 1\n", "{o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (m) **không** `--json` ⇒ TAP y hệt (không thêm trường/làm đổi dòng chẩn đoán).
#[test]
fn check_json_khong_co_tap_khong_doi() {
    let d = tam("tap-check");
    let a = ghi(&d, "a.vis", "fn main():\n    let x: int = \"s\"\n");
    let f = ps(&a);
    let (c, o, e) = vis(&["check", &f]);
    assert_eq!(c, 1, "{e}");
    assert_eq!(e, "");
    let mong = format!(
        "TAP version 13\n1..1\nnot ok 1 - {f}\n# VS0412 2:18 khai báo `let`: cần `int`, được `str`\n"
    );
    assert_eq!(o, mong);
    let _ = std::fs::remove_dir_all(&d);
}
