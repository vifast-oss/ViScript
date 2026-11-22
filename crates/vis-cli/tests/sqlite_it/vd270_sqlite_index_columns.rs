// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-index-columns)** — `sqlite.index_columns(handle, index) -> Result[list[str], Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **tên cột đã khai** của một index trong **schema chính** (`main`), theo **thứ tự index**
//!   (`seqno` tăng dần — đúng thứ tự khai trong `CREATE INDEX`), **không** sắp theo tên; giữ
//!   nguyên hoa/thường và tên Unicode/khoảng trắng/dấu nháy. Index ghi `b` cho cột khai `"B"`
//!   ⇒ trả `B` — cùng quy ước `pragma_index_info`/`sqlite.columns`; **cột sinh** (không có trong
//!   `pragma_table_info`, `pragma_table_xinfo` bị chặn) giữ tên **viết trong `CREATE INDEX`**.
//! - Index được tra theo **tên** — index name là **schema-global**, nên index trên **bảng khác**
//!   vẫn tìm thấy khi gọi đúng tên; **không** lọc theo bảng.
//! - `UNIQUE`/`ASC`/`DESC`/`COLLATE` và ngoặc dư quanh tên cột **không** đổi kết quả (vẫn là
//!   cột đó); mục là **biểu thức** (`lower(a)`, `a + b`, …) và literal **trần**
//!   `TRUE`/`FALSE`/`NULL` bị **bỏ qua** (hợp đồng đã chốt — tương ứng `NULL` name của
//!   `pragma_index_info`), index toàn biểu thức ⇒ `Ok([])`. `TRUE`/`FALSE` vẫn là cột nếu bảng
//!   thật sự có cột cùng tên; bare `NULL` **luôn** là literal (cột tên `null` phải có nháy).
//! - Index **không tồn tại** (kể cả index TEMP ở schema `temp`) ⇒ `Ok([])`; autoindex
//!   `sqlite_autoindex_%` có `sqlite_master.sql = NULL` nên cũng `Ok([])` (không có định nghĩa
//!   cột trong đường đọc được phép).
//! - Tên index được **bind làm tham số** (`SqlValue.Text`), không nối chuỗi SQL: tên kiểu
//!   `i' OR 1=1 --`/`i; DROP INDEX i` chỉ là tên (không có index như vậy ⇒ `Ok([])`) và **không**
//!   tác dụng phụ.
//! - Chạy qua **đúng** đường `DbHost::db_query` hiện có, không thêm method host:
//!   authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên. **Probe authorizer**: host hiện
//!   tại **từ chối** `pragma_index_info`/`pragma_index_xinfo` (`SQLITE_PRAGMA` allow-list chỉ có
//!   `table_info` + đọc `user_version`/`schema_version`/`data_version`) — nên lõi dùng đường
//!   **được phép**: `SELECT sql FROM sqlite_master …` rồi phân tích thuần chuỗi `CREATE INDEX`,
//!   sau đó `SELECT name FROM pragma_table_info(?, 'main')` (tên bảng lấy từ `CREATE INDEX`,
//!   bind tham số) để trả **tên cột đã khai**.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host/kết quả sai hình dạng ⇒ `Err(Error)`
//!   **dữ liệu**, không trap.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary của cây này @ `c99339e`):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 10:19 module `std.sqlite` không có hàm `index_columns`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:10:19: error VS0301: module `std.sqlite` không có hàm `index_columns`
//! $ vis run --no-cache --vm probe.vis  ⇒ exit 2 (cùng dòng)
//! ```
//!
//! **Probe authorizer** (cùng binary, qua `sqlite.query`):
//!
//! ```text
//! index_info chan lỗi khi đọc kết quả bị từ chối bởi quyền (authorizer) (mã SQLite 23)
//! index_xinfo chan lỗi khi đọc kết quả bị từ chối bởi quyền (authorizer) (mã SQLite 23)
//! master=[[Text(CREATE INDEX i_ba ON t(b, a))]]
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
        "vis-vd270-sqlite-index-columns-{tag}-{}-{n}",
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
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
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
/// đường interp. `sqlite.index_columns` chỉ đọc nên không cần tệp DB riêng từng engine.
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
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
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

/// Thứ tự **index** (không alphabet): `(b, a)` ⇒ `[b, a]`; `UNIQUE` cùng quy tắc; `COLLATE`/
/// `ASC`/`DESC`/ngoặc dư không đổi tên cột.
#[test]
fn thu_tu_index_va_unique() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT, c REAL)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_ba ON t(b, a)", [])
            match sqlite.index_columns(h, "i_ba"):
                Ok(xs): print("ba={xs}")
                Err(e): print("ba loi {e.message}")
            let _ = sqlite.execute(h, "CREATE UNIQUE INDEX u_ca ON t(c, a)", [])
            match sqlite.index_columns(h, "u_ca"):
                Ok(xs): print("unique={xs}")
                Err(e): print("unique loi {e.message}")
            let _ = sqlite.execute(h, "CREATE INDEX i_mod ON t(c COLLATE NOCASE DESC, (b) ASC)", [])
            match sqlite.index_columns(h, "i_mod"):
                Ok(xs): print("mod={xs}")
                Err(e): print("mod loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE case_t (\"B\" TEXT, \"A\" INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_case ON case_t(b, a)", [])
            match sqlite.index_columns(h, "i_case"):
                Ok(xs): print("case={xs}")
                Err(e): print("case loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE gen (a INTEGER, b INTEGER GENERATED ALWAYS AS (a + 1) VIRTUAL)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_gen ON gen(b, a)", [])
            match sqlite.index_columns(h, "i_gen"):
                Ok(xs): print("gen={xs}")
                Err(e): print("gen loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("thu-tu", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "ba=[b, a]\nunique=[c, a]\nmod=[c, b]\ncase=[B, A]\ngen=[b, a]\n",
        "thứ tự index (không alphabet), UNIQUE cùng quy tắc, COLLATE/ASC/DESC/ngoặc không đổi cột, tên cột trả theo tên đã khai, cột sinh giữ tên viết"
    );
}

/// Thiếu index (kể cả index TEMP ở schema `temp` và autoindex không có `sql`) ⇒ `Ok([])`;
/// index trên **bảng khác** vẫn tìm thấy theo tên (schema-global); tra tên không phân biệt
/// hoa/thường ASCII như SQLite.
#[test]
fn thieu_bang_khac_autoindex_temp() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", [])
            let _ = sqlite.execute(h, "CREATE TABLE other (x INTEGER, y TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_other ON other(y, x)", [])
            match sqlite.index_columns(h, "i_other"):
                Ok(xs): print("khac-bang={xs}")
                Err(e): print("khac-bang loi {e.message}")
            match sqlite.index_columns(h, "I_OTHER"):
                Ok(xs): print("khac-hoa={xs}")
                Err(e): print("khac-hoa loi {e.message}")
            match sqlite.index_columns(h, "khong_co"):
                Ok(xs): print("thieu={xs.len()} {xs}")
                Err(e): print("thieu loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE uq (z TEXT UNIQUE)", [])
            match sqlite.index_columns(h, "sqlite_autoindex_uq_1"):
                Ok(xs): print("auto={xs.len()} {xs}")
                Err(e): print("auto loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tt (q INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_temp ON tt(q)", [])
            match sqlite.index_columns(h, "i_temp"):
                Ok(xs): print("temp={xs.len()} {xs}")
                Err(e): print("temp loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("thieu", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "khac-bang=[y, x]\nkhac-hoa=[y, x]\nthieu=0 []\nauto=0 []\ntemp=0 []\n",
        "index bảng khác vẫn thấy theo tên; thiếu/autoindex/TEMP ⇒ Ok([])"
    );
}

/// Mục **biểu thức** bị **bỏ qua** (không trả `""`), giữ thứ tự tương đối của cột thật; index
/// toàn biểu thức ⇒ `Ok([])`; ngoặc/`COLLATE`/`DESC` quanh tên cột vẫn là cột.
#[test]
fn bieu_thuc_bi_bo_qua() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT, c REAL)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_mix ON t(b, lower(a), c)", [])
            match sqlite.index_columns(h, "i_mix"):
                Ok(xs): print("mix={xs}")
                Err(e): print("mix loi {e.message}")
            let _ = sqlite.execute(h, "CREATE INDEX i_expr ON t(lower(a), b || c)", [])
            match sqlite.index_columns(h, "i_expr"):
                Ok(xs): print("expr={xs.len()} {xs}")
                Err(e): print("expr loi {e.message}")
            let _ = sqlite.execute(h, "CREATE INDEX i_paren ON t((c) DESC, ((b) COLLATE NOCASE))", [])
            match sqlite.index_columns(h, "i_paren"):
                Ok(xs): print("paren={xs}")
                Err(e): print("paren loi {e.message}")
            let _ = sqlite.execute(h, "CREATE INDEX i_const ON t(true, null, false)", [])
            match sqlite.index_columns(h, "i_const"):
                Ok(xs): print("const={xs.len()} {xs}")
                Err(e): print("const loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE kt (\"true\" TEXT, \"null\" INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_kw ON kt(\"true\", \"null\")", [])
            match sqlite.index_columns(h, "i_kw"):
                Ok(xs): print("kw={xs}")
                Err(e): print("kw loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("bieu-thuc", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "mix=[b, c]\nexpr=0 []\nparen=[c, b]\nconst=0 []\nkw=[true, null]\n",
        "mục biểu thức/literal trần bị bỏ qua (không trả \"\"), ngoặc/COLLATE/DESC quanh cột vẫn là cột, cột keyword có nháy vẫn nhận"
    );
}

/// Tên index **trông như SQL injection** chỉ là tham số bind: `Ok([])`, index thật + bảng + dữ
/// liệu **không** đổi (không tác dụng phụ).
#[test]
fn ten_index_injection_chi_la_ten() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_ba ON t(b, a)", [])
            for ten in ["i_ba' OR 1=1 --", "i_ba; DROP INDEX i_ba", "i_ba' UNION SELECT sql FROM sqlite_master --", "i_ba/*x*/"]:
                match sqlite.index_columns(h, ten):
                    Ok(xs): print("la={xs.len()}")
                    Err(e): print("la loi {e.message}")
            match sqlite.index_columns(h, "i_ba"):
                Ok(xs): print("that={xs}")
                Err(e): print("that loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = 'i_ba'", []):
                Ok(rs): print("con={rs.rows[0][0]}")
                Err(e): print("con loi {e.message}")
            match sqlite.columns(h, "t"):
                Ok(xs): print("cot={xs}")
                Err(e): print("cot loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("injection", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "la=0\nla=0\nla=0\nla=0\nthat=[b, a]\ncon=Integer(1)\ncot=[a, b]\n",
        "tên injection chỉ là tham số: Ok([]), index/bảng/cột không đổi"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0 —
/// không trap, không panic; khác với index thiếu ⇒ `Ok([])`.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_a ON t(a)", [])
            let _ = sqlite.close(h)
            match sqlite.index_columns(h, "i_a"):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.index_columns("db:00000000000000000000000000000000", "i_a"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.index_columns("khong-phai-handle", "i_a"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.index_columns("", "i_a"):
                Ok(_): print("rong khong duoc phep")
                Err(e): print("rong bi chan")
            match sqlite.index_columns("db:00000000000000000000000000000000", "khong_co"):
                Ok(_): print("la+thieu khong duoc phep")
                Err(e): print("la+thieu bi chan")
        Err(e): print("mo loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "lỗi handle phải là Err dữ liệu; stderr: {err}");
    assert_eq!(
        out, "dong bi chan\nla bi chan\nsai dang bi chan\nrong bi chan\nla+thieu bi chan\n",
        "mọi handle hỏng ⇒ Err(Error), không trap (kể cả khi tên index cũng thiếu)"
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "sqlite.index_columns(1, \"i\")", "VS0412"),
        ("kieu-2", "sqlite.index_columns(\"h\", 1)", "VS0412"),
        ("kieu-2-bool", "sqlite.index_columns(\"h\", true)", "VS0412"),
        ("arity-0", "sqlite.index_columns()", "VS0401"),
        ("arity-1", "sqlite.index_columns(\"h\")", "VS0401"),
        (
            "arity-3",
            "sqlite.index_columns(\"h\", \"i\", \"x\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
