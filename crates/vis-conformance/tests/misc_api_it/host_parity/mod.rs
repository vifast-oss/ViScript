// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — parity thư viện `interp ↔ VM` cho **66 hàm host** còn ngoài corpus
//! (`fs` 22, `sqlite` 28, `net` 10, `http` 4, `time` 2 — `JOURNAL/st314a-hau-kiem.md` §23.2).
//!
//! Mỗi ca chạy **cả hai đường** trên cùng tệp + cùng tempdir, đòi cùng stdout/exit; ca lỗi so cả
//! mã chẩn đoán + thông điệp. Chạy qua binary `vis` (đường CLI thật) để dùng đúng host native;
//! đường thư viện thuần không dựng được `FsGuard`/`NetGuard`/`DbGuard` nếu không lộ API nội bộ.
//!
//! **ST-316d (VD-292 §4.1)** — tách theo miền để hết cận trần R1 (1.457 dòng):
//! `host_parity/{mod,fs,sqlite,net_http}.rs`, khai `#[path]` ở `lib_api_it.rs` — **0 binary mới**.
//! `mod.rs` giữ harness dùng chung + `time`; ba tệp con giữ đúng ca/const của miền mình.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

mod fs;
mod net_http;
mod sqlite;

use self::fs::{SRC_FS, SRC_FS_LINK, SRC_FS_LOI};
use self::net_http::{SRC_HTTP, SRC_NET, SRC_NET_CLIENT};
use self::sqlite::{SRC_SQLITE_DML, SRC_SQLITE_META};
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

/// **Mọi script thật sự được chạy** trong harness (tag, nguồn) — nguồn duy nhất cho test khoá
/// 66/66 (F4): chỉ lời gọi nằm trong các script này mới được tính, không tính bảng/chú thích.
const SRC_PHU: &[(&str, &str)] = &[
    ("fs", SRC_FS),
    ("fs-loi", SRC_FS_LOI),
    ("fs-link", SRC_FS_LINK),
    ("time", SRC_TIME),
    ("sqlite-dml", SRC_SQLITE_DML),
    ("sqlite-meta", SRC_SQLITE_META),
    ("net", SRC_NET),
    ("net-client", SRC_NET_CLIENT),
    ("http", SRC_HTTP),
];

/// Thư mục tạm riêng cho từng ca, **tự dọn kể cả khi test panic** (RAII — F7 review).
struct Tam(PathBuf);

impl std::ops::Deref for Tam {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for Tam {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tam {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tam(tag: &str) -> Tam {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-host-parity-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    Tam(d)
}

/// Đường dẫn kiểu VS (dấu `/`, kể cả Windows) để nhúng vào chương trình.
fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// Binary `vis` của workspace — tôn trọng `CARGO_TARGET_DIR`; nếu đường dẫn **tương đối** thì
/// hiểu theo gốc workspace (không theo cwd của process test) — F9 review.
fn vis_bin() -> PathBuf {
    let goc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(t) => {
            let t = PathBuf::from(t);
            if t.is_absolute() {
                t
            } else {
                goc.join(t)
            }
        }
        None => goc.join("target"),
    };
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let ten = if cfg!(windows) { "vis.exe" } else { "vis" };
    let p = target.join(profile).join(ten);
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
/// trạng thái fixture, không để đường interp làm bẩn dữ liệu của VM. Đòi **exit 0** (mọi ca của
/// harness là lỗi dữ liệu, không phải trap — F10 review).
fn hai_duong_voi<F: Fn()>(d: &Path, tag: &str, src: &str, setup: F) -> (String, String) {
    hai_duong_chuan_hoa(d, tag, src, setup, |s: &str| s.to_string())
}

/// Như [`hai_duong_voi`] nhưng áp `chuan_hoa` lên stdout/stderr **trước khi so** hai engine
/// (dùng cho ca có cổng ephemeral hoặc đồng hồ). Vẫn so **từng byte** phần còn lại.
fn hai_duong_chuan_hoa<F: Fn(), C: Fn(&str) -> String>(
    d: &Path,
    tag: &str,
    src: &str,
    setup: F,
    chuan_hoa: C,
) -> (String, String) {
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
    assert_eq!(
        c1, 0,
        "[{tag}] exit phải 0 (lỗi dữ liệu, không trap): interp {c1}\nstderr: {e1}\nvm stderr: {e2}"
    );
    let o1 = chuan_hoa(&o1);
    let e1 = chuan_hoa(&e1);
    let o2 = chuan_hoa(&o2);
    let e2 = chuan_hoa(&e2);
    assert_eq!(o1, o2, "[{tag}] stdout lệch sau chuẩn hoá: vm [{e2}]");
    assert_eq!(e1, e2, "[{tag}] stderr lệch sau chuẩn hoá");
    (o1, e1)
}

/// Thay mọi `127.0.0.1:<cổng>` bằng `127.0.0.1:<P>` — hai lần chạy mở cổng ephemeral khác nhau,
/// nhưng phần còn lại của thông điệp lỗi vẫn được so **từng byte**.
fn chuan_hoa_cong(s: &str) -> String {
    const TIEN_TO: &str = "127.0.0.1:";
    let mut ra = String::with_capacity(s.len());
    let mut con = s;
    while let Some(i) = con.find(TIEN_TO) {
        ra.push_str(&con[..i]);
        ra.push_str("127.0.0.1:<P>");
        let sau = &con[i + TIEN_TO.len()..];
        let het = sau.find(|c: char| !c.is_ascii_digit()).unwrap_or(sau.len());
        con = &sau[het..];
    }
    ra.push_str(con);
    ra
}

/// Bỏ **chú thích ViScript** (`#…` tới cuối dòng, ngoài chuỗi) khỏi script trước khi kiểm lời
/// gọi host — chú thích không chứng minh hàm được chạy (F4 review).
fn bo_chu_thich_vis(s: &str) -> String {
    let mut ra = String::with_capacity(s.len());
    let mut trong_chuoi = false;
    let mut thoat = false;
    let mut trong_chu_thich = false;
    for c in s.chars() {
        if trong_chu_thich {
            if c == '\n' {
                trong_chu_thich = false;
                ra.push(c);
            }
            continue;
        }
        if trong_chuoi {
            ra.push(c);
            if thoat {
                thoat = false;
            } else if c == '\\' {
                thoat = true;
            } else if c == '"' {
                trong_chuoi = false;
            }
            continue;
        }
        match c {
            '"' => {
                trong_chuoi = true;
                ra.push(c);
            }
            '#' => trong_chu_thich = true,
            _ => ra.push(c),
        }
    }
    ra
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

/// **ST-315d** — khoá **66/66 có lời gọi trong script được chạy thật** (không phải tìm chuỗi
/// trong cả tệp): chỉ xét các `const SRC_*` nằm trong bảng [`SRC_PHU`] mà test dùng để chạy, sau
/// khi **bỏ chú thích ViScript** (`#`). Vì vậy đổi lời gọi thật thành chú thích `# fs.read_bytes(`
/// (cú pháp ViScript) sẽ làm test **đỏ**. Bộ bóc **không** hiểu `//` — đó không phải cú pháp chú
/// thích của ViScript, nên `// fs.read_bytes(` trong script *vẫn bị tính* là lời gọi (F-R3 review).
#[test]
fn moi_ham_host_co_loi_goi_trong_harness() {
    let da_boc: Vec<(&str, String)> = SRC_PHU
        .iter()
        .map(|(tag, src)| (*tag, bo_chu_thich_vis(src)))
        .collect();
    let thieu: Vec<&str> = HOST_66
        .iter()
        .copied()
        .filter(|n| {
            let kim = format!("{n}(");
            !da_boc.iter().any(|(_, src)| src.contains(&kim))
        })
        .collect();
    assert!(
        thieu.is_empty(),
        "thiếu lời gọi host trong script được chạy: {thieu:?}"
    );
    for (tag, src) in &da_boc {
        assert!(
            HOST_66.iter().any(|n| src.contains(&format!("{n}("))),
            "script `{tag}` không gọi hàm host nào (rỗng/sai tham chiếu)"
        );
    }
}

/// Tự kiểm bộ bóc chú thích: lời gọi bị comment bằng `#` **không** được tính; lời gọi thật và
/// dấu `#` trong chuỗi vẫn giữ nguyên (nền tảng cho F4).
#[test]
fn bo_chu_thich_vis_khong_tinh_lenh_bi_comment() {
    let gia = "use std.fs\n# fs.read_bytes(\nprint(\"# fs.size(\")\nfs.read(\"x\")\n";
    let s = bo_chu_thich_vis(gia);
    assert!(!s.contains("fs.read_bytes("), "chú thích bị tính: {s}");
    assert!(s.contains("fs.read("), "lời gọi thật bị mất: {s}");
    assert!(
        s.contains("# fs.size("),
        "dấu # trong chuỗi bị bóc sai: {s}"
    );
}

/// `time`: chỉ so **kiểu + bất biến** (đồng hồ thật) — **không** ghim mốc tuyệt đối 2023 (F11).
const SRC_TIME: &str = r#"use std.time

fn main():
    let a = time.now_ms()
    let b = time.monotonic_ms()
    let c = time.monotonic_ms()
    let d = time.now_ms()
    print(a > 0)
    print(b >= 0)
    print(c >= b)
    print(d >= a)
"#;

/// **ST-315d** — 2 hàm `time` chỉ so **kiểu + bất biến** (đồng hồ thật, không tất định).
#[test]
fn time_hai_ham_parity_bat_bien() {
    let d = tam("time");
    let (out, err) = hai_duong(&d, "time", SRC_TIME);
    assert_eq!(out, "true\ntrue\ntrue\ntrue\n", "time: {err}");
}
