// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-126 (VD-129)** — **đóng băng API**: ba danh sách là **cam kết tương thích**, sinh **từ code**:
//!
//! | tệp | nguồn | một dòng |
//! |---|---|---|
//! | `viscript/agent/API-STD.txt` | `vis_types::stdlib::FUNCS` | `tên<TAB>chữ ký` |
//! | `viscript/agent/API-CODES.txt` | `vis_diag::all_codes()` | `mã<TAB>mức: câu ngắn` |
//! | `viscript/agent/API-VBC1.txt` | `vis_bytecode::format::TEN_THE` + `Instr::tag()` | `số thẻ<TAB>tên` |
//!
//! Test này **đỏ** khi code thêm/xoá/đổi tên/đổi nghĩa một mục mà tệp không đổi (và ngược lại).
//! Luật đầy đủ ở **VD-129**: không xoá, không đổi tên, không đổi ý nghĩa, không tái dùng số mã/số thẻ;
//! thêm mới **chỉ** khi có VD nêu đích danh tên được thêm.
//!
//! **Sinh lại tệp** (chỉ Agent 0, sau khi đã có VD nêu đích danh mục thêm/xoá/đổi):
//! `VIS_FREEZE_REGEN=1 ~/.cargo/bin/cargo test -p vis-cli --test api_dong_bang`.

use std::collections::BTreeMap;
use std::path::PathBuf;

const VD: &str = "VD-129";

fn thu_muc_agent() -> PathBuf {
    // crates/vis-cli → viscript/crates/vis-cli → `../../agent`
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../agent")
}

/// Một danh sách đã sinh từ code: `khoá` (tên/mã/số thẻ) → `giá trị` (chữ ký/câu/số thẻ).
type DanhSach = BTreeMap<String, String>;

/// Các dòng **dữ liệu** của tệp: bỏ dòng trống và dòng bắt đầu bằng `#`.
fn doc_tep(ten: &str) -> Option<DanhSach> {
    let s = std::fs::read_to_string(thu_muc_agent().join(ten)).ok()?;
    let mut m = DanhSach::new();
    for d in s.lines() {
        let d = d.trim_end();
        if d.trim().is_empty() || d.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, v)) = d.split_once('\t') {
            m.insert(k.trim().to_string(), v.trim().to_string());
        } else {
            // Dòng không có TAB là dòng hỏng — báo như "giá trị rỗng" để diff nêu ra.
            m.insert(d.trim().to_string(), String::new());
        }
    }
    Some(m)
}

fn viet_tep(ten: &str, mo_ta: &str, ds: &DanhSach) {
    let mut s = String::new();
    s.push_str("# SPDX-License-Identifier: MIT OR Apache-2.0\n");
    s.push_str(&format!("# {mo_ta}\n"));
    s.push_str("#\n");
    s.push_str("# CAM KẾT TƯƠNG THÍCH (VD-129): KHÔNG xoá, KHÔNG đổi tên, KHÔNG đổi nghĩa,\n");
    s.push_str(
        "# KHÔNG tái dùng số mã/số thẻ. Thêm mới CHỈ khi có VD nêu đích danh tên được thêm.\n",
    );
    s.push_str("# Tệp thuộc Agent 0 sau lần tạo đầu (ST-126). Định dạng: <khoá><TAB><giá trị>.\n");
    for (k, v) in ds {
        s.push_str(k);
        s.push('\t');
        s.push_str(v);
        s.push('\n');
    }
    let p = thu_muc_agent().join(ten);
    std::fs::write(&p, s).unwrap_or_else(|e| panic!("ghi {}: {e}", p.display()));
}

/// So code ↔ tệp; in **chính xác** mục thêm/xoá/đổi rồi panic (nhắc `VD-129`).
fn so_khop(ten: &str, mo_ta: &str, code: DanhSach) {
    if std::env::var_os("VIS_FREEZE_REGEN").is_some() {
        viet_tep(ten, mo_ta, &code);
        eprintln!(
            "VIS_FREEZE_REGEN: đã ghi `viscript/agent/{ten}` ({} mục)",
            code.len()
        );
        return;
    }
    let Some(tep) = doc_tep(ten) else {
        panic!(
            "THIẾU tệp cam kết `viscript/agent/{ten}` (code có {} mục) — {VD} đòi ba danh sách \
             phải được ghi thành tệp; tạo lần đầu: VIS_FREEZE_REGEN=1 …",
            code.len()
        );
    };
    let mut them = Vec::new();
    let mut xoa = Vec::new();
    let mut doi = Vec::new();
    for (k, v) in &code {
        match tep.get(k) {
            None => them.push(format!("[{k}] = {v}")),
            Some(cu) if cu != v => doi.push(format!("[{k}] tệp {cu:?} → code {v:?}")),
            Some(_) => {}
        }
    }
    for (k, v) in &tep {
        if !code.contains_key(k) {
            xoa.push(format!("[{k}] = {v}"));
        }
    }
    if them.is_empty() && xoa.is_empty() && doi.is_empty() {
        return;
    }
    let mut chi_tiet = String::new();
    for (nhan, ds) in [("THÊM", &them), ("XOÁ", &xoa), ("ĐỔI", &doi)] {
        if !ds.is_empty() {
            chi_tiet.push_str(&format!("  {nhan} ({}): {}\n", ds.len(), ds.join("; ")));
        }
    }
    panic!(
        "`viscript/agent/{ten}` lệch với code — cam kết tương thích **{VD}** bị phá:\n{chi_tiet}\
         Nếu thay đổi là CHỦ Ý: phải có VD nêu **đích danh** mục được thêm/xoá/đổi rồi mới cập nhật \
         tệp (VIS_FREEZE_REGEN=1). Không có VD ⇒ hoàn nguyên code."
    );
}

// ----- 1) API-STD.txt -----

fn kieu_ty(t: &vis_types::stdlib::Ty) -> String {
    use vis_types::stdlib::Ty;
    match t {
        Ty::Int => "int".to_string(),
        Ty::Str => "str".to_string(),
        Ty::Bool => "bool".to_string(),
        Ty::ListStr => "list[str]".to_string(),
        Ty::FnKhoa => "fn(T) -> int | str".to_string(),
        Ty::ResultStrStr => "Result[str, Error]".to_string(),
        Ty::ResultUnitStr => "Result[(), Error]".to_string(),
        Ty::ResultListStrStr => "Result[list[str], Error]".to_string(),
        Ty::ResultHead => "Result[map[str, {error}], Error]".to_string(),
        // **VD-222 (18/11/2026, uỷ quyền A0-proxy)**: lỗi std → bản ghi dựng sẵn `Error`
        // (`RecId(0)`); `std.json.decode` → `Result[Json, Error]` (enum dựng sẵn `EnumId(0)`).
        Ty::Error => "Error".to_string(),
        Ty::Json => "Json".to_string(),
        Ty::ResultJsonErr => "Result[Json, Error]".to_string(),
        // **VD-241 Gate 1 (2026-11-19)** — variant mới của `std.sqlite` MVP (prototype nội bộ).
        // CHỈ thêm arm render kiểu này — **không** đổi logic so khớp, **không** regen tệp đóng băng;
        // `agent/API-STD.txt` giữ nguyên cho tới cổng tích hợp phê duyệt chữ ký (VD-241 §11).
        Ty::SqlValue => "SqlValue".to_string(),
        Ty::RowSet => "RowSet".to_string(),
        Ty::ResultIntErr => "Result[int, Error]".to_string(),
        Ty::ResultBoolErr => "Result[bool, Error]".to_string(),
        Ty::ResultRowSetErr => "Result[RowSet, Error]".to_string(),
        // Biến kiểu: `T`/`K`/`V` — **cùng** tên với `Ty::sang_type` để chữ ký không mơ hồ.
        Ty::Var(i) => vis_types::stdlib::ten_bien(*i).to_string(),
        Ty::List(t) => format!("list[{}]", kieu_ty(t)),
        Ty::Map(k, v) => format!("map[{}, {}]", kieu_ty(k), kieu_ty(v)),
    }
}

fn ds_std() -> DanhSach {
    let mut ds = DanhSach::new();
    for f in vis_types::stdlib::FUNCS.iter() {
        let mut chu_ky = String::from("(");
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                chu_ky.push_str(", ");
            }
            chu_ky.push_str(&kieu_ty(p));
        }
        chu_ky.push_str(") -> ");
        chu_ky.push_str(&kieu_ty(&f.ret));
        // Trùng tên là lỗi chữ ký — nêu rõ thay vì để `BTreeMap` nuốt im lặng.
        if ds.insert(f.name.to_string(), chu_ky.clone()).is_some() {
            panic!("FUNCS có hai hàm cùng tên `{}` — {VD}", f.name);
        }
    }
    ds
}

#[test]
fn api_std_khop_code() {
    so_khop(
        "API-STD.txt",
        "ĐÓNG BĂNG API std — sinh từ crates/vis-types/src/stdlib.rs::FUNCS (kèm chữ ký).\n# VD-222 (18/11/2026, uỷ quyền A0, Agent 1 thay mặt Agent 0): mọi `Result[_, str]` →\n# `Result[_, Error]` (lỗi std = bản ghi dựng sẵn `Error { message, source, span }`);\n# `json.decode` → `Result[Json, Error]`. Các mục đổi được nêu đích danh trong VD-222.\n# VD-231 (19/11/2026, A0 chốt — ratify): `span` là **[không ổn định]** — sau này đổi sang Span\n# có cấu trúc sẽ phá API; `AnyError` = alias của bản ghi `Error` (RecId(0)), không phải\n# `{error}` nữa (lỗi tĩnh được bắt thay vì bị nuốt).",
        ds_std(),
    );
}

// ----- 2) API-CODES.txt -----

fn ds_ma() -> DanhSach {
    let mut ds = DanhSach::new();
    for c in vis_diag::all_codes() {
        let muc = match c.severity {
            vis_diag::Severity::Error => "error",
            vis_diag::Severity::Warning => "warning",
        };
        let dong = format!("{muc}: {}", c.message_vi);
        if ds.insert(c.short.to_string(), dong).is_some() {
            panic!("all_codes() có hai mã cùng chuỗi `{}` — {VD}", c.short);
        }
    }
    ds
}

#[test]
fn api_codes_khop_code() {
    so_khop(
        "API-CODES.txt",
        "ĐÓNG BĂNG sổ mã chẩn đoán — sinh từ crates/vis-diag/src/lib.rs::all_codes().\n# VD-224 (2026-11-19, uỷ quyền A0-proxy): +**VS0211** (tên trùng module std đã import; cảnh báo);\n# VD-223 (proxy A0): +**VS0209** (proxy — **huỷ ở VD-224**, `--b` hợp lệ mọi hàm); huỷ VS0111 (VD-212).",
        ds_ma(),
    );
}

// ----- 3) API-VBC1.txt -----

/// **Mọi** lệnh `VBC1` theo **đúng thứ tự số thẻ** — để `Instr::tag()` của từng lệnh phải bằng
/// chỉ số của nó. Danh sách này là phần khoá **số ↔ lệnh** (không chỉ tên): đổi số thẻ của một
/// lệnh là test đỏ ngay, đúng VD-129.
fn moi_lenh() -> Vec<vis_bytecode::Instr> {
    use vis_bytecode::Instr as I;
    use vis_types::{BinOp, UnOp};
    vec![
        I::Const(0),
        I::LoadLocal(0),
        I::StoreLocal(0),
        I::Assign(0),
        I::LoadCaptured(0),
        I::Pop,
        I::Bin(BinOp::Add),
        I::Un(UnOp::Neg),
        I::Call(0, 0),
        I::CallBuiltin(0, 0),
        I::CallMethod(0, 0),
        I::Jump(0),
        I::JumpIfFalse(0),
        I::JumpIfNotTrue(0),
        I::JumpIfNone(0),
        I::JumpIfSome(0),
        I::Return,
        I::Halt,
        I::MakeList(0),
        I::MakeSet(0),
        I::MakeTuple(0),
        I::MakeMap(0),
        I::MakeRecord(0, 0),
        I::MakeEnum(0, 0),
        I::MakeSome,
        I::MakeNone,
        I::MakeOk,
        I::MakeErr,
        I::MakeClosure(0, 0),
        I::Field(0),
        I::Index,
        I::Set,
        I::With(0),
        I::Interp(0),
        I::IterInit,
        I::IterNext(0),
        I::TestLit(0),
        I::TestCtor(0, 0),
        I::TestTuple(0),
        I::TestList(0),
        I::TestRecord(0, 0, 0),
        I::TestRange(0, 0, 0),
        I::BindLocal(0),
        I::Try,
        I::Dup,
        I::Unwrap(0),
        I::CallValue(0),
        I::UnwrapOpt,
        I::IsUnit,
        I::BoolOr,
        I::BoolAnd,
        I::Swap,
        I::PushUnit,
        // **R6c (VD-178)**: định dạng `{expr:spec}`.
        I::Format(0),
        I::IndexOpt,
    ]
}

fn ds_the() -> DanhSach {
    let lenh = moi_lenh();
    assert_eq!(
        lenh.len(),
        vis_bytecode::format::TEN_THE.len(),
        "số lệnh dựng trong test ≠ `TEN_THE` — thêm/bớt thẻ {VD} phải khai ở cả hai nơi"
    );
    let mut ds = DanhSach::new();
    for (i, l) in lenh.iter().enumerate() {
        let tag = l.tag();
        assert_eq!(
            tag as usize, i,
            "lệnh thứ {i} có `tag()` = {tag} — đổi số thẻ là phá cam kết {VD} (đổi nghĩa ⇒ VBC2)"
        );
        let ten = vis_bytecode::format::ten_the(tag)
            .unwrap_or_else(|| panic!("thiếu tên cho thẻ {tag} ở `TEN_THE` — {VD}"));
        // Khoá **zero-pad 2 chữ số** để thứ tự từ điển = thứ tự số thẻ (0..99); API-VBC1.txt
        // sắp ổn định theo số, không theo chuỗi ("10" không được đứng trước "2").
        ds.insert(format!("{tag:02}"), ten.to_string());
    }
    ds
}

#[test]
fn api_vbc1_khop_code() {
    so_khop(
        "API-VBC1.txt",
        "ĐÓNG BĂNG bảng thẻ VBC1 — sinh từ crates/vis-bytecode/src/format.rs (TEN_THE + Instr::tag).",
        ds_the(),
    );
}
