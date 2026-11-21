// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (chunks-exact)** — `std.list.chunks_exact(xs, n) -> list[list[T]]`.
//!
//! Hợp đồng được khoá ở đây:
//! - Chia `xs` thành các nhóm **đúng `n`** phần tử theo thứ tự; **bỏ phần dư** cuối —
//!   khác `list.chunk` (giữ nhóm cuối ngắn). `chunks_exact([1,2,3,4,5], 2)` ⇒
//!   `[[1,2],[3,4]]` (phần tử `5` bị bỏ).
//! - `n == 1` ⇒ mỗi phần tử một nhóm; `n == len(xs)` ⇒ **đúng một** nhóm (khi `len > 0`);
//!   `n > len(xs)` ⇒ `[]` (không phải lỗi); `xs` rỗng (với `n > 0`) ⇒ `[]`.
//! - `n <= 0` ⇒ **trap `VS0501`** (exit 101) nêu `list.chunks_exact`, **không** panic Rust;
//!   kiểm `n` **trước khi chia** nên `chunks_exact([], 0)` cũng trap (không trả `[]`).
//! - Nhóm là **list mới**, không alias `xs`: sửa `xs` sau khi chia không đổi kết quả; phần tử
//!   giữ **nguyên giá trị** (list/record lồng nhau không bị làm phẳng hay đổi hình dạng).
//! - Kiểu trả **`list[list[T]]`** (`Ty::List(&Ty::List(&Ty::Var(0)))`, cùng `T` với
//!   `list.chunk`/`list.windows`); `T` suy từ `xs` nên gán `list[list[int]]` hợp lệ, gán sang
//!   `list[int]`/`list[list[str]]` bị `VS0412`.
//! - Sugar R5c `xs.chunks_exact(n)` ≡ `list.chunks_exact(xs, n)` (không cần `use std.list`);
//!   sai kiểu đối số ⇒ `VS0412`, sai arity module/sugar ⇒ `VS0401` (checker chặn trước khi chạy).
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `17b31cb`, chạy đúng bộ test
//! dưới đây):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_chunks_exact
//! running 9 tests
//! ...
//! test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
//!
//! # stderr thật của ca dương (module):
//! cx-bien.vis:5:11: error VS0301: module `std.list` không có hàm `chunks_exact`
//! cx-long.vis:10:11: error VS0301: module `std.list` không có hàm `chunks_exact`
//! # stderr thật của ca sugar:
//! cx-sugar.vis:4:11: error VS0403: `list[int]` không có trường `chunks_exact`
//! cx-sugar.vis:9:11: error VS0403: `list[str]` không có trường `chunks_exact`
//! # ca trap: exit 2 (VS0301) thay vì 101 (VS0501); ca kiểu/arity đỏ VS0301 thay vì VS0412/VS0401
//! ```
//!
//! Sau khi thêm: **9/9 ca xanh**, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-cx-{tag}-{}-{n}", std::process::id()));
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

/// Lỗi **chạy**: hai engine cùng exit **101** (R9/C1), cùng `VS0501`, stderr nêu hàm.
fn trap_ca_hai(tag: &str, src: &str, needle: &str) {
    let (code, out, err) = hai_engine(tag, src);
    assert_eq!(
        code, 101,
        "{tag}: lỗi chạy phải exit 101\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.contains("VS0501"), "{tag}: thiếu VS0501: {err}");
    assert!(
        err.contains(needle),
        "{tag}: stderr phải nêu `{needle}`: {err}"
    );
}

// ===========================================================================
// 1 — n = 1/2/3, độ dài chia hết và có dư (bỏ phần dư cuối)
// ===========================================================================

/// Nhóm **đúng `n`**, giữ thứ tự, bỏ phần dư cuối; `n == 1` và `n == len` cũng đúng.
#[test]
fn n_1_2_3_chia_het_va_du() {
    dung(
        "cx-bien",
        r#"use std.list

fn main():
    print(list.chunks_exact([1, 2, 3, 4, 5, 6], 1))
    print(list.chunks_exact([1, 2, 3, 4, 5, 6], 2))
    print(list.chunks_exact([1, 2, 3, 4, 5, 6], 3))
    print(list.chunks_exact([1, 2, 3, 4, 5, 6], 6))
    print(list.chunks_exact([1, 2, 3, 4, 5, 6, 7], 2))
    print(list.chunks_exact([1, 2, 3, 4, 5, 6, 7], 3))
    print(list.chunks_exact([1, 2, 3, 4, 5], 2))
    print(list.chunks_exact(["a", "b", "c", "d"], 3))
    print(list.chunks_exact([1, 2, 3], 4))
"#,
        "[[1], [2], [3], [4], [5], [6]]\n\
         [[1, 2], [3, 4], [5, 6]]\n\
         [[1, 2, 3], [4, 5, 6]]\n\
         [[1, 2, 3, 4, 5, 6]]\n\
         [[1, 2], [3, 4], [5, 6]]\n\
         [[1, 2, 3], [4, 5, 6]]\n\
         [[1, 2], [3, 4]]\n\
         [[a, b, c]]\n\
         []\n",
    );
}

// ===========================================================================
// 2 — n > len, danh sách rỗng
// ===========================================================================

/// `n > len` ⇒ `[]`; `xs` rỗng với `n > 0` ⇒ `[]` (không phụ thuộc `T`).
#[test]
fn n_lon_hon_len_va_rong() {
    dung(
        "cx-rong",
        r#"use std.list

fn main():
    print(list.chunks_exact([1, 2, 3], 4))
    print(list.chunks_exact([1], 2))
    let r: list[int] = []
    print(list.chunks_exact(r, 1))
    print(list.chunks_exact(r, 9))
    let rs: list[str] = []
    print(list.chunks_exact(rs, 3))
"#,
        "[]\n[]\n[]\n[]\n[]\n",
    );
}

// ===========================================================================
// 3 — nhóm là list mới (không alias), phần tử giữ nguyên giá trị
// ===========================================================================

/// Sửa `xs` sau khi chia không đổi kết quả đã trả — nhóm là list **mới**, phần tử được sao.
#[test]
fn nhom_la_list_moi_khong_alias() {
    dung(
        "cx-alias",
        r#"use std.list

fn main():
    var xs: list[int] = [1, 2, 3, 4, 5]
    let g: list[list[int]] = list.chunks_exact(xs, 2)
    xs[0] = 9
    xs[4] = 8
    print(g)
    print(xs)
    var ys: list[list[int]] = [[1, 2], [3, 4], [5]]
    let h = list.chunks_exact(ys, 2)
    ys[0][0] = 99
    print(h)
"#,
        "[[1, 2], [3, 4]]\n[9, 2, 3, 4, 8]\n[[[1, 2], [3, 4]]]\n",
    );
}

// ===========================================================================
// 4 — phần tử list/record lồng: giữ nguyên từng giá trị
// ===========================================================================

/// Phần tử là list lồng hoặc record (ẩn danh/có tên) được giữ **nguyên** trong nhóm.
#[test]
fn phan_tu_long_list_va_record() {
    dung(
        "cx-long",
        r#"use std.list

type P:
    a: int
    b: str

fn main():
    let xs: list[list[int]] = [[1, 2], [3], [], [4, 5, 6], [7]]
    print(list.chunks_exact(xs, 2))
    print(list.chunks_exact(xs, 3))
    let ps: list[P] = [P(a: 1, b: "x"), P(a: 2, b: "y"), P(a: 3, b: "z")]
    print(list.chunks_exact(ps, 2))
    let anon = [{a: 1, b: "x"}, {a: 2, b: "y"}, {a: 3, b: "z"}, {a: 4, b: "w"}]
    print(list.chunks_exact(anon, 2))
"#,
        "[[[1, 2], [3]], [[], [4, 5, 6]]]\n\
         [[[1, 2], [3], []]]\n\
         [[P(a: 1, b: x), P(a: 2, b: y)]]\n\
         [[{a: 1, b: x}, {a: 2, b: y}], [{a: 3, b: z}, {a: 4, b: w}]]\n",
    );
}

// ===========================================================================
// 5 — sugar R5c + kiểu trả `list[list[T]]` qua biến gán
// ===========================================================================

/// `xs.chunks_exact(n)` ≡ `list.chunks_exact(xs, n)`, không cần `use std.list`; kiểu trả lồng.
#[test]
fn sugar_phuong_thuc_va_kieu_tra() {
    dung(
        "cx-sugar",
        r#"fn main():
    let xs: list[int] = [1, 2, 3, 4, 5]
    print(xs.chunks_exact(2))
    let g: list[list[int]] = xs.chunks_exact(2)
    print(g[0])
    print(g.len())
    print([10, 20, 30].chunks_exact(3))
    print(["a", "b", "c", "d"].chunks_exact(3))
    let n: int = 3
    print(xs.chunks_exact(n))
"#,
        "[[1, 2], [3, 4]]\n[1, 2]\n2\n[[10, 20, 30]]\n[[a, b, c]]\n[[1, 2, 3]]\n",
    );
}

// ===========================================================================
// 6 — ca âm chạy: n <= 0 là trap ở cả hai engine (module/sugar/biến)
// ===========================================================================

/// `n == 0`, `n < 0` (literal và **biến**) ⇒ trap `VS0501` nêu `list.chunks_exact`;
/// áp cho module lẫn sugar, kể cả khi `xs` rỗng (kiểm `n` trước khi chia).
#[test]
fn n_khong_duong_la_trap_ca_hai_engine() {
    trap_ca_hai(
        "cx-0",
        "use std.list\n\nfn main():\n    print(list.chunks_exact([1, 2, 3], 0))\n",
        "list.chunks_exact",
    );
    trap_ca_hai(
        "cx-am",
        "use std.list\n\nfn main():\n    print(list.chunks_exact([1, 2, 3], -2))\n",
        "list.chunks_exact",
    );
    trap_ca_hai(
        "cx-bien-0",
        "use std.list\n\nfn main():\n    let n = 0\n    print(list.chunks_exact([1, 2, 3], n))\n",
        "list.chunks_exact",
    );
    trap_ca_hai(
        "cx-bien-am",
        "use std.list\n\nfn main():\n    let n = -3\n    print(list.chunks_exact([1, 2, 3], n))\n",
        "list.chunks_exact",
    );
    trap_ca_hai(
        "cx-rong-0",
        "use std.list\n\nfn main():\n    let xs: list[int] = []\n    print(list.chunks_exact(xs, 0))\n",
        "list.chunks_exact",
    );
    trap_ca_hai(
        "cx-sugar-0",
        "use std.list\n\nfn main():\n    let xs: list[int] = [1]\n    print(xs.chunks_exact(0))\n",
        "list.chunks_exact",
    );
    trap_ca_hai(
        "cx-sugar-bien-am",
        "fn main():\n    let xs: list[int] = [1, 2]\n    let n = -1\n    print(xs.chunks_exact(n))\n",
        "list.chunks_exact",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: sai kiểu, sai arity (module + sugar)
// ===========================================================================

/// `xs` phải list, `n` phải `int`; kết quả phải `list[list[T]]` — sai ⇒ `VS0412`.
#[test]
fn sai_kieu_bi_chan() {
    for (tag, call, ma) in [
        ("cx-kieu-xs", "list.chunks_exact(\"abc\", 2)", "VS0412"),
        ("cx-kieu-n", "list.chunks_exact([1, 2], \"2\")", "VS0412"),
        (
            "cx-kieu-n-float",
            "list.chunks_exact([1, 2], 2.0)",
            "VS0412",
        ),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
    tu_choi(
        "cx-kieu-ket-qua-phang",
        "use std.list\n\nfn main():\n    let r: list[int] = list.chunks_exact([1, 2, 3], 1)\n    print(r)\n",
        "VS0412",
    );
    tu_choi(
        "cx-kieu-ket-qua-sai-t",
        "use std.list\n\nfn main():\n    let r: list[list[str]] = list.chunks_exact([1, 2, 3], 1)\n    print(r)\n",
        "VS0412",
    );
    tu_choi(
        "cx-sugar-kieu-n",
        "fn main():\n    let xs: list[int] = [1, 2]\n    print(xs.chunks_exact(\"2\"))\n",
        "VS0412",
    );
}

/// Sai số đối số ⇒ `VS0401` (thiếu và thừa), đường module lẫn sugar.
#[test]
fn sai_arity_bi_chan() {
    for (tag, call) in [
        ("cx-arity-0", "list.chunks_exact()"),
        ("cx-arity-1", "list.chunks_exact([1, 2])"),
        ("cx-arity-3", "list.chunks_exact([1, 2], 1, 3)"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
    for (tag, call) in [
        ("cx-sugar-arity-0", "xs.chunks_exact()"),
        ("cx-sugar-arity-2", "xs.chunks_exact(1, 2)"),
    ] {
        let src = format!("fn main():\n    let xs: list[int] = [1, 2]\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}

// ===========================================================================
// 8 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính để khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "cx-hai-engine",
        r#"use std.list

fn main():
    let xs: list[int] = [1, 2, 3, 4, 5, 6, 7]
    print(list.chunks_exact(xs, 1))
    print(list.chunks_exact(xs, 2))
    print(list.chunks_exact(xs, 3))
    print(xs.chunks_exact(3))
    let r: list[int] = []
    print(list.chunks_exact(r, 2))
    print(list.chunks_exact(xs, 8))
    let ys: list[list[str]] = [["a"], [], ["b", "c"], ["d"]]
    print(list.chunks_exact(ys, 2))
    print(ys.chunks_exact(3))
"#,
        "[[1], [2], [3], [4], [5], [6], [7]]\n\
         [[1, 2], [3, 4], [5, 6]]\n\
         [[1, 2, 3], [4, 5, 6]]\n\
         [[1, 2, 3], [4, 5, 6]]\n\
         []\n\
         []\n\
         [[[a], []], [[b, c], [d]]]\n\
         [[[a], [], [b, c]]]\n",
    );
}
