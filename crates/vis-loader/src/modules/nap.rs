// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bộ nạp **đệ quy** (hậu duyệt DFS) và hạ HIR hợp nhất — phần lõi của [`super`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vis_diag::{MODULE_CODES, SYNTAX_CODES};
use vis_hir::{HExpr, HItem, HStmt, Hir, ModInfo, Symbols, Type, TypeTable};
use vis_syntax::{Diagnostic, Mode, NodeKind, Span};
use vis_types::{check, lower_into, LowerOptions, NamePolicy, TraitSig};

use crate::lock_meta::doc_khoa_cua;

use super::duong_dan::{
    chon_tep_pkg, chua_cai, chuan_hoa, doc_tep_goi, rel_cua, span_cua, tach_pkg, ung_vien, vi_tri,
};
use super::kieu::{FileUnit, LoadError, Loaded};
use super::state::LoadState;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mau {
    DangTham,
    Xong,
}

struct Nap<'a> {
    mode: Mode,
    main: PathBuf,
    main_dir: PathBuf,
    symbols: Symbols,
    types: TypeTable,
    files: Vec<FileUnit>,
    items: Vec<HItem>,
    item_rel: Vec<String>,
    stmts: Vec<HStmt>,
    span: Span,
    mod_cua_rel: BTreeMap<String, u16>,
    diags: Vec<(String, Diagnostic)>,
    deps: Vec<(String, Vec<String>)>,
    mau: BTreeMap<String, Mau>,
    pub_types: BTreeMap<String, Vec<(String, Type)>>,
    /// **ST-57 (VD-98 (1))**: `trait` `pub` của từng tệp — để tệp `use` nó kiểm `impl`/siêu trait.
    pub_traits: BTreeMap<String, Vec<TraitSig>>,
    cho_ngoai: bool,
    /// **ST-80**: văn bản tệp chính từ buffer LSP (`None` ⇒ đọc đĩa).
    main_text: Option<String>,
    /// **ST-114a**: chỉ số tệp kế tiếp trong đơn vị nạp (chính = 0, module theo `use`).
    file_id: u16,
    /// **ST-114b**: bảng `file_id → rel` theo đúng thứ tự cấp ở [`Nap::tham`].
    file_rel: Vec<String>,
    /// **ST-317c**: trạng thái nạp (overlay + cache) **tiêm từ ngoài** — không dùng `thread_local`.
    trang_thai: &'a mut LoadState,
}

impl Nap<'_> {
    fn bao(&mut self, rel: &str, code: &'static vis_diag::Code, msg: String, span: Span) {
        let text = self
            .files
            .iter()
            .find(|f| f.rel == rel)
            .map(|f| f.text.clone())
            .or_else(|| {
                if rel == rel_cua(&self.main_dir, &self.main) {
                    std::fs::read_to_string(&self.main).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default();
        // ST-240 (#178): span trỏ vào văn bản đã chuẩn hoá CRLF→LF.
        let norm = vis_syntax::lex::normalize_source(&text);
        let (line, col) = vi_tri(&norm, span);
        self.diags.push((
            rel.to_string(),
            Diagnostic {
                code,
                message: msg,
                span,
                line,
                col,
            },
        ));
    }

    /// Giải một `use` thành đường dẫn tệp (hoặc lỗi `07xx`).
    fn giai_use(
        &self,
        seg: &[String],
        tu_dir: &Path,
    ) -> Result<PathBuf, (&'static vis_diag::Code, String)> {
        let dau = seg.first().map(String::as_str).unwrap_or("");
        if dau == "std" {
            return Err((&MODULE_CODES[4], "`std` chưa có — ST-26".to_string()));
        }
        if dau == "pkg" {
            return self.giai_pkg(seg);
        }
        let ten = seg.join("/");
        for c in ung_vien(tu_dir, &ten, self.mode) {
            if c.is_file() {
                return Ok(chuan_hoa(&c));
            }
        }
        Err((
            &MODULE_CODES[0],
            format!("không thấy tệp module `{ten}` (đã thử .vis/.vish)"),
        ))
    }

    /// **ST-71 (VD-101 (5))**: `use pkg.<tên>[.<module>]` — đọc `vis.lock` + chọn tệp trong gói.
    fn giai_pkg(&self, seg: &[String]) -> Result<PathBuf, (&'static vis_diag::Code, String)> {
        let Some(ten) = seg.get(1).filter(|s| !s.is_empty()) else {
            return Err((
                &MODULE_CODES[5],
                "`use pkg.<tên>` thiếu tên gói".to_string(),
            ));
        };
        if !self.main_dir.join("vis.lock").is_file() {
            return Err((&MODULE_CODES[5], chua_cai(ten)));
        }
        let khoa = doc_khoa_cua(&self.main_dir).map_err(|e| (&MODULE_CODES[5], e))?;
        let Some(goi) = khoa.iter().find(|g| &g.name == ten) else {
            return Err((&MODULE_CODES[5], chua_cai(ten)));
        };
        let relpath = chon_tep_pkg(goi, &seg[2..]).map_err(|e| (&MODULE_CODES[5], e))?;
        Ok(self
            .main_dir
            .join(".vis")
            .join("pkg")
            .join(&goi.name)
            .join(&goi.version)
            .join(relpath))
    }

    fn tham(&mut self, path: &Path) {
        let path = chuan_hoa(path);
        let rel = rel_cua(&self.main_dir, &path);
        if self.mau.contains_key(&rel) {
            return;
        }
        let text = match tach_pkg(&self.main_dir, &path) {
            // **ST-71**: tệp gói — đọc **nội dung đã chuẩn hoá** rồi so `file_sha` với lock.
            Some((ten, ver, relpath)) => match doc_tep_goi(&self.main_dir, &ten, &ver, &relpath) {
                Ok(t) => t,
                Err((ma, msg)) => {
                    self.bao(&rel, ma, msg, Span::new(0, 0));
                    self.mau.insert(rel, Mau::Xong);
                    return;
                }
            },
            None => match match (&self.main_text, path == self.main) {
                (Some(t), true) => Ok(t.clone()),
                // **ST-100**: tệp phụ đọc qua cache (mtime+size); tệp chính luôn đọc theo buffer/đĩa.
                _ => self.trang_thai.doc_tep_phu(&path),
            } {
                Ok(t) => t,
                Err(e) => {
                    self.diags.push((
                        rel.clone(),
                        Diagnostic {
                            code: &MODULE_CODES[0],
                            message: format!("không đọc được tệp: {e}"),
                            span: Span::new(0, 0),
                            line: 1,
                            col: 1,
                        },
                    ));
                    return;
                }
            },
        };
        let mode = if path.extension().map(|x| x == "vish").unwrap_or(false) {
            Mode::Command
        } else {
            self.mode
        };
        // **ST-114a**: cấp chỉ số tệp theo **thứ tự nạp** (chính 0, module theo `use`).
        let file_id = self.file_id;
        self.file_id += 1;
        self.file_rel.push(rel.clone());
        let parsed = vis_syntax::parse_file(&text, mode, file_id);
        let lexed = vis_syntax::lex_file(&text, mode, file_id);
        if !parsed.diagnostics.is_empty() {
            for d in &parsed.diagnostics {
                self.diags.push((rel.clone(), d.clone()));
            }
            self.mau.insert(rel, Mau::Xong);
            return;
        }
        let mod_id = match self.mod_cua_rel.get(&rel) {
            Some(id) => *id,
            None => {
                let id = self.symbols.add_mod(ModInfo {
                    name: path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    path: rel.clone(),
                });
                self.mod_cua_rel.insert(rel.clone(), id);
                id
            }
        };
        self.mau.insert(rel.clone(), Mau::DangTham);
        // 1) Giải `use` từ CST (chưa cần hạ).
        let mut ds: Vec<(Vec<String>, Span)> = Vec::new();
        for n in parsed.cst.root.children() {
            if n.kind != NodeKind::UseDecl {
                continue;
            }
            let Some(dp) = n.child(NodeKind::DottedPath) else {
                continue;
            };
            // **VD-241 § fix(Windows/CRLF #psvn-A1)**: span của CST/Lex trỏ vào văn bản đã chuẩn hoá
            // CRLF→LF (normalize_source). Dùng bản chuẩn hoá (cùng text đã đi qua parser) để slice —
            // không lệch byte khi tệp CRLF (trên Linux cũng bị nếu file có CRLF).
            let raw = vis_syntax::lex::normalize_source(&text)
                .get(dp.span.start..dp.span.end)
                .unwrap_or("")
                .trim()
                .to_string();
            let seg: Vec<String> = if raw.contains('/') {
                vec![raw]
            } else {
                raw.split('.').map(str::to_string).collect()
            };
            ds.push((seg, n.span));
        }
        let tu_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.main_dir.clone());
        let mut con: Vec<String> = Vec::new();
        let mut da_co: Vec<String> = Vec::new();
        let mut std_mods: Vec<String> = Vec::new();
        for (seg, span) in ds {
            // ST-26: `use std.<module>` — module **trong Rust**, không phải tệp.
            if seg.first().map(String::as_str) == Some("std") {
                let m2 = seg.get(1).cloned().unwrap_or_default();
                if vis_hir::stdlib::module_ho_tro(&m2) {
                    if std_mods.contains(&m2) {
                        self.bao(
                            &rel,
                            &MODULE_CODES[2],
                            format!("`use std.{m2}` trùng"),
                            span,
                        );
                    } else {
                        std_mods.push(m2);
                    }
                } else if m2 == "fs" {
                    self.bao(
                        &rel,
                        &MODULE_CODES[4],
                        "`std.fs` chưa có — ST-27 (cần host)".to_string(),
                        span,
                    );
                } else if vis_hir::stdlib::MODULES_HOAN.contains(&m2.as_str()) {
                    self.bao(
                        &rel,
                        &MODULE_CODES[4],
                        format!("`std.{m2}` chưa hỗ trợ (VS1b)"),
                        span,
                    );
                } else {
                    self.bao(
                        &rel,
                        &MODULE_CODES[4],
                        format!(
                            "không có module `std.{m2}` (có: str/list/map/path/time/json/fs/net/http/sqlite)"
                        ),
                        span,
                    );
                }
                continue;
            }
            match self.giai_use(&seg, &tu_dir) {
                Ok(dich) => {
                    let d_rel = rel_cua(&self.main_dir, &dich);
                    if da_co.contains(&d_rel) {
                        self.bao(
                            &rel,
                            &MODULE_CODES[2],
                            format!("`use` trùng module `{d_rel}` (một module, nhiều mục `use`)"),
                            span,
                        );
                        continue;
                    }
                    da_co.push(d_rel.clone());
                    if self.mau.get(&d_rel) == Some(&Mau::DangTham) {
                        self.bao(
                            &rel,
                            &MODULE_CODES[1],
                            format!("vòng phụ thuộc: `{rel}` → `{d_rel}`"),
                            span,
                        );
                        continue;
                    }
                    con.push(d_rel.clone());
                    self.tham(&dich);
                }
                Err((code, msg)) => self.bao(&rel, code, msg, span),
            }
        }
        self.deps.push((rel.clone(), con.clone()));
        // 2) Hạ tệp **vào bảng chung** (phụ thuộc đã hạ xong ⇒ kiểu `pub` sẵn sàng).
        let imports: Vec<u16> = con
            .iter()
            .filter_map(|r| self.mod_cua_rel.get(r).copied())
            .collect();
        let mut imported_types: Vec<(String, Type)> = Vec::new();
        let mut imported_traits: Vec<TraitSig> = Vec::new();
        for r in &con {
            if let Some(ds) = self.pub_types.get(r) {
                imported_types.extend(ds.iter().cloned());
            }
            if let Some(ts) = self.pub_traits.get(r) {
                imported_traits.extend(ts.iter().cloned());
            }
        }
        // Chính sách tên: tệp **không** `use` ai và không bị `use` thì giữ **Permissive** như trước
        // (VD-45: tên tự do ⇒ lỗi runtime `VS0501`); tệp trong module dùng **Strict** để bắt
        // tầm nhìn `pub`/`use` ngay khi biên dịch.
        let trong_module = !con.is_empty() || self.mod_cua_rel.len() > 1;
        let names = if trong_module {
            NamePolicy::Strict
        } else {
            NamePolicy::Permissive
        };
        let opts = LowerOptions {
            names,
            mod_id,
            imports,
            imported_types,
            trait_ngoai: imported_traits,
            std_mods,
            cho_ngoai: self.cho_ngoai,
            file: file_id,
        };
        let out = lower_into(
            &mut self.symbols,
            &mut self.types,
            &parsed.cst,
            &lexed,
            opts,
        );
        for d in &out.diagnostics {
            self.diags.push((rel.clone(), d.clone()));
        }
        // **ST-93b (VD-111)**: trong `.vish`, `fn main` **chỉ** để khai `cap`; thân có câu lệnh sẽ
        // **không** được chạy (trước đây im lặng: có pipeline thì bị bỏ, không pipeline thì chạy
        // main) ⇒ báo `VS0201` tại **câu lệnh đầu**, chặn cả `check` lẫn `run` (hai đường).
        if mode == Mode::Command {
            if let Some(HItem::Fn(f)) = out
                .hir
                .items
                .iter()
                .find(|it| matches!(it, HItem::Fn(f) if f.name == "main"))
            {
                // `pass` không phải câu lệnh thật; thân chỉ có `pass` (hoặc rỗng) ⇒ **hợp lệ**.
                let co_that = f.body.iter().find(
                    |st| !matches!(st, HStmt::Expr(HExpr::Var { name, .. }) if name == "pass"),
                );
                if let Some(st) = co_that {
                    let span = st.span();
                    // ST-240 (#178): cùng bản chuẩn hoá với span của CST/HIR.
                    let norm = vis_syntax::lex::normalize_source(&text);
                    let (line, col) = vi_tri(&norm, span);
                    self.diags.push((
                        rel.clone(),
                        Diagnostic {
                            code: &SYNTAX_CODES[0],
                            message: "trong `.vish`, `fn main` chỉ dùng để khai `cap`; thân `main` không được chạy — chuyển câu lệnh ra cấp tệp hoặc dùng `.vis`".to_string(),
                            span,
                            line,
                            col,
                        },
                    ));
                }
            }
        }
        let mut pubt: Vec<(String, Type)> = Vec::new();
        let mut pubt_trait: Vec<TraitSig> = Vec::new();
        for it in &out.hir.items {
            let la_pub =
                |sym: u32, s: &Symbols| s.pub_of.get(sym as usize).copied().unwrap_or(false);
            if let HItem::Trait(t) = it {
                if la_pub(t.sym, &self.symbols) {
                    pubt_trait.push(TraitSig {
                        name: t.name.clone(),
                        supertraits: t.supertraits.clone(),
                        methods: t
                            .methods
                            .iter()
                            .map(|m| {
                                (
                                    m.name.clone(),
                                    m.params.len(),
                                    m.ret.clone(),
                                    m.params.iter().map(|p| p.ty.clone()).collect(),
                                )
                            })
                            .collect(),
                    });
                }
            }
            match it {
                HItem::Rec(r) if la_pub(r.sym, &self.symbols) => {
                    pubt.push((r.name.clone(), Type::Record(r.id)));
                }
                HItem::Enum(e) if la_pub(e.sym, &self.symbols) => {
                    pubt.push((e.name.clone(), Type::Enum(e.id)));
                }
                HItem::Alias(a) if la_pub(a.sym, &self.symbols) => {
                    pubt.push((a.name.clone(), a.ty.clone()));
                }
                _ => {}
            }
        }
        self.pub_types.insert(rel.clone(), pubt);
        self.pub_traits.insert(rel.clone(), pubt_trait);
        for it in out.hir.items {
            self.item_rel.push(rel.clone());
            self.items.push(it);
        }
        if rel == rel_cua(&self.main_dir, &self.main) {
            self.stmts = out.hir.stmts;
            self.span = out.hir.span;
        }
        self.files.push(FileUnit {
            path: path.clone(),
            rel: rel.clone(),
            text,
        });
        self.mau.insert(rel, Mau::Xong);
    }
}

pub(super) fn load_full_voi(
    main: &str,
    text_chinh: Option<&str>,
    cho_ngoai: bool,
    kiem_kieu: bool,
    main_ao: bool,
    trang_thai: &mut LoadState,
) -> Result<Loaded, LoadError> {
    let main_path = PathBuf::from(main);
    if !main_ao && !main_path.is_file() {
        return Err(LoadError::NotFile(main.to_string()));
    }
    let mode = if main.ends_with(".vish") {
        Mode::Command
    } else {
        Mode::Script
    };
    let main_dir = main_path
        .parent()
        .map(|p| p.to_path_buf())
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut n = Nap {
        mode,
        main: chuan_hoa(&main_path),
        main_dir: main_dir.clone(),
        symbols: Symbols::default(),
        types: TypeTable::new(),
        files: Vec::new(),
        items: Vec::new(),
        item_rel: Vec::new(),
        stmts: Vec::new(),
        span: Span::new(0, 0),
        mod_cua_rel: BTreeMap::new(),
        diags: Vec::new(),
        deps: Vec::new(),
        mau: BTreeMap::new(),
        pub_types: BTreeMap::new(),
        pub_traits: BTreeMap::new(),
        cho_ngoai,
        main_text: text_chinh.map(str::to_string),
        file_id: 0,
        file_rel: Vec::new(),
        trang_thai,
    };
    n.tham(&main_path);
    let rel_chinh = rel_cua(&main_dir, &n.main);
    let hir = Hir {
        items: std::mem::take(&mut n.items),
        stmts: std::mem::take(&mut n.stmts),
        span: n.span,
    };
    // `check` **một** lần trên HIR hợp nhất; quy chẩn đoán về tệp có item chứa span.
    let kq = if kiem_kieu {
        check(&hir, &n.symbols, &n.types)
    } else {
        vis_types::tyck::CheckOutput {
            diags: Vec::new(),
            types_of_expr: Vec::new(),
            file_expr_types: Vec::new(),
            item_types: Vec::new(),
        }
    };
    let item_rel = n.item_rel.clone();
    for d in &kq.diags {
        // **ST-114b**: file-id do lex/parse gắn (ST-114a) quyết định tệp; chỉ fallback heuristic
        // span khi `file_id` ngoài bảng (không bao giờ xảy ra với mã hiện tại — có log cảnh báo).
        let rel = match n.file_rel.get(d.span.file as usize).cloned() {
            Some(r) => r,
            None => {
                eprintln!(
                    "vis: cảnh báo — chẩn đoán {} có file={} ngoài bảng {} tệp; dùng fallback span",
                    d.code.short,
                    d.span.file,
                    n.file_rel.len()
                );
                hir.items
                    .iter()
                    .enumerate()
                    .find(|(_, it)| {
                        let s = span_cua(it);
                        d.span.start >= s.start && d.span.start <= s.end
                    })
                    .map(|(i, _)| {
                        item_rel
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| rel_chinh.clone())
                    })
                    .unwrap_or_else(|| rel_chinh.clone())
            }
        };
        // Tyck không có văn bản ⇒ tự tính `dòng:cột` theo **tệp được gán** (ST-25 mục 3).
        let mut d2 = d.clone();
        if let Some(f) = n.files.iter().find(|f| f.rel == rel) {
            // **ST-240 / VD-242 §4 (nợ #178)**: `span` trỏ vào văn bản đã chuẩn hoá CRLF→LF
            // (`parse_file`/`lex_file`), nhưng `f.text` là nội dung thô trên đĩa ⇒ lệch dòng/cột
            // với tệp CRLF. Dùng cùng bản chuẩn hoá để tính vị trí (`check`, `--json`, `run`).
            let norm = vis_syntax::lex::normalize_source(&f.text);
            let (line, col) = vi_tri(&norm, d2.span);
            d2.line = line;
            d2.col = col;
        }
        n.diags.push((rel, d2));
    }
    Ok(Loaded {
        main: n.main,
        file_rel: n.file_rel,
        files: n.files,
        hir,
        symbols: n.symbols,
        types: n.types,
        diags: n.diags,
        deps: n.deps,
        item_rel,
    })
}
