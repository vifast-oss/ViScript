// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-260 — list rest pattern `[a, ..]` / `[a, ..r]`.**
//!
//! Trước VD-260, `..` trong pattern list bị `lower_pat` nhận nhầm là **range pattern**
//! (`TestRange(0, 0)` trong bytecode) ⇒ pattern không bao giờ khớp. Bộ test này khoá:
//! - `[a, ..]` / `[a, ..r]` khớp list dài **≥** số tiền tố, bind đúng đầu/đuôi;
//! - `[..]` / `[..r]` khớp **mọi** list (kể cả rỗng) và không khớp giá trị khác list;
//! - `[]` / `[a, b]` **không đổi**; `match`/`if let`/`while let`/`for`/`let` đều đi qua;
//! - đuôi là **bản sao** (push vào đuôi không đổi list gốc);
//! - rest sai vị trí / lặp rest bị **từ chối tĩnh** (`VS0201`), không lọt xuống runtime.
//!
//! Mỗi ca chạy **cả hai đường** (`vis run` interp và `vis run --vm`) và so **byte** stdout +
//! exit code — yêu cầu "hai engine byte-identical".

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd260-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

fn chay(tep: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    c.arg(tep);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy cả hai engine, đòi exit 0 + stdout **đúng byte** + hai engine **giống nhau**.
fn chay_ca_hai(tag: &str, src: &str, mong: &str) {
    let (d, p) = ghi(tag, src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, 0, "interp exit={} stderr={}", a.0, a.2);
    assert_eq!(
        a.1, mong,
        "interp stdout khác kỳ vọng:\n--- được ---\n{}--- muốn ---\n{mong}",
        a.1
    );
    assert_eq!(b.0, a.0, "VM khác exit code: vm={:?}", b.0);
    assert_eq!(b.1, a.1, "VM khác byte stdout: vm={:?}", b.1);
}

/// Ca âm **tĩnh**: parse phải từ chối, exit 2, có mã `VS0201`; hai đường cùng kết quả.
fn chay_loi_tinh(tag: &str, src: &str) {
    let (d, p) = ghi(tag, src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    for (nhan, r) in [("interp", a), ("vm", b)] {
        assert_eq!(r.0, 2, "{nhan}: parse error phải exit 2, được {r:?}");
        assert!(
            r.2.contains("VS0201"),
            "{nhan}: phải báo VS0201, stderr={:?}",
            r.2
        );
    }
}

/// (1) `[a, ..r]` — đầu + đuôi cho list dài 1, 2, 3; `[]` vẫn khớp nhánh rỗng.
#[test]
fn vd260_match_head_tail_va_empty() {
    let src = "fn main():\n    for xs in [[], [7], [1, 2], [1, 2, 3]]:\n        match xs:\n            []: print(\"empty\")\n            [a, ..r]: print(\"{a}|{r}\")\n            _: print(\"other\")\n";
    chay_ca_hai("head_tail", src, "empty\n7|[]\n1|[2]\n1|[2, 3]\n");
}

/// (2) `[a, ..]` (không tên đuôi) — chỉ cần **độ dài ≥ 1**, không phải đúng 1.
#[test]
fn vd260_bare_rest_khong_doi_dai_dung_bang() {
    let src = "fn main():\n    for xs in [[1], [1, 2], [1, 2, 3], []]:\n        match xs:\n            [a, ..]: print(\"head {a}\")\n            _: print(\"none\")\n";
    chay_ca_hai("bare_rest", src, "head 1\nhead 1\nhead 1\nnone\n");
}

/// (3) `[..r]` bind **toàn bộ** list (bản sao); `[..]` khớp mọi list kể cả rỗng.
#[test]
fn vd260_rest_only_va_bare_only() {
    let src = "fn main():\n    for xs in [[], [1], [1, 2]]:\n        match xs:\n            [..r]: print(r)\n            _: print(\"no\")\n    match [1, 2]:\n        [..]: print(\"any\")\n        _: print(\"no-any\")\n    match []:\n        [..]: print(\"any-empty\")\n        _: print(\"no-any\")\n    match []:\n        [a, ..r]: print(\"bad {a} {r}\")\n        _: print(\"no-head\")\n";
    chay_ca_hai(
        "rest_only",
        src,
        "[]\n[1]\n[1, 2]\nany\nany-empty\nno-head\n",
    );
}

/// (4) Tiền tố literal + list lồng nhau: `[1, ..r]`, `[[a, ..], ..tail]`.
#[test]
fn vd260_tien_to_literal_va_long_nhau() {
    let src = "fn main():\n    let xs = [[1, 2], [3]]\n    match xs:\n        [[a, ..], ..tail]: print(\"{a} {tail}\")\n        _: print(\"no\")\n    match [9, 8, 7]:\n        [9, ..r]: print(r)\n        _: print(\"no\")\n    for ys in [[], [2], [9, 8]]:\n        match ys:\n            [9, ..r]: print(\"lit {r}\")\n            _: print(\"lit-no\")\n    match 5:\n        [9, ..r]: print(\"bad\")\n        _: print(\"int-no\")\n";
    chay_ca_hai(
        "prefix_nested",
        src,
        "1 [[3]]\n[8, 7]\nlit-no\nlit-no\nlit [8]\nint-no\n",
    );
}

/// (4b) Exact `[a]` / `[a, b]` **không đổi** — list rỗng/ngắn chỉ là **không khớp**, không trap.
#[test]
fn vd260_exact_khong_doi_va_khong_trap() {
    let src = "fn main():\n    for xs in [[], [2], [1]]:\n        match xs:\n            [1]: print(\"one\")\n            _: print(\"other\")\n    match [1, 2]:\n        [a, b]: print(\"pair {a} {b}\")\n        _: print(\"no-pair\")\n";
    chay_ca_hai("exact", src, "other\nother\none\npair 1 2\n");
}

/// (5) `if let` — khớp thì bind, list rỗng/short thì sang `else` (không bind bừa).
#[test]
fn vd260_if_let_rest_va_else() {
    let src = "fn main():\n    let xs = [1, 2]\n    if let [a, ..r] = xs:\n        print(\"{a} {r}\")\n    else:\n        print(\"no\")\n    let ys = []\n    if let [a, ..r] = ys:\n        print(\"bad {a} {r}\")\n    else:\n        print(\"no-empty\")\n";
    chay_ca_hai("if_let", src, "1 [2]\nno-empty\n");
}

/// (6) `while let` — dừng đúng ở phần tử list rỗng, không treo.
#[test]
fn vd260_while_let_rest_dung_o_list_rong() {
    let src = "fn main():\n    let rows = [[1, 2], [3], []]\n    var i = 0\n    while let [h, ..t] = rows[i]:\n        print(\"row {h} {t}\")\n        i = i + 1\n    print(\"stop {i}\")\n";
    chay_ca_hai("while_let", src, "row 1 [2]\nrow 3 []\nstop 2\n");
}

/// (7) `for` pattern — mỗi phần tử là list, bind đầu/đuôi theo từng vòng.
#[test]
fn vd260_for_rest() {
    // **VD-260 semantics**: pattern rest là refutable ⇒ `for`/`let` phải dùng `if let`/`else`.
    let src = "fn main():\n    for xs in [[5, 6], [7]]:\n        if let [h, ..t] = xs:\n            print(\"for {h} {t}\")\n";
    chay_ca_hai("for_rest", src, "for 5 [6]\nfor 7 []\n");
}

/// (8) `let` destructuring + **bản sao**: `r.push(99)` không đổi `xs`/`r`.
#[test]
fn vd260_let_rest_va_copy_semantics() {
    // **VD-260 semantics**: `let [a, ..r]` là refutable ⇒ dùng `if let` (pattern phủ hết
    // mới được `let` trần; `let … else` chỉ cho Option/Result). Ca này khoá bản sao đuôi.
    let src = "fn main():\n    let xs = [1, 2, 3]\n    if let [a, ..r] = xs:\n        let r2 = r.push(99)\n        print(a)\n        print(xs)\n        print(r)\n        print(r2)\n";
    chay_ca_hai("let_copy", src, "1\n[1, 2, 3]\n[2, 3]\n[2, 3, 99]\n");
}

/// (9) Chủ thể **không phải list** (int/str/record) không khớp rest pattern — như interp.
#[test]
fn vd260_khong_phai_list_thi_khong_khop() {
    let src = "fn main():\n    match 5:\n        [..]: print(\"int-list\")\n        _: print(\"int-no\")\n    match \"abc\":\n        [..]: print(\"str-list\")\n        _: print(\"str-no\")\n    match {a: 1}:\n        [..]: print(\"rec-list\")\n        _: print(\"rec-no\")\n";
    chay_ca_hai("not_list", src, "int-no\nstr-no\nrec-no\n");
}

/// (10) Guard: bind đầu/đuôi **trước** guard; or-pattern với rest vẫn bind đúng alternative.
#[test]
fn vd260_guard_va_or_pattern() {
    let src = "fn main():\n    for xs in [[5, 6], [7]]:\n        match xs:\n            [a, ..r] if a > 5: print(\"big {a} {r}\")\n            [a, ..r]: print(\"small {a} {r}\")\n            _: print(\"none\")\n    for xs in [[1, 2], [3]]:\n        match xs:\n            [a, ..] | [a]: print(\"any {a}\")\n            _: print(\"none\")\n";
    chay_ca_hai("guard_or", src, "small 5 [6]\nbig 7 []\nany 1\nany 3\n");
}

/// (11) Rest không ở cuối — parser phải từ chối **tĩnh** (`VS0201`).
#[test]
fn vd260_rest_khong_o_cuoi_bi_tu_choi() {
    let mau = "fn main():\n    match [1, 2]:\n        [a, ..r, b]: print(a)\n        _: print(0)\n";
    chay_loi_tinh("rest_mid", mau);
    let mau = "fn main():\n    match [1, 2]:\n        [a, .., b]: print(a)\n        _: print(0)\n";
    chay_loi_tinh("rest_mid_bare", mau);
}

/// (12) Hậu tố `[.., z]` + rest lặp `[.., ..]` — ngữ pháp hiện **không** hỗ trợ hậu tố sau
/// `..`; phải là lỗi tĩnh `VS0201`, không được lặng lẽ bỏ phần hậu tố.
#[test]
fn vd260_hau_to_va_rest_lap_bi_tu_choi() {
    let hau_to = "fn main():\n    match [1, 2]:\n        [.., z]: print(z)\n        _: print(0)\n";
    chay_loi_tinh("suffix", hau_to);
    let lap = "fn main():\n    match [1, 2]:\n        [a, .., ..]: print(a)\n        _: print(0)\n";
    chay_loi_tinh("double_rest", lap);
}
