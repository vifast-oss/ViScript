// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — hover **phương thức lõi** (`list`/`map`/`str`): `xs.get(0)`, `s.upper()`, `m.get(k)`.
//!
//! Hai nguồn, cùng **một** hình dạng markdown:
//! 1. [`PHUONG_LOI`] — bảng chép **đúng** nhánh `tyck::kiem_method` (phần lõi này **không** nằm
//!    trong `vis_types::stdlib::FUNCS` vì không phải `std.*`). Chữ ký ghi tham số **gồm** người nhận.
//! 2. **Sugar R5c** (`x.f(a) ≡ std.<module>.f(x, a)`) — lấy từ [`FUNCS`](vis_types::stdlib::FUNCS),
//!    chỉ nhận khi tham số đầu khớp kiểu người nhận (vd `str.join` có tham số đầu `list[str]` ⇒
//!    `s.join(…)` **không** có hover, đúng như tyck từ chối).
//!
//! Không thêm gì vào completion: đây chỉ là tra cứu theo **kiểu tĩnh** của người nhận.

use vis_types::stdlib::Ty;

/// Một phương thức lõi: tên đầy đủ + tham số hiển thị (**gồm** người nhận) + kiểu trả.
struct ChuKy {
    ten: &'static str,
    tham: &'static [&'static str],
    tra: &'static str,
}

impl ChuKy {
    fn hien(&self) -> String {
        format!("{}({}) -> {}", self.ten, self.tham.join(", "), self.tra)
    }
}

/// Bảng chữ ký lõi — khớp từng nhánh của `vis_types::tyck::expr::Checker::kiem_method`
/// (`list`/`str`/`map`). `Ty` của `stdlib` không biểu diễn được `T?`, hàm, hay tuple nên bảng giữ
/// **chuỗi hiển thị**; phần sugar bên dưới vẫn in từ `Ty` bằng [`ten_ty`].
const PHUONG_LOI: [ChuKy; 28] = [
    // ----- list (15) -----
    ChuKy {
        ten: "list.len",
        tham: &["list[T]"],
        tra: "int",
    },
    ChuKy {
        ten: "list.get",
        tham: &["list[T]", "int"],
        tra: "T?",
    },
    ChuKy {
        ten: "list.first",
        tham: &["list[T]"],
        tra: "T?",
    },
    ChuKy {
        ten: "list.last",
        tham: &["list[T]"],
        tra: "T?",
    },
    ChuKy {
        ten: "list.push",
        tham: &["list[T]", "T"],
        tra: "list[T]",
    },
    ChuKy {
        ten: "list.map",
        tham: &["list[T]", "fn(T) -> U"],
        tra: "list[U]",
    },
    ChuKy {
        ten: "list.filter",
        tham: &["list[T]", "fn(T) -> bool"],
        tra: "list[T]",
    },
    ChuKy {
        ten: "list.enumerate",
        tham: &["list[T]"],
        tra: "list[tuple[int, T]]",
    },
    ChuKy {
        ten: "list.any",
        tham: &["list[T]", "fn(T) -> bool"],
        tra: "bool",
    },
    ChuKy {
        ten: "list.all",
        tham: &["list[T]", "fn(T) -> bool"],
        tra: "bool",
    },
    ChuKy {
        ten: "list.find",
        tham: &["list[T]", "fn(T) -> bool"],
        tra: "T?",
    },
    ChuKy {
        ten: "list.each",
        tham: &["list[T]", "fn(T) -> ()"],
        tra: "()",
    },
    ChuKy {
        ten: "list.is_empty",
        tham: &["list[T]"],
        tra: "bool",
    },
    ChuKy {
        ten: "list.sum",
        tham: &["list[T]"],
        tra: "T",
    },
    ChuKy {
        ten: "list.join",
        tham: &["list[T]", "str"],
        tra: "str",
    },
    // ----- str (7) -----
    ChuKy {
        ten: "str.len",
        tham: &["str"],
        tra: "int",
    },
    ChuKy {
        ten: "str.upper",
        tham: &["str"],
        tra: "str",
    },
    ChuKy {
        ten: "str.lower",
        tham: &["str"],
        tra: "str",
    },
    ChuKy {
        ten: "str.is_empty",
        tham: &["str"],
        tra: "bool",
    },
    ChuKy {
        ten: "str.chars",
        tham: &["str"],
        tra: "list[str]",
    },
    ChuKy {
        ten: "str.contains",
        tham: &["str", "str"],
        tra: "bool",
    },
    ChuKy {
        ten: "str.split",
        tham: &["str", "str"],
        tra: "list[str]",
    },
    // ----- map (6) -----
    ChuKy {
        ten: "map.len",
        tham: &["map[K, V]"],
        tra: "int",
    },
    ChuKy {
        ten: "map.get",
        tham: &["map[K, V]", "K"],
        tra: "V?",
    },
    ChuKy {
        ten: "map.has",
        tham: &["map[K, V]", "K"],
        tra: "bool",
    },
    ChuKy {
        ten: "map.is_empty",
        tham: &["map[K, V]"],
        tra: "bool",
    },
    ChuKy {
        ten: "map.keys",
        tham: &["map[K, V]"],
        tra: "list[K]",
    },
    ChuKy {
        ten: "map.values",
        tham: &["map[K, V]"],
        tra: "list[V]",
    },
];

/// Module của **kiểu tĩnh** người nhận nếu là kiểu có phương thức lõi.
///
/// Nhận đúng dạng `TypeTable::show` in ra (`str`, `list[int]`, `map[str, int]`); kiểu khác
/// (kể cả `??`/generic chưa chốt) ⇒ `None`, caller trả `null` chứ không đoán.
pub(super) fn module_cua_kieu(kieu: &str) -> Option<&'static str> {
    let k = kieu.trim();
    if k == "str" {
        Some("str")
    } else if k == "list" || k.starts_with("list[") {
        Some("list")
    } else if k == "map" || k.starts_with("map[") {
        Some("map")
    } else {
        None
    }
}

/// Chữ ký lõi đã biết của `module.ten`.
fn chu_ky_loi(module: &str, ten: &str) -> Option<String> {
    let full = format!("{module}.{ten}");
    PHUONG_LOI.iter().find(|c| c.ten == full).map(ChuKy::hien)
}

/// Tên kiểu trong `FUNCS` → hiển thị: biến kiểu `T`/`K`/`V`, `list[…]`, `map[…, …]`;
/// còn lại dùng [`Ty::ten`].
fn ten_ty(t: &Ty) -> String {
    match t {
        Ty::Var(i) => vis_types::stdlib::ten_bien(*i).to_string(),
        Ty::List(x) => format!("list[{}]", ten_ty(x)),
        Ty::Map(k, v) => format!("map[{}, {}]", ten_ty(k), ten_ty(v)),
        _ => t.ten().to_string(),
    }
}

/// Tham số đầu của hàm `FUNCS` có nhận đúng **loại** người nhận không — cùng luật
/// `kiem_std_method` (người nhận là `list`/`map`/`str`).
fn khop_nhan(module: &str, dau: &Ty) -> bool {
    match module {
        "list" => matches!(dau, Ty::List(_) | Ty::ListStr),
        "map" => matches!(dau, Ty::Map(..)),
        "str" => matches!(dau, Ty::Str),
        _ => false,
    }
}

/// Chữ ký **sugar** `std.<module>.<ten>` nếu tham số đầu khớp người nhận.
fn chu_ky_sugar(module: &str, ten: &str) -> Option<String> {
    let f = vis_types::stdlib::sig_of(&format!("{module}.{ten}"))?;
    if !khop_nhan(module, f.params.first()?) {
        return None;
    }
    let tham: Vec<String> = f.params.iter().map(ten_ty).collect();
    Some(format!(
        "{}({}) -> {}",
        f.name,
        tham.join(", "),
        ten_ty(&f.ret)
    ))
}

/// Hover markdown cho `kieu.ten` nếu là phương thức lõi/sugar; `None` ⇒ caller trả `null`.
pub(super) fn hover(kieu: &str, ten: &str) -> Option<String> {
    let module = module_cua_kieu(kieu)?;
    let chu_ky = chu_ky_loi(module, ten).or_else(|| chu_ky_sugar(module, ten))?;
    Some(format!(
        "```vis\n{chu_ky}\n```\n\nphương thức lõi của `{module}`"
    ))
}
