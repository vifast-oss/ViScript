// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (perf-map-eq)** — khoá **vô hướng** của `Value::Map` đi đường nhanh chỉ mục băm;
//! khoá phức tạp rơi về logic O(n²) cũ. Bộ test khoá **ngữ nghĩa không đổi** ở mọi ca biên:
//! map rỗng, 1000 khoá, last-wins, `Int`/`Float`/`Percent` chéo kiểu, `±0.0`, `NaN`,
//! `Size`/`Duration`/`Str`/`Path`/`Bool`/`Char`/`Bytes` tách tag, và fallback list/record/map
//! lồng/enum/unit/range. Map nhỏ (dưới ngưỡng nội bộ 200 entry) đi logic cũ, nên ca biên nhỏ
//! được **đệm** khoá vô hướng để chắc chắn chạy đường nhanh.

use vis_core::value::{eq, Value};

fn m(kv: Vec<(Value, Value)>) -> Value {
    Value::map(kv)
}

/// Đệm 120 khoá Int phân biệt (`100_000 + i`) để tổng entry hai vế vượt ngưỡng đường nhanh.
fn dem(mut kv: Vec<(Value, Value)>) -> Vec<(Value, Value)> {
    for i in 0..120 {
        kv.push((Value::Int(100_000 + i), Value::Int(0)));
    }
    kv
}

/// Một khoá/giá trị cần kiểm + khoá đệm ⇒ map đủ lớn để đi đường nhanh.
fn co_dem(k: Value, v: Value) -> Value {
    m(dem(vec![(k, v)]))
}

fn bang(a: &Value, b: &Value) {
    assert!(eq(a, b), "phải bằng: {a:?} vs {b:?}");
    assert!(eq(b, a), "phải bằng (đảo): {b:?} vs {a:?}");
}

fn khac(a: &Value, b: &Value) {
    assert!(!eq(a, b), "phải khác: {a:?} vs {b:?}");
    assert!(!eq(b, a), "phải khác (đảo): {b:?} vs {a:?}");
}

fn map_so(n: usize, f: impl Fn(usize) -> Value) -> Value {
    m((0..n).map(|i| (f(i), Value::Int(i as i64))).collect())
}

#[test]
fn map_rong_va_khac_do_dai() {
    bang(&m(vec![]), &m(vec![]));
    khac(&m(vec![]), &m(vec![(Value::Int(1), Value::Int(1))]));
    khac(&m(vec![(Value::Int(1), Value::Int(1))]), &m(vec![]));
    // Đủ lớn để chạm đường nhanh: một vế rỗng, vế kia nhiều khoá ⇒ vẫn `false`.
    khac(&m(vec![]), &map_so(200, |i| Value::Int(i as i64)));
}

#[test]
fn mot_nghin_khoa_vo_huong_bang_khac_dao_thu_tu() {
    let n = 1000;
    let a = map_so(n, |i| Value::Int(i as i64));
    let b = map_so(n, |i| Value::Int(i as i64));
    bang(&a, &b);

    let dao = m((0..n)
        .rev()
        .map(|i| (Value::Int(i as i64), Value::Int(i as i64)))
        .collect());
    bang(&a, &dao);

    // Lệch giữa / cuối / thiếu khoá / thừa khoá.
    let mut giua = (0..n)
        .map(|i| {
            let v = if i == n / 2 { -1 } else { i as i64 };
            (Value::Int(i as i64), Value::Int(v))
        })
        .collect::<Vec<_>>();
    khac(&a, &m(giua.clone()));
    giua[n - 1].1 = Value::Int(-2);
    khac(&a, &m(giua));

    let thieu = m((0..n - 1)
        .map(|i| (Value::Int(i as i64), Value::Int(i as i64)))
        .collect());
    khac(&a, &thieu);

    let thua = m((0..=n)
        .map(|i| (Value::Int(i as i64), Value::Int(i as i64)))
        .collect());
    khac(&a, &thua);
}

#[test]
fn mot_nghin_khoa_chuoi_va_dao_thu_tu() {
    let n = 1000;
    let a = m((0..n)
        .map(|i| (Value::Str(format!("khoa-{i}")), Value::Int(i as i64)))
        .collect());
    let b = m((0..n)
        .rev()
        .map(|i| (Value::Str(format!("khoa-{i}")), Value::Int(i as i64)))
        .collect());
    bang(&a, &b);

    let mut c = (0..n)
        .map(|i| (Value::Str(format!("khoa-{i}")), Value::Int(i as i64)))
        .collect::<Vec<_>>();
    c[n / 3].1 = Value::Int(-7);
    khac(&a, &m(c));
}

#[test]
fn trung_khoa_last_wins_hai_ve() {
    // Vị trí lần đầu, giá trị lần cuối — đúng `gop` cũ.
    let a = m(vec![
        (Value::Int(1), Value::Int(10)),
        (Value::Int(1), Value::Int(20)),
        (Value::Int(2), Value::Int(30)),
    ]);
    let b = m(vec![
        (Value::Int(2), Value::Int(30)),
        (Value::Int(1), Value::Int(20)),
    ]);
    bang(&a, &b);
    khac(
        &a,
        &m(vec![
            (Value::Int(1), Value::Int(10)),
            (Value::Int(2), Value::Int(30)),
        ]),
    );

    // Cả hai vế đều có entry trước khác nhau, entry cuối bằng nhau ⇒ vẫn bằng.
    let c = m(vec![
        (Value::Int(9), Value::Int(1)),
        (Value::Int(9), Value::Int(2)),
    ]);
    let d = m(vec![
        (Value::Int(9), Value::Int(99)),
        (Value::Int(9), Value::Int(2)),
    ]);
    bang(&c, &d);

    // Trùng khoá chéo kiểu: entry cuối quyết định cả khoá lẫn giá trị.
    let e = m(vec![
        (Value::Int(1), Value::Str("a".into())),
        (Value::Float(1.0), Value::Str("b".into())),
    ]);
    bang(&e, &m(vec![(Value::Float(1.0), Value::Str("b".into()))]));
    khac(&e, &m(vec![(Value::Int(1), Value::Str("a".into()))]));

    // 2000 entry mỗi vế (mỗi khoá hai lần, lần cuối thắng) gộp còn 1000, đảo thứ tự vẫn bằng.
    let mut f = Vec::with_capacity(2000);
    for i in 0..1000 {
        f.push((Value::Int(i), Value::Int(-1)));
        f.push((Value::Int(i), Value::Int(i)));
    }
    let mut g = Vec::with_capacity(2000);
    for i in (0..1000).rev() {
        g.push((Value::Int(i), Value::Int(-99)));
        g.push((Value::Int(i), Value::Int(i)));
    }
    bang(&m(f), &m(g));
}

#[test]
fn last_wins_khong_bac_cau_float_khop_nhu_cu() {
    // `eq_value` không bắc cầu: Int(1) == Float(1.0), Percent(1.0) == Float(1.0), nhưng
    // Int(1) != Percent(1.0). `gop` cũ thay entry **đầu tiên** bằng entry cuối, nên Float thay
    // Int (không thay Percent); phép so khoá+giá trị cũ được phép dùng lại cùng một entry b.
    let a = m(dem(vec![
        (Value::Int(1), Value::Int(1)),
        (Value::Percent(1.0), Value::Int(2)),
        (Value::Float(1.0), Value::Int(3)),
    ]));
    // Đúng ngữ nghĩa cũ: F(1)=3 khớp Int(1)=3, P(1)=2 khớp P(1)=2 ⇒ bằng.
    bang(
        &a,
        &m(dem(vec![
            (Value::Int(1), Value::Int(3)),
            (Value::Percent(1.0), Value::Int(2)),
        ])),
    );
    // Đúng ngữ nghĩa cũ: F(1)=3 khớp P(1)=3, nhưng P(1)=2 không khớp entry nào ⇒ khác.
    khac(
        &a,
        &m(dem(vec![
            (Value::Int(1), Value::Int(1)),
            (Value::Percent(1.0), Value::Int(3)),
        ])),
    );
}

#[test]
fn khoa_so_cheo_kieu_int_float_percent() {
    for n in [1, 1000] {
        let a = map_so(n, |i| Value::Int(i as i64));
        let b = map_so(n, |i| Value::Float(i as f64));
        bang(&a, &b);

        let c = map_so(n, |i| Value::Percent(i as f64));
        bang(&b, &c);
    }

    // Chỉ `Float` bắc cầu `Int`/`Percent`; `Int` với `Percent` **không** bằng nhau.
    khac(
        &co_dem(Value::Int(1), Value::Int(0)),
        &co_dem(Value::Percent(1.0), Value::Int(0)),
    );

    // `50pct == 0.5` áp cho cả khoá lẫn giá trị (đệm để chắc chắn đi đường nhanh).
    bang(
        &co_dem(Value::Percent(0.5), Value::Int(1)),
        &co_dem(Value::Float(0.5), Value::Int(1)),
    );
    bang(
        &co_dem(Value::Int(7), Value::Percent(0.5)),
        &co_dem(Value::Int(7), Value::Float(0.5)),
    );
}

#[test]
fn khong_am_va_nan() {
    // `-0.0 == +0.0` ⇒ phải cùng băm, kể cả khi khoá là Int(0).
    bang(
        &co_dem(Value::Float(-0.0), Value::Int(1)),
        &co_dem(Value::Float(0.0), Value::Int(1)),
    );
    bang(
        &co_dem(Value::Int(0), Value::Int(1)),
        &co_dem(Value::Float(-0.0), Value::Int(1)),
    );
    bang(
        &co_dem(Value::Percent(-0.0), Value::Int(1)),
        &co_dem(Value::Float(0.0), Value::Int(1)),
    );

    // NaN không bằng gì — hai map cùng khoá NaN vẫn **không** bằng (eq_value NaN ≠ NaN).
    let nan = f64::NAN;
    khac(
        &co_dem(Value::Float(nan), Value::Int(1)),
        &co_dem(Value::Float(nan), Value::Int(1)),
    );
    khac(
        &m(dem(vec![
            (Value::Float(nan), Value::Int(1)),
            (Value::Int(2), Value::Int(3)),
        ])),
        &m(dem(vec![
            (Value::Float(nan), Value::Int(1)),
            (Value::Int(2), Value::Int(3)),
        ])),
    );
    // NaN ở **giá trị** cũng không bằng (khoá vô hướng, đường nhanh so giá trị bằng eq_value).
    khac(
        &co_dem(Value::Int(1), Value::Float(nan)),
        &co_dem(Value::Int(1), Value::Float(nan)),
    );
}

#[test]
fn size_duration_str_path_bool_char_bytes_tach_tag() {
    let p = co_dem;
    // Size/Duration chỉ bằng cùng loại.
    bang(
        &p(Value::Size(10), Value::Int(1)),
        &p(Value::Size(10), Value::Int(1)),
    );
    bang(
        &p(Value::Duration(10), Value::Int(1)),
        &p(Value::Duration(10), Value::Int(1)),
    );
    khac(
        &p(Value::Size(10), Value::Int(1)),
        &p(Value::Duration(10), Value::Int(1)),
    );
    khac(
        &p(Value::Size(1), Value::Int(1)),
        &p(Value::Int(1), Value::Int(1)),
    );

    // Str/Path cùng nội dung vẫn khác tag.
    bang(
        &p(Value::Str("a".into()), Value::Int(1)),
        &p(Value::Str("a".into()), Value::Int(1)),
    );
    bang(
        &p(Value::Path("a".into()), Value::Int(1)),
        &p(Value::Path("a".into()), Value::Int(1)),
    );
    khac(
        &p(Value::Str("a".into()), Value::Int(1)),
        &p(Value::Path("a".into()), Value::Int(1)),
    );

    bang(
        &p(Value::Bool(true), Value::Int(1)),
        &p(Value::Bool(true), Value::Int(1)),
    );
    khac(
        &p(Value::Bool(true), Value::Int(1)),
        &p(Value::Bool(false), Value::Int(1)),
    );
    khac(
        &p(Value::Bool(true), Value::Int(1)),
        &p(Value::Int(1), Value::Int(1)),
    );

    bang(
        &p(Value::Char('đ'), Value::Int(1)),
        &p(Value::Char('đ'), Value::Int(1)),
    );
    khac(
        &p(Value::Char('a'), Value::Int(1)),
        &p(Value::Int(97), Value::Int(1)),
    );

    bang(
        &p(Value::Bytes(vec![0, 255]), Value::Int(1)),
        &p(Value::Bytes(vec![0, 255]), Value::Int(1)),
    );
    khac(
        &p(Value::Bytes(vec![97]), Value::Int(1)),
        &p(Value::Str("a".into()), Value::Int(1)),
    );
}

#[test]
fn gia_tri_khac_va_gia_tri_long() {
    let p = co_dem;
    khac(
        &p(Value::Int(1), Value::Int(2)),
        &p(Value::Int(1), Value::Int(3)),
    );
    // Giá trị cũng dùng eq_value: Int(1) == Float(1.0).
    bang(
        &p(Value::Int(1), Value::Int(1)),
        &p(Value::Int(1), Value::Float(1.0)),
    );
    let ds = || Value::list(vec![Value::Int(1), Value::Str("x".into())]);
    bang(
        &p(Value::Int(1), ds()),
        &p(
            Value::Int(1),
            Value::list(vec![Value::Int(1), Value::Str("x".into())]),
        ),
    );
    khac(
        &p(Value::Int(1), ds()),
        &p(
            Value::Int(1),
            Value::list(vec![Value::Int(1), Value::Str("y".into())]),
        ),
    );
    bang(
        &p(Value::Int(1), m(vec![(Value::Int(2), Value::Int(3))])),
        &p(Value::Int(1), m(vec![(Value::Int(2), Value::Int(3))])),
    );
}

#[test]
fn fallback_khoa_list_record_map_long_enum_unit_range() {
    // List.
    bang(
        &m(vec![(
            Value::list(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(9),
        )]),
        &m(vec![(
            Value::list(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(9),
        )]),
    );
    khac(
        &m(vec![(
            Value::list(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(9),
        )]),
        &m(vec![(
            Value::list(vec![Value::Int(1), Value::Int(3)]),
            Value::Int(9),
        )]),
    );

    // Record (đảo thứ tự trường vẫn bằng theo đa tập cũ).
    let r1 = Value::record(
        "U",
        vec![("a".into(), Value::Int(1)), ("b".into(), Value::Int(2))],
    );
    let r2 = Value::record(
        "U",
        vec![("b".into(), Value::Int(2)), ("a".into(), Value::Int(1))],
    );
    bang(
        &m(vec![(r1.clone(), Value::Int(9))]),
        &m(vec![(r2, Value::Int(9))]),
    );
    let r3 = Value::record(
        "U",
        vec![("a".into(), Value::Int(1)), ("b".into(), Value::Int(3))],
    );
    khac(&m(vec![(r1, Value::Int(9))]), &m(vec![(r3, Value::Int(9))]));

    // Map lồng làm khoá (đảo thứ tự entry của map con vẫn bằng).
    let k1 = m(vec![
        (Value::Int(1), Value::Int(10)),
        (Value::Int(2), Value::Int(20)),
    ]);
    let k2 = m(vec![
        (Value::Int(2), Value::Int(20)),
        (Value::Int(1), Value::Int(10)),
    ]);
    bang(
        &m(vec![(k1.clone(), Value::Int(9))]),
        &m(vec![(k2, Value::Int(9))]),
    );

    // Trùng khoá phức tạp: last-wins vẫn đúng ở fallback.
    let dup = m(vec![
        (Value::list(vec![Value::Int(1)]), Value::Int(10)),
        (Value::list(vec![Value::Int(1)]), Value::Int(20)),
    ]);
    bang(
        &dup,
        &m(vec![(Value::list(vec![Value::Int(1)]), Value::Int(20))]),
    );
    khac(
        &dup,
        &m(vec![(Value::list(vec![Value::Int(1)]), Value::Int(10))]),
    );

    // Enum / Unit / Range.
    bang(
        &m(vec![(Value::some(Value::Int(1)), Value::Int(9))]),
        &m(vec![(Value::some(Value::Int(1)), Value::Int(9))]),
    );
    khac(
        &m(vec![(Value::some(Value::Int(1)), Value::Int(9))]),
        &m(vec![(Value::some(Value::Int(2)), Value::Int(9))]),
    );
    bang(
        &m(vec![(Value::Unit, Value::Int(9))]),
        &m(vec![(Value::Unit, Value::Int(9))]),
    );
    // Range không có arm trong `eq_value` ⇒ hai khoá Range giống hệt vẫn **không** bằng
    // (giữ nguyên ngữ nghĩa cũ; fallback không được "sửa" hành vi này).
    khac(
        &m(vec![(Value::Range(1, 5, false), Value::Int(9))]),
        &m(vec![(Value::Range(1, 5, false), Value::Int(9))]),
    );
    khac(
        &m(vec![(Value::Range(1, 5, false), Value::Int(9))]),
        &m(vec![(Value::Range(1, 5, true), Value::Int(9))]),
    );

    // Trộn khoá vô hướng + phức tạp, đảo thứ tự toàn map.
    let hon = m(vec![
        (Value::Int(1), Value::Int(10)),
        (Value::list(vec![Value::Int(2)]), Value::Int(20)),
    ]);
    let hon_dao = m(vec![
        (Value::list(vec![Value::Int(2)]), Value::Int(20)),
        (Value::Int(1), Value::Int(10)),
    ]);
    bang(&hon, &hon_dao);

    // Vượt ngưỡng đường nhanh nhưng có **một** khoá list ⇒ vẫn fallback nguyên vẹn.
    let mut lon = (0..200)
        .map(|i| (Value::Int(i), Value::Int(i)))
        .collect::<Vec<_>>();
    lon.push((Value::list(vec![Value::Int(1)]), Value::Int(9)));
    let mut lon_dao = lon.iter().rev().cloned().collect::<Vec<_>>();
    bang(&m(lon.clone()), &m(lon_dao.clone()));
    lon_dao[0].0 = Value::list(vec![Value::Int(3)]);
    khac(&m(lon), &m(lon_dao));
}
