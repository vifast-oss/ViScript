// SPDX-License-Identifier: MIT OR Apache-2.0
//! Luật **phân loại** dòng chế độ lệnh: `let` khai báo ↔ shell-let, tên khai, tham số,
//! thân `cmd` dạng khai báo; `KHAI`/`KieuThan` dùng chung với phần parse.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

/// **ST-48**: kiểu thân `cmd` (xem `Parser::than_cmd_kiem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KieuThan {
    Script,
    Command,
    Mixed,
}

/// Từ khoá **khai báo** ở cột 1 trong `.vish` (ST-44) — phần còn lại là stage.
pub(crate) const KHAI: [&str; 13] = [
    "type", "enum", "cmd", "fn", "use", "const", "trait", "impl", "alias", "newtype", "extern",
    "pub", "async",
];

/// **ST-59** — `let` ở dòng cột 1 là **khai báo** (vùng item `.vish`) hay **shell-let** (stage)?
///
/// Luật **một nguồn sự thật** (dùng cả ở parser lẫn `vis-types::lower::khai`):
/// - RHS **không** có `|`, `;`, chuyển hướng `>`/`<`, `&`;
/// - RHS **không** là **một từ trần** (`let out = ls`/`let x = git` ⇒ shell-let) — trừ ba literal
///   `true`/`false`/`none`;
/// - RHS parse được như **biểu thức script** (`let x = 1`, `let xs = [1, 2, 3]` ⇒ khai báo).
/// - **Không** đòi chú thích kiểu (ST-59): `let x = 1` ở vùng item **bind** như `let x: int = 1`.
pub fn let_la_khai(d: &str) -> bool {
    let mac_dinh = ten_mac_dinh();
    let_la_khai_voi(d, &mac_dinh)
}

/// **ST-60b**: tập tên **coi là đã bind** khi phân loại `let` (tên dựng sẵn + `std`/`pkg`/`self`).
///
/// Danh sách **dựng sẵn** phải khớp `vis_types::lower::BUILTIN_VALUE_NAMES` — có test khoá ở
/// `vis-types` (`ten_dung_san_khop_voi_vis_types`), vì `vis-syntax` **không** được phụ thuộc ngược.
pub fn ten_mac_dinh() -> std::collections::HashSet<String> {
    [
        "print", "len", "str", "echo", "lines", "take", "Ok", "Err", "Some", "None", "ref", "panic",
    ]
    .iter()
    .chain(["std", "pkg", "self", "true", "false", "none"].iter())
    .map(|s| s.to_string())
    .collect()
}

/// Tên **được khai** ở một dòng cột 1 (`let x …` ⇒ `x`, `fn f` ⇒ `f`, …); `None` nếu dòng không
/// khai tên đơn giản (`impl`, `use`, `extern`, …).
pub fn ten_khai_bao(d: &str) -> Option<String> {
    let t: Vec<&str> = d.split_whitespace().collect();
    let dau = *t.first()?;
    if !matches!(
        dau,
        "let" | "var" | "fn" | "type" | "enum" | "const" | "alias" | "newtype" | "trait" | "cmd"
    ) {
        return None;
    }
    let ten = t.get(1)?.trim_end_matches(':');
    let ten = ten.split('(').next().unwrap_or(ten);
    let ten = ten.split(':').next().unwrap_or(ten);
    let ten = ten.split('[').next().unwrap_or(ten);
    if ten.is_empty() || !ten.chars().next().unwrap_or(' ').is_alphabetic() {
        return None;
    }
    Some(ten.to_string())
}

/// Mọi **định danh** trong RHS mà **chưa** bind (trừ tên **trường** sau `.` và **khoá** bản ghi
/// trước `:`); trả tên đầu tiên chưa bind, hoặc `None` nếu đủ.
///
/// Nhờ vậy `let out = ls -l` là **shell-let** (`l` chưa bind) còn `let b = a` (sau `let a = …`) là
/// **khai báo** — đúng `VD-95`/ST-60b, không nới cho `ls`/`grep` chạy như biểu thức.
pub fn dinh_danh_chua_bind(
    rhs: &str,
    da_bind: &std::collections::HashSet<String>,
) -> Option<String> {
    let lexed = crate::lex::lex(rhs, crate::lex::Mode::Script);
    let toks: Vec<&crate::lex::Token> = lexed
        .tokens
        .iter()
        .filter(|t| t.kind != crate::lex::Kind::Eof)
        .collect();
    for (i, t) in toks.iter().enumerate() {
        if !matches!(t.kind, crate::lex::Kind::Ident | crate::lex::Kind::Kw) {
            continue;
        }
        // Từ khoá của ngôn ngữ **không** phải tên cần bind (trừ literal đã có ở `ten_mac_dinh`).
        if t.kind == crate::lex::Kind::Kw {
            continue;
        }
        let truoc_la_cham = i > 0 && toks[i - 1].text == ".";
        let sau_la_hai_cham = toks.get(i + 1).map(|n| n.text == ":").unwrap_or(false);
        if truoc_la_cham || sau_la_hai_cham {
            continue;
        }
        if !da_bind.contains(&t.text) {
            return Some(t.text.clone());
        }
    }
    None
}

/// **ST-60b**: `let …` là **khai báo** khi (a) RHS parse thành **biểu thức script**, (b) không phải
/// một **từ trần** (trừ literal), và (c) **mọi định danh** trong RHS **đã bind** (`let out = ls -l`
/// ⇒ shell-let). `da_bind` = tên dựng sẵn + tên khai **trước** đó.
pub fn let_la_khai_voi(d: &str, da_bind: &std::collections::HashSet<String>) -> bool {
    let Some((_trai, phai)) = d.split_once('=') else {
        return false;
    };
    let rhs = phai.trim();
    if rhs.is_empty()
        || rhs.contains('|')
        || rhs.contains(';')
        || rhs.contains('>')
        || rhs.contains('<')
        || rhs.contains('&')
    {
        return false;
    }
    let mot_tu = rhs
        .chars()
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false)
        && rhs
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        && !matches!(rhs, "true" | "false" | "none");
    // **ST-60b**: một **từ trần** mà **đã bind** (`let b = a` sau `let a = …`) vẫn là **khai báo**;
    // chưa bind (`let out = ls`, `let g = git`) ⇒ **shell-let** (giữ như trước ST-59).
    if mot_tu && !da_bind.contains(rhs) {
        return false;
    }
    if !crate::parse::parse(&format!("let __x = {rhs}"), crate::lex::Mode::Script)
        .diagnostics
        .is_empty()
    {
        return false;
    }
    dinh_danh_chua_bind(rhs, da_bind).is_none()
}

/// **ST-60b**: tên **tham số** trong dòng `cmd f(a: int, b: str)` / `fn f(…)` — đã bind trong thân.
pub fn tham_so_cua(d: &str) -> Vec<String> {
    let Some(i) = d.find('(') else {
        return Vec::new();
    };
    let Some(j) = d[i..].find(')').map(|x| i + x) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for phan in d[i + 1..j].split(',') {
        let ten = phan.split(':').next().unwrap_or("").trim();
        let ten = ten.trim_start_matches("var ").trim();
        if !ten.is_empty()
            && ten
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        {
            out.push(ten.to_string());
        }
    }
    out
}

/// **ST-59** — thân `cmd` = **vùng khai báo** (`let …` ở đầu) + **đúng một** dòng cuối là
/// pipeline/biểu thức lệnh? Dùng ở parser (gom raw, không parse lệnh) **và** tầng hạ (hạ hai phần).
/// **ST-60b**: `da_bind` = tên đã bind; tên khai trong **chính** vùng này được cộng dần.
pub fn than_khai_lenh_voi(dong: &[String], da_bind: &std::collections::HashSet<String>) -> bool {
    let mut bind = da_bind.clone();
    let mut i = 0usize;
    while i < dong.len() && dong[i].trim().is_empty() {
        i += 1;
    }
    let dau = i;
    while i < dong.len() && let_la_khai_voi(dong[i].trim(), &bind) {
        if let Some(ten) = ten_khai_bao(&dong[i]) {
            if dong[i].trim_start().starts_with("let ") {
                bind.insert(ten);
            }
        }
        i += 1;
    }
    if i == dau {
        return false;
    }
    let con: Vec<&String> = dong[i..].iter().filter(|l| !l.trim().is_empty()).collect();
    if con.len() != 1 {
        return false;
    }
    crate::parse::parse(con[0].trim(), crate::lex::Mode::Command)
        .diagnostics
        .is_empty()
}

pub fn than_khai_lenh(dong: &[String]) -> bool {
    than_khai_lenh_voi(dong, &ten_mac_dinh())
}
