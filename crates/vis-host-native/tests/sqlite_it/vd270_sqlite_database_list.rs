// SPDX-License-Identifier: Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-database-list)** — `sqlite.database_list(handle) -> Result[list[RowSet], Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Danh sách database đang gắn với connection, đọc bằng `PRAGMA database_list` (câu **hằng**,
//!   không bind tham số) qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host).
//! - **Mỗi hàng** của pragma thành **một `RowSet`** giống `sqlite.query`: `columns` =
//!   `[seq, name, file]`, `rows` đúng **một** hàng cùng thứ tự cột; ô giữ nguyên `SqlValue`
//!   (`Integer(seq)`, `Text(name)`, `Text(file)`). Danh sách sắp theo `seq` tăng dần.
//! - **Probe shape thật** (bundled SQLite 3.50.2, cả hai engine): database `:memory:` **mới** trả
//!   **hai** hàng — `main` (`seq = 0`, `file = ""`) và `temp` (`seq = 1`, `file = ""`). Database
//!   **tệp** trả `main` với `file` = đường dẫn tệp (đúng chuỗi đã mở) và `temp` với `file = ""`.
//!   Ghi chú: yêu cầu ban đầu của task kỳ vọng `:memory:` mới chỉ có `main`; probe cho thấy SQLite
//!   bundled luôn kèm schema `temp`, nên test khoá **shape thật** thay vì tự lọc `temp`.
//! - **ATTACH qua `sqlite.execute` bị authorizer của host từ chối** (mã SQLite 23, chính sách
//!   VD-241/VD-259: cấm ATTACH/DETACH ở **mọi** chế độ). Vì vậy test `attach_bi_chan...` khoá
//!   **hợp đồng bảo mật hiện có**: attach bị chặn, danh sách **không đổi**. Đường `database_list`
//!   vẫn xử lý đúng hàng `aux` khi host trả về (unit test host giả trong
//!   `crates/vis-interp/src/stdlib/sqlitedblist.rs`). Đây là **gap có chủ ý** giữa yêu cầu task và
//!   chính sách host, không phải lỗi hàm.
//! - **Probe authorizer**: trước khi mở, `PRAGMA database_list` bị từ chối «quyền (authorizer)
//!   (mã SQLite 23)», TVF `pragma_database_list` cũng bị từ chối (cùng kiểm tra `SQLITE_PRAGMA`).
//!   Host mở **đúng một mục đọc** `database_list` (không kèm giá trị) trong allow-list
//!   `SQLITE_PRAGMA`; ghi `PRAGMA database_list = …` vẫn bị từ chối. `sqlite3_stmt_readonly` của
//!   pragma này là **true** nên **không** cần nới chính sách chỉ đọc như `journal_mode`.
//! - Handle thiếu/đã đóng/sai dạng, lỗi host/SQL, kết quả sai hình dạng ⇒ `Err(Error)` **dữ liệu**,
//!   không trap.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit; ca dùng tệp DB chạy hai engine trên **cùng** tệp (hàm chỉ đọc).
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — cây nhánh này, base `17b31cb`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_sqlite_database_list
//! running 5 tests
//! test memory_tra_main_temp_file_rong ... FAILED
//! test tep_db_duong_dan_main_va_temp_rong ... FAILED
//! test attach_bi_chan_danh_sach_giu_nguyen ... FAILED
//! test handle_dong_va_sai_dang_la_err_du_lieu ... FAILED
//! test sai_kieu_sai_arity_bi_chan ... FAILED
//! test result: FAILED. 0 passed; 5 failed
//! # mọi ca đỏ vì `VS0301` «module `std.sqlite` không có hàm `database_list`»
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
        "vis-vd270-sqlite-database-list-{tag}-{}-{n}",
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
/// đường interp.
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

/// `:memory:` **mới** — probe bundled SQLite 3.50.2 trả `main` + `temp`, cả hai `file` rỗng;
/// mỗi hàng là một `RowSet` 3 cột `seq,name,file` đúng **một** hàng, sắp theo `seq`.
fn kiem_memory(ds: &str) {
    assert_eq!(
        ds,
        "so=2\n\
         cot=[seq, name, file]\n\
         hang=1\n\
         db=[Integer(0), Text(main), Text()]\n\
         cot=[seq, name, file]\n\
         hang=1\n\
         db=[Integer(1), Text(temp), Text()]\n",
        "shape `:memory:` phải là main + temp, file rỗng"
    );
}

#[test]
fn memory_tra_main_temp_file_rong() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.database_list(h):
                Ok(ds):
                    print("so={ds.len()}")
                    for rs in ds:
                        print("cot={rs.columns}")
                        print("hang={rs.rows.len()}")
                        print("db={rs.rows[0]}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tt (a INTEGER)", [])
            match sqlite.database_list(h):
                Ok(ds): print("sau-temp so={ds.len()} main={ds[0].rows[0][1]}")
                Err(e): print("sau-temp loi {e.message}")
"#;
    let (code, out, err) = hai_engine("memory", SRC);
    assert_eq!(code, 0, "phải chạy xong\nstdout: {out}\nstderr: {err}");
    assert_eq!(err, "", "không được có stderr: {err}");
    let (head, tail) = out
        .split_once("sau-temp so=")
        .unwrap_or_else(|| panic!("thiếu dòng sau-temp trong output: {out:?}"));
    kiem_memory(head);
    assert_eq!(
        tail, "2 main=Text(main)\n",
        "CREATE TEMP TABLE không thêm database mới: {tail:?}"
    );
}

/// Database **tệp** — `main.file` là đúng đường dẫn đã mở; `temp.file` rỗng. Hai engine chạy
/// **cùng** tệp (hàm chỉ đọc) nên byte-identical, không cần đường dẫn riêng.
#[test]
fn tep_db_duong_dan_main_va_temp_rong() {
    let d = tam("tep");
    let db = d.join("du-lieu.db");
    let duong = db.to_string_lossy().to_string();
    let duong_src = duong.replace('\\', "\\\\");
    let src = format!(
        r#"use std.sqlite

fn main():
    match sqlite.open("{duong_src}"):
        Err(e): print("mo loi {{e.message}}")
        Ok(h):
            match sqlite.database_list(h):
                Ok(ds):
                    print("so={{ds.len()}}")
                    for rs in ds:
                        print("cot={{rs.columns}}")
                        print("hang={{rs.rows.len()}}")
                        print("db={{rs.rows[0]}}")
                Err(e): print("loi {{e.message}}")
"#
    );
    let p = ghi(&d, "tep", &src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    assert_eq!(a, b, "tep: interp ≠ `--vm`\n  interp: {a:?}\n  vm: {b:?}");
    let (code, out, err) = a;
    assert_eq!(code, 0, "phải chạy xong\nstdout: {out}\nstderr: {err}");
    assert_eq!(err, "", "không được có stderr: {err}");
    assert_eq!(
        out,
        format!(
            "so=2\n\
             cot=[seq, name, file]\n\
             hang=1\n\
             db=[Integer(0), Text(main), Text({duong})]\n\
             cot=[seq, name, file]\n\
             hang=1\n\
             db=[Integer(1), Text(temp), Text()]\n"
        ),
        "shape DB tệp: main mang đường dẫn, temp rỗng"
    );
    assert!(db.exists(), "tệp DB phải tồn tại: {}", db.display());
    let _ = std::fs::remove_dir_all(&d);
}

/// **Gap có chủ ý**: ATTACH qua `sqlite.execute` bị authorizer của host từ chối ở mọi chế độ
/// (VD-241/VD-259); vì vậy danh sách **không đổi** — không thể chứng minh "ATTACH xuất hiện" qua
/// đường người dùng mà không nới chính sách bảo mật. Hàm vẫn trả đúng hàng `aux` khi host có nó
/// (unit test host giả trong lõi).
#[test]
fn attach_bi_chan_danh_sach_giu_nguyen() {
    let d = tam("attach");
    let aux = d.join("aux.db");
    let duong = aux.to_string_lossy().to_string();
    let duong_src = duong.replace('\\', "\\\\");
    let src = format!(
        r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {{e.message}}")
        Ok(h):
            match sqlite.execute(h, "ATTACH '{duong_src}' AS aux", []):
                Ok(n): print("attach-ok {{n}}")
                Err(e): print("attach-bi-chan")
            match sqlite.database_list(h):
                Ok(ds):
                    print("so={{ds.len()}} ten1={{ds[0].rows[0][1]}} seq1={{ds[0].rows[0][0]}}")
                    print("ten2={{ds[1].rows[0][1]}} seq2={{ds[1].rows[0][0]}}")
                Err(e): print("loi {{e.message}}")
"#
    );
    let (code, out, err) = hai_engine("attach", &src);
    assert_eq!(code, 0, "phải chạy xong\nstdout: {out}\nstderr: {err}");
    assert_eq!(err, "", "không được có stderr: {err}");
    assert_eq!(
        out,
        "attach-bi-chan\nso=2 ten1=Text(main) seq1=Integer(0)\nten2=Text(temp) seq2=Integer(1)\n",
        "ATTACH phải bị chặn và danh sách giữ nguyên main+temp"
    );
    assert!(
        !aux.exists(),
        "ATTACH bị chặn không được tạo tệp: {}",
        aux.display()
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Handle đã đóng/sai dạng/rỗng ⇒ `Err(Error)` dữ liệu, không trap; connection mới vẫn mở được
/// và danh sách đúng.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.close(h):
                Ok(_): print("dong-ok")
                Err(e): print("dong loi {e.message}")
            match sqlite.database_list(h):
                Ok(ds): print("sau-dong ok so={ds.len()}")
                Err(e): print("sau-dong loi {e.message}")
            match sqlite.database_list("db:0"):
                Ok(ds): print("sai-dang ok so={ds.len()}")
                Err(e): print("sai-dang loi {e.message}")
            match sqlite.database_list(""):
                Ok(ds): print("rong ok so={ds.len()}")
                Err(e): print("rong loi {e.message}")
            match sqlite.open(":memory:"):
                Ok(h2):
                    match sqlite.database_list(h2):
                        Ok(ds): print("h2 so={ds.len()} ten={ds[0].rows[0][1]}")
                        Err(e): print("h2 loi {e.message}")
                Err(e): print("mo2 loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "phải chạy xong\nstdout: {out}\nstderr: {err}");
    assert_eq!(err, "", "không được có stderr: {err}");
    let dong = out.lines().collect::<Vec<_>>();
    assert_eq!(dong[0], "dong-ok", "đóng connection phải Ok: {out:?}");
    for (i, nhan) in ["sau-dong loi", "sai-dang loi", "rong loi"]
        .iter()
        .enumerate()
    {
        assert!(
            dong[i + 1].starts_with(nhan),
            "dòng {} phải là `{nhan}` — Err dữ liệu, không trap: {out:?}",
            i + 1
        );
    }
    assert_eq!(
        dong[4], "h2 so=2 ten=Text(main)",
        "connection mới phải đọc lại được: {out:?}"
    );
}

/// Sai kiểu (`int` thay `str`) ⇒ `VS0412`; thiếu/thừa đối số ⇒ `VS0401`; cả hai engine từ chối
/// tĩnh, không chạy chương trình.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    tu_choi(
        "sai-kieu",
        r#"use std.sqlite

fn main():
    let h = 1
    match sqlite.database_list(h):
        _: print("x")
"#,
        "VS0412",
    );
    tu_choi(
        "thieu-arity",
        r#"use std.sqlite

fn main():
    match sqlite.database_list():
        _: print("x")
"#,
        "VS0401",
    );
    tu_choi(
        "thua-arity",
        r#"use std.sqlite

fn main():
    let h = "x"
    match sqlite.database_list(h, h):
        _: print("x")
"#,
        "VS0401",
    );
    // Module-only: không có sugar `h.database_list()` (method trên `str`).
    tu_choi(
        "sugar",
        r#"use std.sqlite

fn main():
    let h = "x"
    match h.database_list():
        _: print("x")
"#,
        "VS0403",
    );
}
