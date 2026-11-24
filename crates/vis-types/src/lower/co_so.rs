// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — phần đầu `impl Lowerer`: khởi tạo, chạy, khung tên, khai báo builtin/kiểu.

use super::*;

impl<'a> Lowerer<'a> {
    pub(crate) fn new(
        cst: &'a Cst,
        lex: &'a LexOutput,
        opts: LowerOptions,
        symbols: &'a mut Symbols,
        tbl: &'a mut TypeTable,
    ) -> Self {
        // ST-25: kiểu **công khai** của các module đã `use` được nhập vào bảng tên của tệp này.
        let mut rec_names: Vec<(String, RecId)> = Vec::new();
        let mut enum_names: Vec<(String, EnumId)> = Vec::new();
        let mut aliases: Vec<(String, Type)> = Vec::new();
        for (n, t) in &opts.imported_types {
            match t {
                Type::Record(id) => rec_names.push((n.clone(), *id)),
                Type::Enum(id) => enum_names.push((n.clone(), *id)),
                other => aliases.push((n.clone(), other.clone())),
            }
        }
        Self {
            file: opts.file,
            generic_scope: Vec::new(),
            impls: Vec::new(),
            traits: Vec::new(),
            traits_ngoai: opts.trait_ngoai.clone(),
            impl_pairs: Vec::new(),
            kieu_ngoai: opts.imported_types.iter().map(|(n, _)| n.clone()).collect(),
            derived: Vec::new(),
            rec_tmpl: Vec::new(),
            enum_tmpl: Vec::new(),
            inst_cache: Vec::new(),
            sym_generic: false,
            cmd_sigs: vis_hir::cmdsig::BuiltinSigs::mac_dinh().sigs().to_vec(),
            tam_n: 0,
            file_long: None,
            prelude_recs: Vec::new(),
            last_stage_ty: Type::Error,
            type_env: std::collections::HashMap::new(),
            cur_impl: None,
            cur_impl_ty: None,
            cur_impl_generics: Vec::new(),
            self_ty: std::collections::HashMap::new(),
            cst,
            text: &lex.text,
            opts,
            diags: Vec::new(),
            error: None,
            symbols,
            tbl,
            scope: Vec::new(),
            items: Vec::new(),
            stmts: Vec::new(),
            rec_names,
            enum_names,
            variant_index: Vec::new(),
            aliases,
            newtypes: Vec::new(),
            item_syms: Vec::new(),
            suppress_dup: false,
            suppress_name_diag: false,
            expect: None,
            cur_ret: None,
            fn_params: std::collections::HashMap::new(),
            fn_ret: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn run(&mut self) {
        // ST-25: đơn vị biên dịch **dùng chung** khung gốc `0` và bảng ký hiệu — tệp thứ hai trở đi
        // **không** tạo khung mới, để mọi item cấp tệp nằm cùng một phạm vi (tầm nhìn lọc bằng `use`).
        if self.symbols.frames.is_empty() {
            self.symbols.frames.push(HFrame {
                parent: None,
                span: self.cst.root.span,
                kind: FrameKind::File,
            });
        }
        self.scope.push(0);
        if self.command_mode() {
            // ST-18 (VD-22 pha 2): `.vish`/`cmd { … }` hạ xuống **cùng HIR** — `cmd_lower`.
            // ST-40a: prelude bản ghi **chỉ** cho chế độ lệnh, đăng ký **trước** khi hạ item để
            // `type FileEntry` của người dùng trùng tên bị `VS0302`.
            let prelude = self.dang_ky_prelude();
            // **ST-85b (VD-109)**: dòng cấp tệp thụt lề **không** thuộc khối nào ⇒ `VS0201` rõ
            // (trước đây bị `khai::tach` nuốt im lặng). `.vis` (script) không đi nhánh này.
            for (line, col) in khai::dong_thut_sai(self.text) {
                let sp = Lowerer::span_dong_cot(self.text, line, col, self.file);
                self.diag(
                    &vis_diag::SYNTAX_CODES[0],
                    "câu lệnh cấp tệp không được thụt lề (`.vish` không có block cấp tệp)",
                    sp,
                );
            }
            // ST-44: **vùng khai báo** ở đầu tệp (type/enum/cmd/let…) hạ trước, vào **cùng**
            // bảng ký hiệu/kiểu; stage chạy sau với mốc tách để **bỏ** các dòng đã hạ.
            let (moc, co_khai) = khai::tach(self.text);
            // ST-48: thân `cmd` **lệnh** thay bằng `()` trong bản sao tiền tố (giữ nguyên byte).
            // **ST-134**: phân loại thân cần biết **tên lệnh** (bảng chuẩn + `cmd` khai trong tệp)
            // để một dòng `ls /tmp/x` là **lệnh**, không bị đọc thành phép chia `tmp` (lỗi ca iv).
            let mut ten_lenh = vis_hir::cmdsig::ten_mac_dinh();
            ten_lenh.extend(khai::ten_cmd_khai(self.text));
            let ds_than = khai::than_cmd(self.text, &ten_lenh);
            let tien_to = khai::filler(self.text, &ds_than);
            if co_khai && moc > 0 {
                self.ha_tien_to_khai(moc, &prelude, &tien_to);
                self.ha_than_cmd(&ds_than);
            } else if let Some((line, col)) = khai::khai_sau_stage(self.text, 0) {
                // Tệp **không** mở đầu bằng khai báo nhưng có khai báo ở cột 1 ⇒ lỗi rõ.
                let sp = Lowerer::span_dong_cot(self.text, line, col, self.file);
                self.diag(
                    &vis_diag::SYNTAX_CODES[0],
                    "khai báo phải đứng **trước** stage đầu tiên (`.vish`: item ở đầu tệp, rồi mới\
                     tới pipeline)",
                    sp,
                );
            }
            self.run_shell_tu(moc);
            return;
        }
        self.declare_builtins();
        self.declare_kieu_dung_san();
        // Lượt 1 — "khai báo trước" (§14.3): tên item + id kiểu, để thân hàm gọi được hàm khai
        // sau nó và `type` tham chiếu được kiểu khai sau.
        for child in self.cst.root.children() {
            if is_item(child.kind) {
                self.predeclare_item(child);
            }
        }
        // ST-29: **lượt 1b** — thu thập `trait`/`impl` + sym phương thức (sau khi mọi `type`/`enum`
        // đã khai ⇒ `impl Show for Hop` phân giải được kể cả khi `type Hop` viết sau).
        for child in self.cst.root.children() {
            match child.kind {
                NodeKind::TraitDecl => self.collect_trait(child),
                NodeKind::ImplDecl => self.collect_impl(child),
                _ => {}
            }
        }
        // **ST-57 (VD-93/VD-98 (3))**: lượt **1c** — `derives Show/Eq` sinh `impl` **thật** **trước**
        // mọi lượt hạ thân (câu `p.show()` trước khai báo vẫn phân giải).
        // **ST-60c (B3)**: lượt 1c chạy **trước** khi kiểm siêu trait ⇒ `impl` **sinh từ `derives`**
        // đã nằm trong `impl_pairs` (trước: `impl TraitCon for T` bị `VS0403` oan).
        self.sinh_derives();
        // **ST-57 (VD-98 (2))**: siêu trait kiểm **một tầng**, **sau** 1b+1c ⇒ thứ tự `impl` tự do.
        self.kiem_sieu_trait();
        // **VD-265**: pre-pass template generic + chữ ký **mọi** hàm/phương thức — call-site
        // trước khai báo vẫn biết `Box[int?]`/kiểu trả về (không phát chẩn đoán).
        self.chuan_bi_chu_ky();
        // Lượt 2 — thân item + câu lệnh cấp tệp, **theo đúng thứ tự nguồn**.
        for child in self.cst.root.children() {
            if is_item(child.kind) {
                if let Some(it) = self.build_item(child) {
                    self.items.push(it);
                }
            } else if let Some(s) = self.lower_stmt(child) {
                self.stmts.push(s);
            }
        }
        // **ST-57**: `impl` sinh từ `derives` nối **cuối** (sym/bảng `impls` đã có từ lượt 1c).
        self.items.append(&mut self.derived);
    }

    pub(crate) fn finish(self) -> HirOutput {
        let symbols = self.symbols.clone();
        let types = self.tbl.clone();
        HirOutput {
            hir: Hir {
                items: self.items,
                stmts: self.stmts,
                span: self.cst.root.span,
            },
            diagnostics: self.diags,
            symbols,
            types,
            error: self.error,
        }
    }

    // ----- tiện ích đọc CST -----

    pub(crate) fn tok(&self, i: u32) -> Option<&'a Token> {
        self.cst.tokens.get(i as usize)
    }

    pub(crate) fn text_of(&self, span: Span) -> &'a str {
        self.text.get(span.start..span.end).unwrap_or("")
    }

    /// Tên khai báo = token `Ident` đầu tiên của node (bỏ qua `pub`/`async`/từ khoá).
    pub(crate) fn decl_name(&self, node: &Node) -> Option<String> {
        node.tokens(&self.cst.tokens)
            .find(|t| t.kind == Kind::Ident)
            .map(|t| t.text.clone())
    }

    /// Token `Ident` đầu tiên + vị trí (dùng cho `var`/`scope`/pattern).
    pub(crate) fn first_ident(&self, node: &Node) -> Option<(String, Span)> {
        node.tokens(&self.cst.tokens)
            .find(|t| t.kind == Kind::Ident)
            .map(|t| (t.text.clone(), t.span))
    }

    pub(crate) fn has_token(&self, node: &Node, kw: &str) -> bool {
        node.tokens(&self.cst.tokens)
            .any(|t| t.kind == Kind::Kw && t.text == kw)
    }

    /// Vị trí `dòng:cột` (1-based; cột theo scalar, tab = 4 — SPEC-lex §4.7).
    pub(crate) fn pos(&self, span: Span) -> (u32, u32) {
        let end = span.start.min(self.text.len());
        let head = self.text.get(..end).unwrap_or("");
        let mut line = 1u32;
        let mut col = 1u32;
        for ch in head.chars() {
            match ch {
                '\n' => {
                    line += 1;
                    col = 1;
                }
                '\t' => col += 4,
                _ => col += 1,
            }
        }
        (line, col)
    }

    pub(crate) fn diag(
        &mut self,
        code: &'static vis_diag::Code,
        msg: impl Into<String>,
        span: Span,
    ) {
        let (line, col) = self.pos(span);
        self.diags.push(Diagnostic {
            code,
            message: msg.into(),
            span,
            line,
            col,
        });
    }

    pub(crate) fn command_mode(&self) -> bool {
        self.cst.cmd_sexp.is_some()
            || self
                .cst
                .root
                .children()
                .any(|n| n.kind == NodeKind::CmdShell || n.kind == NodeKind::CmdItems)
    }

    // ----- khung và ký hiệu -----

    pub(crate) fn push_frame(&mut self, kind: FrameKind, span: Span) {
        let parent = self.scope.last().copied();
        let id = self.symbols.frames.len() as u32;
        self.symbols.frames.push(HFrame { parent, span, kind });
        self.scope.push(id);
    }

    pub(crate) fn pop_frame(&mut self) {
        if self.scope.len() > 1 {
            self.scope.pop();
        }
    }

    pub(crate) fn current_frame(&self) -> u32 {
        self.scope.last().copied().unwrap_or(0)
    }

    pub(crate) fn declare_builtins(&mut self) {
        if self.symbols.syms.iter().any(|s| s.kind == SymKind::Builtin) {
            return;
        }
        for name in BUILTIN_VALUE_NAMES {
            self.symbols.syms.push(HSym {
                name: name.to_string(),
                kind: SymKind::Builtin,
                frame: 0,
                span: Span::new(0, 0),
                ty: None,
            });
            // Dựng sẵn **luôn** thấy, không phụ thuộc module.
            self.symbols.mod_of.push(u16::MAX);
            self.symbols.pub_of.push(true);
        }
    }

    /// **VD-258** — trait `Show`/`Error` dựng sẵn, có ở **mọi** tệp (kể cả đơn vị đa tệp đã
    /// dựng bảng kiểu ở tệp trước). Chỉ thêm một lần cho mỗi `Lowerer`.
    pub(crate) fn declare_builtin_traits(&mut self) {
        if self.traits.iter().any(|t| t.name == "Show") {
            return;
        }
        // --- trait `Show`: `fn show(self) -> str` ---
        self.traits.push(TraitSig {
            name: "Show".into(),
            supertraits: Vec::new(),
            methods: vec![("show".into(), 1, Type::Str, vec![Type::Error])],
        });
        // --- trait `Error`: siêu trait `Show`, 3 phương thức mặc định-less (VD-222 điểm 1; (ii)) ---
        self.traits.push(TraitSig {
            name: "Error".into(),
            supertraits: vec!["Show".into()],
            methods: vec![
                ("message".into(), 1, Type::Str, vec![Type::Error]),
                (
                    "source".into(),
                    1,
                    Type::Opt(Box::new(Type::Record(RecId(0)))),
                    vec![Type::Error],
                ),
                (
                    "span".into(),
                    1,
                    Type::Opt(Box::new(Type::Str)),
                    vec![Type::Error],
                ),
            ],
        });
    }

    /// **VD-222 (điểm 1–4; uỷ quyền A0-proxy)** — dựng sẵn kiểu dựng trước item người dùng:
    /// trait `Show`/`Error`, enum `Json`, bản ghi `Error`. Id ổn định: `EnumId(0) = Json`,
    /// `RecId(0) = Error` (bảng được dựng mới ở mỗi đơn vị biên dịch & được chèn trong cùng lượt).
    /// Người dùng khai trùng tên ⇒ `VS0302` (khai báo trước — VD-204 mở rộng).
    pub(crate) fn declare_kieu_dung_san(&mut self) {
        // **VD-258**: trait dựng sẵn phải có ở **mọi** tệp, kể cả tệp thứ 2+ (bảng kiểu dùng
        // chung đã có `Json`). Trước đây nhánh early-return bỏ qua `Show`/`Error` ⇒ `impl Show`
        // trong module thứ hai báo `VS0403` oan.
        self.declare_builtin_traits();
        if self.tbl.enums().iter().any(|e| e.name == "Json") {
            // **Đơn vị đa tệp** (ST-25): bảng kiểu **dùng chung** — tệp đầu đã dựng Json/SqlValue/
            // Error/RowSet ⇒ id ổn định. Tệp này phải **dựng lại bảng tên** của chính nó (mỗi tệp
            // một `Lowerer`), nếu không tên dựng sẵn chỉ tra được trong tệp đầu.
            const BUILTIN_ENUMS: [&str; 2] = ["Json", "SqlValue"];
            const BUILTIN_RECS: [&str; 2] = ["Error", "RowSet"];
            for (i, e) in self.tbl.enums().iter().enumerate() {
                if !BUILTIN_ENUMS.contains(&e.name.as_str()) {
                    continue;
                }
                if self.enum_names.iter().any(|(n, _)| *n == e.name) {
                    continue;
                }
                let id = EnumId(i as u32);
                self.enum_names.push((e.name.clone(), id));
                for (j, v) in e.variants.iter().enumerate() {
                    self.variant_index.push((v.name.clone(), id, j));
                }
            }
            for (i, r) in self.tbl.recs().iter().enumerate() {
                if let Some(n) = r.name.as_deref() {
                    if BUILTIN_RECS.contains(&n) && !self.rec_names.iter().any(|(rn, _)| rn == n) {
                        self.rec_names.push((n.to_string(), RecId(i as u32)));
                    }
                }
            }
            return;
        }
        let span0 = Span::new(0, 0);
        // --- enum `Json` (VD-222 điểm 4): Null/Bool/Num/Str/Arr/Obj ---
        let json_id = EnumId(self.tbl.enums().len() as u32);
        self.tbl.push_enum(EnumDef {
            name: "Json".into(),
            variants: vec![
                VariantDef {
                    name: "Null".into(),
                    fields: vec![],
                    span: span0,
                },
                VariantDef {
                    name: "Bool".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Bool,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                // **`{error}`** — `Num` giữ cả `int` và `float` (VD-204: "Num payload {error}";
                // dữ liệu thô ở runtime là `Value::Int`/`Value::Float` bóc bởi `stdlib`).
                VariantDef {
                    name: "Num".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Error,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                VariantDef {
                    name: "Str".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Str,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                VariantDef {
                    name: "Arr".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::List(Box::new(Type::Enum(json_id))),
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                VariantDef {
                    name: "Obj".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Map(Box::new(Type::Str), Box::new(Type::Enum(json_id))),
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
            ],
            span: span0,
        });
        self.enum_names.push(("Json".into(), json_id));
        // ctor trần `Null`/`Num`… (như `Some`/`Ok`)
        for (i, n) in ["Null", "Bool", "Num", "Str", "Arr", "Obj"]
            .iter()
            .enumerate()
        {
            self.variant_index.push(((*n).to_string(), json_id, i));
        }
        // --- bản ghi `Error` (VD-222 điểm 2–3): `source: Error?` đệ quy; `span: str?` ---
        let error_id = RecId(self.tbl.recs().len() as u32);
        self.tbl.push_rec(RecDef {
            name: Some("Error".into()),
            fields: vec![
                FieldDef {
                    name: "message".into(),
                    ty: Type::Str,
                    span: span0,
                    has_default: false,
                },
                FieldDef {
                    name: "source".into(),
                    ty: Type::Opt(Box::new(Type::Record(error_id))),
                    span: span0,
                    has_default: false,
                },
                FieldDef {
                    name: "span".into(),
                    ty: Type::Opt(Box::new(Type::Str)),
                    span: span0,
                    has_default: false,
                },
            ],
            span: span0,
            anonymous: false,
        });
        self.rec_names.push(("Error".into(), error_id));
        // **VD-241 (Gate 1, 2026-11-19)** — `std.sqlite` MVP: enum `SqlValue` (`EnumId(1)`) và
        // bản ghi `RowSet` (`RecId(1)`). **Không** vào prelude toàn cục theo nghĩa API: tên chỉ
        // **hiện ra** khi tệp `use std.sqlite` (hạ gate ở `named_type`/`ctor`; pattern **không**
        // gate — xem §2 hợp đồng VD-241, pattern là theo scrutinee). `Value::Enum`/`Value::Record`
        // sẵn có — **không** thêm variant ABI, không thêm opcode.
        let sqlvalue_id = EnumId(self.tbl.enums().len() as u32);
        self.tbl.push_enum(EnumDef {
            name: "SqlValue".into(),
            variants: vec![
                VariantDef {
                    name: "Null".into(),
                    fields: vec![],
                    span: span0,
                },
                VariantDef {
                    name: "Integer".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Int,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                VariantDef {
                    name: "Real".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Float,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                VariantDef {
                    name: "Text".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Str,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
                VariantDef {
                    name: "Blob".into(),
                    fields: vec![FieldDef {
                        name: "v".into(),
                        ty: Type::Bytes,
                        span: span0,
                        has_default: false,
                    }],
                    span: span0,
                },
            ],
            span: span0,
        });
        self.enum_names.push(("SqlValue".into(), sqlvalue_id));
        // **`Null` lặp với `Json.Null`**: `find` trả về khớp **đầu** (Json) ở mọi chỗ tìm theo
        // tên trần ⇒ constructor trần `Null` giữ nghĩa `Json.Null`; dựng `SqlValue.Null` qua
        // đường ctor **có tiền tố enum** (`SqlValue.Null`, `SqlValue.Integer(1)`…).
        for (i, n) in ["Null", "Integer", "Real", "Text", "Blob"]
            .iter()
            .enumerate()
        {
            self.variant_index.push(((*n).to_string(), sqlvalue_id, i));
        }
        // --- bản ghi `RowSet`: `columns` theo thứ tự chọn (trùng tên giữ nguyên), `rows` theo cùng thứ tự cột ---
        let rowset_id = RecId(self.tbl.recs().len() as u32);
        self.tbl.push_rec(RecDef {
            name: Some("RowSet".into()),
            fields: vec![
                FieldDef {
                    name: "columns".into(),
                    ty: Type::List(Box::new(Type::Str)),
                    span: span0,
                    has_default: false,
                },
                FieldDef {
                    name: "rows".into(),
                    ty: Type::List(Box::new(Type::List(Box::new(Type::Enum(sqlvalue_id))))),
                    span: span0,
                    has_default: false,
                },
            ],
            span: span0,
            anonymous: false,
        });
        self.rec_names.push(("RowSet".into(), rowset_id));
    }

    /// Khai một tên trong khung hiện tại; khai lại **cùng khung** ⇒ `VS0302` (VD-44 #2).
    ///
    /// **R16 (VD-178)**: `let` **được che tên** trong cùng phạm vi (`PatBind` không báo trùng);
    /// `var` **không** khai lại (`SymKind::Var` vẫn báo).
    pub(crate) fn declare(
        &mut self,
        name: &str,
        kind: SymKind,
        span: Span,
        ty: Option<Type>,
    ) -> u32 {
        let frame = self.current_frame();
        // **VD-204 (Q9 (4))**: tên của enum std (`Option`/`Result`) và biến thể dựng sẵn của
        // chúng (`Some`/`None`/`Ok`/`Err`) là **tên dự trữ ở phạm vi toàn cục** — khai lại hay
        // che tên ở đó ⇒ **VS0302** (mã `NAME_CODES[1]`, thông điệp nêu rõ lý do). Chỉ áp ở
        // khung tệp (`FrameKind::File`); trong hàm vẫn tự do. Khai báo dựng sẵn của chính trình
        // biên dịch (`SymKind::Builtin`, nạp **trước** mọi mục người dùng) được miễn.
        if kind != SymKind::Builtin
            && matches!(
                name,
                "Option"
                    | "Result"
                    | "Some"
                    | "None"
                    | "Ok"
                    | "Err"
                    | "Json"
                    | "Error"
                    | "AnyError"
                    | "Show"
                    // **VD-241 (Gate 1)**: enum/record dựng sẵn của module `std.sqlite`.
                    | "SqlValue"
                    | "RowSet"
            )
            && self
                .symbols
                .frames
                .get(frame as usize)
                .is_some_and(|f| f.kind == FrameKind::File)
        {
            self.diag(
                &NAME_CODES[1],
                format!(
                    "`{name}` là tên dự trữ của chuẩn dựng sẵn (`Option`/`Result`/`Json`/`Error`) — không được khai lại ở phạm vi toàn cục"
                ),
                span,
            );
        }
        // **VD-223 (điểm 2)** — tên biến/tham số trùng tên module std đã import ⇒ cảnh báo `VS0211`.
        if matches!(kind, SymKind::Var | SymKind::Param | SymKind::PatBind)
            && self.opts.std_mods.iter().any(|m| m == name)
        {
            self.diag(
                &vis_diag::SYNTAX_CODES[8],
                format!(
                    "tên `{name}` là module std đã import — `{name}.xxx(...)` được hiểu là thành viên của giá trị, không phải module"
                ),
                span,
            );
        }
        if kind != SymKind::PatBind {
            if let Some(prev) = self.symbols.in_frame(frame, name) {
                let (pl, pc) = self.pos(prev.span);
                let msg =
                    format!("khai báo trùng `{name}` trong cùng phạm vi (đã khai ở {pl}:{pc})");
                self.diag(&NAME_CODES[1], msg, span);
            }
        }
        let id = self.symbols.syms.len() as u32;
        self.symbols.syms.push(HSym {
            name: name.to_string(),
            kind,
            frame,
            span,
            ty,
        });
        self.symbols.mod_of.push(self.opts.mod_id);
        self.symbols.pub_of.push(false);
        id
    }

    /// Khai tên cho **vế sau** của or-pattern: đã có trong khung ⇒ dùng lại chỉ số, **không**
    /// báo `VS0302` (các vế or-pattern dùng chung một binding — §4.3).
    pub(crate) fn declare_or_reuse(&mut self, name: &str, span: Span) -> u32 {
        let frame = self.current_frame();
        if let Some(i) = self
            .symbols
            .syms
            .iter()
            .rposition(|s| s.frame == frame && s.name == name)
        {
            return i as u32;
        }
        self.declare(name, SymKind::PatBind, span, None)
    }

    /// Giải tên **giá trị** từ khung trong ra ngoài. `None` = chưa phân giải (tên tự do).
    ///
    /// ST-25: tên thuộc **module khác** chỉ thấy khi tệp hiện tại `use` module đó **và** mục có `pub`
    /// (mô hình "chỉ `pub`", §7.10). Tên dựng sẵn (`mod_of == u16::MAX`) luôn thấy.
    pub(crate) fn resolve(&self, name: &str) -> Option<u32> {
        for frame in self.scope.iter().rev() {
            for (i, s) in self.symbols.syms.iter().enumerate().rev() {
                if s.frame == *frame && s.name == name && self.duoc_thay(i) {
                    return Some(i as u32);
                }
            }
        }
        None
    }

    /// Ký hiệu thứ `i` có nằm trong tầm nhìn của tệp đang hạ không.
    pub(crate) fn duoc_thay(&self, i: usize) -> bool {
        let m = match self.symbols.mod_of.get(i) {
            Some(m) => *m,
            // Bảng cũ/không đồng bộ: coi như cùng module (an toàn cho API một tệp).
            None => return true,
        };
        if m == u16::MAX || m == self.opts.mod_id {
            return true;
        }
        self.opts.imports.contains(&m) && self.symbols.pub_of.get(i).copied().unwrap_or(false)
    }

    /// **VD-241 (Gate 1)**: `use std.sqlite` đã có trong tệp đang hạ (tiền tố `sqlite` của lời gọi
    /// `sqlite.open(…)`). Dùng để gate tên kiểu dựng sẵn `SqlValue`/`RowSet` — **thuộc module**,
    /// không thấy ở tệp ngoài (tên dựng sẵn khác luôn thấy).
    pub(crate) fn sqlite_duoc_thay(&self) -> bool {
        self.opts.std_mods.iter().any(|m| m == "sqlite")
    }

    /// **VD-258** — `true` nếu tên là kiểu dựng sẵn đã có trong bảng tên của lượt hạ này
    /// (`Json`, `SqlValue`, `Error`, `RowSet`). Dùng để **không** báo `VS0301` khi tên xuất hiện
    /// làm **tiền tố ctor** (`Json.Arr(…)`, `SqlValue.Integer(…)`) dưới `NamePolicy::Strict` —
    /// `maybe_enum_lit` sẽ phân giải tiếp. Tên **trần** vẫn đi đường cũ.
    pub(crate) fn la_ten_kieu_dung_san(&self, name: &str) -> bool {
        if !matches!(name, "Json" | "SqlValue" | "Error" | "RowSet") {
            return false;
        }
        self.enum_names.iter().any(|(n, _)| n == name)
            || self.rec_names.iter().any(|(n, _)| n == name)
    }

    /// Id enum dựng sẵn `SqlValue` (nếu bảng đã dựng).
    pub(crate) fn sqlvalue_id(&self) -> Option<EnumId> {
        self.enum_names
            .iter()
            .find(|(n, _)| n == "SqlValue")
            .map(|(_, id)| *id)
    }

    /// Tên module sở hữu ký hiệu đầu tiên mang `name` (dùng cho thông điệp `VS0301`).
    pub(crate) fn module_cua_ten(&self, name: &str) -> Option<&str> {
        let i = self.symbols.syms.iter().rposition(|s| s.name == name)?;
        let m = *self.symbols.mod_of.get(i)?;
        if m == u16::MAX {
            return None;
        }
        self.symbols.mods.get(m as usize).map(|x| x.name.as_str())
    }

    /// Hạ **một lần dùng tên giá trị**: phân giải + `VS0301` theo chính sách.
    pub(crate) fn lower_var_use(&mut self, name: &str, span: Span) -> HExpr {
        match self.resolve(name) {
            Some(sym) => HExpr::Var {
                name: name.to_string(),
                sym: Some(sym),
                span,
            },
            None => {
                // **ST-130 (VD-124)**: tên **module `std`** đã `use` (`fs`, `net`, …) không phải biến —
                // nó là **tiền tố** của lời gọi `fs.exists(…)` nên phải **im lặng** cả dưới Strict.
                // Trước ST-130, tệp vừa `use pkg.*` (⇒ Strict) vừa `use std.fs` bị `VS0301` ở `fs`,
                // tức chương trình dùng gói **không thể** chạm fs/net ⇒ không kiểm được VD-124.
                let la_module_std = self.opts.std_mods.iter().any(|m| m == name);
                if self.opts.names == NamePolicy::Strict
                    && !self.suppress_name_diag
                    && !la_module_std
                {
                    // ST-25: nếu tên **có** ở module khác thì nêu tên module và lý do bị chặn.
                    let msg = match self.module_cua_ten(name) {
                        Some(m) => format!(
                            "không thấy tên `{name}`: module `{m}` chưa được `use` hoặc mục không `pub`"
                        ),
                        None => format!("tên không tồn tại: `{name}`"),
                    };
                    self.diag(&NAME_CODES[0], msg, span);
                }
                HExpr::Var {
                    name: name.to_string(),
                    sym: None,
                    span,
                }
            }
        }
    }

    pub(crate) fn item_sym(&self, name: &str) -> u32 {
        self.item_syms
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
            .unwrap_or(0)
    }

    // ----- kiểu -----
}
