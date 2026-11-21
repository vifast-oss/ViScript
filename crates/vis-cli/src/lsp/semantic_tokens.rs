// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/semanticTokens/full` cho LSP.
//!
//! Legend **tối thiểu, ổn định** (chỉ số = vị trí trong [`TOKEN_TYPES`], **không** đổi thứ tự):
//! `keyword` (0), `type` (1), `function` (2), `variable` (3), `string` (4), `number` (5),
//! `comment` (6), `operator` (7), `parameter` (8). Không dùng `tokenModifiers` (luôn `0`).
//!
//! Phân loại **cố ý rẻ, không cần chính xác ngữ nghĩa tuyệt đối**:
//! - `vis_syntax::lex` quyết định `keyword`/`string`/`number`/`comment`/`operator`;
//! - định danh (`Kind::Ident`) lấy **ngữ cảnh CST gần nhất**: khai báo `type`/`enum`/`trait`/
//!   biến thể/generic ⇒ `type`; tên `fn`/`cmd`/`test fn`/phương thức + callee gọi trực tiếp ⇒
//!   `function`; tên tham số ⇒ `parameter`; pattern/`let`/`var`/trường/nhãn ⇒ `variable`;
//!   còn lại ⇒ `variable`.
//!
//! Mã hoá theo LSP: mảng phẳng 5 số/token — `deltaLine`, `deltaStartChar` (0-based, **UTF-16**),
//! `length` (UTF-16), `tokenType`, `tokenModifiers` (= 0). Token đầu có `deltaLine` **tuyệt đối**;
//! token sau là hiệu so với token trước. Token nhiều dòng (chuỗi ba nháy) được **cắt theo dòng**;
//! token rỗng bị bỏ; token chồng lấn bị bỏ (giữ token trước theo nguồn).

use vis_interp::value::Value;
use vis_syntax::{Cst, Elem, Kind, Node, NodeKind, Token};

use super::jsonrpc::{m, phan_hoi};
use super::phan_tich::KetQua;

/// Legend chính thức — **giữ nguyên thứ tự** (chỉ số dùng trong [`SemTok::loai`]).
pub const TOKEN_TYPES: [&str; 9] = [
    "keyword",
    "type",
    "function",
    "variable",
    "string",
    "number",
    "comment",
    "operator",
    "parameter",
];

const KW: u8 = 0;
const TYPE: u8 = 1;
const FN: u8 = 2;
const VAR: u8 = 3;
const STR: u8 = 4;
const NUM: u8 = 5;
const COMMENT: u8 = 6;
const OP: u8 = 7;
const PARAM: u8 = 8;

/// Một token đã quy đổi sang toạ độ LSP (0-based UTF-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemTok {
    pub dong: u32,
    pub cot: u32,
    pub dai: u32,
    /// Chỉ số trong [`TOKEN_TYPES`].
    pub loai: u8,
}

/// `semanticTokensProvider` của `initialize`: `{ legend: { tokenTypes, tokenModifiers: [] },
/// full: true }`.
pub fn provider() -> Value {
    let types = TOKEN_TYPES
        .iter()
        .map(|t| Value::Str((*t).to_string()))
        .collect();
    m(vec![
        (
            "legend",
            m(vec![
                ("tokenTypes", Value::List(types)),
                ("tokenModifiers", Value::List(Vec::new())),
            ]),
        ),
        ("full", Value::Bool(true)),
    ])
}

/// Khung ngữ cảnh CST trong lúc duyệt (một node trên đường từ gốc tới lá).
#[derive(Clone, Copy)]
struct Khung {
    kind: NodeKind,
    /// Token `Ident` **trực tiếp** đầu tiên của node (tên khai báo, tên trường, tên tham số…).
    ten_dau: Option<u32>,
    /// Token `Ident` trực tiếp thứ **hai** — chỉ dùng cho `test fn <tên>` (`test` là Ident).
    ten_hai: Option<u32>,
    /// Callee trực tiếp của `Call`/`GenericCall`/`SpawnExpr` khi nó là node `Ident`.
    callee: Option<u32>,
}

/// Token `Ident` trực tiếp (lá của chính node, không vào node con) theo thứ tự nguồn.
fn ident_truc_tiep<'a>(n: &'a Node, toks: &'a [Token]) -> impl Iterator<Item = u32> + 'a {
    n.elems.iter().filter_map(move |e| match e {
        Elem::Token(i) => toks
            .get(*i as usize)
            .filter(|t| t.kind == Kind::Ident)
            .map(|_| *i),
        Elem::Node(_) => None,
    })
}

/// Callee của lời gọi khi phần tử đầu là node `Ident` (gọi tên trực tiếp, không qua `.`).
fn callee_truc_tiep(n: &Node, toks: &[Token]) -> Option<u32> {
    if !matches!(
        n.kind,
        NodeKind::Call | NodeKind::GenericCall | NodeKind::SpawnExpr
    ) {
        return None;
    }
    n.children()
        .find(|c| c.kind == NodeKind::Ident)
        .and_then(|c| ident_truc_tiep(c, toks).next())
}

/// Điền `vai[i]` cho mọi token `Ident` theo ngữ cảnh CST gần nhất.
fn duyet(n: &Node, cha: &mut Vec<Khung>, toks: &[Token], vai: &mut [Option<u8>]) {
    let mut ids = ident_truc_tiep(n, toks);
    let ten_dau = ids.next();
    let ten_hai = ids.next();
    cha.push(Khung {
        kind: n.kind,
        ten_dau,
        ten_hai,
        callee: callee_truc_tiep(n, toks),
    });
    for e in &n.elems {
        match e {
            Elem::Token(i) => {
                let idx = *i as usize;
                if toks.get(idx).is_some_and(|t| t.kind == Kind::Ident) {
                    vai[idx] = phan_loai(idx as u32, cha);
                }
            }
            Elem::Node(c) => duyet(c, cha, toks, vai),
        }
    }
    cha.pop();
}

/// Vai trò của token `Ident` `idx` theo khung **gần nhất** có luật; mặc định `variable`.
fn phan_loai(idx: u32, cha: &[Khung]) -> Option<u8> {
    for k in cha.iter().rev() {
        match k.kind {
            // Mọi định danh trong cây kiểu là `type` (trừ tên trường do `TypeField` chặn dưới).
            NodeKind::Type => return Some(TYPE),
            // Tên trường/nhãn: chỉ lá Ident **đầu** là tên; biểu thức/kiểu con đi tiếp.
            NodeKind::TypeField
            | NodeKind::VariantField
            | NodeKind::RecordField
            | NodeKind::WithField
            | NodeKind::NamedArg
            | NodeKind::Field
            | NodeKind::OptField => {
                if k.ten_dau == Some(idx) {
                    return Some(VAR);
                }
            }
            // Tên hàm/phương thức (kể cả `cmd`/`extern cmd`).
            NodeKind::FnDecl | NodeKind::CmdDecl | NodeKind::ExternDecl | NodeKind::TraitItem => {
                if k.ten_dau == Some(idx) {
                    return Some(FN);
                }
            }
            // `test` là từ khoá theo ngữ cảnh; tên hàm là Ident thứ **hai**.
            NodeKind::TestDecl => {
                if k.ten_dau == Some(idx) {
                    return Some(KW);
                }
                if k.ten_hai == Some(idx) {
                    return Some(FN);
                }
            }
            NodeKind::Param => {
                if k.ten_dau == Some(idx) {
                    return Some(PARAM);
                }
            }
            // Mọi định danh trong pattern là ràng buộc (kể cả tên biến thể — chấp nhận rẻ).
            NodeKind::Pattern => return Some(VAR),
            // Gọi hàm trực tiếp bằng tên.
            NodeKind::Call | NodeKind::GenericCall => {
                if k.callee == Some(idx) {
                    return Some(FN);
                }
            }
            // `spawn` là soft keyword; hàm được spawn là Ident kế tiếp.
            NodeKind::SpawnExpr => {
                if k.ten_dau == Some(idx) {
                    return Some(KW);
                }
                if k.callee == Some(idx) {
                    return Some(FN);
                }
            }
            // `assert` là soft keyword đầu câu.
            NodeKind::AssertStmt => {
                if k.ten_dau == Some(idx) {
                    return Some(KW);
                }
            }
            // Tên khai báo kiểu + tham số generic + biến thể enum.
            NodeKind::TypeDecl
            | NodeKind::EnumDecl
            | NodeKind::AliasDecl
            | NodeKind::NewtypeDecl
            | NodeKind::TraitDecl
            | NodeKind::Variant
            | NodeKind::Generic => {
                if k.ten_dau == Some(idx) {
                    return Some(TYPE);
                }
            }
            // Tên biến/hằng.
            NodeKind::VarStmt | NodeKind::ConstDecl | NodeKind::LetStmt
                if k.ten_dau == Some(idx) =>
            {
                return Some(VAR);
            }
            _ => {}
        }
    }
    Some(VAR)
}

/// `true` nếu dấu câu `s` là toán tử đáng tô sáng (bỏ ngoặc/phẩy/hai chấm/chấm).
fn la_toan_tu(s: &str) -> bool {
    matches!(
        s,
        "+" | "-"
            | "*"
            | "/"
            | "//"
            | "%"
            | "**"
            | "=="
            | "!="
            | "<"
            | "<="
            | ">"
            | ">="
            | "="
            | "->"
            | "=>"
            | "|>"
            | "??"
            | "?."
            | "!"
            | "&"
            | "|"
            | "~"
            | ".."
            | "..="
    )
}

/// Loại token **không phải định danh** theo `Kind` của lexer.
fn loai_lexical(t: &Token) -> Option<u8> {
    match t.kind {
        Kind::Kw => Some(KW),
        Kind::Str | Kind::RawStr | Kind::MultiStr | Kind::RawMulti | Kind::Bytes | Kind::Char => {
            Some(STR)
        }
        // `Path` là literal đường dẫn (capability `@ ./x`) — hiển thị như chuỗi.
        Kind::Path => Some(STR),
        Kind::Int | Kind::Float | Kind::Size | Kind::Duration | Kind::Percent => Some(NUM),
        Kind::Comment | Kind::DocComment => Some(COMMENT),
        Kind::Punct => {
            if la_toan_tu(&t.text) {
                Some(OP)
            } else {
                None
            }
        }
        // `$x` (biến phiên) / từ trần chế độ lệnh — coi là biến, không tô từ khoá.
        Kind::Dollar | Kind::Word => Some(VAR),
        _ => None,
    }
}

/// Con trỏ byte → `(line, cột UTF-16)` **một lượt** cho các span đã sắp theo nguồn.
struct ViTri<'a> {
    text: &'a str,
    byte: usize,
    dong: u32,
    cot: u32,
}

impl<'a> ViTri<'a> {
    fn moi(text: &'a str) -> Self {
        Self {
            text,
            byte: 0,
            dong: 0,
            cot: 0,
        }
    }

    /// Tiến tới byte `den` (phải ≥ vị trí hiện tại); cập nhật dòng/cột UTF-16.
    fn tien(&mut self, den: usize) {
        let den = den.min(self.text.len());
        if den <= self.byte {
            return;
        }
        for ch in self.text[self.byte..den].chars() {
            if ch == '\n' {
                self.dong += 1;
                self.cot = 0;
            } else {
                self.cot += ch.len_utf16() as u32;
            }
        }
        self.byte = den;
    }
}

/// Gom token ngữ nghĩa từ CST + bảng token lexer, quy đổi **0-based UTF-16**, đã sắp theo nguồn.
///
/// `text` phải là văn bản đã chuẩn hoá của CST (`lexed.text`, NFC + CRLF→LF) — mọi span trỏ vào đó.
pub fn gom(cst: &Cst, text: &str) -> Vec<SemTok> {
    let toks = &cst.tokens;
    let mut vai: Vec<Option<u8>> = vec![None; toks.len()];
    let mut cha: Vec<Khung> = Vec::new();
    duyet(&cst.root, &mut cha, toks, &mut vai);

    let mut out: Vec<SemTok> = Vec::new();
    let mut cur = ViTri::moi(text);
    let mut da_den = 0usize;
    for (i, t) in toks.iter().enumerate() {
        let Some(loai) = loai_lexical(t).or_else(|| match t.kind {
            Kind::Ident => Some(vai[i].unwrap_or(VAR)),
            _ => None,
        }) else {
            continue;
        };
        let start = t.span.start.min(text.len());
        let end = t.span.end.min(text.len());
        if start >= end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            continue; // token rỗng/hỏng — bỏ
        }
        if start < da_den {
            continue; // chồng lấn (không nên xảy ra với token lexer) — giữ token trước
        }
        // Token nhiều dòng (chuỗi ba nháy) ⇒ cắt từng dòng; mỗi mảnh là một token LSP hợp lệ.
        let mut seg = start;
        while seg < end {
            let seg_end = text[seg..end].find('\n').map_or(end, |k| seg + k);
            if seg_end > seg {
                cur.tien(seg);
                let (dong, cot) = (cur.dong, cur.cot);
                cur.tien(seg_end);
                let dai = cur.cot - cot; // cùng dòng ⇒ hiệu cột là độ dài UTF-16
                if dai > 0 {
                    out.push(SemTok {
                        dong,
                        cot,
                        dai,
                        loai,
                    });
                }
            }
            if seg_end == end {
                break;
            }
            cur.tien(seg_end + 1); // bỏ ký tự `\n` ngăn dòng
            seg = seg_end + 1;
        }
        da_den = end;
    }
    out.sort_by_key(|t| (t.dong, t.cot, t.dai));
    out
}

/// Mã hoá danh sách token thành mảng phẳng 5 số/token (LSP `SemanticTokens.data`).
fn ma_hoa(ds: &[SemTok]) -> Value {
    let mut data: Vec<Value> = Vec::with_capacity(ds.len() * 5);
    let (mut dong_truoc, mut cot_truoc) = (0u32, 0u32);
    for (i, t) in ds.iter().enumerate() {
        let (dl, dc) = if i == 0 {
            (t.dong, t.cot)
        } else if t.dong == dong_truoc {
            (0, t.cot.saturating_sub(cot_truoc))
        } else {
            (t.dong - dong_truoc, t.cot)
        };
        data.push(Value::Int(i64::from(dl)));
        data.push(Value::Int(i64::from(dc)));
        data.push(Value::Int(i64::from(t.dai)));
        data.push(Value::Int(i64::from(t.loai)));
        data.push(Value::Int(0));
        dong_truoc = t.dong;
        cot_truoc = t.cot;
    }
    Value::List(data)
}

/// Phản hồi `textDocument/semanticTokens/full`: `{ "data": [...] }`.
pub fn tra_loi(kq: &KetQua, id: &Value) -> Vec<u8> {
    phan_hoi(id, m(vec![("data", ma_hoa(&kq.sem))]))
}

#[cfg(test)]
mod tests {
    use super::{gom, ma_hoa, TOKEN_TYPES};
    use vis_interp::value::Value;
    use vis_syntax::{lex, parse_voi, Mode};

    fn phan_tich(src: &str) -> Vec<(u32, u32, u32, u8)> {
        let lexed = lex(src, Mode::Script);
        let parsed = parse_voi(&lexed.text, lexed.tokens.clone(), Mode::Script);
        gom(&parsed.cst, &lexed.text)
            .into_iter()
            .map(|t| (t.dong, t.cot, t.dai, t.loai))
            .collect()
    }

    /// Các loại cơ bản trên một dòng: keyword/type/function/parameter/number/operator.
    #[test]
    fn phan_loai_co_ban() {
        let ds = phan_tich("fn cong(a: int) -> int:\n    return a + 1\n");
        assert_eq!(TOKEN_TYPES.len(), 9);
        assert_eq!(ds[0], (0, 0, 2, 0)); // fn
        assert_eq!(ds[1], (0, 3, 4, 2)); // cong
        assert_eq!(ds[2], (0, 8, 1, 8)); // a
        assert_eq!(ds[3], (0, 11, 3, 1)); // int
        assert_eq!(ds[4], (0, 16, 2, 7)); // ->
        assert_eq!(ds[5], (0, 19, 3, 1)); // int
        assert!(ds.contains(&(1, 4, 6, 0))); // return
        assert!(ds.contains(&(1, 11, 1, 3))); // a
        assert!(ds.contains(&(1, 13, 1, 7))); // +
        assert!(ds.contains(&(1, 15, 1, 5))); // 1
    }

    /// UTF-16: emoji (2 code unit) và ký tự có dấu (1 code unit) — `length`/`character` đúng.
    #[test]
    fn do_dai_utf16() {
        let ds = phan_tich("let s = \"😀 chào\"\n");
        assert_eq!(ds[0], (0, 0, 3, 0)); // let
        assert_eq!(ds[1], (0, 4, 1, 3)); // s
        assert_eq!(ds[2], (0, 6, 1, 7)); // =
                                         // "😀 chào" = 1 + 2 + 1 + 4 + 1 = 9 code unit UTF-16.
        assert_eq!(ds[3], (0, 8, 9, 4));
    }

    /// Chuỗi nhiều dòng bị cắt theo dòng; token rỗng bị bỏ; không chồng lấn.
    #[test]
    fn chuoi_nhieu_dong_cat_theo_dong() {
        let ds = phan_tich("let s = \"\"\"a\nbb\"\"\"\n");
        let s: Vec<_> = ds
            .iter()
            .filter(|t| t.3 == 4)
            .map(|t| (t.0, t.1, t.2))
            .collect();
        assert_eq!(s, vec![(0, 8, 4), (1, 0, 5)]);
        // Không chồng lấn và tăng theo (dòng, cột).
        for w in ds.windows(2) {
            assert!(w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 + w[0].2 <= w[1].1));
        }
    }

    /// Tệp rỗng/khoảng trắng ⇒ không token.
    #[test]
    fn rong_va_khoang_trang() {
        assert!(phan_tich("").is_empty());
        assert!(phan_tich("   \n\t\n").is_empty());
    }

    /// `test`/`assert`/`spawn` là soft keyword; callee gọi tên là function.
    #[test]
    fn soft_keyword_va_callee() {
        let ds = phan_tich(
            "test fn kiem():\n    assert cong(1) == 2\n\nfn cong(x: int) -> int:\n    spawn f(x)\n",
        );
        assert!(ds.contains(&(0, 0, 4, 0))); // test
        assert!(ds.contains(&(0, 8, 4, 2))); // kiem
        assert!(ds.contains(&(1, 4, 6, 0))); // assert
        assert!(ds.contains(&(1, 11, 4, 2))); // cong (callee)
        assert!(ds.contains(&(4, 4, 5, 0))); // spawn
    }

    /// Mã hoá delta: token đầu tuyệt đối, token sau cùng dòng là hiệu cột.
    #[test]
    fn ma_hoa_delta() {
        let v = ma_hoa(&[
            super::SemTok {
                dong: 2,
                cot: 5,
                dai: 3,
                loai: 0,
            },
            super::SemTok {
                dong: 2,
                cot: 10,
                dai: 1,
                loai: 1,
            },
            super::SemTok {
                dong: 4,
                cot: 7,
                dai: 2,
                loai: 2,
            },
        ]);
        let Value::List(ds) = v else {
            panic!("phải là list");
        };
        let so: Vec<i64> = ds
            .into_iter()
            .map(|v| match v {
                Value::Int(n) => n,
                _ => panic!("phải là Int"),
            })
            .collect();
        assert_eq!(so, vec![2, 5, 3, 0, 0, 0, 5, 1, 1, 0, 2, 7, 2, 2, 0]);
    }
}
