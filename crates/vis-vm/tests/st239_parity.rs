// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-239 / VD-242 đợt B** — hồi quy parity ca 5,6,9,10 (`st234-lech-vm-interp.md`) + hợp đồng
//! Option/map §3.2–3.4 (ca 161,162,165,166,170 của ma trận 28). Nguyên tắc VD-242 §3.1:
//! chương trình **không hợp lệ** phải bị từ chối — parity giữa hai engine cùng sai không tính đạt.

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

/// Parity + đúng kết quả. `ma` là mã chẩn đoán mong đợi nếu chương trình bị từ chối tĩnh;
/// để trống nếu chương trình hợp lệ (khi đó cả hai engine phải in đúng `stdout`).
fn parity(ca: &str, src: &str, stdout: &str) {
    let a = chay_interp(src);
    let b = chay_vm(src);
    assert_eq!(mo_ta(&a), mo_ta(&b), "{ca}: interp ↔ VM lệch");
    assert_eq!(a.stdout, stdout, "{ca}: stdout sai (cả hai cùng sai?)");
    assert!(a.diagnostics.is_empty(), "{ca}: không Expected chẩn đoán");
}

/// Chương trình **không hợp lệ** ⇒ tyck phải từ chối với mã `VS04xx`. Không đưa vào hai engine.
fn tu_choi(ca: &str, src: &str, ma: &str) {
    let parsed = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(
        parsed.diagnostics.is_empty(),
        "{ca}: parse: {:?}",
        parsed.diagnostics
    );
    let lexed = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower(&parsed.cst, &lexed);
    assert!(h.is_clean(), "{ca}: lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(
        t.diags.iter().any(|d| d.code_str() == ma),
        "{ca}: thiếu {ma}: {:?}",
        t.diags
            .iter()
            .map(|d| (d.code_str(), d.message.clone()))
            .collect::<Vec<_>>()
    );
}

// ---- #161 / ca 5: `?.` trên base optional — VM đúng, interp thiếu lift ----

#[test]
fn ca161_hoi_cham_base_optional_nang_mot_lop() {
    parity(
        "ca161",
        "type U:\n    name: str\n\nfn main():\n    let u: U? = Some({name: \"An\"})\n    print(u?.name)\n    let n = u?.name\n    print(n)\n    print(n ?? \"?\")\n    let a: U? = None\n    print(a?.name)\n",
        "Some(An)\nSome(An)\nAn\nNone\n",
    );
}

/// VD-242 §3.2: field `str?` ⇒ kiểu `str??`, không flatten; `lift_opt(1)` của VM không bọc chồng
/// field đã `Opt`, nên `Some("x")` giữ `Some(x)` và `None` giữ `None`.
#[test]
fn ca161_hoi_cham_khong_flatten_field_optional() {
    parity(
        "ca161-field-opt",
        "type U:\n    f: str?\nfn main():\n    let u: U? = Some({f: Some(\"x\")})\n    print(u?.f)\n    let v: U? = Some({f: None})\n    print(v?.f)\n",
        "Some(x)\nNone\n",
    );
}

// ---- #162 / ca 6: `?.` trên base không optional ⇒ từ chối tĩnh ----

#[test]
fn ca162_hoi_cham_base_thuong_bi_tu_choi() {
    tu_choi(
        "ca162",
        "type U:\n    name: str\nfn main():\n    let u: U = {name: \"An\"}\n    print(u?.name)\n",
        "VS0412",
    );
}

// ---- #165 / ca 9: or-pattern bind không đầy đủ ⇒ từ chối (KHÔNG sửa VM trả `2`) ----

#[test]
fn ca165_or_pattern_bind_khong_day_du_bi_tu_choi() {
    tu_choi(
        "ca165",
        "fn main():\n    let n = 2\n    match n:\n        1 | x: print(x)\n",
        "VS0412",
    );
}

/// Or-pattern mà mọi alternative đều bind cùng tên thì vẫn hợp lệ (không từ chối oan).
#[test]
fn ca165_or_pattern_bind_day_du_van_hop_le() {
    parity(
        "ca165-ok",
        "fn main():\n    let n = 2\n    match n:\n        x | x: print(x)\n",
        "2\n",
    );
}

// ---- #166 / ca 10: `set{…}` VM phải dedup như interp ----

#[test]
fn ca166_set_dedup() {
    parity(
        "ca166",
        "fn main():\n    print(set{1, 2, 2, 3, 1})\n",
        "[1, 2, 3]\n",
    );
}

// ---- #170: map khoá trùng (VD-242 Q4a + §3.4) ----

#[test]
fn ca170_literal_khoa_trung_tinh_bi_tu_choi() {
    tu_choi(
        "ca170-static",
        "fn main():\n    let a = {\"x\": 1, \"x\": 2}\n    print(a)\n",
        "VS0412",
    );
    // Cùng giá trị số dù viết khác kind (`1` vs `1.0`) cũng là trùng tĩnh.
    tu_choi(
        "ca170-static-num",
        "fn main():\n    let a = {1: \"a\", 1.0: \"b\"}\n    print(a)\n",
        "VS0412",
    );
}

#[test]
fn ca170_runtime_last_wins_va_equality() {
    parity(
        "ca170-runtime",
        "let k = \"x\"\nlet a = {(k): 1, \"x\": 2}\nprint(a)\nprint(a.len())\nlet b = {\"x\": 2}\nprint(a == b)\n",
        "{\"x\": 2}\n1\ntrue\n",
    );
}

// Chú thích: `json.decode` last-wins được kiểm qua fixture `test/sem/map/trung-khoa-json.vis`
// (cần `use std.json`, chỉ CLI xử lý `use`). Đây là **lựa chọn của ViScript**, không phải luật
// JSON bắt buộc — ghi rõ ở tài liệu `Value::Map` (`crates/vis-interp/src/value.rs`).
