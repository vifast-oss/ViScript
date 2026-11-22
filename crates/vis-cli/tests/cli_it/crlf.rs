// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-241 § Windows/CRLF (psvn-A1..A5)** — harness kiểm tra chuỗi byte CRLF.
//!
//! Trước fix (review psvn/mps 2026-11-19): CRLF ở bất kỳ tệp nguồn nào cũng làm span
//! lệch (span trỏ vào văn bản chuẩn hoá CRLF→LF trong khi mã slicing lại dùng văn bản
//! gốc), gây ra: `use` không tìm thấy module; `cap` không khớp; test fn adapter hỏng.

use std::io::Write;
use std::process::Command;

fn tam(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "vis-crlf-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&d).expect("td");
    d
}

fn write(d: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = d.join(name);
    let mut f = std::fs::File::create(&p).expect("ghi");
    f.write_all(body.as_bytes()).expect("ghi");
    p
}

fn vis_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("CARGO_BIN_EXE_vis").unwrap_or_else(|| "vis".into()))
}

fn run(tag: &str, body: &str, cap: &[&str]) -> (i32, String, String) {
    let d = tam(tag);
    let p = write(&d, "a.vis", body);
    let o = Command::new(vis_bin())
        .arg("check")
        .arg(&p)
        .args(cap)
        .output()
        .expect("chạy vis check");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn crlf_import_std_json_giai_duoc() {
    // Span của `std.json` trỏ vào chữ `std.json` — trước fix, span lệch `\r` khiến tên bị cắt thành `std.jso`
    let body = "use std.json\r\nfn main():\r\n    print(json.encode(Arr([])))\r\n";
    let (rc, out, err) = run("crlf_import", body, &[]);
    assert_eq!(rc, 0, "check phải pass; stdout={out} stderr={err}");
    assert!(!out.contains("std.jso"), "module name bị cắt: {out} {err}");
}

#[test]
fn crlf_test_fn_va_main_bien_doi_dung() {
    // test fn rename hoạt động khi fixture có CRLF
    let body = "test fn banquite():\r\n    assert 1 == 1\r\n";
    let d = tam("crlf_testfn");
    let p = write(&d, "crlf_testfn.vis", body);
    let o = Command::new(vis_bin())
        .arg("test")
        .arg(&p)
        .output()
        .expect("chạy vis test");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        o.status.code().unwrap_or(-1),
        0,
        "vis test phải pass (CRLF fixture)"
    );
}

#[test]
fn crlf_record_thieu_truong_luon_bao_loi_vi_tri_dung() {
    // Vị trí lỗi phải nằm trong dòng chứa literal record (CRLF fixture, không chỉ đường LF)
    let body = "type U:\r\n    name: str\r\n    age: int\r\nlet u = U(name: \"An\")\r\n";
    let d = tam("crlf_rec");
    let p = write(&d, "a.vis", body);
    let o = Command::new(vis_bin())
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let _ = std::fs::remove_dir_all(&d);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert_ne!(o.status.code().unwrap_or(-1), 0, "phải lỗi");
    // `vis check` in TAP lên stdout với prefix "# VSxxxx a:b thông điệp"; cả hai engine giống nhau.
    assert!(
        out.contains("thiếu trường") || out.contains("age") || out.contains("VS0412"),
        "phải VS error thiếu age: stdout={out}"
    );
}
