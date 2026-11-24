// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`Render`** (ST-41, §9.6; ST-310b) — hiển thị **giá trị cuối pipeline** của chế độ lệnh:
//! `list[bản ghi]` ⇒ **bảng** (căn trái cho `str`, căn phải cho số/`size`/`duration`/`percent`);
//! bản ghi **đơn** ⇒ **khối khoá–giá trị**; `list` rỗng ⇒ `(rỗng)`; còn lại ⇒ `None` (dùng `Show` cũ).
//!
//! **ST-310b**: bố cục cột/khoảng đệm/độ rộng không còn viết ở đây — chuyển giá trị sang
//! [`vish_term::Table`]/[`vish_term::Record`] rồi để `vish-term` hạ văn bản. Nhờ vậy chỉ còn
//! **một** quy tắc độ rộng (NFC) và **một** bản hạ bảng trong toàn dòng ViScript.

use vish_term::{Align, AnsiPosix, ColorMode, Column, Record, Table, Terminal};

use vis_core::value::Value;

/// Độ rộng hiển thị: số `char` sau **NFC** (chốt ST-41) — nay lấy từ `vish-term`.
pub fn rong(s: &str) -> usize {
    vish_term::display_width(s)
}

fn so(v: &Value) -> bool {
    matches!(
        v,
        Value::Int(_) | Value::Float(_) | Value::Size(_) | Value::Duration(_) | Value::Percent(_)
    )
}

/// Bảng cho `list[bản ghi]`: cột = trường của **bản ghi đầu**, giữ thứ tự; 2 dấu cách giữa cột.
fn bang(items: &[Value]) -> String {
    let mut cot: Vec<String> = Vec::new();
    for it in items {
        if let Value::Record(_, fields) = it {
            for (k, _) in fields {
                if !cot.contains(k) {
                    cot.push(k.clone());
                }
            }
        }
    }
    let mut table = Table::new(
        cot.iter()
            .map(|c| Column::new(c.clone()))
            .collect::<Vec<_>>(),
    );
    for it in items {
        let mut r: Vec<String> = Vec::new();
        for c in &cot {
            let s = match it {
                Value::Record(_, fields) => fields
                    .iter()
                    .find(|(k, _)| k == c)
                    .map(|(_, v)| v.show())
                    .unwrap_or_default(),
                other => other.show(),
            };
            r.push(s);
        }
        table.push_row(r);
    }
    // Căn phải khi **mọi** ô có mặt ở cột đều là số và có ít nhất một ô như vậy.
    for (j, c) in cot.iter().enumerate() {
        let mut phai = true;
        let mut co_so = false;
        for it in items {
            if let Value::Record(_, fields) = it {
                if let Some((_, v)) = fields.iter().find(|(k, _)| k == c) {
                    if so(v) {
                        co_so = true;
                    } else {
                        phai = false;
                    }
                }
            }
        }
        if co_so && phai {
            table.columns[j].align = Align::Right;
        }
    }
    ha_van_ban(|term| term.table(&table))
}

/// Khối `khoá  giá trị` cho **một** bản ghi (căn khoá theo độ rộng lớn nhất).
fn khoi(fields: &[(String, Value)]) -> String {
    let mut rec = Record::new();
    for (k, v) in fields {
        rec.push(k.clone(), v.show());
    }
    ha_van_ban(|term| term.record(&rec))
}

/// Hạ một giá trị có cấu trúc thành văn bản **không màu** (Render của `.vish` là dữ liệu cho
/// pipeline/ống, không phải TUI). Backend ghi vào bộ nhớ nên lỗi ghi không xảy ra.
fn ha_van_ban(f: impl FnOnce(&mut AnsiPosix<Vec<u8>>) -> std::io::Result<()>) -> String {
    let mut term = AnsiPosix::with_writer(Vec::new()).with_color(ColorMode::None);
    let _ = f(&mut term);
    let _ = term.flush();
    String::from_utf8(term.into_inner()).expect("render văn bản là UTF-8")
}

/// `Some(chuỗi)` nếu giá trị cuối pipeline cần `Render`; `None` ⇒ dùng `Show` cũ.
///
/// `co_ban_ghi` = kiểu **tĩnh** của phần tử là **bản ghi** (do bộ hạ truyền qua đối số thứ hai) —
/// `(rỗng)` **chỉ** cho `list[bản ghi]` rỗng; `list[str]`/`list[int]` rỗng vẫn `[]` (chốt Agent 0).
pub fn in_gia_tri(v: &Value, co_ban_ghi: bool) -> Option<String> {
    match v {
        Value::Record(_, fields) if !fields.is_empty() => Some(khoi(fields)),
        Value::List(items) if items.is_empty() && co_ban_ghi => Some("(rỗng)\n".to_string()),
        Value::List(items)
            if !items.is_empty() && items.iter().all(|x| matches!(x, Value::Record(_, _))) =>
        {
            Some(bang(items))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ban_ghi(ten: &str, n: i64) -> Value {
        Value::Record(
            "F".to_string(),
            vec![
                ("name".to_string(), Value::Str(ten.to_string())),
                ("n".to_string(), Value::Int(n)),
            ],
        )
    }

    #[test]
    fn bang_can_phai_cot_so() {
        let v = Value::List(vec![ban_ghi("aa", 6), ban_ghi("bb", 1)]);
        assert_eq!(
            in_gia_tri(&v, false).unwrap(),
            "name  n\naa    6\nbb    1\n"
        );
    }

    #[test]
    fn khoi_ban_ghi_don() {
        let v = ban_ghi("a", 6);
        let s = in_gia_tri(&v, false).unwrap();
        assert_eq!(s, "name  a\nn     6\n");
    }

    #[test]
    fn list_rong_chi_khi_co_ban_ghi() {
        let v = Value::List(Vec::new());
        assert_eq!(in_gia_tri(&v, false), None);
        assert_eq!(in_gia_tri(&v, true).as_deref(), Some("(rỗng)\n"));
    }

    #[test]
    fn rong_theo_nfc() {
        assert_eq!(rong("Việt"), 4);
        assert_eq!(rong("e\u{0302}\u{0301}"), 1);
    }
}
