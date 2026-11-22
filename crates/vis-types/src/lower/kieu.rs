// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — phần `impl Lowerer`: phân giải chữ ký/kiểu (arg, type body, tên kiểu).

use super::*;

impl<'a> Lowerer<'a> {
    /// Phân giải một nút `Type` (VD-44 #2: **không** báo `VS0301` cho kiểu).
    /// Kiểu của **một đối số generic**: giữ **ký hiệu** `T` khi `T` là tham số kiểu đang có hiệu lực
    /// (ST-35) — nhờ vậy `Hop[T]` trong chữ ký hàm hợp nhất được với `Hop[int]` ở call-site.
    pub(crate) fn resolve_arg_type(&mut self, node: &Node) -> Type {
        // Token đầu (dù nằm trong `elems` hay trong node con) — đủ để nhận ra `T`.
        let dau = node
            .tokens(&self.cst.tokens)
            .next()
            .filter(|t| t.kind == Kind::Ident)
            .map(|t| t.text.clone())
            .or_else(|| {
                let txt = self
                    .text
                    .get(node.span.start..node.span.end)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                txt.chars()
                    .next()
                    .filter(|c| c.is_alphabetic() || *c == '_')
                    .filter(|_| txt.chars().all(|c| c.is_alphanumeric() || c == '_'))
                    .map(|_| txt)
            });
        if let Some(t) = dau {
            if self.generic_scope.iter().any(|g| g == &t) {
                return Type::TypeRef {
                    name: t,
                    args: Vec::new(),
                };
            }
        }
        self.resolve_type(node)
    }

    pub(crate) fn resolve_type(&mut self, node: &Node) -> Type {
        // **R2 (VD-178)**: hậu tố optional lặp — đếm `?` (1 lớp) **và** cặp `??` (2 lớp) ở cuối.
        let q_dau: usize = {
            let toks: Vec<&Token> = node.tokens(&self.cst.tokens).collect();
            toks.iter()
                .rev()
                .take_while(|t| t.is_punct("?") || t.is_punct("??"))
                .map(|t| if t.is_punct("??") { 2 } else { 1 })
                .sum()
        };
        if q_dau > 0 {
            let inner = self.resolve_type_body(node);
            return self.wrap_opt(inner, q_dau);
        }
        self.resolve_type_body(node)
    }

    pub(crate) fn wrap_opt(&mut self, mut t: Type, n: usize) -> Type {
        for _ in 0..n {
            t = Type::Opt(Box::new(t));
        }
        t
    }

    pub(crate) fn resolve_type_body(&mut self, node: &Node) -> Type {
        let kids: Vec<&Node> = node.children().collect();
        let Some(first) = kids.first().copied() else {
            return Type::Error;
        };
        if first.kind == NodeKind::DottedPath {
            let name = self.dotted_text(first);
            let args: Vec<Type> = kids
                .get(1)
                .filter(|n| n.kind == NodeKind::Generics)
                .map(|g| {
                    g.children()
                        .filter(|n| n.kind == NodeKind::Type)
                        .map(|t| self.resolve_arg_type(t))
                        .collect()
                })
                .unwrap_or_default();
            return self.named_type(&name, args, first.span);
        }
        let head = node
            .tokens(&self.cst.tokens)
            .next()
            .map(|t| t.text.clone())
            .unwrap_or_default();
        match head.as_str() {
            "ref" | "weak" => {
                let inner = kids
                    .iter()
                    .find(|n| n.kind == NodeKind::Type)
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(Type::Error);
                Type::TypeRef {
                    name: head,
                    args: vec![inner],
                }
            }
            "cap" => {
                let inner = kids
                    .iter()
                    .find(|n| n.kind == NodeKind::DottedPath)
                    .map(|p| Type::TypeRef {
                        name: self.dotted_text(p),
                        args: Vec::new(),
                    })
                    .unwrap_or(Type::Error);
                Type::TypeRef {
                    name: "cap".into(),
                    args: vec![inner],
                }
            }
            "fn" => {
                let mut types: Vec<Type> = kids
                    .iter()
                    .filter(|n| n.kind == NodeKind::Type)
                    .map(|t| self.resolve_type(t))
                    .collect();
                let ret = types.pop().unwrap_or(Type::Unit);
                Type::Fn(types.into_boxed_slice(), Box::new(ret))
            }
            "(" => {
                let mut types: Vec<Type> = kids
                    .iter()
                    .filter(|n| n.kind == NodeKind::Type)
                    .map(|t| self.resolve_type(t))
                    .collect();
                match types.len() {
                    0 => Type::Unit,
                    1 => types.remove(0),
                    _ => Type::TypeRef {
                        name: "tuple".into(),
                        args: types,
                    },
                }
            }
            "{" => {
                let mut fields = Vec::new();
                for f in kids.iter().filter(|n| n.kind == NodeKind::TypeField) {
                    let hf = self.field_of(f);
                    fields.push(FieldDef {
                        name: hf.name,
                        ty: hf.ty,
                        span: hf.span,
                        has_default: hf.has_default,
                    });
                }
                let id = RecId(self.tbl.recs().len() as u32);
                self.tbl.push_rec(RecDef {
                    name: None,
                    fields,
                    span: node.span,
                    anonymous: true,
                });
                Type::Record(id)
            }
            _ => Type::Error,
        }
    }

    pub(crate) fn named_type(&mut self, name: &str, args: Vec<Type>, span: Span) -> Type {
        // **VD-241 (Gate 1)**: `SqlValue`/`RowSet` thuộc **module `std.sqlite`** — chỉ thấy khi
        // tệp đã `use std.sqlite`. Ngoài phạm vi đó ⇒ kiểu không tồn tại (`VS0301`), **không**
        // âm thầm thành tên lạ. Pattern trên giá trị `SqlValue` **không** gate (theo scrutinee).
        if (name == "SqlValue" || name == "RowSet") && !self.sqlite_duoc_thay() {
            self.diag(
                &NAME_CODES[0],
                format!("không thấy tên `{name}`: module `std.sqlite` chưa được `use`"),
                span,
            );
            return Type::Error;
        }
        // ST-28: tham số kiểu của `type`/`enum` đang khai ⇒ **xoá kiểu** (`{error}`) —
        // chưa có instantiation thật (nợ VS1b), nhờ vậy trường/biến thể generic không bị `VS0301`.
        if args.is_empty() && self.generic_scope.iter().any(|g| g == name) {
            // ST-35: khi dựng **template** (hoặc trong chú thích hàm) giữ **ký hiệu** `T`;
            // ngoài ra vẫn xoá kiểu như ST-28 (thân hàm dựa vào `{error}` để im lặng).
            return if self.sym_generic {
                Type::TypeRef {
                    name: name.to_string(),
                    args: Vec::new(),
                }
            } else {
                Type::Error
            };
        }
        match (name, args.len()) {
            ("Option" | "option", 1) => {
                // **VD-241 §3+ (A0, 2026-11-19)** — spelled-out `Option[T]` phân giải về
                // `Type::Opt` **ngay ở lowering** (như `list`/`map`/`Result`), không giữ
                // `TypeRef` — nhờ vậy HIR (annot tham số / return / let / var / hằng) đi cùng
                // đường với sugar `T?`, và biên annot/đối số/return nâng **như nhau** cho cả ba
                // cách viết (sugar · spelled-out · alias). Alias không tham số được thay ngay
                // bằng chính RHS đã hạ (`Option[Option[int]]` ⇒ `Opt(Opt(int))`).
                let mut it = args.into_iter();
                return Type::Opt(Box::new(it.next().unwrap_or(Type::Error)));
            }
            ("list", 1) => {
                let mut it = args.into_iter();
                return Type::List(Box::new(it.next().unwrap_or(Type::Error)));
            }
            ("map", 2) => {
                let mut it = args.into_iter();
                let k = it.next().unwrap_or(Type::Error);
                let v = it.next().unwrap_or(Type::Error);
                return Type::Map(Box::new(k), Box::new(v));
            }
            ("Result" | "result", 2) => {
                let mut it = args.into_iter();
                let a = it.next().unwrap_or(Type::Error);
                let b = it.next().unwrap_or(Type::Error);
                return Type::Result(Box::new(a), Box::new(b));
            }
            (_, 0) => {}
            _ => {
                // ST-28: `Or[str, int]` / `Hop[T]` — enum/record **generic** đã khai ⇒ bỏ đối số
                // kiểu (chưa có instantiation thật, nợ VS1b) và trả thẳng `Enum`/`Record`.
                // ST-35: còn tham số **ký hiệu** (vd `Hop[T]` trong chữ ký hàm) ⇒ giữ `TypeRef`
                // để hợp nhất ở call-site; ngược lại ⇒ **instantiation thật** (id riêng).
                let co_ky_hieu = args.iter().any(|a| {
                    matches!(a, Type::TypeRef { args, .. } if args.is_empty())
                        || matches!(a, Type::Error)
                });
                if co_ky_hieu {
                    return Type::TypeRef {
                        name: name.to_string(),
                        args,
                    };
                }
                let enum_base = self
                    .enum_names
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, id)| *id);
                if let Some(id) = enum_base {
                    return self
                        .inst_enum(name, id, args)
                        .map(Type::Enum)
                        .unwrap_or(Type::Enum(id));
                }
                let rec_base = self
                    .rec_names
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, id)| *id);
                if let Some(id) = rec_base {
                    return self
                        .inst_rec(name, id, args)
                        .map(Type::Record)
                        .unwrap_or(Type::Record(id));
                }
                return Type::TypeRef {
                    name: name.to_string(),
                    args,
                };
            }
        }
        if let Some(t) = Type::builtin(name) {
            return t;
        }
        if let Some((_, id)) = self.rec_names.iter().find(|(n, _)| n == name) {
            return Type::Record(*id);
        }
        if let Some((_, id)) = self.enum_names.iter().find(|(n, _)| n == name) {
            return Type::Enum(*id);
        }
        if let Some((_, t)) = self.aliases.iter().find(|(n, _)| n == name) {
            return t.clone();
        }
        Type::TypeRef {
            name: name.to_string(),
            args: Vec::new(),
        }
    }

    pub(crate) fn dotted_text(&self, node: &Node) -> String {
        node.tokens(&self.cst.tokens)
            .filter(|t| t.kind == Kind::Ident)
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}
