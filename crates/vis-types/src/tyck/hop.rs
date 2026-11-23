// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — hợp nhất kiểu, so khớp, tự nâng (`coerce`) và phân giải tên kiểu.
//!
//! Tách **nguyên văn** khỏi `tyck/mod.rs` (impl `Checker`) để giữ mỗi tệp ≤ 1.150 dòng.
//! Không đổi ngữ nghĩa: mọi hàm vẫn là phương thức của [`Checker`], chỉ đổi tệp chứa.

use super::*;

impl<'a> Checker<'a> {
    // ----- kiểu: so khớp, tự nâng, tên kiểu -----

    /// ST-28 — **hợp nhất** kiểu khai báo generic với kiểu đối số thật; trả `false` (và báo
    /// `VS0412`) khi mâu thuẫn. `bang` giữ `T → int` đã suy.
    pub(crate) fn hop_nhat(
        &mut self,
        mau: &Type,
        that: &Type,
        gens: &[String],
        bang: &mut Vec<(String, Type)>,
        ten: &str,
        span: Span,
    ) -> bool {
        if matches!(that, Type::Error) {
            return true;
        }
        match mau {
            Type::TypeRef { name, args } if args.is_empty() && gens.iter().any(|g| g == name) => {
                match bang.iter().find(|(n, _)| n == name) {
                    Some((_, da)) => {
                        if self.same(da, that) {
                            true
                        } else {
                            let msg = format!(
                                "`{ten}`: tham số kiểu `{name}` đã suy là `{}`, đối số này có `{}`",
                                self.types.show(da),
                                self.types.show(that)
                            );
                            self.diag(MISMATCH, msg, span);
                            false
                        }
                    }
                    None => {
                        bang.push((name.clone(), that.clone()));
                        true
                    }
                }
            }
            // ST-35: `Hop[T]` — thay ràng buộc đã biết; khớp theo **instantiation** (id riêng) nên
            // `Hop[int]` ≠ `Hop[str]`; chưa suy được `T` ⇒ **bỏ qua** (giữ hành vi xoá kiểu ST-28).
            Type::TypeRef { name, args } if !args.is_empty() => {
                // **VD-270**: kiểu nhận của phương thức `impl Hop[T]` là `TypeRef{"Hop",[T]}`;
                // đối số thật thường là **instantiation** `Record(id)` (không phải `TypeRef`) ⇒
                // buộc `T` theo **vị trí** từ `rec_inst_of`/`enum_inst_of`. Nhờ vậy `a.get()`
                // trả `int` (không còn ký hiệu `T`) và đối số sau bị kiểm theo `T` đã suy.
                let inst = match that {
                    Type::Record(id) => self.types.rec_inst_of(*id).cloned(),
                    Type::Enum(id) => self.types.enum_inst_of(*id).cloned(),
                    _ => None,
                };
                if let Some((ten, ds)) = inst {
                    if ten == *name && ds.len() == args.len() {
                        let mut ok = true;
                        for (mau, that_arg) in args.iter().zip(ds.iter()) {
                            if !self.hop_nhat(mau, that_arg, gens, bang, &ten, span) {
                                ok = false;
                            }
                        }
                        return ok;
                    }
                }
                let cu: Vec<Type> = args.iter().map(|a| thay_gen(a, bang)).collect();
                if cu
                    .iter()
                    .any(|t| matches!(t, Type::TypeRef { args, .. } if args.is_empty()))
                {
                    return true;
                }
                if let Some(id) = self.types.find_rec_inst(name, &cu) {
                    return self.hop_nhat(&Type::Record(id), that, gens, bang, ten, span);
                }
                if let Some(id) = self.types.find_enum_inst(name, &cu) {
                    return self.hop_nhat(&Type::Enum(id), that, gens, bang, ten, span);
                }
                // Chưa có instantiation trong bảng (vd chưa ai viết `let x: Hop[int]`): khớp
                // **cấu trúc** theo vị trí tham số của trường ⇒ vẫn buộc được `T` và báo sai kiểu.
                if let (Type::Record(that_id), Some(base)) = (that, self.types.find_rec_base(name))
                {
                    let (Some(def), Some(that_def)) = (
                        self.types.rec(base).cloned(),
                        self.types.rec(*that_id).cloned(),
                    ) else {
                        return self.hop_nhat_loi(mau, that, ten, span);
                    };
                    let pos = self.types.rec_pos(base).to_vec();
                    if def.fields.len() == that_def.fields.len()
                        && def
                            .fields
                            .iter()
                            .zip(that_def.fields.iter())
                            .all(|(f, g)| f.name == g.name)
                    {
                        let mut ok = true;
                        for (i, g) in that_def.fields.iter().enumerate() {
                            match pos.get(i).copied().flatten() {
                                Some(p) => {
                                    let want = cu.get(p).cloned().unwrap_or(Type::Error);
                                    if !self.hop_nhat(&want, &g.ty, gens, bang, ten, span) {
                                        ok = false;
                                    }
                                }
                                None => {
                                    if !self.same(&def.fields[i].ty, &g.ty) {
                                        ok = false;
                                    }
                                }
                            }
                        }
                        return ok;
                    }
                }
                if self.same(mau, that) {
                    true
                } else {
                    self.hop_nhat_loi(mau, that, ten, span)
                }
            }
            Type::List(a) => match that {
                Type::List(b) => self.hop_nhat(a, b, gens, bang, ten, span),
                _ => self.hop_nhat_loi(mau, that, ten, span),
            },
            Type::Opt(a) => match that {
                Type::Opt(b) => self.hop_nhat(a, b, gens, bang, ten, span),
                _ => true, // tự nâng `T → T?` (§4.4)
            },
            Type::Map(k1, v1) => match that {
                Type::Map(k2, v2) => {
                    self.hop_nhat(k1, k2, gens, bang, ten, span)
                        && self.hop_nhat(v1, v2, gens, bang, ten, span)
                }
                _ => self.hop_nhat_loi(mau, that, ten, span),
            },
            Type::Result(a1, b1) => match that {
                Type::Result(a2, b2) => {
                    self.hop_nhat(a1, a2, gens, bang, ten, span)
                        && self.hop_nhat(b1, b2, gens, bang, ten, span)
                }
                _ => self.hop_nhat_loi(mau, that, ten, span),
            },
            _ => {
                if self.same(mau, that) {
                    true
                } else {
                    self.hop_nhat_loi(mau, that, ten, span)
                }
            }
        }
    }

    fn hop_nhat_loi(&mut self, mau: &Type, that: &Type, ten: &str, span: Span) -> bool {
        let msg = format!(
            "`{ten}`: cần `{}`, được `{}`",
            self.types.show(mau),
            self.types.show(that)
        );
        self.diag(MISMATCH, msg, span);
        false
    }

    /// So khớp **cấu trúc**; `{error}` tương thích mọi kiểu (lan truyền im lặng).
    pub(crate) fn same(&self, a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Error, _) | (_, Type::Error) => true,
            (Type::Record(x), Type::Record(y)) => self.same_record(*x, *y),
            (Type::Enum(x), Type::Enum(y)) => self.same_enum(*x, *y),
            (Type::List(x), Type::List(y)) => self.same(x, y),
            (Type::Map(k1, v1), Type::Map(k2, v2)) => self.same(k1, k2) && self.same(v1, v2),
            (Type::Opt(x), Type::Opt(y)) => self.same(x, y),
            (Type::Result(a1, b1), Type::Result(a2, b2)) => self.same(a1, a2) && self.same(b1, b2),
            (Type::Fn(p1, r1), Type::Fn(p2, r2)) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(x, y)| self.same(x, y))
                    && self.same(r1, r2)
            }
            (Type::TypeRef { name: n1, args: a1 }, Type::TypeRef { name: n2, args: a2 }) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| self.same(x, y))
            }
            _ => a == b,
        }
    }

    /// ST-35: bản gốc ↔ bản instantiation **cùng tên enum** ⇒ coi là cùng họ (kiểm đối số ở
    /// `coerce_e` khi là literal); hai instantiation **khác đối số** vẫn **khác** (`Or[int,str] ≠ Or[str,int]`).
    fn same_enum(&self, x: EnumId, y: EnumId) -> bool {
        if x == y {
            return true;
        }
        let nx = self.types.enum_def(x).map(|d| d.name.clone());
        let ny = self.types.enum_def(y).map(|d| d.name.clone());
        match (self.types.enum_inst_of(x), self.types.enum_inst_of(y)) {
            (Some((b, _)), None) => ny.as_deref() == Some(b.as_str()),
            (None, Some((b, _))) => nx.as_deref() == Some(b.as_str()),
            _ => false,
        }
    }

    fn same_record(&self, x: RecId, y: RecId) -> bool {
        if x == y {
            return true;
        }
        let (Some(rx), Some(ry)) = (self.types.rec(x), self.types.rec(y)) else {
            return false;
        };
        // **VD-241 §6/B-B (A0, 2026-11-19)** — bản ghi so khớp theo **tên trường**; thứ tự khai báo
        // không quyết định (`{b: 1, a: "s"}` ≡ `{a: "s", b: 1}`).
        rx.fields.len() == ry.fields.len()
            && rx.fields.iter().all(|af| {
                ry.fields
                    .iter()
                    .find(|bf| bf.name == af.name)
                    .map(|bf| self.same(&af.ty, &bf.ty))
                    .unwrap_or(false)
            })
    }

    /// Ép `from` về `to` **có báo lỗi**: tự nâng `T → T?`, literal `int` ở vị trí `float` (§3.5).
    /// ST-35: `Hop[{error}]` (đã xoá generic) ⇒ bản ghi **gốc** `Hop`; `Or[..]` ⇒ enum gốc.
    pub(crate) fn chuan(&self, t: &Type) -> Type {
        if let Type::TypeRef { name, .. } = t {
            if let Some(id) = self.types.find_rec_base(name) {
                return Type::Record(id);
            }
            if let Some(id) = self.types.find_enum_base(name) {
                return Type::Enum(id);
            }
        }
        t.clone()
    }

    pub(crate) fn coerce(&mut self, from: &Type, to: &Type, span: Span, what: &str) -> bool {
        // ST-13b (mục 1): khối phân kỳ (`never`) nhận mọi kiểu — không có giá trị để sai kiểu.
        if matches!(from, Type::Never) || matches!(to, Type::Never) {
            return true;
        }
        let (from, to) = (&self.chuan(from), &self.chuan(to));
        if self.same(from, to) {
            return true;
        }
        // **VD-241 §3 (A0)**: nâng **một lớp** duy nhất ở chuyển đổi annot/đối số/return;
        // `T??` = `Option[Option[T]]`; chấp nhận khi lõi khớp, đích ≥ 1 lớp `Opt`,
        // `from_layers == to_layers - 1`.
        let mut from_peel = from;
        let mut from_layers = 0usize;
        while let Type::Opt(inner) = from_peel {
            from_layers += 1;
            from_peel = inner;
        }
        let mut to_peel = to;
        let mut to_layers = 0usize;
        while let Type::Opt(inner) = to_peel {
            to_layers += 1;
            to_peel = inner;
        }
        if to_layers >= 1 && from_layers == to_layers - 1 && self.same(from_peel, to_peel) {
            return true;
        }
        // ST-12b (BLOCKER #2): **không** nâng `int → float` đại trà — chỉ literal số nguyên
        // (§3.5). Việc nâng literal do `coerce_e` quyết định theo biểu thức nguồn.
        let msg = format!(
            "{what}: cần `{}`, được `{}`",
            self.types.show(to),
            self.types.show(from)
        );
        self.diag(MISMATCH, msg, span);
        false
    }

    /// **R17 (VD-178)**: điều kiện `if`/`while` chỉ `bool`; list/map/str ⇒ gợi ý `.is_empty()`.
    pub(crate) fn coerce_cond(&mut self, e: &HExpr, t: &Type, what: &str) -> bool {
        if matches!(t, Type::List(_) | Type::Map(..) | Type::Str) {
            let ten = match e {
                HExpr::Var { name, .. } => name.clone(),
                _ => "xs".to_string(),
            };
            let msg = format!(
                "{what}: cần `bool`, được `{}` — gợi ý: `{ten}.is_empty()`",
                self.show(t)
            );
            self.diag(MISMATCH, msg, e.span());
            return false;
        }
        self.coerce(t, &Type::Bool, e.span(), what)
    }

    /// Ép kiểu **có biết biểu thức nguồn**: chỉ nâng `int → float` khi nguồn là **literal** số
    /// nguyên (§3.5, ngoại lệ duy nhất của "không chuyển đổi ngầm").
    pub(crate) fn coerce_e(&mut self, e: &HExpr, from: &Type, to: &Type, what: &str) -> bool {
        // **VD-238** (giữ) + **VD-241 §7**: `None` hợp lệ khi kiểu đích **ngoài cùng** là `Option`.
        if vd238::la_none_bare(e) && !vd238::la_none_dich_opt(to) {
            self.diag(
                MISMATCH,
                format!(
                    "`{what}`: `None` chỉ gán được cho `T?`, được `{}` (VD-238)",
                    self.types.show(to)
                ),
                e.span(),
            );
            return false;
        }
        if matches!((from, to), (Type::Int, Type::Float)) && vd238::is_int_lit(e) {
            return true;
        }
        // ST-35: `Or.Trai(1)` gán vào `Or[int, str]` — kiểm **đối số biến thể** theo bản
        // instantiation (bản gốc đã xoá kiểu nên không tự bắt được).
        if let (HExpr::EnumLit { variant, args, .. }, Type::Enum(inst)) = (e, to) {
            if let Some(def) = self.types.enum_def(*inst).cloned() {
                if let Some(v) = def.variants.iter().find(|v| &v.name == variant) {
                    let fields = v.fields.clone();
                    for (i, a) in args.iter().enumerate() {
                        if let Some(f) = fields.get(i) {
                            let t = self.kiem_expr(a);
                            let want = f.ty.clone();
                            if !self.coerce_e(a, &t, &want, "đối số biến thể") {
                                return false;
                            }
                        }
                    }
                    return true;
                }
            }
        }
        let span = e.span();
        self.coerce(from, to, span, what)
    }

    /// Ép kiểu có **cờ literal** (tầng toán tử chỉ còn kiểu + cờ).
    pub(crate) fn coerce_lit(
        &mut self,
        from: &Type,
        to: &Type,
        span: Span,
        what: &str,
        from_lit: bool,
    ) -> bool {
        if from_lit && matches!((from, to), (Type::Int, Type::Float)) {
            return true;
        }
        self.coerce(from, to, span, what)
    }

    /// Phân giải một kiểu **chú thích**: `TypeRef` chưa biết ⇒ `VS0301` (VD-43 #3).
    pub(crate) fn resolve_ann(&mut self, ty: &Type, span: Span) -> Type {
        match ty {
            Type::Int
            | Type::Float
            | Type::Size
            | Type::Duration
            | Type::Percent
            | Type::Str
            | Type::Bool
            | Type::Bytes
            | Type::Path
            | Type::Char
            | Type::Unit
            | Type::Never
            | Type::Error
            | Type::Record(_)
            | Type::Enum(_) => ty.clone(),
            Type::Opt(inner) => {
                let t = self.resolve_ann(inner, span);
                // **R2 (VD-178)**: `T??` hợp lệ — optional lồng tuỳ ý (`Opt[Opt[T]]`).
                Type::Opt(Box::new(t))
            }
            Type::List(t) => Type::List(Box::new(self.resolve_ann(t, span))),
            Type::Map(k, v) => Type::Map(
                Box::new(self.resolve_ann(k, span)),
                Box::new(self.resolve_ann(v, span)),
            ),
            Type::Result(a, b) => Type::Result(
                Box::new(self.resolve_ann(a, span)),
                Box::new(self.resolve_ann(b, span)),
            ),
            Type::Fn(args, ret) => Type::Fn(
                args.iter()
                    .map(|a| self.resolve_ann(a, span))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                Box::new(self.resolve_ann(ret, span)),
            ),
            Type::TypeRef { name, args } => {
                // ST-28: `T` là **tham số kiểu** đang có hiệu lực ⇒ giữ nguyên làm **dấu** để
                // hợp nhất ở call-site; nơi dùng cho thân hàm sẽ được `erase_gen` xoá thành `{error}`.
                if args.is_empty() && self.generic_scope.iter().any(|g| g == name) {
                    return Type::TypeRef {
                        name: name.clone(),
                        args: Vec::new(),
                    };
                }
                self.resolve_named(name, args, span)
            }
        }
    }

    fn resolve_named(&mut self, name: &str, args: &[Type], span: Span) -> Type {
        // **ST-72 (VD-106 (1))**: `cap[Dir|File|Net|Proc|Env|Term]` — kiểu quyền **dựng sẵn**.
        // Đối số **không** resolve như kiểu thường: `Dir/File/…` **không** có nghĩa ngoài `cap`
        // (tránh biến chúng thành kiểu toàn cục).
        if name == "cap" || name == "Cap" {
            return self.resolve_cap(args, span);
        }
        let resolved: Vec<Type> = args.iter().map(|a| self.resolve_ann(a, span)).collect();
        // **VD-222** — Json=EnumId(0)·Error=RecId(0)·AnyError=RecId(0) (VD-231).
        if args.is_empty() {
            match name {
                "Json" => return Type::Enum(crate::ty::EnumId(0)),
                "Error" => return Type::Record(crate::ty::RecId(0)),
                "AnyError" => return Type::Record(crate::ty::RecId(0)), // VD-231 alias Error
                _ => {}
            }
        }
        match (name, resolved.len()) {
            ("list", 1) => return Type::List(Box::new(resolved[0].clone())),
            ("map", 2) => {
                return Type::Map(Box::new(resolved[0].clone()), Box::new(resolved[1].clone()))
            }
            // **ST-61 (VD-96)**: `Chan[T]` — kênh đệm hữu hạn (kiểu dựng sẵn, như `Task`).
            ("Chan" | "chan", 1) => {
                return Type::TypeRef {
                    name: "Chan".into(),
                    args: vec![resolved[0].clone()],
                }
            }
            // **ST-60 (VD-95 (3))**: `Task[T]` — kiểu dựng sẵn của tác vụ `async`.
            ("Task" | "task", 1) => {
                return Type::TypeRef {
                    name: "Task".into(),
                    args: vec![resolved[0].clone()],
                }
            }
            ("Result" | "result", 2) => {
                return Type::Result(Box::new(resolved[0].clone()), Box::new(resolved[1].clone()))
            }
            ("Option" | "option", 1) => return Type::Opt(Box::new(resolved[0].clone())),
            // ST-11 mã hoá kiểu tuple là `TypeRef{name:"tuple"}` — giữ nguyên (kiểu cấu trúc).
            ("tuple", _) => {
                return Type::TypeRef {
                    name: "tuple".into(),
                    args: resolved,
                }
            }
            (_, 0) => {
                if let Some(t) = Type::builtin(name) {
                    return t;
                }
            }
            _ => {}
        }
        // ST-35: tham số kiểu **đang có hiệu lực** (`T` trong `fn f[T]`) ⇒ giữ ký hiệu.
        if resolved.is_empty() && self.generic_scope.iter().any(|g| g == name) {
            return Type::TypeRef {
                name: name.to_string(),
                args: Vec::new(),
            };
        }
        // ST-35: kiểu **người dùng generic** — instantiation đã dựng trong bảng, hoặc bản gốc.
        if let Some(id) = self.types.find_rec_inst(name, &resolved) {
            return Type::Record(id);
        }
        if let Some(id) = self.types.find_enum_inst(name, &resolved) {
            return Type::Enum(id);
        }
        if resolved.is_empty() {
            if let Some(id) = self.types.find_rec_base(name) {
                return Type::Record(id);
            }
            if let Some(id) = self.types.find_enum_base(name) {
                return Type::Enum(id);
            }
        } else if resolved
            .iter()
            .any(|t| matches!(t, Type::TypeRef { args, .. } if args.is_empty()))
        {
            return Type::TypeRef {
                name: name.to_string(),
                args: resolved,
            };
        }
        // VD-48(a): kiểu **ngoài VS0** (`ref`, `set`, `stream`, …) vẫn là `VS0301` (thông điệp nói rõ "chưa hỗ trợ (VD-43)").
        let msg = if vd238::la_kieu_ngoai_vs0(name) {
            format!(
                "kiểu `{name}` chưa hỗ trợ — thuộc vis-std/vòng 2; dùng kiểu khác hoặc bỏ chú thích"
            )
        } else {
            format!(
                "tên không tồn tại: kiểu `{name}` chưa hỗ trợ (VD-43) và không được khai trong tệp"
            )
        };
        self.diag(NO_TYPE_NAME, msg, span);
        Type::Error
    }

    /// `cap[X]`: đúng 1 đối số, `X` ∈ {Dir, File, Net, Proc, Env, Term}; biểu diễn
    /// `TypeRef{"cap", [TypeRef{X}]}` (rẻ: dùng lại `TypeRef`, không thêm biến thể `Type`).
    fn resolve_cap(&mut self, args: &[Type], span: Span) -> Type {
        const KINDS: [&str; 6] = ["Dir", "File", "Net", "Proc", "Env", "Term"];
        if args.len() != 1 {
            self.diag(
                NO_TYPE,
                format!(
                    "`cap` cần đúng 1 tham số (Dir/File/Net/Proc/Env/Term), được {}",
                    args.len()
                ),
                span,
            );
            return Type::Error;
        }
        let kind = match &args[0] {
            Type::TypeRef { name, args } if args.is_empty() => name.clone(),
            _ => {
                self.diag(
                    NO_TYPE,
                    "tham số `cap[...]` phải là một trong Dir/File/Net/Proc/Env/Term".to_string(),
                    span,
                );
                return Type::Error;
            }
        };
        if !KINDS.contains(&kind.as_str()) {
            self.diag(
                NO_TYPE,
                format!(
                    "kiểu quyền `{kind}` không hợp lệ — chỉ Dir/File/Net/Proc/Env/Term (VD-106)"
                ),
                span,
            );
            return Type::Error;
        }
        Type::TypeRef {
            name: "cap".into(),
            args: vec![Type::TypeRef {
                name: kind,
                args: Vec::new(),
            }],
        }
    }

    // ----- duyệt chương trình -----

    /// **VD-265** — chữ ký phương thức `impl` vào `sigs`/`env` (cùng quy tắc hàm cấp tệp:
    /// giữ `TypeRef{"T"}` khi hợp nhất call-site, thân dùng bản đã xoá generic).
    /// Không đẩy vào `item_types` (tên phương thức không phải item cấp tệp).
    pub(crate) fn dang_ky_sig_phuong_thuc(&mut self, f: &HFn, recv: Option<&Type>) {
        let luu = std::mem::take(&mut self.generic_scope);
        self.generic_scope = f.generics.clone();
        let raw_params: Vec<Type> = f
            .params
            .iter()
            .map(|p| {
                // **VD-270**: `self` của `impl Hop[T]` mang kiểu nhận **còn ký hiệu**
                // (`TypeRef{"Hop", [T]}`) để `hop_nhat` buộc được `T` từ đối số thật
                // (`Hop[int]`) rồi thay vào kiểu trả. HParam.ty vẫn `{error}` (VD-262) nên
                // chỉ chèn ở đây — thân phương thức không đổi chẩn đoán.
                if p.is_self && !f.generics.is_empty() {
                    if let Some(t) = recv {
                        return t.clone();
                    }
                }
                self.resolve_ann(&p.ty, p.span)
            })
            .collect();
        let raw_ret = self.resolve_ann(&f.ret, f.span);
        self.generic_scope = luu;
        let params: Vec<Type> = raw_params
            .iter()
            .map(|t| self.chuan(&erase_gen(t, &f.generics)))
            .collect();
        let ret = self.chuan(&erase_gen(&raw_ret, &f.generics));
        let ty = Type::Fn(params.clone().into_boxed_slice(), Box::new(ret.clone()));
        self.set_ty(f.sym, ty);
        if let Some(slot) = self.sigs.get_mut(f.sym as usize) {
            *slot = Some(FnSig {
                name: f.name.clone(),
                is_async: f.is_async,
                params: f
                    .params
                    .iter()
                    .zip(params.iter())
                    .map(|(p, t)| (p.name.clone(), t.clone(), p.default.is_some(), p.named))
                    .collect(),
                ret,
                variadic: f.params.last().map(|p| p.variadic).unwrap_or(false),
                generics: f.generics.clone(),
                raw_params,
                raw_ret,
            });
        }
    }
}
