// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-schema-version)** — `sqlite.schema_version(handle) -> Result[int, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **`schema_version`** (schema cookie) của **schema chính** (`main`) của connection, đọc
//!   bằng `PRAGMA schema_version` qua **đúng** đường `sqlite.query`/`DbHost::db_query` hiện có
//!   (không thêm method host): authorizer (allow-list **đọc** `schema_version`/`user_version`/
//!   `data_version` không kèm giá trị), chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần/
//!   timeout/cache áp nguyên. Hàm **chỉ đọc** — không DDL/DML, không đổi schema hay dữ liệu.
//! - Giá trị là `int`. Database `:memory:` mới có cookie `0`; database trên tệp có thể bắt đầu ở
//!   số **không âm** bất kỳ tuỳ lịch sử DDL — hợp đồng chỉ hứa `int` **không âm**, không hứa mốc.
//! - **Mỗi lời gọi đọc lại** từ connection: DDL đổi **schema chính** (`CREATE TABLE`…) trên
//!   **cùng connection** làm giá trị **tăng** ngay ở lời gọi kế tiếp (không cache ở tầng std).
//!   Truy vấn chỉ đọc/ghi dữ liệu **không** đổi giá trị. DDL bảng **TEMP** chỉ đổi
//!   `temp.schema_version`, **không** đổi cookie `main` — hợp đồng **có chủ ý** (hàm mô tả schema
//!   chính). `CREATE TABLE IF NOT EXISTS` trên bảng đã có **không** đổi schema ⇒ giá trị giữ
//!   nguyên.
//! - **Đường đọc đã chọn**: `PRAGMA schema_version` (không phải `SELECT * FROM
//!   pragma_schema_version`): cả hai đều được authorizer cho qua `db_query` (probe), nhưng TVF
//!   `pragma_schema_version` **có thể bị bảng người dùng trùng tên che**, còn pragma thì không.
//!   Câu SQL là **hằng**, không bind tham số — không có bề mặt injection.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!   Kết quả sai hình dạng (không đúng một cột/hàng, ô không phải INTEGER) cũng là `Err` dữ
//!   liệu — kiểm ở unit test host giả của `crates/vis-runtime/src/stdlib/sqlite/pragma.rs`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — `vis` của `main` @ `33106bb`, hàm chưa tồn
//! tại ở bất kỳ cây nào):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 6:19 module `std.sqlite` không có hàm `schema_version`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:6:19: error VS0301: module `std.sqlite` không có hàm `schema_version`
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
        "vis-vd270-sqlite-schema-version-{tag}-{}-{n}",
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
/// đường interp. `sqlite.schema_version` chỉ đọc nên không cần tệp DB riêng từng engine.
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

/// Mỗi dòng stdout là một giá trị in từ `schema_version` (hoặc nhãn `doc-ok`).
///
/// Trình tự: cookie đầu (`:memory:` mới) ≥ 0; `CREATE TABLE` ⇒ **tăng**; một truy vấn chỉ đọc
/// (`SELECT count(*) FROM sqlite_master`) ⇒ **không đổi**; `CREATE TABLE` thứ hai ⇒ **tăng**;
/// `CREATE TEMP TABLE` ⇒ **không đổi** cookie `main`; `CREATE TABLE IF NOT EXISTS` trên bảng đã
/// có ⇒ **không đổi**.
#[test]
fn khoi_tao_ddl_tang_truy_van_va_temp_khong_doi() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.schema_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            match sqlite.schema_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master", []):
                Ok(_): print("doc-ok")
                Err(e): print("doc loi {e.message}")
            match sqlite.schema_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t2 (b INTEGER)", [])
            match sqlite.schema_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tam (q INTEGER)", [])
            match sqlite.schema_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE IF NOT EXISTS t (a TEXT)", [])
            match sqlite.schema_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("thu-tu", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    let dong: Vec<String> = out.lines().map(str::to_string).collect();
    assert_eq!(
        dong.len(),
        7,
        "phải in đúng 7 dòng, được {dong:?}\nstderr: {err}"
    );
    assert_eq!(
        dong[2], "doc-ok",
        "truy vấn chỉ đọc phải chạy được: {dong:?}"
    );
    let v: Vec<i64> = [0usize, 1, 3, 4, 5, 6]
        .iter()
        .map(|&i| {
            dong[i]
                .parse::<i64>()
                .unwrap_or_else(|e| panic!("dòng {} {dong:?} không phải int: {e}", i))
        })
        .collect();
    assert!(v[0] >= 0, "cookie khởi tạo phải ≥ 0, được {}", v[0]);
    assert!(
        v[1] > v[0],
        "CREATE TABLE phải làm schema_version **tăng**: {} → {}",
        v[0],
        v[1]
    );
    assert_eq!(
        v[2], v[1],
        "truy vấn chỉ đọc **không** được đổi schema_version: {} → {}",
        v[1], v[2]
    );
    assert!(
        v[3] > v[2],
        "CREATE TABLE thứ hai phải làm schema_version **tăng**: {} → {}",
        v[2],
        v[3]
    );
    assert_eq!(
        v[4], v[3],
        "DDL TEMP chỉ đổi cookie `temp`, KHÔNG đổi cookie `main`: {} → {}",
        v[3], v[4]
    );
    assert_eq!(
        v[5], v[4],
        "`CREATE TABLE IF NOT EXISTS` trên bảng đã có **không** đổi schema: {} → {}",
        v[4], v[5]
    );
}

/// Handle đã đóng/sai dạng/không tồn tại/rỗng ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0
/// — không trap, không panic.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.close(h)
            match sqlite.schema_version(h):
                Ok(_): print("dong khong duoc phep")
                Err(_): print("dong bi chan")
            match sqlite.schema_version("db:00000000000000000000000000000000"):
                Ok(_): print("la khong duoc phep")
                Err(_): print("la bi chan")
            match sqlite.schema_version("khong-phai-handle"):
                Ok(_): print("sai dang khong duoc phep")
                Err(_): print("sai dang bi chan")
            match sqlite.schema_version(""):
                Ok(_): print("rong khong duoc phep")
                Err(_): print("rong bi chan")
        Err(e): print("mo loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "lỗi handle phải là Err dữ liệu; stderr: {err}");
    assert_eq!(
        out, "dong bi chan\nla bi chan\nsai dang bi chan\nrong bi chan\n",
        "mọi handle hỏng ⇒ Err(Error), không trap"
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "sqlite.schema_version(1)", "VS0412"),
        ("kieu-bool", "sqlite.schema_version(true)", "VS0412"),
        ("arity-0", "sqlite.schema_version()", "VS0401"),
        ("arity-2", "sqlite.schema_version(\"h\", \"x\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
