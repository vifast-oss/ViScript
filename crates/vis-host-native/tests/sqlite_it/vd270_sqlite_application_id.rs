// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-application-id)** — `sqlite.application_id(handle) -> Result[int, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **`application_id`** — số nguyên **32-bit có dấu** trong header tệp database (offset
//!   68, big-endian) — đọc bằng `PRAGMA application_id` qua **đúng** đường
//!   `sqlite.query`/`DbHost::db_query` hiện có (không thêm method host, **không** ghi/DDL).
//!   Đây là metadata **chỉ đọc**: SQLite **không** dùng số này cho việc gì; nó **dành cho ứng
//!   dụng tự định danh** (ví dụ công cụ ngoài đặt một magic number). DDL (`CREATE`/`DROP`/
//!   `ALTER`), ghi dữ liệu và truy vấn chỉ đọc **không** đổi giá trị.
//! - Database `:memory:` **mới** ⇒ `0`. Giá trị **lưu theo tệp**: đóng rồi mở lại vẫn giữ
//!   nguyên.
//! - Hợp đồng chỉ nhận **`0..=i32::MAX`** (không âm). Đường std **không có** hàm ghi
//!   `application_id` (authorizer chỉ cho **đọc**, xem dưới), nên giá trị âm/ngoài phạm vi chỉ
//!   xuất hiện ở tệp do **công cụ ngoài** tạo — khi đó `Err(Error)` **dữ liệu**, không trap.
//! - **Đường đọc đã chọn**: `PRAGMA application_id` (không phải `SELECT * FROM
//!   pragma_application_id`). **Probe** trên host hiện tại **trước khi sửa** (cả hai engine):
//!   `PRAGMA application_id` ⇒ `Err` «bị từ chối bởi quyền (authorizer) (mã SQLite 23)»;
//!   `SELECT * FROM pragma_application_id` **cũng** `Err` «bị từ chối bởi quyền (authorizer)
//!   (mã SQLite 23)» — TVF của pragma cũng đi qua kiểm tra `SQLITE_PRAGMA` với chính tên
//!   `application_id`. Vì vậy host phải mở **đúng một mục đọc** `application_id` (không kèm
//!   giá trị) trong allow-list `SQLITE_PRAGMA`; ghi `PRAGMA application_id = …` vẫn bị **từ
//!   chối** (test `ghi_pragma_bi_chan...`). Khi cả hai đường đều được phép, chọn **pragma** vì
//!   TVF `pragma_application_id` **có thể bị bảng người dùng trùng tên che**, còn pragma không
//!   có không gian tên đó — hợp đồng không phụ thuộc schema người dùng. Câu SQL là **hằng**,
//!   không bind tham số — không có bề mặt injection.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!   Kết quả sai hình dạng (không đúng một cột/hàng, ô không phải INTEGER) cũng là `Err` dữ
//!   liệu — kiểm ở unit test host giả của `crates/vis-interp/src/stdlib/sqliteappid.rs`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — `main` @ `2b20b55`, hàm chưa tồn tại):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_sqlite_application_id
//! ...
//! test result: FAILED. 0 passed; 5 failed
//! # mọi ca đỏ vì `VS0301` «module `std.sqlite` không có hàm `application_id`»
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
        "vis-vd270-sqlite-application-id-{tag}-{}-{n}",
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

/// Chạy một nguồn (ghi vào thư mục tạm riêng) trên một engine.
fn chay_nguon(tag: &str, src: &str, vm: bool) -> (i32, String, String) {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let r = chay(&p, vm);
    let _ = std::fs::remove_dir_all(&d);
    r
}

/// Chạy hai engine trên cùng nguồn và đòi stdout/stderr/exit **byte-identical**; trả kết quả
/// đường interp. `sqlite.application_id` chỉ đọc nên không cần tệp DB riêng từng engine khi
/// nguồn dùng `:memory:`.
fn hai_engine(tag: &str, src: &str) -> (i32, String, String) {
    let a = chay_nguon(tag, src, false);
    let b = chay_nguon(tag, src, true);
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

/// `:memory:` mới ⇒ `0`; DDL (bảng/index/view/drop) và ghi dữ liệu **không** đổi
/// `application_id`; truy vấn chỉ đọc vẫn chạy được.
#[test]
fn khoi_tao_0_va_ddl_du_lieu_khong_doi() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master", []):
                Ok(_): print("doc-ok")
                Err(e): print("doc loi {e.message}")
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE INDEX i_t ON t (a)", [])
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "DROP TABLE t", [])
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("ddl", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        dong,
        ["0", "0", "doc-ok", "0", "0", "0", "0"],
        "`:memory:` mới ⇒ 0; DDL/ghi dữ liệu **không** đổi application_id: {dong:?}\nstderr: {err}"
    );
}

/// Ghi `application_id` (big-endian 32-bit có dấu) thẳng vào header tệp DB — mô phỏng công cụ
/// **ngoài** đặt giá trị, vì đường std không có hàm ghi (authorizer chỉ cho đọc).
fn dat_application_id(db: &Path, v: i32) {
    let mut b = std::fs::read(db).expect("đọc tệp DB");
    assert!(b.len() >= 100, "tệp DB phải có header 100 byte");
    b[68..72].copy_from_slice(&v.to_be_bytes());
    std::fs::write(db, b).expect("ghi header DB");
}

/// Một lượt chạy nguồn trên tệp DB thật, trả `(exit, stdout, stderr)`.
fn chay_tren_tep(tag: &str, db: &Path, than: &str, vm: bool) -> (i32, String, String) {
    // Dùng `/` trong literal VS (SQLite nhận cả `\` trên Windows); đường dẫn tệp tạm không chứa
    // dấu `"` nên không cần escape thêm.
    let db = db.display().to_string().replace('\\', "/");
    let src = format!("use std.sqlite\n\nfn main():\n    let db = \"{db}\"\n{than}");
    chay_nguon(tag, &src, vm)
}

/// Giá trị **lưu theo tệp**: `:memory:`/tệp mới ⇒ `0`; đóng/mở lại giữ nguyên; DDL/ghi dữ liệu
/// **không** đổi; giá trị ngoài `0..=i32::MAX` do tệp ngoài đặt ⇒ `Err(Error)`; hai engine
/// byte-identical.
#[test]
fn gia_tri_luu_theo_tep_va_ngoai_pham_vi_la_err() {
    // Mỗi engine một tệp DB riêng (cùng kịch bản), so từng bước.
    fn mot_luot(nhan: &str, vm: bool) -> Vec<(i32, String, String)> {
        let d = tam(&format!("tep-{nhan}"));
        let db = d.join("a.db");
        let tao = r#"    match sqlite.open(db):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
        let mo = r#"    match sqlite.open(db):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
        let ddl_sau_42 = r#"    match sqlite.open(db):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t2 (b INTEGER)", [])
            let _ = sqlite.execute(h, "INSERT INTO t2 VALUES (7)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i2 ON t2 (b)", [])
            let _ = sqlite.execute(h, "DROP TABLE t2", [])
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
        let mut kq = Vec::new();
        let tao_r = chay_tren_tep(&format!("{nhan}-tao"), &db, tao, vm);
        assert_eq!(
            (tao_r.0, tao_r.1.as_str()),
            (0, "0\n"),
            "{nhan}: tệp mới ⇒ 0; stderr: {}",
            tao_r.2
        );
        kq.push(tao_r);
        let mo0 = chay_tren_tep(&format!("{nhan}-mo0"), &db, mo, vm);
        assert_eq!(
            (mo0.0, mo0.1.as_str()),
            (0, "0\n"),
            "{nhan}: đóng/mở lại ⇒ vẫn 0; stderr: {}",
            mo0.2
        );
        kq.push(mo0);
        dat_application_id(&db, 42);
        let v42 = chay_tren_tep(&format!("{nhan}-42"), &db, ddl_sau_42, vm);
        assert_eq!(
            (v42.0, v42.1.as_str()),
            (0, "42\n42\n"),
            "{nhan}: giá trị 42 phải lưu theo tệp và DDL/ghi dữ liệu **không** đổi; stderr: {}",
            v42.2
        );
        kq.push(v42);
        let mo42 = chay_tren_tep(&format!("{nhan}-mo42"), &db, mo, vm);
        assert_eq!(
            (mo42.0, mo42.1.as_str()),
            (0, "42\n"),
            "{nhan}: đóng/mở lại phải giữ 42; stderr: {}",
            mo42.2
        );
        kq.push(mo42);
        // Magic number ứng dụng điển hình (0x1234_5678) — vẫn trong `0..=i32::MAX`.
        dat_application_id(&db, 0x1234_5678);
        let vmagic = chay_tren_tep(&format!("{nhan}-magic"), &db, mo, vm);
        assert_eq!(
            (vmagic.0, vmagic.1.as_str()),
            (0, "305419896\n"),
            "{nhan}: application_id 0x12345678 phải đọc đúng; stderr: {}",
            vmagic.2
        );
        kq.push(vmagic);
        dat_application_id(&db, i32::MAX);
        let vmax = chay_tren_tep(&format!("{nhan}-max"), &db, mo, vm);
        assert_eq!(
            (vmax.0, vmax.1.as_str()),
            (0, "2147483647\n"),
            "{nhan}: i32::MAX phải đọc đúng; stderr: {}",
            vmax.2
        );
        kq.push(vmax);
        for (tag, v) in [("am", -1), ("min", i32::MIN)] {
            dat_application_id(&db, v);
            let r = chay_tren_tep(&format!("{nhan}-{tag}"), &db, mo, vm);
            assert_eq!(
                r.0, 0,
                "{nhan}: application_id={v} phải là Err dữ liệu; stderr: {}",
                r.2
            );
            assert!(
                r.1.starts_with("loi ") && r.1.contains("application_id"),
                "{nhan}: giá trị {v} (ngoài 0..=i32::MAX) phải là Err dữ liệu nêu \
                 `application_id`, được {:?}; stderr: {}",
                r.1,
                r.2
            );
            kq.push(r);
        }
        let _ = std::fs::remove_dir_all(&d);
        kq
    }

    let a = mot_luot("interp", false);
    let b = mot_luot("vm", true);
    assert_eq!(
        a, b,
        "interp ≠ `--vm` trên tệp DB thật\n  interp: {a:?}\n  vm: {b:?}"
    );
}

/// Authorizer **chặn ghi** `PRAGMA application_id = …` (chỉ đọc); bảng người dùng trùng tên TVF
/// `pragma_application_id` **không** ảnh hưởng đường đọc pragma. Giá trị vẫn `0`.
#[test]
fn ghi_pragma_bi_chan_va_tvf_trung_ten_khong_anh_huong() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.execute(h, "PRAGMA application_id = 5", []):
                Ok(_): print("ghi khong duoc phep")
                Err(_): print("ghi bi chan")
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            match sqlite.query(h, "SELECT * FROM pragma_application_id", []):
                Ok(rs): print("tvf {rs.rows[0][0]}")
                Err(e): print("tvf loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE pragma_application_id (application_id INTEGER)", [])
            let _ = sqlite.execute(h, "INSERT INTO pragma_application_id VALUES (999)", [])
            match sqlite.query(h, "SELECT * FROM pragma_application_id", []):
                Ok(rs): print("che {rs.rows[0][0]}")
                Err(e): print("che loi {e.message}")
            match sqlite.application_id(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("ghi", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "ghi bi chan\n0\ntvf Integer(0)\nche Integer(999)\n0\n",
        "ghi pragma phải bị authorizer chặn; TVF trùng tên bị che; pragma vẫn đọc đúng 0"
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
            match sqlite.application_id(h):
                Ok(_): print("dong khong duoc phep")
                Err(_): print("dong bi chan")
            match sqlite.application_id("db:00000000000000000000000000000000"):
                Ok(_): print("la khong duoc phep")
                Err(_): print("la bi chan")
            match sqlite.application_id("khong-phai-handle"):
                Ok(_): print("sai dang khong duoc phep")
                Err(_): print("sai dang bi chan")
            match sqlite.application_id(""):
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
        ("kieu-int", "sqlite.application_id(1)", "VS0412"),
        ("kieu-bool", "sqlite.application_id(true)", "VS0412"),
        ("arity-0", "sqlite.application_id()", "VS0401"),
        ("arity-2", "sqlite.application_id(\"h\", \"x\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
