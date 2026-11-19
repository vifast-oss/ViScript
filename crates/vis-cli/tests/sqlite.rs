// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-241 (Gate 1, 2026-11-19)** — `std.sqlite` e2e: **rusqlite bundled thật**, host thật;
//! mọi ca chạy **cả hai đường** (`run` và `run --vm --no-cache`) và so **từng byte**
//! stdout/stderr/exit. Không mạng, không đĩa ngoài thư mục tạm; `:memory:` là chính.
//!
//! Bổ sung cho corpus `test/sem/sqlite/` (chạy qua `vis test`): các ca cần **tệp** (lock/WAL/
//! strict-caps) hoặc **nhiều lần chạy** (rò tài nguyên) nằm ở đây.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// **Chống flake** — các ca đo thời gian chạy **tuần tự**.
fn kho() -> std::sync::MutexGuard<'static, ()> {
    static KHO: Mutex<()> = Mutex::new(());
    KHO.lock().unwrap_or_else(|e| e.into_inner())
}

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-sqlite-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(format!("{ten}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
    p
}

fn chay(p: &Path, vm: bool, explain: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run");
    if vm {
        c.arg("--vm");
        c.arg("--no-cache");
    }
    if explain {
        c.arg("--explain-perms");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// **Review độc lập (2026-11-19)** — ép `SqlValue.Null()` (constructor zero-arg, mẫu `Json.Null()`)
/// **phải chạy** ở cả hai đường (trước báo cáo ghi sai là "không tồn tại ở tầng giá trị" — P3).
#[test]
fn sqlvalue_null_ctor_chay() {
    let d = tam("sqlvalue-null");
    let (rc, out, err) = ca(
        &d,
        "sqlvalue_null_ctor",
        "use std.sqlite\nfn main() -> ():\n    let v = SqlValue.Null()\n    match v:\n        Null: print(\"null-ctor\")\n        _: print(\"other\")\n",
    );
    assert_eq!(rc, 0, "interp rc khác 0; stderr: {err}");
    assert_eq!(out, "null-ctor\n", "interp phải in null-ctor");
    let _ = std::fs::remove_dir_all(&d);
}

/// Chạy **hai đường** + so byte; trả kết quả đường interp.
fn ca(d: &Path, ten: &str, src: &str) -> (i32, String, String) {
    let _g = kho();
    let p = ghi(d, ten, src);
    let a = chay(&p, false, false);
    let b = chay(&p, true, false);
    assert_eq!(
        a, b,
        "interp ≠ `--vm --no-cache`\n  interp: {a:?}\n  vm: {b:?}"
    );
    a
}

/// Như [`ca`] nhưng không khoá (ca không đo thời gian, không đụng đĩa dùng chung).
fn ca_song_song(d: &Path, ten: &str, src: &str) -> (i32, String, String) {
    ca(d, ten, src)
}

const MO_DONG: &str = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Err(e): print("loi {e.message}")
        Ok(h):
            match sqlite.close(h):
                Ok(_): print("mo dong ok")
                Err(e): print("loi dong {e.message}")
"#;

#[test]
fn mo_va_dong_memory_hai_duong() {
    let d = tam("mo");
    let (code, out, err) = ca(&d, "mo-dong", MO_DONG);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "mo dong ok\n");
}

#[test]
fn ro_canh_bao_chi_diagnostics() {
    let d = tam("leak");
    // Program mở 2 connection, không close — rò phải được đếm khi phiên kết thúc.
    let src = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Ok(_): print("mo 1")
        Err(e): print("loi 1")
    match sqlite.open(":memory:"):
        Ok(_): print("mo 2")
        Err(e): print("loi 2")
"#;
    let p = ghi(&d, "ro-canh-bao", src);
    // Mặc định: **không** in gì về rò ra stderr (chỉ diagnostics mới in).
    for vm in [false, true] {
        let (code, _, err) = chay(&p, vm, false);
        assert_eq!(code, 0, "vm={vm}: {err}");
        assert!(
            !err.to_lowercase().contains("leak"),
            "vm={vm}: stderr mặc định không được báo rò, được: {err}"
        );
    }
    // Diagnostics (`--explain-perms`): phải in dòng rò.
    for vm in [false, true] {
        let (code, out, err) = chay(&p, vm, true);
        assert_eq!(code, 0, "vm={vm}: {err}");
        assert_eq!(out, "mo 1\nmo 2\n");
        assert!(
            err.contains("SQLITE leak: 2 connection"),
            "vm={vm}: diagnostics phải báo rò 2 connection, được: {err}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn nhieu_lan_chay_trong_cung_mot_process() {
    let d = tam("leak2");
    let src = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Ok(_): print("ok")
        Err(e): print("loi {e.message}")
"#;
    let p = ghi(&d, "ro-nhieu-lan", src);
    // 20 lần chạy trên CÙNG tiến trình test — mỗi lần phải báo đúng rò của lần đó (không tích luỹ).
    for i in 0..20 {
        let (code, _, err) = chay(&p, i % 2 == 0, true);
        assert_eq!(code, 0, "lần {i}: {err}");
        assert!(
            err.contains("SQLITE leak: 1 connection"),
            "lần {i} (vm={}): rò phải đúng 1, được: {err}",
            i % 2 == 0
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn hai_ket_noi_tranh_khoa_busy_timeout() {
    let d = tam("lock");
    let db = d.join("app.db");
    let src = format!(
        r#"use std.sqlite
fn main():
    match sqlite.open_with({:?}, true, 150):
        Err(e): print("loi mo 1 {{e.message}}")
        Ok(h1):
            match sqlite.open_with({:?}, true, 150):
                Err(e): print("loi mo 2 {{e.message}}")
                Ok(h2):
                    let _ = sqlite.execute(h1, "CREATE TABLE t (a INTEGER)", [])
                    let _ = sqlite.begin(h1)
                    let _ = sqlite.execute(h1, "INSERT INTO t VALUES (1)", [])
                    match sqlite.execute(h2, "INSERT INTO t VALUES (2)", []):
                        Ok(_): print("ghi thu 2: khong duoc phu")
                        Err(e): print("ghi thu 2 bi chan: {{e.message}}")
                    let _ = sqlite.rollback(h1)
                    let _ = sqlite.close(h1)
                    let _ = sqlite.close(h2)
                    print("xong")
"#,
        db.to_str().unwrap(),
        db.to_str().unwrap()
    );
    let (code, out, err) = ca(&d, "tranh-khoa", &src);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("ghi thu 2 bi chan: "),
        "ghi tranh chấp phải lỗi, được: {out}"
    );
    assert_eq!(out.lines().last(), Some("xong"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn strict_caps_tu_choi_file_backed() {
    let d = tam("strict");
    // File-backed với --strict-caps ⇒ VS0604, exit 2 — kể cả khi cap[Dir] phủ đúng thư mục.
    let db = d.join("app.db");
    let src_file = format!(
        r#"use std.sqlite
fn main():
    match sqlite.open({:?}):
        Ok(_): print("khong duoc phu")
        Err(e): print("chan {{e.message}}")
"#,
        db.to_str().unwrap()
    );
    let p = ghi(&d, "file-backed", &src_file);
    for vm in [false, true] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--strict-caps");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        let o = c.arg(&p).output().expect("chạy `vis run --strict-caps`");
        let code = o.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&o.stderr).to_string();
        assert_eq!(code, 2, "vm={vm}: exit 2 ([quyền]), được {code}: {stderr}");
        assert!(stderr.contains("VS0604"), "vm={vm}: {stderr}");
        assert!(stderr.contains("[quyền]"), "vm={vm}: {stderr}");
    }
    // `:memory:` với --strict-caps thì vẫn được phép.
    let p2 = ghi(&d, "memory", MO_DONG);
    for vm in [false, true] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--strict-caps");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        let o = c.arg(&p2).output().expect("chạy `vis run --strict-caps`");
        assert_eq!(o.status.code(), Some(0));
        assert_eq!(o.stdout, b"mo dong ok\n");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn uri_va_ten_rong_bi_tu_choi() {
    let d = tam("uri");
    let src = r#"use std.sqlite
fn main():
    match sqlite.open("file:/tmp/khong-duoc.db"):
        Ok(_): print("URI: khong duoc phu")
        Err(e): print("URI: {e.message}")
"#;
    let (code, out, err) = ca(&d, "uri", src);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("URI: "), "phải là lỗi dữ liệu, được: {out}");
    let src2 = r#"use std.sqlite
fn main():
    match sqlite.open(""):
        Ok(_): print("ten rong: khong duoc phu")
        Err(e): print("ten rong: {e.message}")
"#;
    let (code2, out2, err2) = ca(&d, "ten-rong", src2);
    assert_eq!(code2, 0, "{err2}");
    assert!(
        out2.starts_with("ten rong: "),
        "phải là lỗi dữ liệu, được: {out2}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn create_false_thieu_tep_la_err_du_lieu() {
    let d = tam("cfalse");
    let db = d.join("khong-ton-tai.db");
    let src = format!(
        r#"use std.sqlite
fn main():
    match sqlite.open_with({:?}, false, 5000):
        Ok(_): print("khong duoc phu")
        Err(e): print("chan {{e.message}}")
"#,
        db.to_str().unwrap()
    );
    let (code, out, err) = ca(&d, "create-false", &src);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("chan "), "phải là lỗi dữ liệu: {out}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn bind_thieu_hai_duong_cung_stderr() {
    let d = tam("bind");
    let src = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Err(e): print("loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b INTEGER)", [])
            match sqlite.execute(h, "INSERT INTO t VALUES (?, ?)", [SqlValue.Integer(1)]):
                Ok(_): print("khong duoc phu")
                Err(e): print("chan {e.message}")
"#;
    let (code, out, err) = ca(&d, "bind-thieu", src);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("chan "), "phải là lỗi dữ liệu, được: {out}");
}

#[test]
fn attach_bi_chan_va_khong_tao_tep() {
    let d = tam("attach");
    let db = d.join("xx-khong-duoc.db");
    // **psvn (Windows, 2026-11-19)**: `C:\…` backslash nhúng thô vào chuỗi `.vis` ⇒ VS0102.
    // Dùng `/` — Windows/SQLite chấp nhận và chuỗi không cần escape.
    let path_for_sql = db.to_string_lossy().replace('\\', "/");
    let src = format!(
        r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Err(e): print("loi {{e.message}}")
        Ok(h):
            match sqlite.execute(h, "ATTACH '{path}' AS x", []):
                Ok(_): print("attach: KHONG DUOC PHU")
                Err(e): print("attach bi chan")
            match sqlite.execute(h, "VACUUM INTO '{path}'", []):
                Ok(_): print("vacuum into: KHONG DUOC PHU")
                Err(e): print("vacuum into bi chan")
            let _ = sqlite.close(h)
            print("xong")
"#,
        path = path_for_sql
    );
    let (code, out, err) = ca_song_song(&d, "attach-chan", &src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out, "attach bi chan\nvacuum into bi chan\nxong\n",
        "authorizer phải chặn cả hai"
    );
    assert!(
        !db.exists(),
        "VACUUM INTO / ATTACH không được tạo tệp: {}",
        db.display()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn phien_ban_sqlite_duoc_log_o_diagnostics() {
    let d = tam("ver");
    let p = ghi(&d, "phien-ban", MO_DONG);
    for vm in [false, true] {
        let (code, _, err) = chay(&p, vm, true);
        assert_eq!(code, 0);
        assert!(
            err.contains("SQLITE version=3."),
            "vm={vm}: diagnostics phải in version, được: {err}"
        );
        assert!(
            err.contains("compile_options="),
            "vm={vm}: diagnostics phải in compile_options, được: {err}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn db_da_co_wal_van_mo_duoc() {
    let d = tam("wal");
    let db = d.join("wal.db");
    // Tạo WAL thật bằng rusqlite trực tiếp.
    {
        let c = rusqlite::Connection::open(&db).expect("tạo wal.db");
        c.pragma_update(None, "journal_mode", "WAL").unwrap();
        c.execute("CREATE TABLE t (a INTEGER)", []).unwrap();
        c.execute("INSERT INTO t VALUES (7)", []).unwrap();
        c.close().unwrap();
    }
    let src = format!(
        r#"use std.sqlite
fn main():
    match sqlite.open({:?}):
        Err(e): print("loi {{e.message}}")
        Ok(h):
            match sqlite.query(h, "SELECT a FROM t", []):
                Err(e): print("loi query {{e.message}}")
                Ok(rs):
                    for row in rs.rows:
                        for cell in row:
                            print(cell)
            let _ = sqlite.close(h)
"#,
        db.to_str().unwrap()
    );
    let (code, out, err) = ca(&d, "wal-open", &src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "Integer(7)\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// Wall clock 5 s mỗi câu SQL (hợp đồng VD-241 §6) — `progress_handler` ngắt cứng. Chậm ~5 s.
#[test]
fn gioi_han_thoi_gian_5s_bi_ngat() {
    let d = tam("wallclock");
    let src = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Err(e): print("loi {e.message}")
        Ok(h):
            match sqlite.query(h, "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000000) SELECT count(*) FROM c", []):
                Ok(_): print("khong duoc phu")
                Err(e): print("dung: {e.message}")
            let _ = sqlite.close(h)
"#;
    let p = ghi(&d, "wallclock", src);
    let t0 = std::time::Instant::now();
    let (code, out, err) = chay(&p, false, false);
    let dt = t0.elapsed();
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("OperationInterrupted"),
        "query dài phải bị ngắt ở 5 s (mã SQLite OperationInterrupted), được: {out}"
    );
    assert!(dt < std::time::Duration::from_secs(30), "quá lâu: {dt:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **VD-241 pre-merge §8 (2026-11-19)** — thông điệp lỗi mặc định **không** chứa SQL fragment /
/// bound values / nội dung câu lệnh; chỉ mã SQLite + tên lớp lỗi (+ offset nếu có từ prepare).
/// `--sqlite-debug` (opt-in) hiển thị thông điệp thô.
#[test]
fn thong_diep_mac_dinh_khong_lo_fragment() {
    let d = tam("sqlite-debug");
    let src = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Err(e): print("loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            match sqlite.execute(h, "INSERT INTO t VALUES ('O'Clock; SELECT secret')", []):
                Ok(_): print("khong duoc phu")
                Err(e): print("chan {e.message}")
            let _ = sqlite.close(h)
"#;
    let p = ghi(&d, "weird", src);
    for vm in [false, true] {
        let (code, out, err) = chay(&p, vm, false);
        assert_eq!(code, 0, "vm={vm}: {err}");
        assert!(
            !out.contains("O'Clock") && !out.contains("secret"),
            "vm={vm}: thông điệp mặc định lộ fragment: {out}"
        );
        assert!(
            out.contains("mã SQLite"),
            "vm={vm}: phải có mã SQLite: {out}"
        );
        // Opt-in: thông điệp thô (có thể chứa fragment) chỉ hiện khi user chủ động bật.
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--sqlite-debug");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        let o = c.arg(&p).output().expect("chạy `vis run --sqlite-debug`");
        let out2 = String::from_utf8_lossy(&o.stdout).to_string();
        assert_eq!(o.status.code(), Some(0));
        assert!(
            out2.contains("O'Clock") || out2.contains("secret"),
            "vm={vm}: --sqlite-debug phải thấy nội dung thô: {out2}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// **VD-241 pre-merge §10 (2026-11-19)** — `:memory:` dưới workload lớn: 50_000 hàng,
/// `SELECT sum(x), avg(x)` — tổng output < 16 MiB; chạy **cả hai engine**, so byte.
#[test]
fn workload_50k_rows_sum_avg_ca_hai_duong() {
    let d = tam("workload-50k");
    let src = r#"use std.sqlite
fn main():
    match sqlite.open(":memory:"):
        Err(e): print("loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (x INTEGER)", [])
            let kq = sqlite.execute(h, "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 50000) INSERT INTO t SELECT x FROM c", [])
            match kq:
                Ok(n): print("inserted {n}")
                Err(e): print("loi insert {e.message}")
            match sqlite.query(h, "SELECT sum(x), avg(x) FROM t", []):
                Err(e): print("loi query {e.message}")
                Ok(rs):
                    for row in rs.rows:
                        for cell in row:
                            match cell:
                                Integer(n): print("int {n}")
                                Real(f): print("real {f}")
                                _: print("khac")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = ca(&d, "workload-50k", src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "inserted 50000\nint 1250025000\nreal 25000.5\n");
    let _ = std::fs::remove_dir_all(&d);
}
