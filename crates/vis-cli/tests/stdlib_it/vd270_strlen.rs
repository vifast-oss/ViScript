// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strlen)** — độ dài chuỗi và rỗng: `std.str.byte_len`, `str.char_count`,
//! `str.is_empty`, `std.list.is_empty`, `std.map.is_empty`.
//!
//! Hợp đồng được khoá ở đây:
//! - `byte_len(s)` = **số byte UTF-8** (`"đ"` = 2, `"😀"` = 4, `"đ😀x"` = 7).
//! - `char_count(s)` = số **scalar Unicode** (`"đ😀x"` = 3) — **không** phải byte, **không**
//!   phải grapheme; cùng đơn vị `s.chars().count()`.
//! - `is_empty` cho `str`/`list`/`map` trả `bool`: `""`/`[]`/`{}` ⇒ `true`; có phần tử (kể cả
//!   một khoảng trắng) ⇒ `false`.
//! - Kiểu trả: `byte_len`/`char_count` = `int`, ba `is_empty` = `bool`; sugar R5c
//!   `s.byte_len()`, `s.char_count()`, `s.is_empty()`, `xs.is_empty()`, `m.is_empty()` chạy
//!   **không** cần `use`.
//! - **Không** có `grapheme_count`: cần bảng phân đoạn Unicode (`unicode-segmentation`) = một
//!   dependency mới, ngoài phạm vi VD-270 (zero-dep) — hoãn tới khi A0 chốt dependency.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `target/debug/vis` dựng từ cây này @ `e8e5332`):
//!
//! ```text
//! $ vis check probe.vis                    # use std.str/list/map, 5 lời gọi
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 7:11 module `std.str` không có hàm `byte_len`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:7:11: error VS0301: module `std.str` không có hàm `byte_len`
//! probe.vis:8:11: error VS0301: module `std.str` không có hàm `char_count`
//! probe.vis:9:11: error VS0301: module `std.str` không có hàm `is_empty`
//! probe.vis:10:11: error VS0301: module `std.list` không có hàm `is_empty`
//! probe.vis:11:11: error VS0301: module `std.map` không có hàm `is_empty`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng năm dòng)
//! $ vis check sugar.vis                    # "đ😀x".byte_len(), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `str` không có trường `byte_len`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-sl-{tag}-{}-{n}", std::process::id()));
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
// 1 — byte_len / char_count: ASCII, Unicode, emoji
// ===========================================================================

/// `byte_len` đếm **byte UTF-8**, `char_count` đếm **scalar**; ASCII bằng nhau, Unicode khác.
#[test]
fn byte_len_char_count_ascii_unicode_emoji() {
    dung(
        "dai",
        r#"use std.str

fn main():
    print(str.byte_len(""))
    print(str.byte_len("abc"))
    print(str.byte_len("đ"))
    print(str.byte_len("😀"))
    print(str.byte_len("đ😀x"))
    print(str.byte_len("xin chào"))
    print(str.char_count(""))
    print(str.char_count("abc"))
    print(str.char_count("đ"))
    print(str.char_count("😀"))
    print(str.char_count("đ😀x"))
    print(str.char_count("xin chào"))
    print(str.char_count("😀😀"))
"#,
        "0\n3\n2\n4\n7\n9\n0\n3\n1\n1\n3\n8\n2\n",
    );
}

/// Chuỗi chỉ khoảng trắng / xuống dòng vẫn **không** rỗng; `is_empty` theo byte ⇔ scalar.
#[test]
fn is_empty_chuoi_rong_va_khoang_trang() {
    dung(
        "rong",
        r#"use std.str

fn main():
    print(str.is_empty(""))
    print(str.is_empty(" "))
    print(str.is_empty("đ"))
    print(str.is_empty("😀"))
    print(str.byte_len(" "))
    print(str.char_count(" "))
"#,
        "true\nfalse\nfalse\nfalse\n1\n1\n",
    );
}

// ===========================================================================
// 2 — is_empty cho list / map: rỗng và không rỗng
// ===========================================================================

/// `[]`/`{}` ⇒ `true`; có phần tử ⇒ `false`; đúng cho mọi kiểu phần tử (kể cả list lồng).
#[test]
fn is_empty_list_map() {
    dung(
        "rong-lm",
        r#"use std.list
use std.map

fn main():
    let a: list[int] = []
    let b: list[int] = [1]
    print(list.is_empty(a))
    print(list.is_empty(b))
    print(list.is_empty([]))
    print(list.is_empty([1, 2]))
    print(list.is_empty(["a"]))
    print(list.is_empty([[]]))
    let m: map[str, int] = {}
    let n: map[str, int] = {"a": 1}
    print(map.is_empty(m))
    print(map.is_empty(n))
    print(map.is_empty({}))
    print(map.is_empty({"a": 1}))
    print(map.is_empty({"a": 1, "b": 2}))
"#,
        "true\nfalse\ntrue\nfalse\nfalse\nfalse\ntrue\nfalse\ntrue\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 3 — sugar phương thức R5c (không cần `use`)
// ===========================================================================

/// `s.byte_len()`/`s.char_count()`/`s.is_empty()`/`xs.is_empty()`/`m.is_empty()` ≡ lời gọi
/// module; kiểu trả qua sugar vẫn `int`/`bool` nên gán có chú thích hợp lệ.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s = "đ😀x"
    print(s.byte_len())
    print(s.char_count())
    print(s.is_empty())
    let xs: list[int] = []
    print(xs.is_empty())
    let ys: list[str] = ["a"]
    print(ys.is_empty())
    let m: map[str, int] = {}
    print(m.is_empty())
    print("".is_empty())
    print("😀".char_count())
    let n: int = s.char_count()
    let b: bool = xs.is_empty()
    print(n)
    print(b)
"#,
        "7\n3\nfalse\ntrue\nfalse\ntrue\ntrue\n1\n3\ntrue\n",
    );
}

// ===========================================================================
// 4 — kiểu trả int / bool qua biên gán
// ===========================================================================

/// `byte_len`/`char_count` là `int` (không phải `str`), ba `is_empty` là `bool` (không phải
/// `int`); biên gán đúng kiểu chạy sạch ở cả hai engine.
#[test]
fn kieu_tra_int_bool_qua_bien_gan() {
    dung(
        "kieu",
        r#"use std.str
use std.list
use std.map

fn main():
    let b: int = str.byte_len("đ")
    let c: int = str.char_count("đ")
    let e: bool = str.is_empty("")
    let le: bool = list.is_empty([])
    let me: bool = map.is_empty({})
    print(b)
    print(c)
    print(e)
    print(le)
    print(me)
"#,
        "2\n1\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 5 — ca âm tĩnh: kiểu đối số / kiểu kết quả / arity (module + sugar)
// ===========================================================================

/// Sai kiểu đối số bị checker chặn **trước khi chạy** (`VS0412`): `str` cho ba hàm `str.*`,
/// `list`/`map` cho `list.is_empty`/`map.is_empty`; sugar `is_empty` cũng kiểm.
#[test]
fn sai_kieu_doi_so_bi_chan() {
    for (tag, call) in [
        ("b-int", "str.byte_len(1)"),
        ("c-int", "str.char_count(1)"),
        ("s-int", "str.is_empty(1)"),
        ("l-str", "list.is_empty(\"abc\")"),
        ("m-list", "map.is_empty([])"),
    ] {
        let src =
            format!("use std.str\nuse std.list\nuse std.map\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0412");
    }
}

/// Sai số đối số ở lời gọi module ⇒ `VS0401`; sugar `is_empty` cũng `VS0401`.
#[test]
fn sai_so_doi_bi_chan() {
    for (tag, call) in [
        ("b-thieu", "str.byte_len()"),
        ("b-thua", "str.byte_len(\"a\", \"b\")"),
        ("c-thieu", "str.char_count()"),
        ("s-thua", "str.is_empty(\"\", 1)"),
        ("l-thieu", "list.is_empty()"),
        ("m-thua", "map.is_empty({}, 1)"),
        ("sugar-is-empty-thua", "\"a\".is_empty(1)"),
    ] {
        let src =
            format!("use std.str\nuse std.list\nuse std.map\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}

/// Kết quả `int` gán sang `str`/`bool`, kết quả `bool` gán sang `int` ⇒ `VS0412` (module + sugar).
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    tu_choi(
        "byte-len-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.byte_len(\"a\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "char-count-sang-bool",
        "use std.str\n\nfn main():\n    let x: bool = str.char_count(\"a\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "is-empty-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.is_empty(\"\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "list-is-empty-sang-int",
        "use std.list\n\nfn main():\n    let x: int = list.is_empty([])\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "map-is-empty-sang-str",
        "use std.map\n\nfn main():\n    let x: str = map.is_empty({})\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-byte-len-sang-str",
        "fn main():\n    let x: str = \"a\".byte_len()\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-is-empty-sang-int",
        "fn main():\n    let x: int = \"a\".is_empty()\n    print(x)\n",
        "VS0412",
    );
}

/// **VD-270 (R5c arity)**: sugar `s.byte_len(1)` thừa đối số bị checker chặn trước khi chạy —
/// `VS0401` `str.byte_len cần 0 đối số`, đúng như module call; không còn trap `VS0501`.
#[test]
fn sugar_sai_arity_bi_chan() {
    tu_choi(
        "sugar-arity",
        "fn main():\n    print(\"a\".byte_len(1))\n",
        "VS0401",
    );
}
