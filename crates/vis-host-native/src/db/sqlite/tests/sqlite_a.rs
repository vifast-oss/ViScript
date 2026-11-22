// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-296c — nhóm ca sqlite (sqlite_a); tách nguyên văn, không đổi tên ca.

use super::*;

#[test]
fn moi_co_version_va_compile_options_that() {
    let (ver, cc) = thong_tin_sqlite();
    assert!(ver.starts_with("3."), "SQLite version lạ: {ver}");
    assert!(
        cc.iter().any(|o| o.contains("ENABLE_FTS5")),
        "bundled phải có ENABLE_FTS5, thấy: {cc:?}"
    );
}

#[test]
fn token_128_bit_hex_khong_doan_duoc_khong_tai_su_dung() {
    let mut d = Db::moi(false);
    let a = mo_db(&mut d);
    let b = mo_db(&mut d);
    // Format: `db:` + đúng 32 hex thường (128 bit).
    for h in [&a, &b] {
        assert!(h.starts_with("db:"), "{h}");
        assert_eq!(h.len(), 35, "{h}");
        assert!(
            h[3..]
                .bytes()
                .all(|x| matches!(x, b'0'..=b'9' | b'a'..=b'f')),
            "{h}"
        );
    }
    assert_ne!(a, b, "hai lần mở không sinh cùng token");
    d.db_close(&a).unwrap();
    let c = mo_db(&mut d);
    assert_ne!(c, a);
    assert_ne!(c, b);
    // Handle cũ (tombstone) vẫn báo đã đóng, không trỏ nhầm connection mới.
    let e = d.db_execute(&a, "SELECT 1", &[]).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
    // Token cũ không được cứu sống bằng bất kỳ biến thể nào của nó.
    for bien_the in [
        a.to_uppercase(),
        format!("{a} "),
        format!(" {a}"),
        format!("{}0", &a[..34]),
    ] {
        let e = d.db_execute(&bien_the, "SELECT 1", &[]).unwrap_err();
        assert!(!e.is_empty());
    }
    // Handle đã đóng không được trỏ sang connection mới: đóng a, mở mới ⇒ token mới khác a.
    let e = d.db_close(&a).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
}

/// **VD-259** — OS RNG hai nửa 64 bit: 512 lần mở (đóng ngay) không trùng token, vẫn dưới cap
/// handle 1_024. Bắt hồi quy về seed một nguồn 64 bit/thời gian đơn thuần.
#[test]
fn token_moi_lan_mo_khong_trung_512_lan() {
    let mut d = Db::moi(false);
    let mut da_co = std::collections::HashSet::new();
    for i in 0..512 {
        let h = mo_db(&mut d);
        assert!(da_co.insert(h.clone()), "lần {i}: token trùng {h}");
        d.db_close(&h).unwrap();
    }
    assert_eq!(da_co.len(), 512);
}

/// Nguồn OS thật đọc được 2 lần khác nhau (Unix: `/dev/urandom`). Nếu môi trường không có
/// `/dev/urandom` thì fallback vẫn chạy — test này chỉ áp dụng Unix và bỏ qua khi `None`.
#[cfg(unix)]
#[test]
fn nguon_ngau_nhien_os_hai_lan_khac_nhau() {
    if let (Some(a), Some(b)) = (ngau_nhien_os(), ngau_nhien_os()) {
        assert_ne!(a, b, "hai lần đọc OS RNG không được trùng");
    }
}

/// Fallback std `RandomState` (được seed từ OS) tách miền + bộ đếm ⇒ hai nửa khác nhau giữa các
/// lần gọi; dùng khi `/dev/urandom` không có (Windows/nền tảng khác).
#[test]
fn entropy_du_phong_tach_mien_va_bo_dem() {
    let (a1, b1) = ngau_nhien_du_phong(1);
    let (a2, b2) = ngau_nhien_du_phong(2);
    assert_ne!((a1, b1), (a2, b2), "bộ đếm phải đổi token");
    assert_ne!(a1, b1, "hai miền SipHash không được trùng");
}

#[test]
fn token_gia_sai_dang_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let hex32 = format!("db:{}", "0".repeat(32));
    assert!(token_dung_dang(&hex32));
    for bad in [
        "db:0",
        "db:00",
        "db:2",
        "db:99",
        "db:1x",
        "db:",
        "tcp:0",
        "DB:2",
        // hex nhưng hoa — từ chối (chỉ hex thường).
        "db:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        // hex giả — đúng dạng ký tự nhưng chưa mở trong phiên này.
        "db:deadbeefdeadbeefdeadbeefdeadbeef",
        "1",
        "",
    ] {
        let e = d.db_execute(bad, "SELECT 1", &[]).unwrap_err();
        if bad.starts_with("db:")
            && bad.len() == 35
            && bad[3..]
                .bytes()
                .all(|x| matches!(x, b'0'..=b'9' | b'a'..=b'f'))
        {
            assert!(e.contains("không tồn tại"), "handle {bad:?}: {e}");
        } else {
            assert!(e.contains("sai dạng"), "handle {bad:?}: {e}");
        }
    }
    // Token hợp lệ nhưng đóng rồi ⇒ "đã đóng" (không rơi về nhánh khác).
    d.db_close(&h).unwrap();
    let e = d.db_execute(&h, "SELECT 1", &[]).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
    // Không panic, không trap — chỉ `Err`.
}

#[test]
fn uri_ten_rong_memory_thua_bi_tu_choi() {
    let mut d = Db::moi(false);
    for p in [
        "",
        "file:/tmp/x.db",
        "FILE:/tmp/x.db",
        ":memory:x",
        ":MEMORY:",
    ] {
        let e = d.db_open(p, true, 5_000).unwrap_err();
        assert!(!e.is_empty(), "path {p:?}: {e}");
    }
    // `:memory:` thật thì hợp lệ.
    assert!(d.db_open(":memory:", true, 5_000).is_ok());
}

#[test]
fn busy_ngoai_khoang_bi_tu_choi() {
    let mut d = Db::moi(false);
    for ms in [-1, 60_001, i64::MAX] {
        let e = d.db_open(":memory:", true, ms).unwrap_err();
        assert!(e.contains("busy_timeout_ms"), "busy {ms}: {e}");
    }
    assert!(d.db_open(":memory:", true, 60_000).is_ok());
}

#[test]
fn connection_thu_17_bi_tu_choi() {
    let mut d = Db::moi(false);
    for _ in 0..MAX_CONNECTIONS {
        mo_db(&mut d);
    }
    let e = d.db_open(":memory:", true, 5_000).unwrap_err();
    assert!(e.contains("16"), "{e}");
}

#[test]
fn create_false_thieu_tep_la_loi_du_lieu_khong_phai_du_lieu_cat() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-create-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("khong-ton-tai.db");
    let mut d = Db::moi(false);
    let e = d.db_open(p.to_str().unwrap(), false, 5_000).unwrap_err();
    assert!(!e.starts_with(DAU_QUYEN), "không phải lỗi quyền: {e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multi_statement_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_execute(&h, "SELECT 1; SELECT 2", &[]).unwrap_err();
    assert!(e.contains("multi-statement"), "{e}");
    let e2 = d.db_query(&h, "SELECT 1; SELECT 2", &[]).unwrap_err();
    assert!(e2.contains("multi-statement"), "{e2}");
}

#[test]
fn bind_thieu_thua_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER, b INTEGER)", &[])
        .unwrap();
    let thieu = d
        .db_execute(&h, "INSERT INTO t VALUES (?, ?)", &[sv_int(1)])
        .unwrap_err();
    assert!(thieu.contains("SQL cần"), "{thieu}");
    let thua = d
        .db_execute(
            &h,
            "INSERT INTO t VALUES (?, ?)",
            &[sv_int(1), sv_int(2), sv_int(3)],
        )
        .unwrap_err();
    assert!(thua.contains("SQL cần"), "{thua}");
    // Thiếu nhưng **đúng với câu SQL** thì ok.
    assert!(d
        .db_execute(&h, "INSERT INTO t VALUES (?, ?)", &[sv_int(1), sv_int(2)])
        .is_ok());
}

#[test]
fn sql_qua_64kib_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let sql = format!("SELECT '{}'", "x".repeat(MAX_SQL_BYTES));
    let e = d.db_query(&h, &sql, &[]).unwrap_err();
    assert!(e.contains("65536"), "{e}");
}

#[test]
fn params_qua_1000_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let params: Vec<Value> = (0..(MAX_PARAMS + 1) as i64).map(sv_int).collect();
    let e = d.db_execute(&h, "SELECT 1", &params).unwrap_err();
    assert!(e.contains("1000"), "{e}");
}

#[test]
fn query_vuot_10000_hang_err_khong_tra_du_lieu_cat() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 10001) SELECT x FROM c",
            &[],
        )
        .unwrap_err();
    assert!(e.contains("10000"), "{e}");
}

/// **VD-266** — `query_one` dừng sau hàng đầu: CTE 10_001 hàng (vượt cap [`MAX_ROWS`] của
/// `query`) vẫn `Ok` đúng 1 hàng. Nếu host materialize cả kết quả rồi cắt, ca này sẽ `Err`.
#[test]
fn query_one_dung_sau_hang_dau_khong_vuot_cap_hang() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let (cols, rows) = d
        .db_query_one(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 10001) SELECT x FROM c",
            &[],
        )
        .expect("query_one phải dừng sau hàng đầu");
    assert_eq!(cols, vec!["x".to_string()]);
    assert_eq!(rows.len(), 1, "chỉ được trả 1 hàng: {rows:?}");
    assert_eq!(o_int(&rows), 1);
}

#[test]
fn query_one_khong_va_mot_hang() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    let (cols, rows) = d
        .db_query_one(&h, "SELECT a FROM t WHERE a = ?", &[sv_int(1)])
        .expect("0 hàng vẫn Ok");
    assert_eq!(cols, vec!["a".to_string()]);
    assert!(rows.is_empty(), "0 hàng ⇒ rows rỗng: {rows:?}");

    d.db_execute(&h, "INSERT INTO t (a) VALUES (7)", &[])
        .unwrap();
    let (_, rows) = d.db_query_one(&h, "SELECT a FROM t", &[]).expect("1 hàng");
    assert_eq!(rows.len(), 1);
    assert_eq!(o_int(&rows), 7);
}
