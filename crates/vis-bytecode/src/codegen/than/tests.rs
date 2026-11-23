// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — test đơn vị của codegen thân hàm (`codegen/than.rs`), tách ra cùng thư mục.

use super::*;

fn ha(src: &str) -> vis_types::HirOutput {
    let src = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}");
    let lex = vis_syntax::lex(&src, vis_syntax::Mode::Script);
    let out = vis_syntax::parse(&src, vis_syntax::Mode::Script);
    vis_types::lower(&out.cst, &lex)
}

/// Đổi `Call(Var("R.io"), [r])` (dispatch tĩnh) thành `Call(Field(r, "io"), [])` — dạng HIR mà
/// **VD-265** phải suy được khi không có dispatch (callee `Field`).
fn doi_thanh_field(e: &mut HExpr) -> bool {
    if let HExpr::Call {
        callee,
        args,
        named,
        span,
    } = e
    {
        if let HExpr::Var { name, .. } = &**callee {
            if name == "R.io" && !args.is_empty() {
                let base = args.remove(0);
                **callee = HExpr::Field {
                    base: Box::new(base),
                    name: "io".into(),
                    optional: false,
                    span: *span,
                };
                named.clear();
                return true;
            }
        }
        for a in args.iter_mut() {
            if doi_thanh_field(a) {
                return true;
            }
        }
        return doi_thanh_field(callee);
    }
    false
}

fn doi(sts: &mut [HStmt]) -> bool {
    for s in sts {
        if let HStmt::Let { init, .. } | HStmt::Var { init, .. } = s {
            if doi_thanh_field(init) {
                return true;
            }
        }
    }
    false
}

/// **VD-265 (đỏ-trước ở tầng này)** — trước bản sửa, `infer_ty(Call(Field(Record, m)))` trả
/// `None` (chỉ có `list`/`map` dựng sẵn); nay tra [`MethodRet`] và biết `int?` như interp.
#[test]
fn infer_ty_phuong_thuc_nguoi_dung_callee_field() {
    let mut h = ha("type R:\n    f: int?\n\nimpl R:\n    fn io(self) -> int?:\n        None\n\nfn main():\n    let r: R = R(f: None)\n    let x = r.io()\n");
    let mut found = false;
    for it in h.hir.items.iter_mut() {
        if let HItem::Fn(f) = it {
            if f.name == "main" {
                found = doi(&mut f.body);
            }
        }
    }
    assert!(found, "phải đổi được lời gọi `r.io()` thành callee Field");
    let mret = MethodRet::from_hir_voi_bang(&h.hir, &h.types);
    let fn_rets = HashMap::new(); // cố ý rỗng: chỉ bảng phương thức quyết định
    let sym = collect_sym_tys(&h.hir, &h.symbols, Some(&h.types));
    let rec_fields = collect_rec_fields(&h.hir, Some(&h.types));
    let got = h
        .hir
        .items
        .iter()
        .find_map(|it| match it {
            HItem::Fn(f) if f.name == "main" => f.body.iter().find_map(|s| match s {
                HStmt::Let { init, .. } if matches!(init, HExpr::Call { .. }) => {
                    infer_ty(init, &sym, &fn_rets, &mret, &rec_fields)
                }
                _ => None,
            }),
            _ => None,
        })
        .expect("phải suy được kiểu lời gọi phương thức");
    assert_eq!(got, Type::Opt(Box::new(Type::Int)));
}
