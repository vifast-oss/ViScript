// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-262 — biến cấp tệp (`let`/`var` ở thân tệp) phải được khởi tạo trước `fn main`**.
//!
//! Trước bản sửa: chương trình `let x = 1` + `fn main(): print(x)` qua `vis check` xanh nhưng
//! interp trap `VS0501 tên x chưa có giá trị`, VM trap `VS0501 bytecode hỏng: ô global 0 ngoài
//! số global thực tế 0` (codegen không phát câu khai báo cấp tệp nào khi có `main`).
//!
//! Hợp đồng sau bản sửa (một ngữ nghĩa cho cả hai engine):
//! 1. **Mọi** `let`/`var` cấp tệp (kể cả khai sau `fn main`) được thi hành **theo thứ tự nguồn**
//!    trước khi gọi `main`; `const` cấp tệp vẫn đi trước như cũ.
//! 2. Câu lệnh cấp tệp **không phải** khai báo (`print(...)`, gán, `if`, …) **không** chạy khi
//!    tệp có `fn main`.
//! 3. Đọc một ô cấp tệp chưa kịp khởi tạo (đọc chéo trong lúc khởi tạo) vẫn là trap `VS0501`
//!    — không có giá trị mặc định. Nhánh `let … else` không gán được **từ chối tĩnh** `VS0206`
//!    (khối `else` phải thoát — VD-263), nên không còn đường “else rơi xuống cuối rồi đọc biến”.
//!
//! Mỗi ca chạy **cả hai engine** trên **cùng một tệp**: `vis run --no-cache` (interp) và
//! `vis run --no-cache --vm`. Ca thành công so **byte** exit/stdout/stderr; ca trap chỉ so
//! exit/stdout vì hai engine có câu chữ chẩn đoán riêng (cùng mã `VS0501`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd262-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay_tep(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM trên cùng tệp, trả `[interp, vm]`. Không tự so gì.
fn hai_duong(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// Ca thành công: đòi exit + stdout + stderr **byte-khớp**, `exit = 0`, stderr rỗng.
fn thanh_cong(tag: &str, src: &str, stdout: &str) {
    let [a, b] = hai_duong(tag, src);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "{tag}: interp ≠ VM (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(a.0, 0, "{tag}: phải chạy sạch, stderr: {}", a.2);
    assert_eq!(a.1, stdout, "{tag}: stdout");
    assert_eq!(a.2, "", "{tag}: stderr phải rỗng");
}

/// Ca trap: đòi exit/stdout khớp; mỗi engine phải báo `VS0501` (câu chữ riêng được phép khác).
fn trap_hai_duong(tag: &str, src: &str, stdout: &str) {
    let [a, b] = hai_duong(tag, src);
    assert_eq!(
        (a.0, &a.1),
        (b.0, &b.1),
        "{tag}: interp ≠ VM (exit/stdout)\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_ne!(a.0, 0, "{tag}: interp phải trap");
    assert_ne!(b.0, 0, "{tag}: VM phải trap");
    assert_eq!(a.1, stdout, "{tag}: stdout interp");
    assert_eq!(b.1, stdout, "{tag}: stdout VM");
    assert!(a.2.contains("VS0501"), "{tag}: stderr interp: {}", a.2);
    assert!(b.2.contains("VS0501"), "{tag}: stderr VM: {}", b.2);
}

/// Ca **từ chối tĩnh** ở cả hai engine (exit 2, cùng mã) — dùng cho luật VD-263 áp ở cấp tệp.
fn tu_choi_hai_duong(tag: &str, src: &str, ma: &str) {
    let [a, b] = hai_duong(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(
            o.0, 2,
            "{tag}: {name} phải từ chối tĩnh (exit 2), được {}\nstdout: {}\nstderr: {}",
            o.0, o.1, o.2
        );
        assert!(
            o.1.contains(ma) || o.2.contains(ma),
            "{tag}: {name} thiếu mã `{ma}`\nstdout: {}\nstderr: {}",
            o.1,
            o.2
        );
    }
}

// ===========================================================================
// 1 — `let` cấp tệp đơn giản
// ===========================================================================

#[test]
fn let_don_gian_khoi_tao_truoc_main() {
    thanh_cong(
        "let-don-gian",
        "let x = 1\nfn main():\n    print(x)\n",
        "1\n",
    );
}

// ===========================================================================
// 2 — `var` cấp tệp: khởi tạo trước main + gán lại trong main thấy ô chung
// ===========================================================================

#[test]
fn var_khoi_tao_va_gan_lai_trong_main() {
    thanh_cong(
        "var-gan-lai",
        "var x = 1\nfn main():\n    x = 5\n    print(x)\n",
        "5\n",
    );
}

// ===========================================================================
// 3 — thứ tự nguồn: khai báo sau thấy khai báo trước
// ===========================================================================

#[test]
fn thu_tu_phu_thuoc_theo_nguon() {
    thanh_cong(
        "thu-tu",
        "let a = 2\nlet b = a + 3\nfn main():\n    print(a)\n    print(b)\n",
        "2\n5\n",
    );
}

// ===========================================================================
// 4 — initializer của khai báo cấp tệp có tác dụng phụ, chạy **trước** main
// ===========================================================================

#[test]
fn init_co_tac_dung_phu_chay_truoc_main() {
    thanh_cong(
        "init-phu",
        "fn side() -> int:\n    print(\"init\")\n    return 1\n\
         let x = side()\nfn main():\n    print(x)\n",
        "init\n1\n",
    );
}

// ===========================================================================
// 5 — `const` cấp tệp (đã có từ trước) + `let` cấp tệp
// ===========================================================================

#[test]
fn const_roi_let_cap_tep() {
    thanh_cong(
        "const-let",
        "const C: int = 2\nlet x = C + 1\nfn main():\n    print(x)\n",
        "3\n",
    );
}

// ===========================================================================
// 6 — câu lệnh cấp tệp **không phải** khai báo không chạy khi có `fn main`
// ===========================================================================

#[test]
fn bieu_thuc_cap_tep_khong_chay_khi_co_main() {
    thanh_cong(
        "bieu-thuc-khong-chay",
        "let x = 1\nprint(\"KHONG_CHAY\")\nfn main():\n    print(x)\n",
        "1\n",
    );
}

// ===========================================================================
// 6b — phép gán cấp tệp cũng là câu lệnh **không phải khai báo** ⇒ không chạy
// ===========================================================================

#[test]
fn gan_cap_tep_khong_chay_khi_co_main() {
    thanh_cong(
        "gan-khong-chay",
        "var x = 1\nx = 2\nfn main():\n    print(x)\n",
        "1\n",
    );
}

// ===========================================================================
// 7 — khai báo **sau** `fn main` vẫn được khởi tạo trước khi main được gọi
// ===========================================================================

#[test]
fn khai_bao_sau_main_van_khoi_tao_truoc() {
    thanh_cong(
        "sau-main",
        "fn main():\n    f()\nlet x = 7\nfn f():\n    print(x)\n",
        "7\n",
    );
}

// ===========================================================================
// 8 — pattern cấp tệp: mọi tên buộc phải có ô global và được ghi
// ===========================================================================

#[test]
fn let_pattern_cap_tep_gan_du_cac_ten() {
    thanh_cong(
        "let-pattern",
        "let (a, b) = (3, 4)\nfn main():\n    print(a + b)\n",
        "7\n",
    );
}

// ===========================================================================
// 9 — `let … else` cấp tệp: `else` không thoát bị từ chối tĩnh (VD-263)
// ===========================================================================

#[test]
fn let_else_cap_tep_khong_thoat_bi_tu_choi() {
    // Trước VD-263, `None` đi vào `else`, in "khong-co" rồi main đọc `x` ⇒ trap `VS0501` với
    // hai engine lệch nhau (interp trap, VM `()`). Nay luật “`else` phải thoát trên mọi nhánh”
    // áp cả cấp tệp ⇒ `VS0206` ở check/run/run --vm, không engine nào chạy tiếp.
    tu_choi_hai_duong(
        "let-else-khong-thoat",
        "let r: int? = None\nlet x = r else:\n    print(\"khong-co\")\n\
         fn main():\n    print(x)\n",
        "VS0206",
    );
}

// ===========================================================================
// 10 — đọc chéo trong lúc khởi tạo (khai báo sau chưa chạy) ⇒ trap `VS0501`
// ===========================================================================

#[test]
fn doc_cheo_trong_luc_khoi_tao_trap() {
    // `a` khởi tạo bằng `f()`; `f` thấy `b` (khai sau `a`) nhưng lúc này `b` chưa được gán.
    // In "f" trước khi đọc ⇒ chứng minh khai báo cấp tệp **đã chạy**, rồi mới trap.
    trap_hai_duong(
        "doc-cheo",
        "let a = f()\nlet b = 1\nfn f() -> int:\n    print(\"f\")\n    print(b)\n    return 0\n\
         fn main():\n    print(a)\n",
        "f\n",
    );
}

// ===========================================================================
// 11 — đường frame (test-only) cũng phải khởi tạo biến cấp tệp trước main
// ===========================================================================

#[test]
fn frame_interp_va_frame_vm_cung_khoi_tao_global() {
    use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
    use vis_vm::vm_frame::run_frame_for_test as run_v;

    let src = "let a = 2\nlet b = a + 1\nvar c = 4\nfn main():\n    c = c + b\n    print(c)\n";
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower(&p.cst, &l);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);

    let i = run_i(
        src,
        &h.hir,
        &h.symbols,
        &h.types,
        &[],
        FrameConfig::default(),
    );
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    let v = run_v(&m, src, &[], FrameConfig::default());
    assert_eq!(i.exit, 0, "frame interp: {:?}", i.diagnostics);
    assert_eq!(v.exit, 0, "frame VM: {:?}", v.diagnostics);
    assert_eq!(i.stdout, "7\n", "frame interp stdout");
    assert_eq!(v.stdout, i.stdout, "frame interp ↔ frame VM stdout");
}
