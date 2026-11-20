// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.with_extension)** — `std.path.with_extension(p: str, ext: str) -> str`, thay /
//! đặt / bỏ **phần mở rộng** của **thành phần cuối** đường dẫn, **thuần từ vựng POSIX**: không
//! chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra tồn tại.
//!
//! Hợp đồng được khoá ở đây:
//! - Có phần mở rộng (theo **đúng** `path.ext`): thay nó bằng `ext`; `stem` lấy từ **đúng**
//!   `path.stem` ⇒ `"a/b.tar.gz"` + `"xz"` ⇒ `"a/b.tar.xz"` (chỉ thay phần mở rộng **cuối**).
//! - Không có phần mở rộng: nối thêm `"."` + `ext`; `ext` rỗng ⇒ **nguyên văn**.
//! - `ext` rỗng: xoá phần mở rộng **kể cả dấu chấm** (`"a/b.txt"` ⇒ `"a/b"`).
//! - Tên **bắt đầu bằng `.`** (`.gitignore`, `.a.b`) **không** có phần mở rộng theo `path.ext`
//!   hiện hành (dấu chấm đầu tên không tính), nên `with_extension` **nối thêm**:
//!   `.gitignore` + `txt` ⇒ `.gitignore.txt`, `.a.b` + `txt` ⇒ `.a.b.txt`.
//! - Thư mục giữ **nguyên văn** từng byte; dấu chấm trong thư mục không phải phần mở rộng.
//! - `/` cuối chỉ đánh dấu thư mục, **không** phải tên tệp; khi có tên tệp, mọi `/` cuối bị bỏ
//!   (cùng `Path::with_extension`): `"a/b/"` + `md` ⇒ `"a/b.md"`, `"a/b/"` + `""` ⇒ `"a/b"`.
//! - **Không có tên tệp** (`""`, chỉ toàn `/`, `"."`, `".."`, `"a/.."`, gốc ổ Windows) ⇒ trả
//!   `p` **nguyên văn**, bỏ qua `ext` — không tạo tên giả.
//! - `ext` dùng **nguyên văn**, không kiểm/tước dấu chấm: `"a"` + `".md"` ⇒ `"a..md"`.
//! - **Không sugar** (module-only): `p.with_extension(...)` ⇒ `VS0403`; sai kiểu ⇒ `VS0412`;
//!   sai arity ⇒ `VS0401`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `9099201`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.path, 4 lời gọi with_extension
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 4:11 module `std.path` không có hàm `with_extension`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:4:11: error VS0301: module `std.path` không có hàm `with_extension`
//! probe.vis:5:11: error VS0301: module `std.path` không có hàm `with_extension`
//! probe.vis:6:11: error VS0301: module `std.path` không có hàm `with_extension`
//! probe.vis:7:11: error VS0301: module `std.path` không có hàm `with_extension`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng bốn dòng)
//! $ vis check sugar.vis                    # "a.txt".with_extension("md"), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `with_extension`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pwe-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi_tep(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache").args(extra).arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM; assert hai engine giống **byte** stdout/stderr/exit.
fn hai_engine(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, b.0,
        "{tag}: exit interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.1, b.1,
        "{tag}: stdout interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.2, b.2,
        "{tag}: stderr interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    a
}

/// Chương trình hợp lệ: hai engine cùng chạy sạch, stdout đúng kỳ vọng.
fn dung(tag: &str, src: &str, stdout: &str) {
    let (code, out, err) = hai_engine(tag, src);
    assert_eq!(
        code, 0,
        "{tag}: phải chạy sạch\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.is_empty(), "{tag}: stderr phải rỗng: {err}");
    assert_eq!(out, stdout, "{tag}: stdout sai");
}

/// Lỗi **tĩnh**: `vis check` đỏ (exit 1, mã trong TAP) và **cả hai** engine từ chối (exit 2).
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (d, p) = ghi_tep(tag, src);
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
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
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

// ===========================================================================
// 1 — đã có phần mở rộng: thay phần mở rộng **cuối**
// ===========================================================================

/// `a/b.txt` → `a/b.md`; `a/b.tar.gz` → `a/b.tar.xz` (chỉ `gz`, không phải `tar.gz`).
#[test]
fn thay_phan_mo_rong_hien_co() {
    dung(
        "pwe-thay",
        r#"use std.path

fn main():
    print(path.with_extension("a/b.txt", "md"))
    print(path.with_extension("a/b.txt", "txt"))
    print(path.with_extension("a/b.tar.gz", "xz"))
    print(path.with_extension("a/b.tar.gz", "gz"))
    let r: str = path.with_extension("a/b.txt", "rs")
    print(r)
"#,
        "a/b.md\na/b.txt\na/b.tar.xz\na/b.tar.gz\na/b.rs\n",
    );
}

// ===========================================================================
// 2 — chưa có phần mở rộng: nối `.` + ext
// ===========================================================================

/// `a/b` + `txt` ⇒ `a/b.txt`; một thành phần và nhiều cấp đều giữ tiền tố thư mục.
#[test]
fn khong_co_phan_mo_rong_thi_them() {
    dung(
        "pwe-them",
        r#"use std.path

fn main():
    print(path.with_extension("a", "txt"))
    print(path.with_extension("a/b", "txt"))
    print(path.with_extension("a/b/c", "rs"))
    print(path.with_extension("./a", "txt"))
    print(path.with_extension("../a", "md"))
"#,
        "a.txt\na/b.txt\na/b/c.rs\n./a.txt\n../a.md\n",
    );
}

// ===========================================================================
// 3 — ext rỗng: xoá phần mở rộng (kể cả dấu chấm); không có ext ⇒ nguyên văn
// ===========================================================================

/// `a/b.txt` + `""` ⇒ `a/b`; `a/b.tar.gz` + `""` ⇒ `a/b.tar`; `a/b` + `""` ⇒ `a/b`.
#[test]
fn ext_rong_thi_xoa() {
    dung(
        "pwe-xoa",
        r#"use std.path

fn main():
    print(path.with_extension("a/b.txt", ""))
    print(path.with_extension("a/b.tar.gz", ""))
    print(path.with_extension("a/b", ""))
    print(path.with_extension("a", ""))
"#,
        "a/b\na/b.tar\na/b\na\n",
    );
}

// ===========================================================================
// 4 — tên bắt đầu bằng `.`: không phải phần mở rộng (theo `path.ext`)
// ===========================================================================

/// Dấu chấm **đầu tên** không tính là phần mở rộng: nối thêm bình thường, `""` ⇒ nguyên văn.
/// `.a.b` cũng vậy — **mọi** tên bắt đầu bằng `.` đều không có ext theo `path.ext` hiện hành.
#[test]
fn ten_bat_dau_bang_cham() {
    dung(
        "pwe-cham-dau",
        r#"use std.path

fn main():
    print(path.with_extension(".gitignore", "txt"))
    print(path.with_extension(".gitignore", ""))
    print(path.with_extension("a/.gitignore", "txt"))
    print(path.with_extension("a/.gitignore", ""))
    print(path.with_extension(".a.b", "txt"))
    print(path.with_extension(".a.b", ""))
"#,
        ".gitignore.txt\n.gitignore\na/.gitignore.txt\na/.gitignore\n.a.b.txt\n.a.b\n",
    );
}

// ===========================================================================
// 5 — nhiều dấu chấm + thư mục có dấu chấm
// ===========================================================================

/// Dấu chấm trong **thư mục** không phải ext; chỉ thành phần cuối quyết định.
#[test]
fn nhieu_cham_va_thu_muc_co_cham() {
    dung(
        "pwe-nhieu-cham",
        r#"use std.path

fn main():
    print(path.with_extension("a.b/c", "txt"))
    print(path.with_extension("a.b/c.txt", "md"))
    print(path.with_extension("a.b.c/d.tar.gz", "xz"))
    print(path.with_extension("a/b.c.d", "e"))
"#,
        "a.b/c.txt\na.b/c.md\na.b.c/d.tar.xz\na/b.c.e\n",
    );
}

// ===========================================================================
// 6 — `/` cuối: chỉ đánh dấu thư mục, bỏ khi ghi tên tệp
// ===========================================================================

/// `a/b/` + `md` ⇒ `a/b.md`; `a/b/` + `""` ⇒ `a/b` (cùng `Path::with_extension`).
#[test]
fn slash_cuoi() {
    dung(
        "pwe-slash-cuoi",
        r#"use std.path

fn main():
    print(path.with_extension("a/b/", "md"))
    print(path.with_extension("a/b.txt/", "md"))
    print(path.with_extension("a/b/", ""))
    print(path.with_extension("a/b.txt/", ""))
    print(path.with_extension("a/b.txt//", "md"))
    print(path.with_extension("/a/b/", "md"))
"#,
        "a/b.md\na/b.md\na/b\na/b\na/b.md\n/a/b.md\n",
    );
}

// ===========================================================================
// 7 — tuyệt đối / tương đối; giữ nguyên tiền tố thư mục
// ===========================================================================

/// `/a/b.txt` ⇒ `/a/b.md`; `//srv/share` giữ `//`; `./`/`../` giữ nguyên.
#[test]
fn tuyet_doi_va_tuong_doi() {
    dung(
        "pwe-tuyet-doi",
        r#"use std.path

fn main():
    print(path.with_extension("/a/b.txt", "md"))
    print(path.with_extension("a/b.txt", "md"))
    print(path.with_extension("//srv/share/x.txt", "md"))
    print(path.with_extension("./a", "txt"))
    print(path.with_extension("../a.txt", "md"))
"#,
        "/a/b.md\na/b.md\n//srv/share/x.md\n./a.txt\n../a.md\n",
    );
}

// ===========================================================================
// 8 — Unicode: giữ nguyên từng scalar
// ===========================================================================

/// Tên tiếng Việt/emoji/tổ hợp NFD: chỉ đuôi sau dấu chấm cuối bị thay.
#[test]
fn unicode_giu_nguyen_scalar() {
    dung(
        "pwe-unicode",
        r#"use std.path

fn main():
    print(path.with_extension("thư/mục/tệp.txt", "md"))
    print(path.with_extension("đ😀.txt", "md"))
    print(path.with_extension("thư/mục/không", "txt"))
    print(path.with_extension("e\u{0302}\u{0301}.txt", "md"))
"#,
        "thư/mục/tệp.md\nđ😀.md\nthư/mục/không.txt\ne\u{0302}\u{0301}.md\n",
    );
}

// ===========================================================================
// 9 — không có tên tệp: nguyên văn, bỏ qua ext
// ===========================================================================

/// Rỗng/chỉ toàn `/`/`.`/`..`: không tạo tên giả. `a/.` và `a/..` cũng **nguyên văn** vì thành
/// phần cuối không phải tên tệp (khác Rust ở `a/.` — đã ghi trong tài liệu hàm).
#[test]
fn khong_co_ten_tep() {
    dung(
        "pwe-khong-ten",
        r#"use std.path

fn main():
    print(path.with_extension("", "txt"))
    print(path.with_extension("", ""))
    print(path.with_extension("/", "txt"))
    print(path.with_extension("///", "md"))
    print(path.with_extension(".", "txt"))
    print(path.with_extension("..", "txt"))
    print(path.with_extension("a/..", "txt"))
    print(path.with_extension("a/.", "txt"))
"#,
        "\n\n/\n///\n.\n..\na/..\na/.\n",
    );
}

// ===========================================================================
// 10 — `ext` nguyên văn; dấu chấm cuối tên theo `path.ext`
// ===========================================================================

/// `ext` không bị kiểm/tước dấu chấm; `foo.` **không** có ext theo `path.ext` ⇒ nối thêm `.`.
/// `\` là ký tự **thường** trên POSIX.
#[test]
fn ext_dung_nguyen_van() {
    dung(
        "pwe-ext-nguyen-van",
        r#"use std.path

fn main():
    print(path.with_extension("a/b", "tar.gz"))
    print(path.with_extension("a/b.txt", ".md"))
    print(path.with_extension("foo.", "txt"))
    print(path.with_extension("foo.", ""))
    print(path.with_extension("a\\b.txt", "md"))
"#,
        "a/b.tar.gz\na/b..md\nfoo..txt\nfoo.\na\\b.md\n",
    );
}

// ===========================================================================
// 11 — **không** sugar: module-only
// ===========================================================================

/// `p.with_extension(...)` **không** phải sugar: `str` không có trường `with_extension` ⇒
/// `VS0403`, kể cả khi đã `use std.path`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pwe-sugar-literal",
            "fn main():\n    print(\"a.txt\".with_extension(\"md\"))\n",
        ),
        (
            "pwe-sugar-bien",
            "fn main():\n    let p = \"a.txt\"\n    print(p.with_extension(\"md\"))\n",
        ),
        (
            "pwe-sugar-co-use",
            "use std.path\n\nfn main():\n    print(\"a.txt\".with_extension(\"md\"))\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 12 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 1/3 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("pwe-kieu-dau", "path.with_extension(1, \"md\")", "VS0412"),
        ("pwe-kieu-sau", "path.with_extension(\"a\", 2)", "VS0412"),
        (
            "pwe-kieu-bool",
            "path.with_extension(\"a\", true)",
            "VS0412",
        ),
        ("pwe-arity-1", "path.with_extension(\"a\")", "VS0401"),
        (
            "pwe-arity-3",
            "path.with_extension(\"a\", \"b\", \"c\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 13 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp thay/thêm/xoá/chấm đầu/nhiều chấm/slash cuối/tuyệt đối/Unicode/rỗng vào một chương
/// trình để khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pwe-hai-engine",
        r#"use std.path

fn main():
    print(path.with_extension("a/b.txt", "md"))
    print(path.with_extension("a/b", "txt"))
    print(path.with_extension("a/b.txt", ""))
    print(path.with_extension(".gitignore", "txt"))
    print(path.with_extension("a.b/c.tar.gz", "xz"))
    print(path.with_extension("a/b/", "md"))
    print(path.with_extension("/a/b/", ""))
    print(path.with_extension("thư/mục/tệp.txt", "md"))
    print(path.with_extension("", "txt"))
    print(path.with_extension("/", "md"))
    let r: str = path.with_extension("x.y", "z")
    print(r)
"#,
        "a/b.md\na/b.txt\na/b\n.gitignore.txt\na.b/c.tar.xz\na/b.md\n/a/b\nthư/mục/tệp.md\n\n/\nx.z\n",
    );
}
