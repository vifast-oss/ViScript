// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-238 / VD-242 đợt A** — hồi quy parity ca 1,2,3,4,7,8 từ
//! `agent/JOURNAL/st234-lech-vm-interp.md`. Mỗi ca: (a) interp ↔ VM **khớp byte**, và
//! (b) kết quả **đúng** theo căn cứ đã ghi (không nhận parity nếu cả hai cùng sai — VD-242 §3.1).

use vis_bytecode::compile;
use vis_interp::RunOutput;

fn chay_interp(text: &str) -> RunOutput {
    let parsed = vis_syntax::parse(text, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(text, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    vis_interp::run(text, &h.hir, &h.symbols, &h.types, &[])
}

fn chay_vm(text: &str) -> RunOutput {
    let parsed = vis_syntax::parse(text, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(text, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let m = compile(&h.hir, &h.symbols, text).expect("compile");
    vis_vm::run(&m, text, &[])
}

fn mo_ta(o: &RunOutput) -> String {
    let d: Vec<String> = o
        .diagnostics
        .iter()
        .map(|x| format!("{}:{}:{} {}", x.code_str(), x.line, x.col, x.message))
        .collect();
    format!("stdout={:?} exit={} diag={:?}", o.stdout, o.exit, d)
}

/// Kiểm parity + đúng kết quả. `diag_contains` là một phần thông điệp chẩn đoán cần có (để phân
/// biệt trap thật với im-lặng-thành-công); để trống nếu ca không có chẩn đoán.
fn parity(ca: &str, src: &str, stdout: &str, diag_contains: &str) {
    let a = chay_interp(src);
    let b = chay_vm(src);
    assert_eq!(mo_ta(&a), mo_ta(&b), "{ca}: interp ↔ VM lệch");
    assert_eq!(a.stdout, stdout, "{ca}: stdout sai (cả hai cùng sai?)");
    // exit được CLI ánh xạ trap → 101; ở harness in-process trap trả 1. Parity ở `mo_ta` đã so exit.
    if diag_contains.is_empty() {
        assert!(a.diagnostics.is_empty(), "{ca}: không Expected chẩn đoán");
    } else {
        assert!(
            a.diagnostics
                .iter()
                .any(|d| d.message.contains(diag_contains)),
            "{ca}: thiếu chẩn đoán chứa `{diag_contains}`"
        );
    }
}

#[test]
fn ca1_for_long_break_break_pop_iterator() {
    parity(
        "ca1",
        "fn main():\n    for i in 0..2:\n        for j in 0..3:\n            break\n        print(i)\n    print(\"done\")\n",
        "0\n1\ndone\n",
        "",
    );
}

#[test]
fn ca2_while_let_stack_can_bang() {
    parity(
        "ca2",
        "fn main():\n    if true:\n        var x: int? = None\n        while let Some(v) = x:\n            print(v)\n    print(\"after\")\n",
        "after\n",
        "",
    );
}

#[test]
fn ca3_let_cap_tep_trong_ham() {
    parity(
        "ca3",
        "let base = 100\n\nfn add(x: int) -> int:\n    x + base\n\nprint(add(1))\n",
        "101\n",
        "",
    );
}

#[test]
fn ca4_const_cap_tep() {
    parity(
        "ca4",
        "const N: int = 10\n\nfn f() -> int:\n    N + 1\n\nfn main():\n    print(N)\n    print(f())\n",
        "10\n11\n",
        "",
    );
}

#[test]
fn ca7_match_roi_khong_im_lang() {
    parity(
        "ca7",
        "fn main():\n    let o: int? = Some(2)\n    match o:\n        Some(1): print(\"one\")\n        None: print(\"none\")\n    print(\"after\")\n",
        "",
        "không khớp nhánh nào",
    );
}

#[test]
fn ca8_closure_ghi_bien_bat_giu_trang_thai() {
    parity(
        "ca8",
        "fn main():\n    var x = 1\n    let f = fn() -> int:\n        x = x + 10\n        x\n    print(f())\n    print(f())\n    print(x)\n",
        "11\n21\n1\n",
        "",
    );
}
