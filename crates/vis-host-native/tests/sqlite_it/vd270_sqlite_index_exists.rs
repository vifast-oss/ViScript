// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-index-exists)** — `sqlite.index_exists(handle, name) -> Result[bool, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - `true` **khi và chỉ khi** schema **chính** (`main`) có **index** tên đúng:
//!   `SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ? LIMIT 1`. Bảng/view/trigger
//!   cùng tên **không** tính (`type` khác); index TEMP nằm ở schema `temp`
//!   (`sqlite_temp_master`) nên **không** thuộc schema chính ⇒ `false` (kể cả khi trùng tên index
//!   chính — index chính vẫn `true`).
//! - Tên là **tham số bind** (`SqlValue.Text`), không nối chuỗi SQL: tên kiểu `i' OR 1=1 --`/
//!   `i; DROP INDEX i` chỉ là tên (không có index như vậy ⇒ `Ok(false)`) và **không** tác dụng
//!   phụ. Hàm **chỉ đọc**, đi qua **đúng** đường `db_query` hiện có (không thêm method host):
//!   authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên.
//! - So khớp **phân biệt hoa/thường** (`name = ?` dùng collation `BINARY` mặc định):
//!   `CREATE INDEX "I_Bang"` tra `I_Bang` ⇒ `true`, tra `i_bang`/`I_BANG` ⇒ `false` — hợp đồng
//!   **có chủ ý**.
//! - **Không có tên hoặc tên rỗng ⇒ `Ok(false)`**. Index nội bộ `sqlite_autoindex_%` theo **đúng
//!   truy vấn**: có hàng `type = 'index'` với tên đó ⇒ `true` (tham chiếu được theo tên), sai
//!   hoa/thường/thiếu hậu tố ⇒ `false`.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary của cây này @ `7bef0b3`):
//!
//! ```text
//! $ vis check /tmp/opencode/probe-index-exists.vis
//! TAP version 13
//! 1..1
//! not ok 1 - /tmp/opencode/probe-index-exists.vis
//! # VS0301 8:21 module `std.sqlite` không có hàm `index_exists`
//! check exit=1
//! $ vis run --no-cache /tmp/opencode/probe-index-exists.vis       ⇒ exit 2
//! probe-index-exists.vis:8:21: error VS0301: module `std.sqlite` không có hàm `index_exists`
//! $ vis run --no-cache --vm /tmp/opencode/probe-index-exists.vis  ⇒ exit 2 (cùng dòng)
//! ```
//!
//! `cargo test -p vis-cli --test vd270_sqlite_index_exists` trước khi thêm: **0 passed;
//! 6 failed** — cả 6 ca đỏ vì `VS0301` (không có hàm).
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-sqlite-index-exists-{tag}-{}-{n}",
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
/// đường interp. `sqlite.index_exists` chỉ đọc nên không cần tệp DB riêng từng engine.
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

/// Index thường/UNIQUE (autoindex) ⇒ `true`; bảng/view/trigger, tên thiếu/rỗng, autoindex sai
/// tên đều **không** tính. Tên Unicode + khoảng trắng đi qua bind nguyên vẹn.
#[test]
fn index_true_con_lai_false() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT UNIQUE)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_ta ON t(a)", [])
            let _ = sqlite.execute(h, "CREATE INDEX \"chỉ mục\" ON t(a)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            let _ = sqlite.execute(h, "CREATE TRIGGER trg_t AFTER INSERT ON t BEGIN UPDATE t SET a = a; END", [])
            let _ = sqlite.execute(h, "CREATE TABLE u (b TEXT UNIQUE)", [])
            for ten in ["i_ta", "sqlite_autoindex_t_1", "sqlite_autoindex_u_1", "chỉ mục", "t", "u", "v_t", "trg_t", "khong_co", "", "sqlite_autoindex_t_2"]:
                match sqlite.index_exists(h, ten):
                    Ok(b): print("{ten}={b}")
                    Err(e): print("{ten} loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("index-true", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "i_ta=true\n\
         sqlite_autoindex_t_1=true\n\
         sqlite_autoindex_u_1=true\n\
         chỉ mục=true\n\
         t=false\n\
         u=false\n\
         v_t=false\n\
         trg_t=false\n\
         khong_co=false\n\
         =false\n\
         sqlite_autoindex_t_2=false\n",
        "index/autoindex ⇒ true; bảng/view/trigger/thiếu/rỗng/sai autoindex ⇒ false"
    );
}

/// So khớp **BINARY** — phân biệt hoa/thường với cả index người dùng lẫn autoindex.
#[test]
fn phan_biet_hoa_thuong() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE Bang (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX \"I_Bang\" ON Bang(a)", [])
            let _ = sqlite.execute(h, "CREATE TABLE Uniq (b TEXT UNIQUE)", [])
            for ten in ["I_Bang", "i_bang", "I_BANG", "sqlite_autoindex_Uniq_1", "sqlite_autoindex_uniq_1"]:
                match sqlite.index_exists(h, ten):
                    Ok(b): print("{ten}={b}")
                    Err(e): print("{ten} loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("case", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "I_Bang=true\n\
         i_bang=false\n\
         I_BANG=false\n\
         sqlite_autoindex_Uniq_1=true\n\
         sqlite_autoindex_uniq_1=false\n",
        "tên đúng hoa/thường ⇒ true; khác case ⇒ false (BINARY)"
    );
}

/// Index TEMP **không** thuộc schema chính ⇒ `false` (vẫn tồn tại + đọc được qua
/// `sqlite_temp_master`); TEMP trùng tên index chính **không** che index chính.
#[test]
fn index_tam_khong_thuoc_schema_chinh() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE chinh (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_chinh ON chinh(a)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tam (x INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_tam ON tam(x)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_chinh ON tam(x)", [])
            for ten in ["i_chinh", "i_tam"]:
                match sqlite.index_exists(h, ten):
                    Ok(b): print("{ten}={b}")
                    Err(e): print("{ten} loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_temp_master WHERE type = 'index' AND name = 'i_tam'", []):
                Ok(rs): print("temp_i_tam={rs.rows[0][0]}")
                Err(e): print("temp loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_temp_master WHERE type = 'index' AND name = 'i_chinh'", []):
                Ok(rs): print("temp_i_chinh={rs.rows[0][0]}")
                Err(e): print("temp loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("temp", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "i_chinh=true\n\
         i_tam=false\n\
         temp_i_tam=Integer(1)\n\
         temp_i_chinh=Integer(1)\n",
        "index TEMP ⇒ false (vẫn tồn tại); TEMP trùng tên không che index chính"
    );
}

/// Tên **trông như SQL injection** được bind làm tham số: không khớp index nào ⇒ `false`, và
/// index thật + dữ liệu + schema **không** đổi (không tác dụng phụ).
#[test]
fn ten_injection_chi_la_ten() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_t ON t(a)", [])
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            for ten in ["i_t' OR 1=1 --", "i_t; DROP INDEX i_t", "i_t\" OR \"1\"=\"1", "i_t' UNION SELECT 1 FROM sqlite_master --", "i_t'; DROP INDEX i_t; --"]:
                match sqlite.index_exists(h, ten):
                    Ok(b): print("injection={b}")
                    Err(e): print("injection loi {e.message}")
            match sqlite.index_exists(h, "i_t"):
                Ok(b): print("i_t={b}")
                Err(e): print("i_t loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("rows={rs.rows[0][0]}")
                Err(e): print("rows loi {e.message}")
            match sqlite.indexes(h, "t"):
                Ok(xs): print("indexes={xs}")
                Err(e): print("indexes loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("injection", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "injection=false\n\
         injection=false\n\
         injection=false\n\
         injection=false\n\
         injection=false\n\
         i_t=true\n\
         rows=Integer(1)\n\
         indexes=[i_t]\n",
        "mọi tên injection ⇒ false; index `i_t`, dữ liệu và schema không đổi"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0 —
/// không trap, không panic.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.close(h)
            match sqlite.index_exists(h, "i"):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.index_exists("db:00000000000000000000000000000000", "i"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.index_exists("khong-phai-handle", "i"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.index_exists("", "i"):
                Ok(_): print("rong khong duoc phep")
                Err(e): print("rong bi chan")
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
        ("kieu-1", "sqlite.index_exists(1, \"i\")", "VS0412"),
        ("kieu-1-bool", "sqlite.index_exists(true, \"i\")", "VS0412"),
        ("kieu-2", "sqlite.index_exists(\"h\", 1)", "VS0412"),
        ("kieu-2-bool", "sqlite.index_exists(\"h\", true)", "VS0412"),
        ("arity-0", "sqlite.index_exists()", "VS0401"),
        ("arity-1", "sqlite.index_exists(\"h\")", "VS0401"),
        (
            "arity-3",
            "sqlite.index_exists(\"h\", \"i\", \"x\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
