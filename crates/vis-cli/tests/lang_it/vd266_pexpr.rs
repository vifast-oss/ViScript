// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-266 — `if`/`match` dạng biểu thức trong ngoặc (đối số, danh sách, map, ngoặc).**
//!
//! Trước vòng này parser chỉ nhận `if_expr`/`match_expr` khi khối kết thúc bằng
//! NEWLINE/EOF/DEDENT/`else`/`elif`; trong ngoặc lexer **không** sinh NEWLINE/INDENT/DEDENT
//! (SPEC-lex §4.3) nên mọi dạng sau đều đỏ `VS0201`:
//!
//! ```text
//! print(f(if true: 1 else: 2))              # ... mong xuống dòng cuối khối, thật ')'
//! let xs = [if true: 1 else: 2, 3]          # ... thật ','
//! let x = (if true: 1 else: 2)              # ... thật ')'
//! f(match 1:\n 1: 10\n _: 20\n)             # ... mong xuống dòng, thật '1'
//! ```
//!
//! Nay `suite()` nhận dấu đóng `)`/`]`/`}`/`,` và đầu dòng vật lý mới **khi** parser đang trong
//! ngoặc (`delim > 0`); `match` trong ngoặc lấy ranh giới nhánh theo dòng + cột. Cấp câu lệnh
//! (không ngoặc) giữ nguyên đường NEWLINE/INDENT/DEDENT.
//!
//! Mỗi ca hợp lệ chạy **cả hai engine** (`vis check`, `vis run --no-cache`, `vis run --no-cache
//! --vm`) và đòi byte exit/stdout/stderr khớp nhau. Ca âm đòi `VS0201`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd266-{tag}-{}-{n}", std::process::id()));
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
    c.arg("run").arg("--no-cache");
    c.args(extra);
    c.arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn check_src(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM, đòi **byte** exit/stdout/stderr khớp nhau.
fn chay_ca_hai(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, b.0,
        "{tag}: exit interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(
        a.1, b.1,
        "{tag}: stdout interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(
        a.2, b.2,
        "{tag}: stderr interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    [a, b]
}

/// Ca **hợp lệ**: `vis check` sạch + hai engine chạy ra cùng stdout.
fn chay_duoc(tag: &str, src: &str, muon: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_eq!(
        c, 0,
        "{tag}: `vis check` phải xanh\nstdout: {out}\nstderr: {err}"
    );
    let [a, b] = chay_ca_hai(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, muon, "{name}: stdout sai\nstderr: {}", o.2);
    }
}

/// Ca **phải bị từ chối cú pháp**: `vis check` đỏ + đúng mã; cả hai engine cũng từ chối
/// (exit 2) với cùng mã.
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_ne!(
        c, 0,
        "{tag}: `vis check` phải đỏ\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains(ma) || err.contains(ma),
        "{tag}: thiếu mã `{ma}`\nstdout: {out}\nstderr: {err}"
    );
    let (d, p) = ghi_tep(&format!("{tag}-run"), src);
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "{tag} engine={i}: thiếu mã `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 1 — `if` biểu thức trong ngoặc
// ===========================================================================

/// Hai dạng gốc của báo cáo: call lồng trong call, và call có nhiều đối số (dấu `,` kết khối).
#[test]
fn if_trong_doi_so_goi() {
    chay_duoc(
        "if-args",
        r#"fn f(x: int) -> int:
    x + 1

fn pick(a: int, b: int) -> int:
    a

fn main():
    print(f(if true: 1 else: 2))
    print(f(if false: 1 else: 2))
    print(pick(if true: 7 else: 8, 10))
"#,
        "2\n3\n7\n",
    );
}

/// Danh sách, ngoặc nhóm, chỉ số — cả ba đều là ngoặc với parser.
#[test]
fn if_trong_danh_sach_ngoac_va_chi_so() {
    chay_duoc(
        "if-list",
        r#"fn main():
    let xs = [if true: 1 else: 2, 3]
    print(xs[0])
    print(xs.len())
    let x = (if false: 10 else: 20)
    print(x)
    let y = xs[if true: 1 else: 0]
    print(y)
    var z = 0
    z = (if true: 5 else: 6)
    print(z)
"#,
        "1\n2\n20\n3\n5\n",
    );
}

/// Giá trị của map (khoá chuỗi) và của bản ghi (khoá định danh) đều nhận `if`.
#[test]
fn if_trong_gia_tri_map_va_ban_ghi() {
    chay_duoc(
        "if-map",
        r#"fn main():
    let m = {"k": if true: 1 else: 2}
    print(m["k"])
    let r = {k: if false: 3 else: 4}
    print(r.k)
"#,
        "1\n4\n",
    );
}

/// `if` nhiều dòng **trong ngoặc** (lexer nuốt NEWLINE/INDENT): thân nhánh và `else` nằm khác
/// dòng vẫn phải ghép đúng.
#[test]
fn if_nhieu_dong_trong_ngoac() {
    chay_duoc(
        "if-paren-ml",
        r#"fn main():
    let x = (
        if true:
            1
        else:
            2
    )
    print(x)
"#,
        "1\n",
    );
}

/// `if` lồng nhau: `else` bám `if` gần nhất, nên nhánh trong dùng ngoặc.
#[test]
fn if_long_nhau_trong_doi_so() {
    chay_duoc(
        "if-nested",
        r#"fn main():
    print((if true: (if false: 1 else: 2) else: 3))
    print((if false: (if true: 4 else: 5) else: 6))
"#,
        "2\n6\n",
    );
}

// ===========================================================================
// 2 — `match` biểu thức trong ngoặc
// ===========================================================================

/// Ca gốc của báo cáo: `match` nhiều nhánh làm đối số, nhánh ranh giới bằng dòng + cột.
#[test]
fn match_trong_doi_so_goi_nhieu_dong() {
    chay_duoc(
        "match-args",
        r#"fn main():
    print(match 1:
        1: 10
        _: 20
    )
    print(match 2:
        1: 10
        2: 20
        _: 30
    )
"#,
        "10\n20\n",
    );
}

/// `match` trong ngoặc nhóm và trong giá trị map — cùng đường `match_body` không NEWLINE.
#[test]
fn match_trong_ngoac_va_gia_tri_map() {
    chay_duoc(
        "match-paren-map",
        r#"fn main():
    let x = (match 1:
        1: 10
        _: 20
    )
    print(x)
    let m = {"k": match 2:
        1: 10
        2: 20
        _: 30
    }
    print(m["k"])
"#,
        "10\n20\n",
    );
}

/// `match` lồng trong nhánh `match`: cột nhỏ hơn trả nhánh về `match` cha.
#[test]
fn match_long_nhau_trong_doi_so() {
    chay_duoc(
        "match-nested",
        r#"fn main():
    print(match 1:
        1: match 2:
            3: 30
            _: 40
        _: 50
    )
"#,
        "40\n",
    );
}

/// `match` một dòng trong đối số/danh sách (một nhánh cùng dòng) + `match` nhiều nhánh trong
/// danh sách (dấu `]` kết khối).
#[test]
fn match_trong_danh_sach_mot_dong_va_nhieu_dong() {
    chay_duoc(
        "match-list",
        r#"fn main():
    print(match 1: _: 99)
    let xs = [match 2: _: 20, 7]
    print(xs[0])
    print(xs[1])
    let ys = [match 3:
        3: 30
        _: 40
    ]
    print(ys[0])
"#,
        "99\n20\n7\n30\n",
    );
}

// ===========================================================================
// 3 — cấp câu lệnh không đổi
// ===========================================================================

/// `if`/`match` cấp câu lệnh nhiều dòng vẫn đi đường NEWLINE/INDENT/DEDENT như trước.
#[test]
fn dang_cau_lenh_nhieu_dong_giu_nguyen() {
    chay_duoc(
        "stmt-ml",
        r#"fn f(n: int) -> int:
    if n > 0:
        let x = 1
        return x
    else:
        return 0

fn main():
    print(f(1))
    match 1:
        1:
            print(10)
        _:
            print(20)
"#,
        "1\n10\n",
    );
}

/// CST lossless + có đúng node biểu thức; `vis fmt` là điểm bất động và idempotent.
#[test]
fn cst_lossless_va_fmt_giu_nguyen() {
    use vis_syntax::{parse, Mode, NodeKind};

    let src = r#"fn main():
    let xs = [if true: 1 else: 2, 3]
    print(xs[0])
    let x = (match 1:
        1: 10
        _: 20
    )
    print(x)
"#;
    let out = parse(src, Mode::Script);
    assert!(
        out.diagnostics.is_empty(),
        "không được có chẩn đoán: {:?}",
        out.diagnostics
    );
    assert_eq!(out.cst.render(&out.text), out.text, "CST phải lossless");
    fn dem(n: &vis_syntax::Node, k: NodeKind) -> usize {
        let mut c = usize::from(n.kind == k);
        for x in n.children() {
            c += dem(x, k);
        }
        c
    }
    assert!(dem(&out.cst.root, NodeKind::IfExpr) >= 1);
    assert!(dem(&out.cst.root, NodeKind::MatchExpr) >= 1);

    let f = vis_fmt::format(src).expect("fmt phải parse được cú pháp mới");
    assert_eq!(f, src, "tệp đã đúng chuẩn phải là điểm bất động");
    assert_eq!(
        vis_fmt::format(&f).expect("fmt lần 2"),
        f,
        "fmt phải idempotent"
    );
}

// ===========================================================================
// 4 — ca âm
// ===========================================================================

/// `if` dùng như biểu thức **vẫn** bắt buộc `else` — dấu `)` không được thay `else`.
#[test]
fn if_thieu_else_van_la_vs0201() {
    tu_choi(
        "if-thieu-else",
        r#"fn main():
    let x = (if true: 1)
    print(x)
"#,
        "VS0201",
    );
}

/// Thụt lề sai ở cấp câu lệnh **vẫn** đỏ như trước (đường suite không ngoặc không bị nới).
#[test]
fn thut_le_sai_cap_cau_lenh_van_la_vs0201() {
    tu_choi(
        "indent-stmt",
        r#"fn main():
    if true:
    print(1)
"#,
        "VS0201",
    );
}

/// Nhánh `match` lệch cột trong ngoặc: nhánh không thuộc `match` nào ⇒ `VS0201`, không đoán.
#[test]
fn match_lech_cot_trong_doi_so_van_la_vs0201() {
    tu_choi(
        "match-lech-cot",
        r#"fn main():
    print(match 1:
        1: 10
       _: 20
    )
"#,
        "VS0201",
    );
}
