// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-296c — nhóm ca sqlite (sqlite_c); tách nguyên văn, không đổi tên ca.

use super::*;

#[test]
fn thoi_gian_chay_nho_bi_ngat_nhanh_khong_phai_doi_5s() {
    let mut d = Db::moi_voi_query_ms(false, 100);
    let h = mo_db(&mut d);
    let t0 = Instant::now();
    let e = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000000) SELECT count(*) FROM c",
            &[],
        )
        .unwrap_err();
    let dt = t0.elapsed();
    assert!(e.contains("OperationInterrupted"), "{e}");
    assert!(dt < Duration::from_secs(5), "quá lâu: {dt:?}");
    // Handler được reset — câu sau chạy bình thường.
    let (_, rows) = d.db_query(&h, "SELECT 1", &[]).unwrap();
    assert_eq!(o_int(&rows), 1);
}

#[test]
fn sqlite_debug_in_thong_diep_tho_ro_fragment_chi_khi_bat() {
    // Câu lỗi có fragment nhạy cảm trong SQL.
    let sql = "INSERT INTO khong_co_bang VALUES ('O'Clock; SELECT secret')";
    // Mặc định: thông điệp **không** chứa fragment.
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_execute(&h, sql, &[]).unwrap_err();
    assert!(
        !e.contains("O'Clock"),
        "thông điệp mặc định lộ fragment: {e}"
    );
    assert!(
        !e.contains("secret"),
        "thông điệp mặc định lộ fragment: {e}"
    );
    assert!(e.contains("mã SQLite"), "phải có mã SQLite: {e}");
    // `--sqlite-debug`: thông điệp thô được hiển thị (user chủ động bật).
    let mut d2 = Db::moi(false);
    d2.dat_sqlite_debug(true);
    let h2 = mo_db(&mut d2);
    let e2 = d2.db_execute(&h2, sql, &[]).unwrap_err();
    assert!(
        e2.contains("O'Clock") || e2.contains("secret"),
        "--sqlite-debug phải thấy nội dung thô: {e2}"
    );
}

#[test]
fn loi_offset_duoc_giu_tu_loi_prepare() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_execute(&h, "SELEC 1", &[]).unwrap_err();
    assert!(e.contains("mã SQLite"), "{e}");
    assert!(e.contains("offset"), "phải giữ offset: {e}");
    assert!(!e.contains("SELEC"), "không được lộ SQL text: {e}");
}

fn chinh_sach_tai(action: AuthAction<'_>) -> Authorization {
    let ctx = AuthContext {
        action,
        database_name: None,
        accessor: None,
    };
    chinh_sach(ctx)
}

#[test]
fn chinh_sach_action_code_ma_tran() {
    use rusqlite::hooks::TransactionOperation::*;
    use AuthAction::*;
    let allow = [
        CreateTable { table_name: "t" },
        DropTable { table_name: "t" },
        AlterTable {
            database_name: "main",
            table_name: "t",
        },
        CreateIndex {
            index_name: "i",
            table_name: "t",
        },
        DropIndex {
            index_name: "i",
            table_name: "t",
        },
        Insert { table_name: "t" },
        Update {
            table_name: "t",
            column_name: "a",
        },
        Delete { table_name: "t" },
        Select,
        Read {
            table_name: "t",
            column_name: "a",
        },
        Recursive,
        Transaction { operation: Begin },
        Transaction { operation: Release },
        Transaction {
            operation: Rollback,
        },
        // `COMMIT` từ SQLite map về `Unknown` — vẫn phải Allow.
        Transaction { operation: Unknown },
        Savepoint {
            operation: Begin,
            savepoint_name: "sp",
        },
        Savepoint {
            operation: Release,
            savepoint_name: "sp",
        },
        Reindex { index_name: "i" },
        Analyze { table_name: "t" },
        CreateView { view_name: "v" },
        DropView { view_name: "v" },
        CreateTrigger {
            trigger_name: "tg",
            table_name: "t",
        },
        DropTrigger {
            trigger_name: "tg",
            table_name: "t",
        },
        CreateTempTable { table_name: "t" },
        DropTempTable { table_name: "t" },
        Function {
            function_name: "count",
        },
        Function {
            function_name: "randomblob",
        },
        Function {
            function_name: "sqlite_version",
        },
        Pragma {
            pragma_name: "table_info",
            pragma_value: None,
        },
        Pragma {
            pragma_name: "user_version",
            pragma_value: None,
        },
        Pragma {
            pragma_name: "schema_version",
            pragma_value: None,
        },
        Pragma {
            pragma_name: "application_id",
            pragma_value: None,
        },
        Pragma {
            pragma_name: "journal_mode",
            pragma_value: None,
        },
        Pragma {
            pragma_name: "database_list",
            pragma_value: None,
        },
    ];
    for a in allow {
        assert_eq!(
            chinh_sach_tai(a),
            Authorization::Allow,
            "phải cho phép: {a:?}"
        );
    }
    let deny = [
        Attach {
            filename: "/tmp/x.db",
        },
        Detach { database_name: "x" },
        CreateVtable {
            table_name: "t",
            module_name: "fts5",
        },
        DropVtable {
            table_name: "t",
            module_name: "fts5",
        },
        Function {
            function_name: "load_extension",
        },
        Function {
            function_name: "sqlite_load_extension",
        },
        Function {
            function_name: "LOAD_EXTENSION",
        },
        // **VD-259** — nếu build tương lai bật `SQLITE_ENABLE_OFFSET_SQL_FUNC`, offset byte
        // trong tệp cũng là layout trang thô ⇒ deny (không phụ thuộc hoa/thường).
        Function {
            function_name: "sqlite_offset",
        },
        Function {
            function_name: "SQLITE_OFFSET",
        },
        Pragma {
            pragma_name: "user_version",
            pragma_value: Some("42"),
        },
        Pragma {
            pragma_name: "user_version",
            pragma_value: Some(""),
        },
        Pragma {
            pragma_name: "application_id",
            pragma_value: Some("42"),
        },
        Pragma {
            pragma_name: "application_id",
            pragma_value: Some(""),
        },
        Pragma {
            pragma_name: "temp_store",
            pragma_value: Some("FILE"),
        },
        Pragma {
            pragma_name: "journal_mode",
            pragma_value: Some("WAL"),
        },
        Pragma {
            pragma_name: "database_list",
            pragma_value: Some("1"),
        },
        Pragma {
            pragma_name: "database_list",
            pragma_value: Some(""),
        },
        Pragma {
            pragma_name: "synchronous",
            pragma_value: Some("OFF"),
        },
        Pragma {
            pragma_name: "cache_size",
            pragma_value: Some("-1"),
        },
        Pragma {
            pragma_name: "temp_store",
            pragma_value: None,
        },
        // **VD-259** — trang/freelist thô: mọi biến thể tên (hoa/thường, có/không schema) bị chặn.
        Read {
            table_name: "dbstat",
            column_name: "name",
        },
        Read {
            table_name: "DBSTAT",
            column_name: "pageno",
        },
        Read {
            table_name: "sqlite_dbpage",
            column_name: "data",
        },
        Read {
            table_name: "sqlite_dbdata",
            column_name: "data",
        },
        Read {
            table_name: "sqlite_dbptr",
            column_name: "pgno",
        },
        AuthAction::Unknown {
            code: 999,
            arg1: None,
            arg2: None,
        },
    ];
    for a in deny {
        assert_eq!(
            chinh_sach_tai(a),
            Authorization::Deny,
            "phải từ chối: {a:?}"
        );
    }
}

/// **VD-259** — `dbstat` (bundled `SQLITE_ENABLE_DBSTAT_VTAB`) đọc trang/freelist thô; SQL người
/// dùng phải bị authorizer từ chối, không lộ layout/byte. Chứng minh module **có** trong build
/// bằng connection raw trước, rồi mới kiểm host chặn.
#[test]
fn dbstat_bi_authorizer_tu_choi_khong_doc_trang_tho() {
    // Raw connection (không authorizer): dbstat phải tồn tại trong bundled build.
    let raw = Connection::open_in_memory().unwrap();
    let n: i64 = raw
        .query_row("SELECT count(*) FROM dbstat", [], |r| r.get(0))
        .expect("bundled phải có dbstat (SQLITE_ENABLE_DBSTAT_VTAB)");
    assert!(n >= 0);
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    d.db_execute(&h, "INSERT INTO t VALUES (1)", &[]).unwrap();
    for sql in [
        "SELECT * FROM dbstat",
        "SELECT * FROM main.dbstat",
        "SELECT * FROM DBSTAT",
        "SELECT name FROM dbstat WHERE name = 't'",
        "WITH x AS (SELECT * FROM dbstat) SELECT * FROM x",
    ] {
        let e = d.db_query(&h, sql, &[]).unwrap_err();
        assert!(
            e.contains("quyền (authorizer)"),
            "{sql}: phải bị authorizer chặn, được: {e}"
        );
    }
    // Bảng/view thường + schema vẫn đọc được (chặn đúng phạm vi).
    assert!(d.db_query(&h, "SELECT a FROM t", &[]).is_ok());
    assert!(d
        .db_query(&h, "SELECT name FROM sqlite_master", &[])
        .is_ok());
    // `execute` cũng không được dùng để sao chép trang thô sang bảng thường.
    let e = d
        .db_execute(&h, "CREATE TABLE copy AS SELECT * FROM dbstat", &[])
        .unwrap_err();
    assert!(e.contains("quyền (authorizer)"), "{e}");
    // `table_info(dbstat)` chỉ là metadata, không đọc trang — vẫn chạy (không lộ byte).
    assert!(d.db_query(&h, "PRAGMA table_info(dbstat)", &[]).is_ok());
}

#[test]
fn chinh_sach_ham_load_extension_bi_cam() {
    for f in ["load_extension", "sqlite_load_extension", "LOAD_EXTENSION"] {
        assert_eq!(
            chinh_sach_tai(AuthAction::Function { function_name: f }),
            Authorization::Deny,
            "{f}"
        );
    }
}

/// **F-A (2026-11-19)** — đường dẫn cắt ngang codepoint đa byte ở byte 120 không được panic.
#[test]
fn mo_ta_path_utf8_split_tai_byte_120_khong_panic() {
    // 118 'a' (byte 0..117) + 'é' (byte 118..119) + tiếp byte 120 là giữa dấu nhọn
    let path = format!("{}étail.db", "a".repeat(118));
    let d = super::mo_ta(&path);
    assert!(d.ends_with('…'), "kỳ vọng giữ tới ranh giới byte + dấu …");
    // đường open (tạo file trong /tmp) cũng không được panic
    let d_tmp = std::env::temp_dir().join(format!("vis-f-a-{}.db", std::process::id()));
    let mut db = super::Db::moi_voi_query_ms(false, 5_000);
    let _ = db.db_open(&d_tmp.to_string_lossy(), true, 5_000);
    let _ = std::fs::remove_file(&d_tmp);
}

// ===========================================================================
// VD-258 — cap bộ nhớ SQLite (ô/overhead/tên cột), pre-materialization,
// schema không-UTF-8 và cap tổng handle.
// ===========================================================================

/// `Db` với trần tuỳ ý — test ép cap thấp, không cần dữ liệu khổng lồ.
pub(super) fn db_gioi_han(f: impl FnOnce(&mut GioiHan)) -> Db {
    let mut d = Db::moi(false);
    f(&mut d.gioi_han);
    d
}

#[test]
fn cap_o_chan_bang_rong_nhieu_cot() {
    let mut d = db_gioi_han(|g| g.cells = 10);
    let h = mo_db(&mut d);
    // 3 hàng × 4 cột = 12 ô > 10 ⇒ Err (không trả dữ liệu cắt).
    let e = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 3) \
             SELECT x, x, x, x FROM c",
            &[],
        )
        .unwrap_err();
    assert!(e.contains("10") && e.contains("ô"), "{e}");
    // 2 hàng × 4 cột = 8 ô ⇒ vẫn qua.
    let (_, rows) = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 2) \
             SELECT x, x, x, x FROM c",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn overhead_o_null_tinh_vao_tong_byte() {
    // Một hàng 2 ô NULL: payload 0 nhưng overhead 2×64 = 128 > 100 ⇒ phải bị chặn.
    let mut d = db_gioi_han(|g| g.total_bytes = 100);
    let h = mo_db(&mut d);
    let e = d.db_query(&h, "SELECT NULL, NULL", &[]).unwrap_err();
    assert!(e.contains("100") && e.contains("overhead"), "{e}");
    // 1 ô NULL: 64 ≤ 100 ⇒ qua (và chứng minh cap không chặn oan payload nhỏ).
    let (_, rows) = d.db_query(&h, "SELECT NULL", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn tran_cung_chan_zeroblob_truoc_khi_cap_phat() {
    let mut d = db_gioi_han(|g| {
        g.row_bytes = 4 << 10;
        g.hard_cell_bytes = 8 << 10;
    });
    let h = mo_db(&mut d);
    // 1 MiB > trần cứng 8 KiB ⇒ SQLite báo TOOBIG, không materialize 1 MiB; host map thành
    // lỗi cap hàng (không trả blob cắt).
    let t0 = Instant::now();
    let e = d.db_query(&h, "SELECT zeroblob(1048576)", &[]).unwrap_err();
    assert!(e.contains("4096"), "{e}");
    assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    // Trong trần cứng và trần hàng ⇒ đọc được.
    let (_, rows) = d.db_query(&h, "SELECT zeroblob(4096)", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

/// **VD-259** — đường **đọc** hạ `SQLITE_LIMIT_LENGTH` về trần hàng (`min(row_bytes,
/// hard_cell_bytes)`) trước step, nên cell 1–17 MiB không materialize rồi mới bị cap hàng từ
/// chối. Đường **ghi** vẫn dùng trần cứng (row 8 KiB ghi được dù trần đọc 4 KiB).
#[test]
fn doc_ha_tran_cung_ve_tran_hang_ghi_van_dung_tran_cung() {
    let mut d = db_gioi_han(|g| {
        g.row_bytes = 4 << 10;
        g.hard_cell_bytes = 64 << 10;
    });
    let h = mo_db(&mut d);
    // `length(zeroblob(8192))` cho kết quả 8 byte: nếu chỉ cap sau khi đọc, nó **thành công**
    // (cell 8 KiB < trần cứng 64 KiB). Với trần đọc 4 KiB, zeroblob bị TOOBIG ngay tại step ⇒
    // host map thành lỗi cap hàng ⇒ chứng minh trần đọc đã hạ thật.
    let e = d
        .db_query(&h, "SELECT length(zeroblob(8192))", &[])
        .unwrap_err();
    assert!(e.contains("4096") && e.contains("một hàng"), "{e}");
    // Đúng trần đọc vẫn đọc được (limit là max, không phải exclusive).
    let (_, rows) = d
        .db_query(&h, "SELECT length(zeroblob(4096))", &[])
        .unwrap();
    assert_eq!(o_int(&rows), 4096);
    // Đường ghi dùng trần cứng 64 KiB: row 8 KiB ghi được (không bị trần đọc 4 KiB).
    d.db_execute(&h, "CREATE TABLE b (x BLOB)", &[]).unwrap();
    d.db_execute(&h, "INSERT INTO b VALUES (zeroblob(8192))", &[])
        .unwrap();
    let (_, rows) = d.db_query(&h, "SELECT count(*) FROM b", &[]).unwrap();
    assert_eq!(o_int(&rows), 1);
    // Trần cứng vẫn chặn ghi > 64 KiB.
    let e = d
        .db_execute(&h, "INSERT INTO b VALUES (zeroblob(65537))", &[])
        .unwrap_err();
    assert!(e.contains("TooBig") || e.contains("mã SQLite"), "{e}");

    // Mặc định (đọc 1 MiB): cell 2 MiB bị chặn **trước** khi materialize, kể cả khi kết quả chỉ
    // là `length(...)` 8 byte — nếu chỉ cap sau khi đọc, câu này sẽ trả 2097152.
    let mut mac_dinh = Db::moi(false);
    let h2 = mo_db(&mut mac_dinh);
    let e = mac_dinh
        .db_query(&h2, "SELECT length(zeroblob(2097152))", &[])
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");
}

#[test]
fn zeroblob_512m_bi_chan_voi_rss_huu_han() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    #[cfg(target_os = "linux")]
    let truoc = vm_hwm_kb();
    let t0 = Instant::now();
    let e = d
        .db_query(&h, "SELECT zeroblob(536870912)", &[])
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");
    assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    // Trước đây `zeroblob(512 MiB)` materialize ~519 MB. Với trần cứng, mức tăng HWM phải nhỏ.
    #[cfg(target_os = "linux")]
    if let (Some(a), Some(b)) = (truoc, vm_hwm_kb()) {
        let tang_kb = b.saturating_sub(a);
        assert!(
            tang_kb < 256 * 1024,
            "RSS tăng {tang_kb} KiB — trần cứng không chặn materialize?"
        );
    }
}

#[cfg(target_os = "linux")]
fn vm_hwm_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    for d in s.lines() {
        if let Some(rest) = d.strip_prefix("VmHWM:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

#[test]
fn tong_handle_cap_ke_ca_tombstone() {
    let mut d = db_gioi_han(|g| g.handles = 4);
    for _ in 0..4 {
        let h = mo_db(&mut d);
        d.db_close(&h).unwrap();
    }
    let e = d.db_open(":memory:", true, 5_000).unwrap_err();
    assert!(e.contains('4') && e.contains("handle"), "{e}");
}

/// Ghi tệp SQLite, thay **nguyên văn** chuỗi `tim` trong byte schema bằng `thay` (cùng độ dài,
/// byte không-UTF-8). Dùng để tái hiện schema ngoài bị hỏng encoding — không qua host.
fn tao_db_schema_hong(
    dir: &std::path::Path,
    ten: &str,
    sql: &str,
    tim: &str,
    thay: &[u8],
) -> String {
    assert_eq!(tim.len(), thay.len(), "thay phải cùng độ dài byte");
    let p = dir.join(ten);
    {
        let c = Connection::open(&p).unwrap();
        c.execute_batch(sql).unwrap();
        c.close().unwrap();
    }
    let mut b = std::fs::read(&p).unwrap();
    let cu = tim.as_bytes();
    let mut n = 0usize;
    let mut i = 0usize;
    while i + cu.len() <= b.len() {
        if &b[i..i + cu.len()] == cu {
            b[i..i + cu.len()].copy_from_slice(thay);
            n += 1;
            i += cu.len();
        } else {
            i += 1;
        }
    }
    assert!(n > 0, "không tìm thấy {tim:?} trong schema để thay");
    std::fs::write(&p, &b).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn schema_khong_utf8_tra_loi_du_lieu_khong_panic() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-schema-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // "cột" = 5 byte UTF-8; thay bằng 5 byte 0xFF (không-UTF-8, không NUL).
    let path = tao_db_schema_hong(
        &dir,
        "bad.db",
        "CREATE TABLE t (\"cột\" TEXT); INSERT INTO t VALUES ('x');",
        "cột",
        &[0xff; 5],
    );
    let mut d = Db::moi(false);
    let h = d.db_open(&path, false, 5_000).expect("mở được tệp");
    let e = d.db_query(&h, "SELECT * FROM t", &[]).unwrap_err();
    assert!(
        e.contains("UTF-8"),
        "phải là lỗi schema không-UTF-8 (không panic), được: {e}"
    );
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

/// **VD-259** — precheck UTF-8 không được chặn oan schema **hợp lệ** có định danh Unicode: bảng/
/// cột tiếng Việt vẫn tạo/đọc qua host bình thường, tên cột trả đúng.
#[test]
fn schema_utf8_hop_le_voi_dinh_danh_unicode_van_doc_duoc() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE \"bảng\" (\"cột\" TEXT)", &[])
        .unwrap();
    d.db_execute(&h, "INSERT INTO \"bảng\" VALUES ('x')", &[])
        .unwrap();
    let (cols, rows) = d.db_query(&h, "SELECT \"cột\" FROM \"bảng\"", &[]).unwrap();
    assert_eq!(cols, vec!["cột".to_string()]);
    assert_eq!(rows.len(), 1);
}

/// **VD-261 (tối ưu)** — schema UTF-8 chỉ được quét lại khi `schema_version` đổi.
/// Không quét lại mỗi `execute`/`query` (regression đo 6,8× với schema 300 bảng).
#[test]
fn schema_utf8_cache_theo_schema_version() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[])
        .expect("create");
    // DDL vừa đổi schema_version; một lời gọi kế tiếp làm ấm cache rồi mới đo.
    d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_int(0)])
        .expect("insert warm");
    let sau_create = schema_scans_for_test();
    for i in 1..21 {
        d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_int(i)])
            .expect("insert");
    }
    assert_eq!(
        schema_scans_for_test(),
        sau_create,
        "schema không đổi ⇒ không được quét lại mỗi lời gọi"
    );
    d.db_execute(&h, "ALTER TABLE t ADD COLUMN b TEXT", &[])
        .expect("alter");
    // DDL đổi version **sau** lần kiểm đầu của chính câu DDL; lời gọi kế tiếp phải quét lại.
    d.db_execute(&h, "INSERT INTO t (a) VALUES (?)", &[sv_int(99)])
        .expect("insert after ddl");
    assert!(
        schema_scans_for_test() > sau_create,
        "DDL đổi schema_version ⇒ lời gọi kế tiếp phải quét lại"
    );
}

#[test]
fn ten_cot_khong_utf8_tra_loi_du_lieu_khong_panic() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-cot-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // View alias không-UTF-8: authorizer có thể chỉ thấy cột gốc `a` (hợp lệ); đường
    // `Statement::column_name` của rusqlite sẽ panic nếu host dùng nó. Host phải trả Err.
    let path = tao_db_schema_hong(
        &dir,
        "bad-view.db",
        "CREATE TABLE t (a TEXT); CREATE VIEW v AS SELECT a AS \"cột\" FROM t;",
        "cột",
        &[0xff; 5],
    );
    let mut d = Db::moi(false);
    let h = d.db_open(&path, false, 5_000).expect("mở được tệp");
    let r = d.db_query(&h, "SELECT * FROM v", &[]);
    let e = r.expect_err("tên cột không-UTF-8 phải là lỗi dữ liệu");
    assert!(
        e.contains("UTF-8") || e.contains("quyền (authorizer)") || e.contains("mã SQLite"),
        "phải là lỗi dữ liệu rõ, được: {e}"
    );
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
