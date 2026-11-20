// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (flatten)** — `std.list.flatten(xs) -> list[T]` với `xs: list[list[T]]`.
//!
//! Hợp đồng được khoá ở đây:
//! - Nối các list con **theo thứ tự**, **một cấp** duy nhất: `[[1, 2], [3]]` ⇒ `[1, 2, 3]`.
//! - `xs` rỗng ⇒ `[]`; list con rỗng **không** đóng góp phần tử nào.
//! - Phần tử của list con **vốn là list** được giữ **nguyên** — không làm phẳng đệ quy:
//!   `[[[1], [2]], [[3]]]` ⇒ `[[1], [2], [3]]`.
//! - Chữ ký `(list[list[T]]) -> list[T]` (`Ty::List(&Ty::List(&Ty::Var(0)))` → `Ty::List(&Ty::Var(0))`);
//!   `T` buộc từ `xs`, sai kiểu/sai số đối số bị checker chặn (`VS0412`/`VS0401`).
//! - Sugar R5c `xs.flatten()` ≡ `list.flatten(xs)`, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `580123b`):
//!
//! ```text
//! $ vis check probe.vis                  # use std.list; list.flatten(xs), xs.flatten()
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.list` không có hàm `flatten`
//! exit=1
//! $ vis run --no-cache probe.vis
//! probe.vis:5:11: error VS0301: module `std.list` không có hàm `flatten`
//! probe.vis:6:11: error VS0403: `list[list[int]]` không có trường `flatten`
//! exit=2
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng hai dòng)
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-fl-{tag}-{}-{n}", std::process::id()));
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
// 1 — nhiều độ dài list con, thứ tự; `str` cũng chạy
// ===========================================================================

/// Độ dài 0, 1, nhiều; thứ tự giữ nguyên; list con rỗng không đóng góp gì.
#[test]
fn nhieu_do_dai_va_thu_tu() {
    dung(
        "fl-do-dai",
        r#"use std.list

fn main():
    let xs: list[list[int]] = [[1, 2, 3], [], [4], [5, 6]]
    print(list.flatten(xs))
    print(list.flatten([[7]]))
    print(list.flatten([[], []]))
    print(list.flatten([["a", "b"], [], ["c"]]))
"#,
        "[1, 2, 3, 4, 5, 6]\n[7]\n[]\n[a, b, c]\n",
    );
}

// ===========================================================================
// 2 — xs rỗng, list con rỗng
// ===========================================================================

/// `xs` rỗng (có chú thích kiểu) ⇒ `[]`; toàn list con rỗng ⇒ `[]`; `str` rỗng cũng vậy.
#[test]
fn xs_rong_va_list_con_rong() {
    dung(
        "fl-rong",
        r#"use std.list

fn main():
    let r: list[list[int]] = []
    print(list.flatten(r))
    print(r.flatten())
    print(list.flatten([[], []]))
    let rs: list[list[str]] = []
    print(list.flatten(rs))
"#,
        "[]\n[]\n[]\n[]\n",
    );
}

// ===========================================================================
// 3 — một cấp: phần tử vốn là list được giữ nguyên
// ===========================================================================

/// `[[[1], [2]], [[3], []]]` ⇒ `[[1], [2], [3], []]` — **không** đệ quy thành `[1, 2, 3]`.
#[test]
fn giu_nguyen_phan_tu_list_khong_de_quy() {
    dung(
        "fl-mot-cap",
        r#"use std.list

fn main():
    let xss = [[[1], [2]], [[3], []], []]
    print(list.flatten(xss))
    let mot: list[list[int]] = list.flatten(xss)
    print(mot)
    print([[[1, 2]], [[3]]].flatten())
"#,
        "[[1], [2], [3], []]\n[[1], [2], [3], []]\n[[1, 2], [3]]\n",
    );
}

// ===========================================================================
// 4 — kiểu trả `list[T]` qua biến gán
// ===========================================================================

/// `T` suy từ `xs`; gán `list[int]` hợp lệ, chỉ số cho `int`, `.len()` đúng tổng.
#[test]
fn kieu_tra_mot_cap_qua_bien_gan() {
    dung(
        "fl-kieu",
        r#"use std.list

fn main():
    let xs: list[list[int]] = [[3, 1], [4, 1, 5]]
    let r: list[int] = list.flatten(xs)
    print(r)
    print(r[0])
    print(r.len())
    let s: list[list[str]] = [["a"], ["b", "c"]]
    print(list.flatten(s))
"#,
        "[3, 1, 4, 1, 5]\n3\n5\n[a, b, c]\n",
    );
}

// ===========================================================================
// 5 — sugar R5c
// ===========================================================================

/// `xs.flatten()` ≡ `list.flatten(xs)`; chạy trên `list[list[int]]` lẫn `list[list[list[int]]]`,
/// không cần `use std.list`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "fl-sugar",
        r#"fn main():
    let xs: list[list[int]] = [[1, 2], [], [3]]
    print(xs.flatten())
    let r: list[int] = xs.flatten()
    print(r)
    print([["a"], ["b", "c"]].flatten())
    let xss: list[list[list[int]]] = [[[1], [2]], [[3]]]
    let p: list[list[int]] = xss.flatten()
    print(p)
"#,
        "[1, 2, 3]\n[1, 2, 3]\n[a, b, c]\n[[1], [2], [3]]\n",
    );
}

// ===========================================================================
// 6 — ca âm tĩnh: kiểu phần tử / kiểu kết quả / arity / sugar
// ===========================================================================

/// `xs` phải `list[list[T]]` (`list[int]`/`str` ⇒ `VS0412`); kết quả là **một** cấp nên gán
/// `list[list[int]]` ⇔ `list[int]` sai chiều bị chặn; arity 0/2 ⇒ `VS0401`; sugar trên
/// `list[int]` ⇒ `VS0403` (không có trường `flatten`).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "list.flatten([1, 2])", "VS0412"),
        ("kieu-str", "list.flatten(\"abc\")", "VS0412"),
        ("arity-0", "list.flatten()", "VS0401"),
        ("arity-2", "list.flatten([[]], 1)", "VS0401"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
    tu_choi(
        "kieu-ket-qua-hai-cap",
        "use std.list\n\nfn main():\n    let r: list[list[int]] = list.flatten([[1], [2]])\n    print(r)\n",
        "VS0412",
    );
    tu_choi(
        "kieu-ket-qua-mot-cap",
        "use std.list\n\nfn main():\n    let r: list[int] = list.flatten([[[1]]])\n    print(r)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-kieu-nhan",
        "fn main():\n    print([1, 2].flatten())\n",
        "VS0403",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: sugar sai arity ⇒ checker chặn (`VS0401`), không trap
// ===========================================================================

/// **VD-270 (R5c arity)**: sugar `xs.flatten(1)` **thừa** đối số bị checker chặn trước khi chạy
/// — `VS0401` `list.flatten cần 0 đối số`, cùng đường module call; không còn trap `VS0501`.
#[test]
fn sugar_thua_doi_bi_chan() {
    tu_choi(
        "fl-sugar-thua",
        "fn main():\n    let xs: list[list[int]] = [[1], [2]]\n    print(xs.flatten(1))\n",
        "VS0401",
    );
}

// ===========================================================================
// 8 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính vào một chương trình để khoá **byte** stdout/stderr/exit giữa hai
/// engine, kể cả khi in trực tiếp kết quả lồng nhau.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "fl-hai-engine",
        r#"use std.list

fn main():
    let xs: list[list[int]] = [[1, 2, 3], [], [4], [5, 6]]
    print(list.flatten(xs))
    print(xs.flatten())
    let rong: list[list[int]] = []
    print(list.flatten(rong))
    let xss: list[list[list[int]]] = [[[1], [2]], [[3], []]]
    print(list.flatten(xss))
    let p: list[list[int]] = xss.flatten()
    print(p)
    print([["a", "b"], ["c"]].flatten())
"#,
        "[1, 2, 3, 4, 5, 6]\n[1, 2, 3, 4, 5, 6]\n[]\n[[1], [2], [3], []]\n[[1], [2], [3], []]\n[a, b, c]\n",
    );
}
