// SPDX-License-Identifier: Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-journal-mode)** — `sqlite.journal_mode(handle) -> Result[str, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **chuỗi chế độ journal hiện tại** của connection, đọc bằng `PRAGMA journal_mode`
//!   (không tham số) qua **đúng** đường `sqlite.query`/`DbHost::db_query` hiện có (không thêm
//!   method host, **không** ghi/DDL). Giá trị là chuỗi SQLite trả: `delete`/`truncate`/`persist`/
//!   `memory`/`wal`/`off`; hợp đồng **không** hứa danh sách đóng — chuỗi khác vẫn trả nguyên văn.
//! - Database `:memory:` **mới** ⇒ `"memory"`; database **tệp mới** ⇒ `"delete"` (probe cả hai
//!   engine bên dưới). **Đọc lặp không đổi mode** — `PRAGMA journal_mode` không tham số là truy
//!   vấn chỉ đọc.
//! - **Đường đọc đã chọn**: `PRAGMA journal_mode` (không phải `SELECT * FROM
//!   pragma_journal_mode`). **Probe** authorizer trước khi mở quyền: `PRAGMA journal_mode` bị
//!   từ chối «quyền (authorizer) (mã SQLite 23)», và TVF `pragma_journal_mode` cũng đi qua kiểm
//!   tra `SQLITE_PRAGMA` với chính tên `journal_mode` nên cũng bị từ chối. Host mở **đúng một
//!   mục đọc** `journal_mode` (không kèm giá trị) trong allow-list `SQLITE_PRAGMA`; ghi
//!   `PRAGMA journal_mode = …` vẫn bị **từ chối** (test `ghi_pragma_bi_chan...`). Khi cả hai
//!   đường đều qua, chọn **pragma** vì TVF `pragma_journal_mode` **có thể bị bảng người dùng
//!   trùng tên che** (probe: bảng `pragma_journal_mode` trả `wal`), còn pragma không có không
//!   gian tên đó. Câu SQL là **hằng**, không bind tham số — không có bề mặt injection.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!   Kết quả sai hình dạng (không đúng một cột/hàng, ô không phải `SqlValue.Text`) cũng là
//!   `Err` dữ liệu — kiểm ở unit test host giả của
//!   `crates/vis-runtime/src/stdlib/sqlite/schema.rs`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit; ca dùng tệp DB thì mỗi engine một tệp riêng.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — cây `main` @ `1734ab5`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_sqlite_journal_mode
//! running 5 tests
//! test sai_kieu_sai_arity_bi_chan ... FAILED
//! test tep_moi_delete_va_dong_mo_lai_khong_doi ... FAILED
//! test handle_dong_va_sai_dang_la_err_du_lieu ... FAILED
//! test memory_mac_dinh_va_doc_lap_khong_doi ... FAILED
//! test ghi_pragma_bi_chan_va_tvf_trung_ten_khong_anh_huong ... FAILED
//! test result: FAILED. 0 passed; 5 failed
//! # mọi ca đỏ vì `VS0301` «module `std.sqlite` không có hàm `journal_mode`»
//! # (ca sai kiểu/arity cũng đỏ VS0301 vì hàm chưa tồn tại, chưa tới VS0412/VS0401)
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
        "vis-vd270-sqlite-journal-mode-{tag}-{}-{n}",
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
/// đường interp. `sqlite.journal_mode` chỉ đọc nên không cần tệp DB riêng từng engine khi nguồn
/// dùng `:memory:`.
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

/// `:memory:` mới ⇒ `"memory"`; đọc lặp **không** đổi mode; DDL/ghi dữ liệu vẫn `"memory"`;
/// giá trị là `str` dùng được như chuỗi.
#[test]
fn memory_mac_dinh_va_doc_lap_khong_doi() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.journal_mode(h):
                Ok(v): print("lan1={v}")
                Err(e): print("loi {e.message}")
            match sqlite.journal_mode(h):
                Ok(v): print("lan2={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            match sqlite.journal_mode(h):
                Ok(v): print("sau-ghi={v}")
                Err(e): print("loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(_): print("doc-ok")
                Err(e): print("doc loi {e.message}")
            match sqlite.journal_mode(h):
                Ok(v):
                    let s: str = v
                    print("sau-doc={s}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("memory", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        dong,
        [
            "lan1=memory",
            "lan2=memory",
            "sau-ghi=memory",
            "doc-ok",
            "sau-doc=memory"
        ],
        "`:memory:` mới ⇒ memory; đọc lặp/DDL/ghi dữ liệu **không** đổi mode: {dong:?}\n\
         stderr: {err}"
    );
}

/// Database **tệp mới** ⇒ `"delete"`; đọc lặp và đóng/mở lại **không** đổi mode; DDL/ghi dữ liệu
/// không đổi; hai engine byte-identical (mỗi engine một tệp riêng, cùng kịch bản).
#[test]
fn tep_moi_delete_va_dong_mo_lai_khong_doi() {
    fn mot_luot(nhan: &str, vm: bool) -> Vec<(i32, String, String)> {
        let d = tam(&format!("tep-{nhan}"));
        let db = d.join("a.db");
        let tao = r#"    match sqlite.open(db):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.journal_mode(h):
                Ok(v): print("lan1={v}")
                Err(e): print("loi {e.message}")
            match sqlite.journal_mode(h):
                Ok(v): print("lan2={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            match sqlite.journal_mode(h):
                Ok(v): print("sau-ghi={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
        let mo = r#"    match sqlite.open(db):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.journal_mode(h):
                Ok(v): print("mo-lai={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.query(h, "SELECT count(*) FROM t", [])
            match sqlite.journal_mode(h):
                Ok(v): print("sau-doc={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
        let mut kq = Vec::new();
        let tao_r = chay_tren_tep(&format!("{nhan}-tao"), &db, tao, vm);
        assert_eq!(
            (tao_r.0, tao_r.1.as_str()),
            (0, "lan1=delete\nlan2=delete\nsau-ghi=delete\n"),
            "{nhan}: tệp mới ⇒ delete; đọc lặp/DDL/ghi dữ liệu không đổi; stderr: {}",
            tao_r.2
        );
        kq.push(tao_r);
        let mo_r = chay_tren_tep(&format!("{nhan}-mo"), &db, mo, vm);
        assert_eq!(
            (mo_r.0, mo_r.1.as_str()),
            (0, "mo-lai=delete\nsau-doc=delete\n"),
            "{nhan}: đóng/mở lại vẫn delete; stderr: {}",
            mo_r.2
        );
        kq.push(mo_r);
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

/// Một lượt chạy nguồn trên tệp DB thật, trả `(exit, stdout, stderr)`.
fn chay_tren_tep(tag: &str, db: &Path, than: &str, vm: bool) -> (i32, String, String) {
    // Dùng `/` trong literal VS (SQLite nhận cả `\` trên Windows); đường dẫn tệp tạm không chứa
    // dấu `"` nên không cần escape thêm.
    let db = db.display().to_string().replace('\\', "/");
    let src = format!("use std.sqlite\n\nfn main():\n    let db = \"{db}\"\n{than}");
    chay_nguon(tag, &src, vm)
}

/// Authorizer **chặn ghi** `PRAGMA journal_mode = …` (chỉ đọc); mode vẫn `memory`. Đường đọc
/// `PRAGMA journal_mode` qua `sqlite.query` (cùng authorizer) trả `Text(memory)`; bảng người dùng
/// trùng tên TVF `pragma_journal_mode` **che** TVF nhưng **không** ảnh hưởng pragma.
#[test]
fn ghi_pragma_bi_chan_va_tvf_trung_ten_khong_anh_huong() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.execute(h, "PRAGMA journal_mode = WAL", []):
                Ok(_): print("ghi khong duoc phep")
                Err(_): print("ghi bi chan")
            match sqlite.execute(h, "PRAGMA journal_mode = DELETE", []):
                Ok(_): print("ghi khong duoc phep")
                Err(_): print("ghi bi chan")
            match sqlite.journal_mode(h):
                Ok(v): print("mode={v}")
                Err(e): print("loi {e.message}")
            match sqlite.query(h, "PRAGMA journal_mode", []):
                Ok(rs): print("pragma={rs.rows[0][0]}")
                Err(e): print("pragma loi {e.message}")
            match sqlite.query(h, "SELECT * FROM pragma_journal_mode", []):
                Ok(rs): print("tvf={rs.rows[0][0]}")
                Err(e): print("tvf loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE pragma_journal_mode (journal_mode TEXT)", [])
            let _ = sqlite.execute(h, "INSERT INTO pragma_journal_mode VALUES ('wal')", [])
            match sqlite.query(h, "SELECT * FROM pragma_journal_mode", []):
                Ok(rs): print("che={rs.rows[0][0]}")
                Err(e): print("che loi {e.message}")
            match sqlite.journal_mode(h):
                Ok(v): print("mode-sau={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("ghi", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "ghi bi chan\nghi bi chan\nmode=memory\npragma=Text(memory)\ntvf=Text(memory)\n\
         che=Text(wal)\nmode-sau=memory\n",
        "ghi pragma phải bị authorizer chặn; TVF trùng tên bị che nhưng pragma vẫn đọc đúng \
         memory"
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
            match sqlite.journal_mode(h):
                Ok(_): print("dong khong duoc phep")
                Err(_): print("dong bi chan")
            match sqlite.journal_mode("db:00000000000000000000000000000000"):
                Ok(_): print("la khong duoc phep")
                Err(_): print("la bi chan")
            match sqlite.journal_mode("khong-phai-handle"):
                Ok(_): print("sai dang khong duoc phep")
                Err(_): print("sai dang bi chan")
            match sqlite.journal_mode(""):
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
        ("kieu-int", "sqlite.journal_mode(1)", "VS0412"),
        ("kieu-bool", "sqlite.journal_mode(true)", "VS0412"),
        ("arity-0", "sqlite.journal_mode()", "VS0401"),
        ("arity-2", "sqlite.journal_mode(\"h\", \"x\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
