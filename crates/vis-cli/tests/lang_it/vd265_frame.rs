// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-265 — `let … else` trên đường frame phải khớp interp mặc định và VM.**
//!
//! VD-263 đã chốt ngữ nghĩa `let … else` (khối `else` phải thoát, binder theo pattern, hoàn
//! nguyên binding một phần) ở **interp mặc định + VM**. Đường frame test
//! (`crates/vis-interp/src/frame/proc.rs`) còn giữ bản cũ:
//!
//! 1. `None` rơi vào `else` nhưng **không bind** `else e` ⇒ đọc `e` trap thay vì `()`.
//! 2. `Ok`/`Some` bị coi là “khớp” **bất kể pattern**, rồi bind pattern lên *payload*: pattern
//!    bác được (`Ok(v)`, `Some(x)`, `None`, `Ok(Some(x))`, …) hoặc không bind, hoặc chạy sai
//!    nhánh (kể cả `Err(e)` trên `Err(…)` bị đẩy vào `else`).
//! 3. Pattern khác `Ok`/`Some`/`Err` bind bừa rồi **rơi xuống** dù không khớp.
//! 4. Binding một phần khi pattern bác được **không được hoàn nguyên** trước `else`.
//!
//! Bộ test này chạy **bốn đường** trên cùng HIR: interp mặc định, VM mặc định, frame interp và
//! frame VM; đòi exit/stdout byte-khớp và đúng kỳ vọng. Phần cuối khoá luôn pattern list
//! `[a, ..]`/`[a, ..r]`/`[]` và `()` trên đường frame để không lệch với hai engine chính.

use vis_core::frame::FrameConfig;
use vis_interp::frame::run_frame_for_test as run_frame_i;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;
use vis_vm::vm_frame::run_frame_for_test as run_frame_v;

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

/// Một kết quả engine đã chuẩn hoá để so chéo.
#[derive(Debug)]
struct Kq {
    stdout: String,
    exit: i32,
    codes: Vec<String>,
}

fn ma(ds: &[vis_syntax::Diagnostic]) -> Vec<String> {
    ds.iter().map(|d| d.code.short.to_string()).collect()
}

/// Chạy **bốn đường** (interp/VM mặc định + frame interp/VM) trên cùng nguồn.
fn bon_duong(src: &str) -> [Kq; 4] {
    let (hir, syms, tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");

    let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let mut vm = Vm::new(&m, src);
    let v = vm.run(&[]);
    let fi = run_frame_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    let fv = run_frame_v(&m, src, &[], FrameConfig::default());

    let i_codes = i.codes().iter().map(|s| s.to_string()).collect();
    let v_codes = v.codes().iter().map(|s| s.to_string()).collect();
    [
        Kq {
            stdout: i.stdout,
            exit: i.exit,
            codes: i_codes,
        },
        Kq {
            stdout: v.stdout,
            exit: v.exit,
            codes: v_codes,
        },
        Kq {
            stdout: fi.stdout,
            exit: fi.exit,
            codes: ma(&fi.diagnostics),
        },
        Kq {
            stdout: fv.stdout,
            exit: fv.exit,
            codes: ma(&fv.diagnostics),
        },
    ]
}

const TEN: [&str; 4] = ["interp", "vm", "frame-interp", "frame-vm"];

/// Chương trình hợp lệ: cả bốn đường chạy sạch, stdout đúng byte và **giống nhau**.
fn dung(tag: &str, src: &str, mong: &str) {
    let kq = bon_duong(src);
    for (ten, o) in TEN.iter().zip(kq.iter()) {
        assert_eq!(
            o.exit, 0,
            "{tag} [{ten}]: phải chạy sạch, được exit={} codes={:?}",
            o.exit, o.codes
        );
        assert!(
            o.codes.is_empty(),
            "{tag} [{ten}]: không được có chẩn đoán: {:?}",
            o.codes
        );
        assert_eq!(
            o.stdout, mong,
            "{tag} [{ten}]: stdout sai\n--- được ---\n{}--- muốn ---\n{mong}",
            o.stdout
        );
    }
    for (ten, o) in TEN.iter().zip(kq.iter()).skip(1) {
        assert_eq!(
            o.stdout, kq[0].stdout,
            "{tag} [{ten}] lệch stdout interp mặc định"
        );
        assert_eq!(
            o.exit, kq[0].exit,
            "{tag} [{ten}] lệch exit interp mặc định"
        );
        assert_eq!(
            o.codes, kq[0].codes,
            "{tag} [{ten}] lệch chẩn đoán interp mặc định"
        );
    }
}

// ===========================================================================
// 1 — binder `else e` theo pattern, cả bốn đường
// ===========================================================================

/// Tên trần trên `Option`: thất bại vì `None` ⇒ binder `()` (không phải payload).
#[test]
fn binder_ten_tran_option_la_unit() {
    dung(
        "binder-option",
        r#"fn main():
    let o: int? = None
    let x = o else e:
        print(e)
        return
    print(x)
"#,
        "()\n",
    );
}

/// Tên trần trên `Result`: thất bại vì `Err` ⇒ binder là payload `Err` (E).
#[test]
fn binder_ten_tran_result_la_payload_err() {
    dung(
        "binder-result",
        r#"fn main():
    let r: Result[int, str] = Err("bad")
    let x = r else e:
        print(e)
        return
    print(x)
"#,
        "bad\n",
    );
}

/// `Ok(x)` thất bại vì `Err` ⇒ binder là payload `Err`.
#[test]
fn binder_ok_la_payload_err() {
    dung(
        "binder-ok",
        r#"fn main():
    let r: Result[int, str] = Err("hong")
    let Ok(v) = r else e:
        print("else {e}")
        return
    print("ok {v}")
"#,
        "else hong\n",
    );
}

/// `Ok(_)` thất bại vì `Err` ⇒ binder vẫn là payload `Err` (pattern con không bác được).
#[test]
fn binder_ok_wild_la_payload_err() {
    dung(
        "binder-ok-wild",
        r#"fn main():
    let r: Result[int, str] = Err("x")
    let Ok(_) = r else e:
        print("else {e}")
        return
    print("after")
"#,
        "else x\n",
    );
}

/// `Err(e)` thất bại vì `Ok` ⇒ binder `()`, **không** payload `Ok`.
#[test]
fn binder_err_la_unit() {
    dung(
        "binder-err",
        r#"fn main():
    let r: Result[int, int] = Ok(5)
    let Err(e) = r else err:
        print(err)
        return
    print(e)
"#,
        "()\n",
    );
}

/// `Some(x)` thất bại vì `None` ⇒ binder `()`.
#[test]
fn binder_some_la_unit() {
    dung(
        "binder-some",
        r#"fn main():
    let o: int? = None
    let Some(x) = o else e:
        print("else {e}")
        return
    print(x)
"#,
        "else ()\n",
    );
}

/// `None` thất bại vì `Some` ⇒ binder `()`, không phải payload `Some`.
#[test]
fn binder_none_la_unit() {
    dung(
        "binder-none",
        r#"fn main():
    let o: int? = Some(5)
    let None = o else e:
        print("else {e}")
        return
    print("after")
"#,
        "else ()\n",
    );
}

/// `Ok(Some(x))` có thể thất bại vì `Ok(None)` ⇒ binder luôn `()` (không bóc payload `Err`).
#[test]
fn binder_ok_some_la_unit() {
    dung(
        "binder-ok-some",
        r#"fn main():
    let r: Result[int?, int] = Ok(None)
    let Ok(Some(x)) = r else e:
        print("else {e}")
        return
    print(x)
"#,
        "else ()\n",
    );
}

// ===========================================================================
// 2 — pattern KHỚP phải bind đúng và **không** chạy `else`
// ===========================================================================

/// `Ok(v)` trên `Ok(42)`: bind `v = 42`, không chạy `else`.
#[test]
fn ok_khop_bind_payload_khong_chay_else() {
    dung(
        "ok-khop",
        r#"fn main():
    let r: Result[int, str] = Ok(42)
    let Ok(v) = r else e:
        print("else {e}")
        return
    print("ok {v}")
"#,
        "ok 42\n",
    );
}

/// `Err(e)` trên `Err("bad")`: bind `e = "bad"`, không chạy `else`.
#[test]
fn err_khop_bind_payload_khong_chay_else() {
    dung(
        "err-khop",
        r#"fn main():
    let r: Result[int, str] = Err("bad")
    let Err(e) = r else err:
        print("else {err}")
        return
    print("ok {e}")
"#,
        "ok bad\n",
    );
}

/// `Some(x)` trên `Some(7)`: bind `x = 7`, không chạy `else`.
#[test]
fn some_khop_bind_payload_khong_chay_else() {
    dung(
        "some-khop",
        r#"fn main():
    let o: int? = Some(7)
    let Some(x) = o else e:
        print("else {e}")
        return
    print("ok {x}")
"#,
        "ok 7\n",
    );
}

/// `None` trên `None`: khớp, không chạy `else`.
#[test]
fn none_tren_none_khop_khong_chay_else() {
    dung(
        "none-khop",
        r#"fn main():
    let o: int? = None
    let None = o else e:
        print("else {e}")
        return
    print("after")
"#,
        "after\n",
    );
}

// ===========================================================================
// 3 — scope: binder `else e` và binding của pattern cùng sống trong env của khung
// ===========================================================================

/// `else e` phải **che** biến ngoài cùng tên. Trên `Option` binder là `()`; bản cũ không bind
/// nên `else` đọc nhầm biến ngoài `e = "ngoai"`.
#[test]
fn binder_else_che_bien_ngoai_cung_ten() {
    dung(
        "binder-shadow",
        r#"fn main():
    let e = "ngoai"
    let o: int? = None
    let x = o else e:
        print(e)
        return
    print(x)
"#,
        "()\n",
    );
}

/// Pattern khớp phải bind **và che** biến ngoài cùng tên (không chỉ `Ok`/`Some` là “khớp”).
#[test]
fn pattern_bind_che_bien_ngoai_cung_ten() {
    dung(
        "pattern-shadow",
        r#"fn main():
    let x = 99
    let r: Result[int, str] = Ok(5)
    let Ok(x) = r else e:
        print("else {e}")
        return
    print(x)
"#,
        "5\n",
    );
}

// ===========================================================================
// 4 — thất bại: hoàn nguyên binding một phần + không rơi xuống nhánh khớp
// ===========================================================================

/// `Ok((x, 2))` trên `Ok((1, 3))`: `x` đã bind `1` trước khi literal `2` bác được ⇒ phải hoàn
/// nguyên để `else` thấy biến **ngoài** `x = 99`, không thấy binding dở dang.
#[test]
fn that_bai_hoan_nguyen_binding_mot_phan() {
    dung(
        "hoan-nguyen",
        r#"fn main():
    let x = 99
    let r: Result[(int, int), str] = Ok((1, 3))
    let Ok((x, 2)) = r else e:
        print("else {x}")
        return
    print("ok {x}")
"#,
        "else 99\n",
    );
}

/// Cùng ca nhưng không có biến ngoài: sau khi hoàn nguyên, `else` không được thấy `x` của
/// pattern. Checker đã chặn tĩnh (`VS0301`) nên ca runtime chỉ cần chứng minh **giá trị** đúng
/// khi có biến ngoài; ca tĩnh nằm ở `vd263_letelse.rs`.
#[test]
fn that_bai_khong_roi_xuong_nhanh_khop() {
    dung(
        "khong-roi",
        r#"fn main():
    let r: Result[(int, int), str] = Ok((1, 3))
    let Ok((x, 2)) = r else e:
        print("else {e}")
        return
    print("after {x}")
"#,
        "else ()\n",
    );
}

/// `else` thoát bằng `break`/`continue`: nhánh lỗi phải thoát vòng, không rơi xuống câu sau
/// `let` và không chạy nhánh khớp.
#[test]
fn else_break_continue_khong_roi_xuong() {
    dung(
        "else-break-continue",
        r#"fn main():
    let o: int? = None
    var i = 0
    while i < 3:
        let Some(x) = o else:
            break
        print(x)
        i += 1
    print("while {i}")
    var dem = 0
    for k in 0..3:
        let Some(x) = o else:
            continue
        print(k)
        dem += 1
    print("for {dem}")
"#,
        "while 0\nfor 0\n",
    );
}

// ===========================================================================
// 5 — pattern list rest / `[]` / `()` trên đường frame
// ===========================================================================

/// `[a, ..]` / `[a, ..r]` / `[]` trong `match` và `if let` — frame phải khớp như hai engine
/// chính (không bind bừa, không trap khi list ngắn).
#[test]
fn list_rest_va_empty_khop_bon_duong() {
    dung(
        "list-rest",
        r#"fn main():
    for xs in [[], [7], [1, 2], [1, 2, 3]]:
        match xs:
            []: print("empty")
            [a, ..r]: print("{a}|{r}")
            _: print("other")
    let ys = [1, 2]
    if let [a, ..] = ys:
        print("head {a}")
    let zs = []
    if let [a, ..r] = zs:
        print("bad")
    else:
        print("no-empty")
"#,
        "empty\n7|[]\n1|[2]\n1|[2, 3]\nhead 1\nno-empty\n",
    );
}

/// `()` là pattern của `unit`, `[]` là pattern của list rỗng — không lẫn nhau trên đường frame.
#[test]
fn unit_va_list_rong_khong_lan_nhau() {
    dung(
        "unit-list",
        r#"fn main():
    match ():
        (): print("unit")
        _: print("bad")
    match []:
        []: print("empty-list")
        _: print("bad")
    match [1]:
        []: print("bad")
        _: print("nonempty")
"#,
        "unit\nempty-list\nnonempty\n",
    );
}
