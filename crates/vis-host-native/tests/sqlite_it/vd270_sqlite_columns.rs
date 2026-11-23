// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-columns)** — `sqlite.columns(handle, table) -> Result[list[str], Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **tên cột** của một **bảng hoặc view** trong **schema chính** (`main`), theo **thứ tự
//!   khai báo** (`cid` tăng dần) — **không** sắp theo tên — giữ nguyên hoa/thường và cả tên có
//!   khoảng trắng/Unicode/dấu nháy.
//! - Tên đối tượng được **bind làm tham số** (`pragma_table_info(?, 'main')`), không nối chuỗi
//!   SQL: tên kiểu `t' OR 1=1 --`/`t; DROP TABLE t` chỉ là tên (không có bảng như vậy ⇒ `Ok([])`)
//!   và **không** tác dụng phụ.
//! - Chạy qua **đúng** đường `sqlite.query`/`db_query` hiện có, không thêm method host:
//!   authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên. Probe xác nhận authorizer cho
//!   đọc `pragma_table_info` qua `query` (chỉ `dbstat`/trang thô bị chặn); schema **cố định**
//!   `'main'` nên bảng TEMP trùng tên **không** che bảng chính.
//! - Bảng/view **không tồn tại** ⇒ `Ok([])` (khác hẳn handle thiếu/đã đóng/sai dạng hoặc lỗi
//!   SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap).
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary `vis` hiện có của main, code stdlib
//! cùng mốc `254c03f`):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 8:19 module `std.sqlite` không có hàm `columns`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:8:19: error VS0301: module `std.sqlite` không có hàm `columns`
//! $ vis run --no-cache --vm probe.vis  ⇒ exit 2 (cùng dòng)
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-sqlite-columns-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(format!("{ten}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình tạm");
    p
}

fn chay(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(crate::common::vis_bin());
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy hai engine trên cùng nguồn và đòi stdout/stderr/exit **byte-identical**; trả kết quả
/// đường interp. `sqlite.columns` chỉ đọc nên không cần tệp DB riêng từng engine.
fn hai_engine(tag: &str, src: &str) -> (i32, String, String) {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a, b, "{tag}: interp ≠ `--vm`\n  interp: {a:?}\n  vm: {b:?}");
    a
}

/// `vis check` phải **đỏ** (exit 1) với mã chẩn đoán `ma`; `vis run` cả hai engine từ chối tĩnh
/// (exit 2) cùng mã — không chạy chương trình.
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let o = Command::new(crate::common::vis_bin())
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let co = String::from_utf8_lossy(&o.stdout).to_string();
    let ce = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(
        o.status.code().unwrap_or(-1),
        1,
        "{tag}: `check` phải đỏ (exit 1)\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        co.contains(ma) || ce.contains(ma),
        "{tag}: `check` thiếu `{ma}`\nstdout: {co}\nstderr: {ce}"
    );
    for (i, vm) in [false, true].iter().enumerate() {
        let (code, so, se) = chay(&p, *vm);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "{tag} engine={i}: thiếu `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// DB rỗng/chưa có bảng ⇒ `Ok([])`; bảng 3 cột **không** theo alphabet vẫn đúng thứ tự khai báo;
/// view giữ thứ tự cột của câu `SELECT` (khác bảng gốc) và giữ nguyên hoa/thường (`Giữa`).
#[test]
fn thu_tu_khai_bao_va_view() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.columns(h, "chua_co"):
                Ok(xs): print("rong={xs.len()} {xs}")
                Err(e): print("rong loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE ba_cot (zeta TEXT, alpha INTEGER, \"Giữa\" REAL)", [])
            match sqlite.columns(h, "ba_cot"):
                Ok(xs): print("bang={xs}")
                Err(e): print("bang loi {e.message}")
            let _ = sqlite.execute(h, "CREATE VIEW v_ba_cot AS SELECT zeta, \"Giữa\", alpha FROM ba_cot", [])
            match sqlite.columns(h, "v_ba_cot"):
                Ok(xs): print("view={xs}")
                Err(e): print("view loi {e.message}")
            match sqlite.columns(h, "khong_co"):
                Ok(xs): print("thieu={xs.len()} {xs}")
                Err(e): print("thieu loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("thu-tu", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "rong=0 []\nbang=[zeta, alpha, Giữa]\nview=[zeta, Giữa, alpha]\nthieu=0 []\n",
        "thứ tự khai báo (không alphabet), view theo SELECT, giữ hoa/thường; thiếu bảng ⇒ Ok([])"
    );
}

/// Tên cột có khoảng trắng/Unicode/dấu gạch/từ khoá SQL/dấu nháy kép; tên bảng Unicode; thứ tự
/// khai báo giữ nguyên từng ký tự.
#[test]
fn ten_cot_unicode_va_dau_nhay() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.execute(h, "CREATE TABLE \"Bảng Kỳ Lạ\" (\"Tên Cột\" TEXT, \"cột ünïcode\" INTEGER, \"x-y\" REAL, \"select\" TEXT, \"a\"\"b\" TEXT)", []):
                Ok(_): print("tao ok")
                Err(e): print("tao loi {e.message}")
            match sqlite.columns(h, "Bảng Kỳ Lạ"):
                Ok(xs): print("cot={xs}")
                Err(e): print("cot loi {e.message}")
            match sqlite.columns(h, "bảng kỳ lạ"):
                Ok(xs): print("khac-hoa={xs.len()}")
                Err(e): print("khac-hoa loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("unicode", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "tao ok\ncot=[Tên Cột, cột ünïcode, x-y, select, a\"b]\nkhac-hoa=5\n",
        "tên cột Unicode/khoảng trắng/từ khoá/nháy kép giữ nguyên; tra cứu tên bảng không phân biệt hoa thường"
    );
}

/// Tên bảng **trông như SQL injection** được bind làm tham số: không có bảng tương ứng ⇒
/// `Ok([])`, và bảng thật + cột của nó **không** đổi (không có tác dụng phụ).
#[test]
fn ten_bang_injection_chi_la_ten() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (z TEXT, a INTEGER)", [])
            for ten in ["t' OR 1=1 --", "t; DROP TABLE t", "t\" OR \"1\"=\"1", "t/*x*/", "t' UNION SELECT name FROM sqlite_master --"]:
                match sqlite.columns(h, ten):
                    Ok(xs): print("la={xs.len()}")
                    Err(e): print("la loi {e.message}")
            match sqlite.columns(h, "t"):
                Ok(xs): print("that={xs}")
                Err(e): print("that loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 't'", []):
                Ok(rs): print("con={rs.rows[0][0]}")
                Err(e): print("con loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("injection", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "la=0\nla=0\nla=0\nla=0\nla=0\nthat=[z, a]\ncon=Integer(1)\n",
        "tên injection chỉ là tham số: Ok([]), bảng thật và cột không đổi"
    );
}

/// Schema **chính**: bảng TEMP trùng tên **không** che bảng chính; bảng chỉ có trong TEMP ⇒
/// `Ok([])` (không thuộc `main`). `tables` vẫn bỏ bảng TEMP như hợp đồng cũ.
#[test]
fn schema_chinh_khong_bi_temp_che() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (main_z TEXT, main_a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE t (temp_q INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE chi_tam (q INTEGER)", [])
            match sqlite.columns(h, "t"):
                Ok(xs): print("chinh={xs}")
                Err(e): print("chinh loi {e.message}")
            match sqlite.columns(h, "chi_tam"):
                Ok(xs): print("temp={xs.len()} {xs}")
                Err(e): print("temp loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("temp", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "chinh=[main_z, main_a]\ntemp=0 []\n",
        "schema cố định 'main': TEMP trùng tên không che, bảng TEMP-only không xuất hiện"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0 —
/// không trap, không panic; khác với bảng thiếu ⇒ `Ok([])`.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (z TEXT)", [])
            let _ = sqlite.close(h)
            match sqlite.columns(h, "t"):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.columns("db:00000000000000000000000000000000", "t"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.columns("khong-phai-handle", "t"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.columns("", "t"):
                Ok(_): print("rong khong duoc phep")
                Err(e): print("rong bi chan")
            match sqlite.columns("db:00000000000000000000000000000000", "khong_co"):
                Ok(_): print("la+thieu khong duoc phep")
                Err(e): print("la+thieu bi chan")
        Err(e): print("mo loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "lỗi handle phải là Err dữ liệu; stderr: {err}");
    assert_eq!(
        out, "dong bi chan\nla bi chan\nsai dang bi chan\nrong bi chan\nla+thieu bi chan\n",
        "mọi handle hỏng ⇒ Err(Error), không trap (kể cả khi tên bảng cũng thiếu)"
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "sqlite.columns(1, \"t\")", "VS0412"),
        ("kieu-2", "sqlite.columns(\"h\", 1)", "VS0412"),
        ("kieu-2-bool", "sqlite.columns(\"h\", true)", "VS0412"),
        ("arity-0", "sqlite.columns()", "VS0401"),
        ("arity-1", "sqlite.columns(\"h\")", "VS0401"),
        ("arity-3", "sqlite.columns(\"h\", \"t\", \"x\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
