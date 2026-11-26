// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — parity thư viện `interp ↔ VM` cho **66 hàm host** còn ngoài corpus
//! (`fs` 22, `sqlite` 28, `net` 10, `http` 4, `time` 2 — `JOURNAL/st314a-hau-kiem.md` §23.2).
//!
//! Mỗi ca chạy **cả hai đường** trên cùng tệp + cùng tempdir, đòi cùng stdout/exit; ca lỗi so cả
//! mã chẩn đoán + thông điệp. Chạy qua binary `vis` (đường CLI thật) để dùng đúng host native;
//! đường thư viện thuần không dựng được `FsGuard`/`NetGuard`/`DbGuard` nếu không lộ API nội bộ.
//!
//! Tệp nằm trong `misc_api_it/` và được khai `#[path]` ở `lib_api_it.rs` — **0 binary cấp 1 mới**
//! (R8). Không dùng máy `mps`; không đụng `docs/SPEC*`.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static DEM: AtomicU32 = AtomicU32::new(0);

/// 66 hàm host bắt buộc phủ — khoá với `StdId::ALL` bằng test `phu_du_66_ham_host`.
pub const HOST_66: [&str; 66] = [
    // fs (22)
    "fs.append",
    "fs.canonicalize",
    "fs.copy",
    "fs.current_dir",
    "fs.exists",
    "fs.glob",
    "fs.is_dir",
    "fs.is_file",
    "fs.is_symlink",
    "fs.list",
    "fs.mkdir",
    "fs.mkdir_all",
    "fs.read",
    "fs.read_bytes",
    "fs.read_lines",
    "fs.read_link",
    "fs.remove",
    "fs.rename",
    "fs.size",
    "fs.walk",
    "fs.write",
    "fs.write_bytes",
    // sqlite (28)
    "sqlite.application_id",
    "sqlite.begin",
    "sqlite.changes",
    "sqlite.close",
    "sqlite.columns",
    "sqlite.commit",
    "sqlite.database_list",
    "sqlite.data_version",
    "sqlite.execute",
    "sqlite.execute_batch",
    "sqlite.explain",
    "sqlite.index_columns",
    "sqlite.indexes",
    "sqlite.index_exists",
    "sqlite.in_transaction",
    "sqlite.journal_mode",
    "sqlite.last_insert_rowid",
    "sqlite.open",
    "sqlite.open_with",
    "sqlite.primary_keys",
    "sqlite.query",
    "sqlite.query_one",
    "sqlite.rollback",
    "sqlite.schema_sql",
    "sqlite.schema_version",
    "sqlite.table_exists",
    "sqlite.tables",
    "sqlite.user_version",
    // net (10)
    "net.accept",
    "net.accept_timeout",
    "net.close",
    "net.close_listen",
    "net.connect",
    "net.connect_timeout",
    "net.listen",
    "net.local_addr",
    "net.read",
    "net.write",
    // http (4)
    "http.get",
    "http.get_timeout",
    "http.head",
    "http.head_timeout",
    // time (2)
    "time.now_ms",
    "time.monotonic_ms",
];

/// Thư mục tạm riêng cho từng ca (test chạy song song), tự dọn.
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-host-parity-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Đường dẫn kiểu VS (dấu `/`, kể cả Windows) để nhúng vào chương trình.
fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// Binary `vis` của workspace — tôn trọng `CARGO_TARGET_DIR` (worktree dùng chung target).
fn vis_bin() -> PathBuf {
    let goc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| goc.join("target"));
    let ten = if cfg!(windows) { "vis.exe" } else { "vis" };
    let p = target.join("debug").join(ten);
    assert!(
        p.is_file(),
        "thiếu binary `{}` — build `cargo build -p vis-cli` trước khi chạy parity",
        p.display()
    );
    p
}

fn vis(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let mut c = Command::new(vis_bin());
    for a in args {
        c.arg(a);
    }
    let o = c.output().expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn viet_tep(p: &Path, src: &str) {
    std::fs::write(
        p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
}

/// Chạy **cả hai đường** (interp + `--vm --no-cache`) trên cùng tệp, đòi cùng stdout/exit.
/// Trả `(stdout, stderr)` để ca lỗi so tiếp mã/thông điệp.
fn hai_duong(d: &Path, tag: &str, src: &str) -> (String, String) {
    hai_duong_voi(d, tag, src, || {})
}

/// Như [`hai_duong`] nhưng gọi `setup` **trước mỗi đường** — ca ghi/xoá phải bắt đầu từ cùng
/// trạng thái fixture, không để đường interp làm bẩn dữ liệu của VM.
fn hai_duong_voi<F: Fn()>(d: &Path, tag: &str, src: &str, setup: F) -> (String, String) {
    let p = d.join(format!("{tag}.vis"));
    viet_tep(&p, src);
    setup();
    let (c1, o1, e1) = vis(&[std::ffi::OsStr::new("run"), p.as_os_str()]);
    setup();
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        p.as_os_str(),
    ]);
    assert_eq!(c1, c2, "[{tag}] exit lệch: interp {c1} / vm {c2}\nvm: {e2}");
    assert_eq!(o1, o2, "[{tag}] stdout lệch: vm [{e2}]");
    assert_eq!(e1, e2, "[{tag}] stderr lệch");
    (o1, e1)
}

/// **ST-315d** — bảng 66 tên phải khớp đúng tập host của `StdId::ALL`.
#[test]
fn phu_du_66_ham_host() {
    use vis_hir::stdlib::StdId;
    let mut tu_all: Vec<&str> = StdId::ALL
        .iter()
        .map(|id| id.as_str())
        .filter(|n| {
            ["fs.", "sqlite.", "net.", "http.", "time."]
                .iter()
                .any(|m| n.starts_with(m))
        })
        .collect();
    tu_all.sort_unstable();
    let mut bang = HOST_66.to_vec();
    bang.sort_unstable();
    assert_eq!(tu_all.len(), 66, "StdId host phải đúng 66: {tu_all:?}");
    assert_eq!(bang, tu_all, "bảng HOST_66 lệch StdId::ALL");
}

/// **ST-315d** — khoá **66/66 có ca chạy thật**: mỗi tên trong [`HOST_66`] phải xuất hiện dưới
/// dạng lời gọi `tên(` trong chính tệp này (các `const SRC` của 5 nhóm). Bảng khai báo và chú
/// thích không có dấu `(` ngay sau tên nên không tạo dương tính giả.
#[test]
fn moi_ham_host_co_loi_goi_trong_harness() {
    let src = include_str!("host_parity.rs");
    let thieu: Vec<&str> = HOST_66
        .iter()
        .copied()
        .filter(|n| !src.contains(&format!("{n}(")))
        .collect();
    assert!(
        thieu.is_empty(),
        "thiếu lời gọi host trong harness: {thieu:?}"
    );
}

/// **ST-315d** — 2 hàm `time` chỉ so **kiểu + bất biến** (đồng hồ thật, không tất định).
#[test]
fn time_hai_ham_parity_bat_bien() {
    let d = tam("time");
    let (out, err) = hai_duong(
        &d,
        "time",
        "use std.time\nfn main():\n    let a = time.now_ms()\n    let b = time.monotonic_ms()\n    print(a >= 0)\n    print(b >= 0)\n    print(a > 1700000000000)\n",
    );
    assert_eq!(out, "true\ntrue\ntrue\n", "time: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

fn dat_fixture(work: &Path) {
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work.join("sub")).expect("mkdir sub");
    std::fs::write(work.join("a.txt"), "hi\n").expect("a.txt");
    std::fs::write(work.join("sub/b.txt"), "b").expect("b.txt");
    std::fs::write(work.join("bytes.bin"), [65u8, 66, 67]).expect("bytes.bin");
    #[cfg(unix)]
    std::os::unix::fs::symlink("a.txt", work.join("link.txt")).expect("symlink");
}

/// **ST-315d** — 20 hàm `fs` tất định trên tempdir (2 hàm symlink ở ca `#[cfg(unix)]` riêng).
#[test]
fn fs_20_ham_parity_tren_tempdir() {
    let d = tam("fs");
    let work = d.join("work");
    let w = duong(&work);
    let src = format!(
        r#"use std.fs
use std.path
fn main():
    let d = "{w}"
    print(fs.exists(d + "/a.txt"))
    print(fs.exists(d + "/missing"))
    match fs.is_file(d + "/a.txt"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.is_dir(d + "/sub"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.size(d + "/a.txt"):
        Ok(n): print(n)
        Err(e): print("E")
    match fs.read(d + "/a.txt"):
        Ok(s): print(s)
        Err(e): print("E")
    match fs.read_lines(d + "/a.txt"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.read_bytes(d + "/bytes.bin"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.write(d + "/w.txt", "w\n"):
        Ok(_): print("W")
        Err(e): print("E")
    match fs.write_bytes(d + "/w.bin", [68, 69]):
        Ok(_): print("B")
        Err(e): print("E")
    match fs.append(d + "/a.txt", "more\n"):
        Ok(_): print("A")
        Err(e): print("E")
    match fs.read(d + "/a.txt"):
        Ok(s): print(s)
        Err(e): print("E")
    match fs.copy(d + "/a.txt", d + "/cp.txt"):
        Ok(_): print("C")
        Err(e): print("E")
    match fs.rename(d + "/cp.txt", d + "/mv.txt"):
        Ok(_): print("R")
        Err(e): print("E")
    print(fs.exists(d + "/mv.txt"))
    match fs.mkdir(d + "/newdir"):
        Ok(_): print("M")
        Err(e): print("E")
    match fs.mkdir_all(d + "/deep/x"):
        Ok(_): print("MA")
        Err(e): print("E")
    match fs.is_dir(d + "/newdir"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.is_dir(d + "/deep/x"):
        Ok(b): print(b)
        Err(e): print("E")
    match fs.list(d + "/sub"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.walk(d):
        Ok(xs): print(xs.len())
        Err(e): print("E")
    match fs.glob(d, "*.txt"):
        Ok(xs): print(xs)
        Err(e): print("E")
    match fs.canonicalize(d + "/a.txt"):
        Ok(p): print(path.basename(p))
        Err(e): print("E")
    match fs.current_dir():
        Ok(p): print(p.len() > 0)
        Err(e): print("E")
    match fs.remove(d + "/mv.txt"):
        Ok(_): print("RM")
        Err(e): print("E")
    print(fs.exists(d + "/mv.txt"))
"#
    );
    let (out, err) = hai_duong_voi(&d, "fs", &src, || dat_fixture(&work));
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        &dong[..6],
        &["true", "false", "true", "true", "3", "hi"],
        "6 dòng đầu: {out}\nstderr: {err}"
    );
    for token in ["A", "C", "R", "M", "MA", "RM"] {
        assert!(dong.contains(&token), "thiếu `{token}`: {out}");
    }
    assert!(dong.contains(&"[b.txt]"), "fs.list: {out}");
    assert!(out.contains("a.txt"), "canonicalize/glob: {out}");
    assert_eq!(dong.last(), Some(&"false"), "remove phải xoá: {out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-315d** — `fs.is_symlink` + `fs.read_link` (POSIX; Windows không tạo symlink không đặc quyền).
#[cfg(unix)]
#[test]
fn fs_symlink_parity() {
    let d = tam("fs-link");
    let work = d.join("work");
    let w = duong(&work);
    let src = format!(
        "use std.fs\nuse std.path\nfn main():\n    let d = \"{w}\"\n    match fs.is_symlink(d + \"/link.txt\"):\n        Ok(b): print(b)\n        Err(e): print(\"E\")\n    match fs.read_link(d + \"/link.txt\"):\n        Ok(p): print(path.basename(p))\n        Err(e): print(\"E\")\n"
    );
    let (out, err) = hai_duong_voi(&d, "fs-link", &src, || dat_fixture(&work));
    assert_eq!(out, "true\na.txt\n", "symlink: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

// ===== sqlite: 28 hàm (VD-291 §3.4) =====

/// Dựng lại thư mục DB rỗng **trước mỗi đường** — nếu không, đường interp để lại bảng/dữ liệu
/// và đường VM sẽ chạy trên trạng thái khác.
fn dat_sqlite(work: &Path) {
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work).expect("mkdir sqlite");
}

/// **ST-315d** — 13 hàm `sqlite` DML/truy vấn/giao dịch trên **tệp DB thật**, reset trước mỗi
/// engine: `open_with`, `close`, `execute`, `execute_batch`, `query`, `query_one`, `explain`,
/// `last_insert_rowid`, `changes`, `begin`, `commit`, `rollback`, `in_transaction`.
/// Mọi lỗi SQL/params/handle là `Result.Err` **dữ liệu** (exit 0), in cả thông điệp để so parity.
#[test]
fn sqlite_dml_va_giao_dich_parity() {
    const SRC: &str = r#"use std.sqlite

fn hien(v: SqlValue) -> str:
    match v:
        Null: "null"
        Integer(n): "int({n})"
        Real(f): "real({f})"
        Text(s): "text({s})"
        Blob(b): "blob({len(b)})"

fn dem(h: str) -> str:
    match sqlite.query(h, "SELECT count(*) FROM t", []):
        Ok(rs): "dem={rs.rows[0][0]}"
        Err(e): "dem loi {e.message}"

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            print("tx0={sqlite.in_transaction(h)}")
            match sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", []):
                Ok(n): print("create n={n}")
                Err(e): print("create loi {e.message}")
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(1), SqlValue.Text("mot")]):
                Ok(n): print("insert n={n}")
                Err(e): print("insert loi {e.message}")
            print("rowid={sqlite.last_insert_rowid(h)}")
            print("changes={sqlite.changes(h)}")
            match sqlite.query(h, "SELECT a, b FROM t", []):
                Ok(rs):
                    print("cols={rs.columns}")
                    print("rows={rs.rows.len()}")
                    print("cell={hien(rs.rows[0][0])} {hien(rs.rows[0][1])}")
                Err(e): print("query loi {e.message}")
            match sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(9)]):
                Ok(rs): print("q0={rs.rows.len()}")
                Err(e): print("q0 loi {e.message}")
            match sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(1)]):
                Ok(rs): print("q1={rs.rows.len()} {hien(rs.rows[0][0])}")
                Err(e): print("q1 loi {e.message}")
            match sqlite.explain(h, "SELECT a FROM t WHERE a = 1", []):
                Ok(rs): print("explain cols={rs.columns} co-plan={rs.rows.len() > 0}")
                Err(e): print("explain loi {e.message}")
            match sqlite.execute_batch(h, "INSERT INTO t (a, b) VALUES (2, 'hai'); INSERT INTO t (a, b) VALUES (3, 'ba');"):
                Ok(n): print("batch n={n}")
                Err(e): print("batch loi {e.message}")
            match sqlite.begin(h):
                Ok(_): print("begin1 ok")
                Err(e): print("begin1 loi {e.message}")
            print("tx1={sqlite.in_transaction(h)}")
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (4, 'bon')", []):
                Ok(n): print("ins-tx n={n}")
                Err(e): print("ins-tx loi {e.message}")
            print("tx1b={sqlite.in_transaction(h)}")
            match sqlite.rollback(h):
                Ok(_): print("rollback ok")
                Err(e): print("rollback loi {e.message}")
            print("tx2={sqlite.in_transaction(h)} {dem(h)}")
            let _ = sqlite.begin(h)
            let _ = sqlite.execute(h, "INSERT INTO t (a, b) VALUES (5, 'nam')", [])
            match sqlite.commit(h):
                Ok(_): print("commit ok")
                Err(e): print("commit loi {e.message}")
            print("tx3={sqlite.in_transaction(h)} {dem(h)}")
            let _ = sqlite.begin(h)
            match sqlite.begin(h):
                Ok(_): print("begin-long: KHONG DUOC PHEP")
                Err(e): print("begin-long bi chan")
            let _ = sqlite.rollback(h)
            match sqlite.commit(h):
                Ok(_): print("commit-ngoai: KHONG DUOC PHEP")
                Err(e): print("commit-ngoai bi chan")
            match sqlite.execute(h, "SELECT * FROM khong_co_bang", []):
                Ok(_): print("sql-sai: KHONG DUOC PHEP")
                Err(e): print("sql-sai {e.message}")
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(7)]):
                Ok(_): print("thieu-param: KHONG DUOC PHEP")
                Err(e): print("thieu-param {e.message}")
            match sqlite.query_one(h, "SELECT 1", [SqlValue.Integer(7)]):
                Ok(_): print("thua-param: KHONG DUOC PHEP")
                Err(e): print("thua-param {e.message}")
            match sqlite.execute(h, "DELETE FROM t", []):
                Ok(n): print("delete n={n}")
                Err(e): print("delete loi {e.message}")
            print("changes-cuoi={sqlite.changes(h)}")
            match sqlite.close(h):
                Ok(_): print("close ok")
                Err(e): print("close loi {e.message}")
            match sqlite.execute(h, "INSERT INTO t (a) VALUES (9)", []):
                Ok(_): print("sau-close: KHONG DUOC PHEP")
                Err(e): print("sau-close {e.message}")
    match sqlite.open_with("__MISS__", false, 5000):
        Ok(_): print("mo-thieu-tep: KHONG DUOC PHEP")
        Err(e): print("mo-thieu-tep bi chan")
"#;
    let d = tam("sqlite-dml");
    let work = d.join("db");
    let db = work.join("p.db");
    let src = SRC
        .replace("__DB__", &duong(&db))
        .replace("__MISS__", &duong(&work.join("khong-co.db")));
    let (out, err) = hai_duong_voi(&d, "sqlite-dml", &src, || dat_sqlite(&work));
    assert_eq!(err, "", "sqlite DML không được có stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        &dong[..],
        &[
            "tx0=Ok(false)",
            "create n=0",
            "insert n=1",
            "rowid=Ok(1)",
            "changes=Ok(1)",
            "cols=[a, b]",
            "rows=1",
            "cell=int(1) text(mot)",
            "q0=0",
            "q1=1 int(1)",
            "explain cols=[id, parent, notused, detail] co-plan=true",
            "batch n=2",
            "begin1 ok",
            "tx1=Ok(true)",
            "ins-tx n=1",
            "tx1b=Ok(true)",
            "rollback ok",
            "tx2=Ok(false) dem=Integer(3)",
            "commit ok",
            "tx3=Ok(false) dem=Integer(4)",
            "begin-long bi chan",
            "commit-ngoai bi chan",
            "sql-sai lỗi SQL (execute) (mã SQLite 1 — SqlError)",
            "thieu-param SQL cần 2 tham số, được 1",
            "thua-param SQL cần 0 tham số, được 1",
            "delete n=4",
            "changes-cuoi=Ok(4)",
            "close ok",
            "sau-close handle đã đóng",
            "mo-thieu-tep bi chan",
        ],
        "sqlite DML/tx: {out}\nstderr: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-315d** — 15 hàm `sqlite` metadata/introspection trên `:memory:` (DDL tất định):
/// `open`, `application_id`, `data_version`, `schema_version`, `user_version`, `journal_mode`,
/// `tables`, `table_exists`, `columns`, `indexes`, `index_exists`, `index_columns`,
/// `primary_keys`, `schema_sql`, `database_list`; ca lỗi: mọi hàm sau `close` ⇒ `Err` dữ liệu.
#[test]
fn sqlite_metadata_parity() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            print("appid={sqlite.application_id(h)}")
            print("userv={sqlite.user_version(h)}")
            let sv0 = match sqlite.schema_version(h):
                Ok(v): v
                Err(e): -1
            let dv0 = match sqlite.data_version(h):
                Ok(v): v
                Err(e): -1
            print("sv0-khong-am={sv0 >= 0}")
            print("datav-duong={dv0 > 0}")
            match sqlite.journal_mode(h):
                Ok(v): print("journal={v}")
                Err(e): print("journal loi {e.message}")
            match sqlite.tables(h):
                Ok(xs): print("tables0={xs}")
                Err(e): print("tables0 loi {e.message}")
            match sqlite.database_list(h):
                Ok(ds):
                    print("dbs={ds.len()}")
                    for rs in ds:
                        print("db={rs.rows[0]}")
                Err(e): print("dbs loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT UNIQUE, b REAL)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_ab ON t(b, a)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            match sqlite.tables(h):
                Ok(xs): print("tables={xs}")
                Err(e): print("tables loi {e.message}")
            print("table-t={sqlite.table_exists(h, "t")}")
            print("table-v={sqlite.table_exists(h, "v_t")}")
            print("table-x={sqlite.table_exists(h, "khong_co")}")
            match sqlite.columns(h, "t"):
                Ok(xs): print("cols={xs}")
                Err(e): print("cols loi {e.message}")
            match sqlite.indexes(h, "t"):
                Ok(xs): print("idx={xs}")
                Err(e): print("idx loi {e.message}")
            print("idx-ab={sqlite.index_exists(h, "i_ab")}")
            print("idx-auto={sqlite.index_exists(h, "sqlite_autoindex_t_1")}")
            print("idx-x={sqlite.index_exists(h, "khong_co")}")
            match sqlite.index_columns(h, "i_ab"):
                Ok(xs): print("idxcols={xs}")
                Err(e): print("idxcols loi {e.message}")
            match sqlite.primary_keys(h, "t"):
                Ok(xs): print("pk={xs}")
                Err(e): print("pk loi {e.message}")
            for ten in ["t", "v_t", "i_ab", "sqlite_autoindex_t_1", "khong_co"]:
                match sqlite.schema_sql(h, ten):
                    Ok(s):
                        match s:
                            Some(x): print("sql {ten}={x}")
                            None: print("sql {ten}=None")
                    Err(e): print("sql {ten} loi {e.message}")
            let sv1 = match sqlite.schema_version(h):
                Ok(v): v
                Err(e): -1
            let dv1 = match sqlite.data_version(h):
                Ok(v): v
                Err(e): -1
            print("sv-tang={sv1 > sv0}")
            print("datav-khong-doi={dv1 == dv0}")
            match sqlite.close(h):
                Ok(_): print("close ok")
                Err(e): print("close loi {e.message}")
            match sqlite.application_id(h):
                Ok(_): print("appid-sau-close: KHONG DUOC PHEP")
                Err(e): print("appid-sau-close {e.message}")
            match sqlite.tables(h):
                Ok(_): print("tables-sau-close: KHONG DUOC PHEP")
                Err(e): print("tables-sau-close {e.message}")
            match sqlite.journal_mode(h):
                Ok(_): print("journal-sau-close: KHONG DUOC PHEP")
                Err(e): print("journal-sau-close {e.message}")
"#;
    let d = tam("sqlite-meta");
    let (out, err) = hai_duong(&d, "sqlite-meta", SRC);
    assert_eq!(err, "", "sqlite metadata không được có stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        &dong[..],
        &[
            "appid=Ok(0)",
            "userv=Ok(0)",
            "sv0-khong-am=true",
            "datav-duong=true",
            "journal=memory",
            "tables0=[]",
            "dbs=2",
            "db=[Integer(0), Text(main), Text()]",
            "db=[Integer(1), Text(temp), Text()]",
            "tables=[t, v_t]",
            "table-t=Ok(true)",
            "table-v=Ok(true)",
            "table-x=Ok(false)",
            "cols=[id, a, b]",
            "idx=[i_ab]",
            "idx-ab=Ok(true)",
            "idx-auto=Ok(true)",
            "idx-x=Ok(false)",
            "idxcols=[b, a]",
            "pk=[id]",
            "sql t=CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT UNIQUE, b REAL)",
            "sql v_t=CREATE VIEW v_t AS SELECT a FROM t",
            "sql i_ab=CREATE INDEX i_ab ON t(b, a)",
            "sql sqlite_autoindex_t_1=None",
            "sql khong_co=None",
            "sv-tang=true",
            "datav-khong-doi=true",
            "close ok",
            "appid-sau-close handle đã đóng",
            "tables-sau-close handle đã đóng",
            "journal-sau-close handle đã đóng",
        ],
        "sqlite metadata: {out}\nstderr: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ===== net: 10 hàm (VD-291 §3.4) =====

/// **ST-315d** — trọn 10 hàm `net` trên loopback thật **trong cùng tiến trình `vis`**:
/// `listen` (lấy cổng `:0`), `local_addr`, `connect`, `accept`, `read`, `write`, `close`,
/// `accept_timeout` (không ai nối ⇒ `Err` hết giờ), `close_listen`, `connect_timeout`; kèm ca lỗi
/// listen trùng cổng / tay đã đóng / tay lạ / cổng đóng. Kết nối trước `accept` dựa vào backlog
/// TCP của kernel nên **không cần** canh thời gian giữa hai tiến trình.
#[test]
fn net_loopback_parity() {
    const SRC: &str = r#"use std.net

fn main():
    match net.listen("127.0.0.1:0"):
        Err(e): print("listen loi {e.message}")
        Ok(l):
            match net.local_addr(l):
                Ok(a):
                    print("loopback={a.starts_with("127.0.0.1:")}")
                    match net.listen(a):
                        Ok(_): print("double-listen: KHONG DUOC PHEP")
                        Err(e): print("listen busy ok")
                    match net.connect(a):
                        Ok(c):
                            match net.accept(l):
                                Ok(s):
                                    match net.write(c, "ping"):
                                        Ok(_): print("ghi-c ok")
                                        Err(e): print("ghi-c loi {e.message}")
                                    match net.read(s, 64, 500):
                                        Ok(x): print("server doc {x}")
                                        Err(e): print("server doc loi {e.message}")
                                    match net.write(s, "pong"):
                                        Ok(_): print("ghi-s ok")
                                        Err(e): print("ghi-s loi {e.message}")
                                    match net.read(c, 64, 500):
                                        Ok(x): print("client doc {x}")
                                        Err(e): print("client doc loi {e.message}")
                                    match net.close(c):
                                        Ok(_): print("dong-c ok")
                                        Err(e): print("dong-c loi {e.message}")
                                    match net.read(c, 64, 100):
                                        Ok(_): print("doc-sau-dong: KHONG DUOC PHEP")
                                        Err(e): print("doc sau dong bi chan")
                                    match net.close(s):
                                        Ok(_): print("dong-s ok")
                                        Err(e): print("dong-s loi {e.message}")
                                Err(e): print("accept loi {e.message}")
                        Err(e): print("connect loi {e.message}")
                    match net.accept_timeout(l, 80):
                        Ok(_): print("accept_timeout: KHONG DUOC PHEP")
                        Err(e): print("accept_timeout het gio")
                    match net.close_listen(l):
                        Ok(_): print("dong-l ok")
                        Err(e): print("dong-l loi {e.message}")
                    match net.close_listen(l):
                        Ok(_): print("dong-l2: KHONG DUOC PHEP")
                        Err(e): print("dong-l2 bi chan")
                Err(e): print("local_addr loi {e.message}")
    match net.connect("127.0.0.1:1"):
        Ok(_): print("connect1: KHONG DUOC PHEP")
        Err(e): print("connect tu choi")
    match net.connect_timeout("127.0.0.1:1", 100):
        Ok(_): print("ct1: KHONG DUOC PHEP")
        Err(e): print("connect_timeout tu choi")
    match net.read("C99", 8, 50):
        Ok(_): print("read C99: KHONG DUOC PHEP")
        Err(e): print("read tay la bi chan")
    match net.write("C99", "x"):
        Ok(_): print("write C99: KHONG DUOC PHEP")
        Err(e): print("write tay la bi chan")
    match net.close("C99"):
        Ok(_): print("close C99: KHONG DUOC PHEP")
        Err(e): print("close tay la bi chan")
"#;
    let d = tam("net");
    let (out, err) = hai_duong(&d, "net", SRC);
    assert_eq!(
        out,
        "loopback=true\n\
         listen busy ok\n\
         ghi-c ok\n\
         server doc ping\n\
         ghi-s ok\n\
         client doc pong\n\
         dong-c ok\n\
         doc sau dong bi chan\n\
         dong-s ok\n\
         accept_timeout het gio\n\
         dong-l ok\n\
         dong-l2 bi chan\n\
         connect tu choi\n\
         connect_timeout tu choi\n\
         read tay la bi chan\n\
         write tay la bi chan\n\
         close tay la bi chan\n",
        "net loopback: {out}\nstderr: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Server TCP loopback phục vụ `n` kết nối: đọc tới 64 byte, ghi `hi`, đóng chiều ghi.
/// `stop` + hạn 8 s giữ `join` **có hạn** (không treo khi client lỗi vì không bao giờ nối).
fn tcp_server(
    n: usize,
) -> (
    String,
    Arc<AtomicBool>,
    std::thread::JoinHandle<Vec<String>>,
) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = l.local_addr().expect("local_addr").to_string();
    l.set_nonblocking(true).expect("nonblocking loopback");
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    let h = std::thread::spawn(move || {
        let han = Instant::now() + Duration::from_secs(8);
        let mut nhan = Vec::new();
        while nhan.len() < n && !stop2.load(Ordering::Relaxed) && Instant::now() < han {
            match l.accept() {
                Ok((mut s, _)) => {
                    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
                    // Client gửi 3–4 byte một lần ⇒ đọc **một** khối là đủ; đừng chờ thêm
                    // (mỗi vòng chờ thêm 500 ms chỉ làm test chậm vô ích).
                    let mut buf = [0u8; 64];
                    let k = s.read(&mut buf).unwrap_or(0);
                    nhan.push(String::from_utf8_lossy(&buf[..k]).to_string());
                    let _ = s.write_all(b"hi");
                    let _ = s.shutdown(Shutdown::Write);
                    let mut rac = [0u8; 64];
                    while matches!(s.read(&mut rac), Ok(k) if k > 0) {}
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
        nhan
    });
    (addr, stop, h)
}

/// **ST-315d** — `net.connect` + `net.connect_timeout` + `net.read`/`write`/`close` tới
/// **server Rust thật** (thread trong test, `TcpListener` loopback). Server ghi lại đúng byte
/// client gửi ⇒ chứng minh chiều ghi của cả hai engine, không chỉ tự nói với mình.
#[test]
fn net_client_toi_server_rust_parity() {
    const SRC: &str = r#"use std.net

fn main():
    match net.connect("__A__"):
        Err(e): print("connect loi {e.message}")
        Ok(t):
            match net.write(t, "ping"):
                Ok(_): print("ghi1 ok")
                Err(e): print("ghi1 loi {e.message}")
            match net.read(t, 64, 2000):
                Ok(s): print("doc1={s}")
                Err(e): print("doc1 loi {e.message}")
            match net.close(t):
                Ok(_): print("dong1 ok")
                Err(e): print("dong1 loi {e.message}")
    match net.connect_timeout("__A__", 2000):
        Err(e): print("ct loi {e.message}")
        Ok(t):
            match net.write(t, "bye"):
                Ok(_): print("ghi2 ok")
                Err(e): print("ghi2 loi {e.message}")
            match net.read(t, 64, 2000):
                Ok(s): print("doc2={s}")
                Err(e): print("doc2 loi {e.message}")
            match net.close(t):
                Ok(_): print("dong2 ok")
                Err(e): print("dong2 loi {e.message}")
"#;
    let (addr, stop, h) = tcp_server(4);
    let d = tam("net-rust");
    let src = SRC.replace("__A__", &addr);
    let (out, err) = hai_duong(&d, "net-rust", &src);
    assert_eq!(
        out, "ghi1 ok\ndoc1=hi\ndong1 ok\nghi2 ok\ndoc2=hi\ndong2 ok\n",
        "net client: {out}\nstderr: {err}"
    );
    stop.store(true, Ordering::Relaxed);
    let nhan = h.join().expect("join server TCP");
    assert_eq!(
        nhan,
        ["ping", "bye", "ping", "bye"],
        "server Rust phải nhận đúng `ping`/`bye` của interp rồi VM"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ===== http: 4 hàm (VD-291 §3.4) =====

/// Dựng phản hồi HTTP/1.1 với `Content-Length` **tường minh** + một header phụ.
fn dap_ung(status: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\n{extra}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

/// Server HTTP/1.1 tối giản loopback: trả theo **đường dẫn** request, ghi lại **dòng request**
/// từng kết nối. `/cham*` ngủ 300 ms để client timeout 80 ms. `stop` + hạn 8 s giữ `join` có hạn.
fn http_server(
    n: usize,
) -> (
    String,
    Arc<AtomicBool>,
    std::thread::JoinHandle<Vec<String>>,
) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind http");
    let addr = l.local_addr().expect("local_addr http").to_string();
    l.set_nonblocking(true).expect("nonblocking http");
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    let h = std::thread::spawn(move || {
        let han = Instant::now() + Duration::from_secs(8);
        let mut thay = Vec::new();
        let mut tra_loi = Vec::new();
        while thay.len() < n && !stop2.load(Ordering::Relaxed) && Instant::now() < han {
            match l.accept() {
                Ok((mut s, _)) => {
                    // **Đọc request ngay trong vòng accept** (nhanh) rồi mới trả lời ở thread
                    // riêng: endpoint `/cham` ngủ 300 ms; nếu ngủ ngay trong vòng accept thì kết
                    // nối kế tiếp còn trong backlog có thể bị kernel huỷ khi client kịp thoát
                    // (ca `head_timeout` 80 ms) ⇒ mất dòng request của engine sau.
                    let _ = s.set_read_timeout(Some(Duration::from_millis(2_000)));
                    let mut req = Vec::new();
                    let mut buf = [0u8; 4096];
                    while req.len() < 65_536 && !req.windows(4).any(|w| w == b"\r\n\r\n") {
                        match s.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(k) => req.extend_from_slice(&buf[..k]),
                        }
                    }
                    let dong = String::from_utf8_lossy(&req)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let duong_dan = dong.split_whitespace().nth(1).unwrap_or("").to_string();
                    thay.push(dong);
                    let cham = matches!(duong_dan.as_str(), "/cham" | "/cham-head");
                    let phan = match duong_dan.as_str() {
                        "/ok" => dap_ung("200 OK", "xin chào".as_bytes(), "X-Test: yes"),
                        "/head" => {
                            b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\nX-Test: yes\r\n\r\n"
                                .to_vec()
                        }
                        "/nhanh" => dap_ung("200 OK", b"nhanh", "X-Test: yes"),
                        "/head-nhanh" => dap_ung("200 OK", b"", "X-Test: fast"),
                        "/cham" | "/cham-head" => dap_ung("200 OK", b"muon", "X-Test: yes"),
                        _ => dap_ung("404 Not Found", b"", "X-Test: no"),
                    };
                    tra_loi.push(std::thread::spawn(move || {
                        if cham {
                            std::thread::sleep(Duration::from_millis(300));
                        }
                        let _ = s.write_all(&phan);
                        let _ = s.shutdown(Shutdown::Both);
                    }));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
        for t in tra_loi {
            let _ = t.join();
        }
        thay
    });
    (addr, stop, h)
}

/// **ST-315d** — 4 hàm `http` (`get`, `get_timeout`, `head`, `head_timeout`) tới server
/// HTTP/1.1 loopback thật. `head` nhận `Content-Length: 999999` **không body** mà vẫn `Ok`
/// (chứng minh không đọc thân); server trễ 300 ms > timeout 80 ms ⇒ **lỗi dữ liệu** parity;
/// cổng đóng ⇒ `Err` ("bị từ chối"/timeout tuỳ OS), vẫn cùng byte giữa hai engine.
#[test]
fn http_bon_ham_parity() {
    const SRC: &str = r#"use std.http

fn main():
    match http.get("__OK__"):
        Ok(s): print("get={s}")
        Err(e): print("get loi {e.message}")
    match http.head("__HEAD__"):
        Ok(m): print("head={m["status"]} {m["headers"]["x-test"]}")
        Err(e): print("head loi {e.message}")
    match http.get_timeout("__FAST__", 2000):
        Ok(s): print("get_timeout={s}")
        Err(e): print("get_timeout loi {e.message}")
    match http.head_timeout("__HEADFAST__", 2000):
        Ok(m): print("head_timeout={m["status"]} {m["headers"]["x-test"]}")
        Err(e): print("head_timeout loi {e.message}")
    match http.get_timeout("__SLOW__", 80):
        Ok(s): print("slow: KHONG DUOC PHEP {s}")
        Err(e): print("slow={e.message}")
    match http.head_timeout("__SLOWHEAD__", 80):
        Ok(m): print("slowhead: KHONG DUOC PHEP {m["status"]}")
        Err(e): print("slowhead={e.message}")
    match http.get("__CLOSED__"):
        Ok(_): print("closed: KHONG DUOC PHEP")
        Err(e): print("closed={e.message}")
"#;
    let (addr, stop, h) = http_server(12);
    // Cổng **đóng** thật: bind rồi thả ⇒ `connect` bị từ chối (không dùng cổng cố định).
    let l = TcpListener::bind("127.0.0.1:0").expect("bind cổng đóng");
    let dong_addr = l.local_addr().expect("addr cổng đóng").to_string();
    drop(l);
    let d = tam("http");
    let src = SRC
        .replace("__OK__", &format!("http://{addr}/ok"))
        .replace("__HEAD__", &format!("http://{addr}/head"))
        .replace("__FAST__", &format!("http://{addr}/nhanh"))
        .replace("__HEADFAST__", &format!("http://{addr}/head-nhanh"))
        .replace("__SLOW__", &format!("http://{addr}/cham"))
        .replace("__SLOWHEAD__", &format!("http://{addr}/cham-head"))
        .replace("__CLOSED__", &format!("http://{dong_addr}/"));
    let (out, err) = hai_duong(&d, "http", &src);
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(dong.len(), 7, "http: {out}\nstderr: {err}");
    assert_eq!(
        &dong[..4],
        &[
            "get=xin chào",
            "head=200 yes",
            "get_timeout=nhanh",
            "head_timeout=200 fast",
        ],
        "http OK/HEAD: {out}"
    );
    assert!(
        dong[4].starts_with("slow=") && dong[4].contains("quá thời gian"),
        "get_timeout phải hết giờ: {}",
        dong[4]
    );
    assert!(
        dong[5].starts_with("slowhead=") && dong[5].contains("quá thời gian"),
        "head_timeout phải hết giờ: {}",
        dong[5]
    );
    assert!(
        dong[6].starts_with("closed=") && dong[6] != "closed: KHONG DUOC PHEP",
        "cổng đóng phải là Err: {}",
        dong[6]
    );
    stop.store(true, Ordering::Relaxed);
    let thay = h.join().expect("join server HTTP");
    assert_eq!(
        thay,
        [
            "GET /ok HTTP/1.1",
            "HEAD /head HTTP/1.1",
            "GET /nhanh HTTP/1.1",
            "HEAD /head-nhanh HTTP/1.1",
            "GET /cham HTTP/1.1",
            "HEAD /cham-head HTTP/1.1",
            "GET /ok HTTP/1.1",
            "HEAD /head HTTP/1.1",
            "GET /nhanh HTTP/1.1",
            "HEAD /head-nhanh HTTP/1.1",
            "GET /cham HTTP/1.1",
            "HEAD /cham-head HTTP/1.1",
        ],
        "server HTTP phải thấy đúng method/đường dẫn của cả hai engine"
    );
    let _ = std::fs::remove_dir_all(&d);
}
