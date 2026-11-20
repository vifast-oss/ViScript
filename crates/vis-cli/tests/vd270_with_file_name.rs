// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.with_file_name)** — `std.path.with_file_name(p: str, name: str) -> str`, thay
//! **toàn bộ thành phần cuối** (tên tệp **kể cả** phần mở rộng) bằng `name`, **giữ nguyên tiền tố**
//! thư mục; **thuần từ vựng POSIX**: không chạm đĩa, không đọc `cwd`, không phân giải symlink,
//! không kiểm tra tồn tại.
//!
//! **Probe Rust (toolchain ghim 1.100.0)** — bảng dưới lấy từ `Path::with_file_name` thật (test
//! `probe_rust_path_with_file_name` chạy lại **đúng** bảng này trên Unix):
//!
//! ```text
//! ""        + "x"   => "x"           "."       + "x"   => "./x"
//! ".."      + "x"   => "../x"        "/"       + "x"   => "/x"
//! "a/"      + "x"   => "x"           "a/b.txt" + "c"   => "a/c"
//! "a/b.txt" + "x/y" => "a/x/y"       "a/b.txt" + ""    => "a/"
//! "a/b.txt" + "/x"  => "/x"          "a/b.txt" + "."   => "a/."
//! "a//b"    + "x"   => "a/x"         "a/./b"   + "x"   => "a/x"
//! "./a"     + "x"   => "./x"         "a/."     + "x"   => "x"
//! "a/.."    + "x"   => "a/../x"      "///"     + "x"   => "///x"
//! "foo."    + "c"   => "c"           "a/b.txt" + "x/"  => "a/x/"
//! ```
//!
//! **Chỗ ViScript cố ý khác Rust** (giữ **nguyên văn** tiền tố, và **không** chuẩn hoá thành
//! phần như `Components` của Rust):
//! - **Không có tên tệp** (`""`, `/`, `///`, `.`, `..`, `a/.`, `a/..`, `a/b/.`, `a/b/..`, gốc ổ
//!   Windows) ⇒ trả `p` **nguyên văn**, bỏ qua `name`. Rust lại dựng đường dẫn mới: `""` + `x`
//!   ⇒ `x`, `/` + `x` ⇒ `/x`, `.` + `x` ⇒ `./x`, `a/.` + `x` ⇒ `x`, `a/..` + `x` ⇒ `a/../x`.
//! - Tiền tố giữ **nguyên văn**: `a//b` + `c` ⇒ `a//c` (Rust `a/c`), `a/./b` + `c` ⇒ `a/./c`
//!   (Rust `a/c`), `//a` + `""` ⇒ `//` (Rust `/`).
//! - `name` dùng **nguyên văn**: tên tuyệt đối **không** thay cả đường dẫn — `a/b.txt` + `/x`
//!   ⇒ `a//x` (Rust `/x`); tên rỗng ⇒ **bỏ** thành phần cuối, giữ tiền tố + `/` (`a/b.txt` ⇒
//!   `a/`, cùng Rust); tên `/`, `x/y`, `x/`, `.`, `..`, `\`, NUL, dấu cách, Unicode đều được nối
//!   nguyên văn, **không** kiểm tra/chuẩn hoá.
//! - Tên tệp nhận bằng **đúng** `path.basename` (`basename_voi`); phần bị bỏ là **chính** tên đó,
//!   tiền tố là các byte còn lại **trước** nó. **Không** dùng `path.dirname` vì `dirname` bỏ `/`
//!   cuối, trả `.` cho tên trần và `/` cho gốc — sẽ không giữ được byte nguyên văn.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `88da99c`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_with_file_name
//! test result: FAILED. 2 passed; 11 failed
//! ```
//!
//! - **11 ca** đỏ `VS0301`: `vis check` báo `module `std.path` không có hàm `with_file_name``;
//!   `vis run` và `vis run --vm` đều exit 2 với đúng dòng lỗi cho **từng** lời gọi.
//! - **1 ca no-sugar** đã xanh đúng `VS0403` `` `str` không có trường `with_file_name` `` — không
//!   phụ thuộc hàm mới.
//! - **1 ca probe Rust** đã xanh (chỉ dùng `std::path`), khoá bảng tham chiếu.
//!
//! Sau khi thêm: 13/13 xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pfn-{tag}-{}-{n}", std::process::id()));
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
// 0 — probe Rust `Path::with_file_name` (bảng tham chiếu của tài liệu hàm)
// ===========================================================================

/// Chạy lại **đúng** bảng probe bằng `std::path` của toolchain ghim; đây là mốc so sánh cho các
/// chỗ ViScript cố ý khác (xem đầu tệp). Chỉ Unix: bảng dùng `/` và `\` là ký tự thường.
#[cfg(unix)]
#[test]
fn probe_rust_path_with_file_name() {
    fn r(p: &str, name: &str) -> String {
        Path::new(p)
            .with_file_name(name)
            .to_string_lossy()
            .into_owned()
    }
    // Cùng Rust.
    for (p, n, want) in [
        ("a/", "x", "x"),
        ("a/b.txt", "c", "a/c"),
        ("a/b.txt", "x/y", "a/x/y"),
        ("a/b.txt", "", "a/"),
        ("a/b", "", "a/"),
        ("/a/b", "", "/a/"),
        ("a/b.txt", "c.d", "a/c.d"),
        ("a/b.txt", "x/", "a/x/"),
        ("a/b.txt", "x//y", "a/x//y"),
        ("a/b.txt", ".", "a/."),
        ("a/b.txt", "..", "a/.."),
        ("a/b.txt", " ", "a/ "),
        ("a/b.txt", "\0", "a/\0"),
        ("a/b.txt", r"c\d", r"a/c\d"),
        ("a/b.txt", "tệp😀", "a/tệp😀"),
        ("a/b", "x", "a/x"),
        ("a/b/", "x", "a/x"),
        ("a/b//", "x", "a/x"),
        ("/a/b", "x", "/a/x"),
        ("//a/b", "x", "//a/x"),
        ("./a", "x", "./x"),
        ("../a", "x", "../x"),
        ("foo.", "c", "c"),
        ("foo.", "", ""),
        ("a\\b.txt", "c", "c"),
    ] {
        assert_eq!(r(p, n), want, "Rust probe: {p:?} + {n:?}");
    }
    // Rust **khác** ViScript (ViScript giữ `p`/tiền tố nguyên văn — ghi ở đầu tệp).
    for (p, n, want) in [
        ("", "x", "x"),
        (".", "x", "./x"),
        ("..", "x", "../x"),
        ("/", "x", "/x"),
        ("///", "x", "///x"),
        (".", "", "./"),
        ("/", "", "/"),
        ("a//b", "x", "a/x"),
        ("a/./b", "x", "a/x"),
        ("a/.", "x", "x"),
        ("a/..", "x", "a/../x"),
        ("a/b/.", "x", "a/x"),
        ("a/b/..", "x", "a/b/../x"),
        ("a/b.txt", "/x", "/x"),
        ("a/b.txt", "///", "///"),
        ("a/b.txt", "/", "/"),
    ] {
        assert_eq!(r(p, n), want, "Rust probe (khác VS): {p:?} + {n:?}");
    }
}

// ===========================================================================
// 1 — thay **toàn bộ** thành phần cuối (kể cả phần mở rộng), giữ tiền tố
// ===========================================================================

/// `a/b.txt` + `c` ⇒ `a/c`; không còn logic stem/ext: `b.tar.gz` mất **cả** tên; tiền tố giữ.
#[test]
fn thay_toan_bo_ten_giu_tien_to() {
    dung(
        "pfn-co-ban",
        r#"use std.path

fn main():
    print(path.with_file_name("a/b.txt", "c"))
    print(path.with_file_name("a/b.tar.gz", "c"))
    print(path.with_file_name("a/b", "c"))
    print(path.with_file_name("a", "c"))
    print(path.with_file_name("./a", "c"))
    print(path.with_file_name("../a.txt", "c"))
    print(path.with_file_name("/a/b.txt", "c"))
    print(path.with_file_name("//srv/share/x.txt", "c"))
    print(path.with_file_name("thư/mục/tệp.txt", "mới"))
    let r: str = path.with_file_name("x.y", "z")
    print(r)
"#,
        "a/c\na/c\na/c\nc\n./c\n../c\n/a/c\n//srv/share/c\nthư/mục/mới\nz\n",
    );
}

// ===========================================================================
// 2 — `name` dùng **nguyên văn**: dấu chấm, `/`, `\`, NUL, dấu cách, tuyệt đối
// ===========================================================================

/// Không kiểm tra/chuẩn hoá `name`: `x/y` nối nguyên văn, `/x` **không** thay cả đường dẫn
/// (khác Rust), `x/` giữ `/` cuối, `\`/NUL là ký tự **thường** trên POSIX.
#[test]
fn ten_nguyen_van() {
    dung(
        "pfn-ten-nguyen-van",
        r#"use std.path

fn main():
    print(path.with_file_name("a/b.txt", "c.d"))
    print(path.with_file_name("a/b.txt", "x.y.z"))
    print(path.with_file_name("a/b.txt", "x/y"))
    print(path.with_file_name("a/b.txt", "/x"))
    print(path.with_file_name("a/b.txt", "x/"))
    print(path.with_file_name("a/b.txt", "."))
    print(path.with_file_name("a/b.txt", ".."))
    print(path.with_file_name("a/b.txt", "x\\y"))
    print(path.with_file_name("a/b.txt", "\0"))
    print(path.with_file_name("a/b.txt", " "))
    print(path.with_file_name("a/b.txt", "tệp😀"))
"#,
        "a/c.d\na/x.y.z\na/x/y\na//x\na/x/\na/.\na/..\na/x\\y\na/\0\na/ \na/tệp😀\n",
    );
}

// ===========================================================================
// 3 — `name` rỗng: **bỏ** thành phần cuối, giữ tiền tố + `/`
// ===========================================================================

/// `a/b.txt` + `""` ⇒ `a/` (cùng Rust); tên trần + rỗng ⇒ `""`; không có tên tệp ⇒ nguyên văn.
#[test]
fn ten_rong_bo_thanh_phan_cuoi() {
    dung(
        "pfn-ten-rong",
        r#"use std.path

fn main():
    print(path.with_file_name("a/b.txt", ""))
    print(path.with_file_name("a/b", ""))
    print(path.with_file_name("a", ""))
    print(path.with_file_name("/a/b", ""))
    print(path.with_file_name("a/b/", ""))
    print(path.with_file_name("a/", ""))
    print(path.with_file_name("/", ""))
    print(path.with_file_name("", ""))
"#,
        "a/\na/\n\n/a/\na/\n\n/\n\n",
    );
}

// ===========================================================================
// 4 — `/` cuối của `p`: chỉ đánh dấu thư mục, bỏ khi có tên tệp
// ===========================================================================

/// `a/b/` + `c` ⇒ `a/c`; `a/` + `c` ⇒ `c` (thành phần cuối là `a`, tiền tố rỗng);
/// `/a/b//` + `c` ⇒ `/a/c`.
#[test]
fn slash_cuoi() {
    dung(
        "pfn-slash-cuoi",
        r#"use std.path

fn main():
    print(path.with_file_name("a/b/", "c"))
    print(path.with_file_name("a/b//", "c"))
    print(path.with_file_name("a/b.txt//", "c"))
    print(path.with_file_name("a/", "c"))
    print(path.with_file_name("/a/b/", "c"))
    print(path.with_file_name("/a/b//", "c"))
    print(path.with_file_name("//a/b/", "c"))
"#,
        "a/c\na/c\na/c\nc\n/a/c\n/a/c\n//a/c\n",
    );
}

// ===========================================================================
// 5 — tiền tố giữ **nguyên văn** (khác `path.normalize`/Rust)
// ===========================================================================

/// `a//b` + `c` ⇒ `a//c`; `a/./b` + `c` ⇒ `a/./c`; `//a` + `""` ⇒ `//` (Rust `/`).
#[test]
fn tien_to_nguyen_van() {
    dung(
        "pfn-tien-to",
        r#"use std.path

fn main():
    print(path.with_file_name("a//b", "c"))
    print(path.with_file_name("a/./b", "c"))
    print(path.with_file_name("a//b", ""))
    print(path.with_file_name("a/./b", ""))
    print(path.with_file_name("./a", "x"))
    print(path.with_file_name(".//a", "x"))
    print(path.with_file_name("//a/b", "x"))
    print(path.with_file_name("//a", ""))
    print(path.with_file_name("/a", "x"))
"#,
        "a//c\na/./c\na//\na/./\n./x\n.//x\n//a/x\n//\n/x\n",
    );
}

// ===========================================================================
// 6 — không có tên tệp: trả `p` **nguyên văn**, bỏ qua `name`
// ===========================================================================

/// `""`, chỉ toàn `/`, `.`/`..`/`a/..`/`a/.`/`a/b/.`/`a/b/..`: nguyên văn (khác Rust, đã ghi).
#[test]
fn khong_co_ten_tep() {
    dung(
        "pfn-khong-ten",
        r#"use std.path

fn main():
    print(path.with_file_name("", "x"))
    print(path.with_file_name("/", "x"))
    print(path.with_file_name("///", "x"))
    print(path.with_file_name(".", "x"))
    print(path.with_file_name("..", "x"))
    print(path.with_file_name("a/..", "x"))
    print(path.with_file_name("a/.", "x"))
    print(path.with_file_name("a/b/.", "x"))
    print(path.with_file_name("a/b/..", "x"))
    print(path.with_file_name("", ""))
    print(path.with_file_name("a/.", ""))
    print(path.with_file_name("a/..", ""))
"#,
        "\n/\n///\n.\n..\na/..\na/.\na/b/.\na/b/..\n\na/.\na/..\n",
    );
}

// ===========================================================================
// 7 — tên bắt đầu/chấm cuối **không** đặc biệt: thay cả tên, không tách ext
// ===========================================================================

/// `.gitignore` + `c` ⇒ `c`; `foo.` + `c` ⇒ `c` (cùng Rust); `a/.a.b` + `*.txt` ⇒ `a/*.txt`
/// — `with_file_name` **không** dùng `path.ext`/`path.stem`.
#[test]
fn cham_dau_va_cham_cuoi_khong_dac_biet() {
    dung(
        "pfn-cham",
        r#"use std.path

fn main():
    print(path.with_file_name(".gitignore", "c"))
    print(path.with_file_name("a/.gitignore", "c"))
    print(path.with_file_name("a/.a.b", "c.txt"))
    print(path.with_file_name("foo.", "c"))
    print(path.with_file_name("a/b.", ""))
    print(path.with_file_name("a/b.", "x."))
"#,
        "c\na/c\na/c.txt\nc\na/\na/x.\n",
    );
}

// ===========================================================================
// 8 — Unicode: giữ nguyên từng scalar
// ===========================================================================

/// Tên tiếng Việt/emoji/tổ hợp NFD trong **cả** đường dẫn lẫn `name`: giữ nguyên từng scalar.
#[test]
fn unicode_giu_nguyen_scalar() {
    dung(
        "pfn-unicode",
        r#"use std.path

fn main():
    print(path.with_file_name("thư/mục/tệp.txt", "mới"))
    print(path.with_file_name("đ😀.txt", "x"))
    print(path.with_file_name("thư/mục/không", "tệp😀.md"))
    print(path.with_file_name("e\u{0302}\u{0301}.txt", "m"))
    print(path.with_file_name("a/b.txt", "e\u{0302}\u{0301}"))
"#,
        "thư/mục/mới\nx\nthư/mục/tệp😀.md\nm\na/e\u{0302}\u{0301}\n",
    );
}

// ===========================================================================
// 9 — bất biến với `path.basename`: đặt lại **đúng** tên cũ ⇒ `p` không đổi
// ===========================================================================

/// Với `p` không có `/` cuối, `with_file_name(p, path.basename(p)) == p`; và tên mới đơn giản
/// luôn là `path.basename` của kết quả. Chứng minh hàm dùng **đúng** `path.basename`.
#[test]
fn bat_bien_voi_basename() {
    dung(
        "pfn-bat-bien",
        r#"use std.path

fn main():
    let ps = ["a/b.txt", "a//b", "a/./b", "./a", "../a", "/a/b", "foo.", ".gitignore", "thư/tệp.txt"]
    for p in ps:
        print(path.with_file_name(p, path.basename(p)) == p)
        print(path.basename(path.with_file_name(p, "x")) == "x")
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 10 — **không** sugar: module-only
// ===========================================================================

/// `p.with_file_name(...)` **không** phải sugar: `str` không có trường `with_file_name` ⇒
/// `VS0403`, kể cả khi đã `use std.path`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pfn-sugar-literal",
            "fn main():\n    print(\"a.txt\".with_file_name(\"c\"))\n",
        ),
        (
            "pfn-sugar-bien",
            "fn main():\n    let p = \"a.txt\"\n    print(p.with_file_name(\"c\"))\n",
        ),
        (
            "pfn-sugar-co-use",
            "use std.path\n\nfn main():\n    print(\"a.txt\".with_file_name(\"c\"))\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 11 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 1/3 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("pfn-kieu-dau", "path.with_file_name(1, \"c\")", "VS0412"),
        ("pfn-kieu-sau", "path.with_file_name(\"a\", 2)", "VS0412"),
        (
            "pfn-kieu-bool",
            "path.with_file_name(\"a\", true)",
            "VS0412",
        ),
        ("pfn-arity-1", "path.with_file_name(\"a\")", "VS0401"),
        (
            "pfn-arity-3",
            "path.with_file_name(\"a\", \"b\", \"c\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 12 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp thay tên/không ext/tên nguyên văn/tên rỗng/slash cuối/tiền tố/không tên/Unicode/NUL để
/// khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pfn-hai-engine",
        r#"use std.path

fn main():
    print(path.with_file_name("a/b.txt", "c"))
    print(path.with_file_name("a/b.tar.gz", "c"))
    print(path.with_file_name("a/b.txt", ""))
    print(path.with_file_name("a/b.txt", "x/y"))
    print(path.with_file_name("a//b", "c"))
    print(path.with_file_name("a/./b", "c"))
    print(path.with_file_name("a/b/", "c"))
    print(path.with_file_name("/a/b/", ""))
    print(path.with_file_name("thư/mục/tệp.txt", "mới"))
    print(path.with_file_name("", "c"))
    print(path.with_file_name("/", "c"))
    print(path.with_file_name("a/.", "c"))
    print(path.with_file_name("a/b.txt", "/x"))
    print(path.with_file_name("a/b.txt", "\0"))
    let r: str = path.with_file_name("x.y", "z")
    print(r)
"#,
        "a/c\na/c\na/\na/x/y\na//c\na/./c\na/c\n/a/\nthư/mục/mới\n\n/\na/.\na//x\na/\0\nz\n",
    );
}
