// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268 — gán lồng** (`a.b.f = v`, `xs[0].f = v`, `m["k"].f = v`, `a.bs[0].f = v`).
//!
//! Trước VD-268 cả ba engine đều **từ chối tĩnh** mọi đích sâu hơn một tầng
//! (`VS0412: gán **lồng** …`). Nay đích là **đường** từ một biến `var` (hoặc `var self`) đi
//! xuống qua **trường bản ghi** và **chỉ số list/map**; mỗi bước được kiểm kiểu, `?.` bị từ chối,
//! gốc bất biến / biểu thức tạm bị từ chối, và cả hai engine phải **byte-khớp** (exit/stdout/
//! stderr). Ngữ nghĩa là **giá trị**: đọc bản sao rồi dựng lại từng tầng, nên `let b = a.b`
//! không thấy thay đổi của `a.b.f`.
//!
//! **Đỏ-trước (bằng chứng)**: trước bản sửa, mọi ca `.out` dưới đây đều đỏ với
//! `VS0412 … gán **lồng** … chưa hỗ trợ` (và ca `+=` còn đỏ sớm hơn ở parser `VS0201`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-vd268-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(format!("{ten}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    p
}

fn chay(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run");
    if vm {
        c.arg("--vm").arg("--no-cache");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn check(p: &Path) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(p)
        .output()
        .expect("chạy `vis check`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy **cả hai engine** trên cùng tệp; đòi exit + stdout + stderr **byte-khớp**.
fn hai_duong(d: &Path, ten: &str, src: &str) -> (i32, String, String) {
    let p = ghi(d, ten, src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "interp ≠ `--vm` (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    a
}

/// Từ chối **tĩnh**: `vis check` đỏ + `vis run` hai đường **cùng** chẩn đoán, exit **2**.
fn tu_choi_tinh(d: &Path, ten: &str, src: &str, ma: &str, manh: &[&str]) -> String {
    let p = ghi(d, ten, src);
    let (cc, co, ce) = check(&p);
    assert_eq!(cc, 1, "`check` phải đỏ: stdout={co:?} stderr={ce:?}");
    let a = chay(&p, false);
    let b = chay(&p, true);
    assert_eq!((a.0, b.0), (2, 2), "interp: {a:?}\nvm: {b:?}");
    assert_eq!(a.2, b.2, "chẩn đoán hai đường phải **y hệt**");
    assert!(a.2.contains(ma), "stderr: {}", a.2);
    for m in manh {
        assert!(a.2.contains(m), "thông điệp phải nêu `{m}`: {}", a.2);
    }
    assert!(
        !a.2.contains("chưa hỗ trợ"),
        "không còn thông điệp 'chưa hỗ trợ': {}",
        a.2
    );
    a.2
}

const P: &str = "type P:\n    x: int\n\ntype Q:\n    p: P\n\n";

// ---------------------------------------------------------------------------
// Các đích lồng chạy được — cả hai engine, stdout từng byte
// ---------------------------------------------------------------------------

#[test]
fn ban_ghi_trong_ban_ghi() {
    // `q.p.x = 2` — hai tầng trường, gốc `var`.
    let d = tam("rec-rec");
    let src =
        format!("{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    print(q.p.x)\n");
    let (c, o, e) = hai_duong(&d, "rec-rec", &src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "2\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn phan_tu_list_la_ban_ghi() {
    // `xs[1].x = 9` — `Index` rồi `Field`, gốc `var`.
    let d = tam("list-rec");
    let src = format!(
        "{P}fn main():\n    var xs: list[P] = [{{x: 1}}, {{x: 2}}]\n    xs[1].x = 9\n    print(xs[0].x)\n    print(xs[1].x)\n"
    );
    let (c, o, e) = hai_duong(&d, "list-rec", &src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "1\n9\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn truong_ban_ghi_la_list_va_list_long() {
    // `p.xs[1] = 9` (một tầng trường + chỉ số) và `a.bs[0].n = 7` (ba tầng).
    let d = tam("rec-list");
    let src = "type P:\n    xs: list[int]\n\ntype I:\n    n: int\n\ntype O:\n    bs: list[I]\n\nfn main():\n    var p: P = {xs: [1, 2, 3]}\n    p.xs[1] = 9\n    print(p.xs)\n    var a: O = {bs: [{n: 1}, {n: 2}]}\n    a.bs[0].n = 7\n    print(a.bs[0].n)\n    print(a.bs[1].n)\n";
    let (c, o, e) = hai_duong(&d, "rec-list", src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "[1, 9, 3]\n7\n2\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn gia_tri_map_la_ban_ghi() {
    // `m["b"].x = 9` — `Index` map rồi `Field`.
    let d = tam("map-rec");
    let src = format!(
        "{P}fn main():\n    var m: map[str, P] = {{\"a\": {{x: 1}}, \"b\": {{x: 2}}}}\n    m[\"b\"].x = 9\n    print(m[\"a\"].x)\n    print(m[\"b\"].x)\n"
    );
    let (c, o, e) = hai_duong(&d, "map-rec", &src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "1\n9\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn chi_so_long_hai_tang() {
    // `xs[1][0] = 9` và `m["a"][1] += 1` — chuỗi `Index`/`Index`, kể cả gán ghép.
    let d = tam("idx-idx");
    let src = "fn main():\n    var xs: list[list[int]] = [[1, 2], [3, 4]]\n    xs[1][0] = 9\n    print(xs)\n    var m: map[str, list[int]] = {\"a\": [1, 2]}\n    m[\"a\"][1] += 8\n    print(m)\n";
    let (c, o, e) = hai_duong(&d, "idx-idx", src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "[[1, 2], [9, 4]]\n{\"a\": [1, 10]}\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn ghep_long_tren_so_nguyen() {
    // `q.p.n += 2`: parser nhận `+=` sau **đường** lvalue, tyck kiểm như `n + 2`, runtime ghi lá.
    let d = tam("ghep-long");
    let src = "type P:\n    n: int\n\ntype Q:\n    p: P\n\nfn main():\n    var q: Q = {p: {n: 40}}\n    q.p.n += 2\n    print(q.p.n)\n";
    let (c, o, e) = hai_duong(&d, "ghep-long", src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "42\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn ghep_mot_tang_tren_truong_va_phan_tu() {
    // Hồi quy ngữ nghĩa cũ: `p.x += 1`, `xs[1] += 1` vẫn là lá **một tầng**, hai engine khớp.
    let d = tam("ghep-mot-tang");
    let src = format!(
        "{P}fn main():\n    var p: P = {{x: 1}}\n    p.x += 1\n    print(p.x)\n    var xs = [1, 2]\n    xs[1] += 5\n    print(xs)\n"
    );
    let (c, o, e) = hai_duong(&d, "ghep-mot-tang", &src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "2\n[1, 7]\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn doc_lap_voi_ban_sao() {
    // `let b = q.p` là **bản sao giá trị**; `q.p.x = 2` không được đổi `b.x`.
    let d = tam("copy");
    let src = format!(
        "{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    let b: P = q.p\n    q.p.x = 2\n    print(b.x)\n    print(q.p.x)\n"
    );
    let (c, o, e) = hai_duong(&d, "copy", &src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "1\n2\n");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn chi_so_ngoai_pham_vi_trap_hai_duong() {
    // Đọc tầng giữa ngoài phạm vi ⇒ trap `VS0503` **trước** khi đánh giá vế phải / ghi.
    let d = tam("oob");
    let src = format!(
        "{P}fn hong() -> int:\n    print(\"rhs\")\n    1\n\nfn main():\n    var xs: list[P] = [{{x: 1}}]\n    xs[3].x = hong()\n    print(\"khong\")\n"
    );
    let p = ghi(&d, "oob", &src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    assert_eq!((a.0, b.0), (101, 101), "interp: {a:?}\nvm: {b:?}");
    assert_eq!((&a.1, &a.2), (&b.1, &b.2), "hai đường phải byte-khớp");
    assert!(a.2.contains("VS0503"), "stderr: {}", a.2);
    assert!(a.2.contains("ngoài phạm vi"), "stderr: {}", a.2);
    assert!(!a.1.contains("rhs"), "vế phải không được chạy: {}", a.1);
    assert!(!a.1.contains("khong"), "không được chạy tiếp: {}", a.1);
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------------------
// Từ chối tĩnh — gốc bất biến / tạm / `?.` / sai kiểu giữa đường
// ---------------------------------------------------------------------------

#[test]
fn goc_let_bi_tu_choi() {
    let d = tam("goc-let");
    let src =
        format!("{P}fn main():\n    let q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    print(q.p.x)\n");
    let e = tu_choi_tinh(&d, "goc-let", &src, "VS0412", &["`let`/tham số"]);
    assert!(!e.contains("lồng"), "không được báo 'lồng': {e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goc_la_bieu_thuc_tam_bi_tu_choi() {
    let d = tam("goc-tam");
    let src = format!(
        "{P}fn lay() -> Q:\n    {{p: {{x: 1}}}}\n\nfn main():\n    lay().p.x = 2\n    print(1)\n"
    );
    tu_choi_tinh(&d, "goc-tam", &src, "VS0412", &["tạm"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goc_literal_bi_tu_choi() {
    // `1 = 2` parse được nhưng không có ô để ghi — VS0412 tĩnh, không để codegen/interp lệch.
    let d = tam("goc-literal");
    tu_choi_tinh(
        &d,
        "goc-literal",
        "fn main():\n    1 = 2\n",
        "VS0412",
        &["tạm"],
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn optional_trong_duong_bi_tu_choi() {
    let d = tam("optional");
    let src =
        format!("{P}fn main():\n    var q: Q? = {{p: {{x: 1}}}}\n    q?.p.x = 2\n    print(1)\n");
    tu_choi_tinh(&d, "optional", &src, "VS0412", &["`?.`"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn sai_kieu_giua_duong_bi_tu_choi() {
    // `q.p.x` là `int` ⇒ `[0]` giữa đường là VS0412 ("không chỉ số được"), không để runtime lệch.
    let d = tam("sai-kieu");
    let src =
        format!("{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    q.p.x[0] = 9\n    print(1)\n");
    tu_choi_tinh(&d, "sai-kieu", &src, "VS0412", &["không chỉ số được"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn chi_so_sai_kieu_bi_tu_choi() {
    let d = tam("chi-so-sai-kieu");
    let src = "type P:\n    xs: list[int]\n\ntype Q:\n    p: P\n\nfn main():\n    var q: Q = {p: {xs: [1, 2]}}\n    q.p.xs[\"a\"] = 9\n    print(1)\n";
    tu_choi_tinh(&d, "chi-so-sai-kieu", src, "VS0412", &["chỉ số"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goc_self_khong_var_bi_tu_choi() {
    // `self` không `var` là tham số bất biến — đích lồng cũng phải chặn tại phép gán.
    let d = tam("self");
    let src = format!(
        "{P}type R:\n    q: Q\n\nimpl R:\n    fn set(self) -> int:\n        self.q.p.x = 2\n        return self.q.p.x\n\nfn main():\n    let r: R = {{q: {{p: {{x: 1}}}}}}\n    print(r.set())\n"
    );
    tu_choi_tinh(&d, "self", &src, "VS0413", &["`let`/tham số"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goc_self_var_gan_long_duoc() {
    // `var self` được rebind cục bộ; người gọi không thấy đổi (giá trị).
    let d = tam("var-self");
    let src = format!(
        "{P}type R:\n    q: Q\n\nimpl R:\n    fn bump(var self) -> int:\n        self.q.p.x = 9\n        return self.q.p.x\n\nfn main():\n    let r: R = {{q: {{p: {{x: 1}}}}}}\n    print(r.bump())\n    print(r.q.p.x)\n"
    );
    let (c, o, e) = hai_duong(&d, "var-self", &src);
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(o, "9\n1\n");
    let _ = std::fs::remove_dir_all(&d);
}
