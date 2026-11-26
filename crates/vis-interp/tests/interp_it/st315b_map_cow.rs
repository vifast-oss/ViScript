// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315b (VD-291 §3.2)** — `Value::Map(Rc<VisMap>)` CoW: khoá **Q1–Q3** ở tầng `Value`
//! (nguồn duy nhất mà cả interp, VM, `json.decode` và stdlib dùng).
//!
//! - **Q1**: khoá trùng theo `eq_value` ⇒ cập nhật **tại chỗ**, giữ khoá gốc + vị trí đầu.
//! - **Q2**: `json.decode` trùng khoá ⇒ vị trí đầu / giá trị cuối, một entry.
//! - **Q3**: `remove` rồi `set` lại ⇒ chèn **cuối**, byte-for-byte như `Vec` cũ.
//!
//! Ca ở đây **không** lặp lại `vd270_map_eq*` (đã khoá ngữ nghĩa `eq_value`); chúng khoá hợp
//! đồng CoW + chỉ mục băm lười + kết quả `Show`/`keys` của ba phán quyết.

use vis_core::json;
use vis_core::value::{Value, VisMap, NGUONG_CHI_MUC};

fn s(x: &str) -> Value {
    Value::Str(x.into())
}

fn map(kv: Vec<(&str, Value)>) -> Value {
    Value::map(kv.into_iter().map(|(k, v)| (s(k), v)).collect())
}

#[test]
fn q1_khoa_trung_giu_khoa_goc_va_vi_tri_dau() {
    let m = map(vec![
        ("a", Value::Int(1)),
        ("b", Value::Int(2)),
        ("a", Value::Int(3)),
    ]);
    // Khoá gốc `Str("a")` giữ nguyên (không bị thay bằng khoá sau), giá trị cuối `3`.
    assert_eq!(m.show(), "{\"a\": 3, \"b\": 2}");
    let Value::Map(rc) = &m else {
        panic!("phải là map")
    };
    assert_eq!(rc.len(), 2);
    assert_eq!(rc.keys().map(Value::show).collect::<Vec<_>>(), ["a", "b"]);

    // `int`/`float`/`percent` cùng miền số: khoá **đầu** (Int) ở lại, giá trị cuối thắng.
    let m = Value::map(vec![
        (Value::Int(1), Value::Int(10)),
        (Value::Float(1.0), Value::Int(20)),
        (Value::Percent(0.5), Value::Int(30)),
        (Value::Float(0.5), Value::Int(40)),
    ]);
    assert_eq!(m.show(), "{1: 20, 50pct: 40}");
    let Value::Map(rc) = &m else {
        panic!("phải là map")
    };
    assert!(matches!(&rc[0].0, Value::Int(1)));
    assert!(matches!(&rc[1].0, Value::Percent(_)));
}

#[test]
fn q2_json_trung_khoa_vi_tri_dau_gia_tri_cuoi() {
    let v = json::decode("{\"b\":1,\"a\":2,\"b\":3}").expect("JSON hợp lệ");
    assert_eq!(v.show(), "{\"b\": 3, \"a\": 2}");
    let Value::Map(rc) = &v else {
        panic!("phải là map")
    };
    assert_eq!(rc.len(), 2);
    assert_eq!(rc.keys().map(Value::show).collect::<Vec<_>>(), ["b", "a"]);

    // Nhiều khoá trùng + lồng nhau: vẫn một entry/khoá, giữ thứ tự xuất hiện **đầu**.
    let v = json::decode("{\"x\":1,\"y\":2,\"x\":3,\"x\":4}").expect("JSON hợp lệ");
    assert_eq!(v.show(), "{\"x\": 4, \"y\": 2}");
}

#[test]
fn q3_remove_roi_set_lai_chen_cuoi() {
    let mut rc = match map(vec![
        ("a", Value::Int(1)),
        ("b", Value::Int(2)),
        ("c", Value::Int(3)),
    ]) {
        Value::Map(rc) => rc,
        _ => unreachable!(),
    };
    let m = std::rc::Rc::make_mut(&mut rc);
    assert!(m
        .remove(&s("b"))
        .is_some_and(|v| v.eq_value(&Value::Int(2))));
    assert_eq!(m.keys().map(Value::show).collect::<Vec<_>>(), ["a", "c"]);
    m.insert(s("b"), Value::Int(9));
    assert_eq!(
        m.keys().map(Value::show).collect::<Vec<_>>(),
        ["a", "c", "b"]
    );
    assert_eq!(m.get(&s("b")).map(Value::show), Some("9".to_string()));
    assert_eq!(Value::Map(rc).show(), "{\"a\": 1, \"c\": 3, \"b\": 9}");
}

#[test]
fn co_chep_khi_chia_se_va_ghi_tai_cho_khi_doc() {
    // 64 khoá **phân biệt** ⇒ map đủ ngưỡng, có chỉ mục băm.
    let a = Value::map(
        (0..64)
            .map(|i| (s(&format!("k{i}")), Value::Int(i)))
            .collect(),
    );
    let mut b = a.clone();
    let truoc = match (&a, &b) {
        (Value::Map(ra), Value::Map(rb)) => {
            // Đọc trên bản chia sẻ không nhân bản entry (chỉ tăng refcount).
            assert!(std::rc::Rc::ptr_eq(ra, rb));
            ra.len()
        }
        _ => unreachable!(),
    };
    assert_eq!(truoc, 64);
    if let Value::Map(rc) = &mut b {
        let m = std::rc::Rc::make_mut(rc);
        assert_eq!(m.len(), 64);
        // Ghi **khoá đã có** trên bản clone: index phải được dựng lại (review F3).
        m.insert(s("k0"), Value::Int(100));
        assert!(m.co_chi_muc(), "clone + sửa khoá cũ phải dựng lại chỉ mục");
    }
    let lay = |v: &Value, k: &str| match v {
        Value::Map(m) => m.get(&s(k)).map(Value::show),
        _ => None,
    };
    assert_eq!(lay(&a, "k0"), Some("0".to_string()), "bản gốc không đổi");
    assert_eq!(lay(&b, "k0"), Some("100".to_string()));
    assert_eq!(lay(&b, "k63"), Some("63".to_string()));
}

#[test]
fn chi_muc_bam_luoi_dung_tu_nguong() {
    let nho =
        VisMap::from_pairs((0..NGUONG_CHI_MUC - 1).map(|i| (Value::Int(i as i64), Value::Int(0))));
    assert!(!nho.co_chi_muc(), "map nhỏ không được cấp phát chỉ mục");

    let mut lon = VisMap::from_pairs(
        (0..NGUONG_CHI_MUC).map(|i| (Value::Int(i as i64), Value::Int(i as i64 + 100))),
    );
    assert!(lon.co_chi_muc(), "map đủ ngưỡng phải có chỉ mục");

    // Tra khoá vô hướng qua chỉ mục; khoá phức tạp rơi về quét tuyến tính (không âm tính giả).
    for i in 0..NGUONG_CHI_MUC {
        assert!(lon
            .get(&Value::Int(i as i64))
            .is_some_and(|v| v.eq_value(&Value::Int(i as i64 + 100))));
    }
    lon.insert(Value::list(vec![Value::Int(1)]), Value::Int(7));
    assert!(lon
        .get(&Value::list(vec![Value::Int(1)]))
        .is_some_and(|v| v.eq_value(&Value::Int(7))));
    assert!(lon.get(&Value::list(vec![Value::Int(2)])).is_none());

    // `remove` giữ chỉ mục đồng bộ: xoá một khoá rồi tra lại.
    assert!(lon
        .remove(&Value::Int(3))
        .is_some_and(|v| v.eq_value(&Value::Int(103))));
    assert!(lon.get(&Value::Int(3)).is_none());
    assert!(lon
        .get(&Value::Int(4))
        .is_some_and(|v| v.eq_value(&Value::Int(104))));
}

#[test]
fn eq_value_map_dung_duong_chi_muc() {
    let a = Value::map(
        (0..64)
            .map(|i| (Value::Str(format!("k{i}")), Value::Int(i)))
            .collect(),
    );
    let mut b = a.clone();
    assert!(a.eq_value(&b));
    // Đảo thứ tự entry: ánh xạ không đổi ⇒ vẫn bằng.
    if let Value::Map(rc) = &mut b {
        let mut entries = rc.as_slice().to_vec();
        entries.reverse();
        *rc = std::rc::Rc::new(VisMap::from_pairs(entries));
    }
    assert!(a.eq_value(&b));
    assert!(b.eq_value(&a));
    // Lệch một giá trị ⇒ khác.
    if let Value::Map(rc) = &mut b {
        std::rc::Rc::make_mut(rc).insert(Value::Str("k0".into()), Value::Int(-1));
    }
    assert!(!a.eq_value(&b));
}
