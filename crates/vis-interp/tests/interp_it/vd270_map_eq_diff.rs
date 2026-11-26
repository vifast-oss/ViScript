// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (perf-map-eq)** — đối chiếu **ngẫu nhiên tất định** (LCG, seed cố định) đường nhanh
//! của `Value::Map` với **logic O(n²) cũ** trên cùng dữ liệu: khoá vô hướng lẫn phức tạp, trùng
//! khoá, `NaN`, `±0.0`, `Int`/`Float`/`Percent` không bắc cầu, map đảo thứ tự. Mỗi map được
//! **đệm** 120 khoá vô hướng để chắc chắn vượt ngưỡng đường nhanh (≥ 200 entry hai vế).
use vis_core::value::{eq, Value};

/// Nguyên văn `gop` + phép so map cũ (oracle độc lập với `value.rs`).
fn gop(xs: &[(Value, Value)]) -> Vec<&(Value, Value)> {
    let mut out: Vec<&(Value, Value)> = Vec::with_capacity(xs.len());
    for e in xs {
        if let Some(prev) = out.iter_mut().find(|p| p.0.eq_value(&e.0)) {
            *prev = e;
        } else {
            out.push(e);
        }
    }
    out
}

fn eq_map_cu(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Map(a), Value::Map(b)) => {
            let (a, b) = (gop(a), gop(b));
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.iter().any(|(k2, v2)| k.eq_value(k2) && v.eq_value(v2)))
        }
        _ => a.eq_value(b),
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn khoa_mau() -> Vec<Value> {
    vec![
        Value::Int(0),
        Value::Int(1),
        Value::Int(2),
        Value::Float(0.0),
        Value::Float(-0.0),
        Value::Float(1.0),
        Value::Float(f64::NAN),
        Value::Percent(0.5),
        Value::Percent(1.0),
        Value::Size(1),
        Value::Duration(1),
        Value::Str("a".into()),
        Value::Str("b".into()),
        Value::Path("a".into()),
        Value::Bool(true),
        Value::Bool(false),
        Value::Char('a'),
        Value::Bytes(vec![97]),
        Value::Unit,
        Value::list(vec![Value::Int(1)]),
        Value::list(vec![Value::Int(2)]),
        Value::record("U", vec![("a".into(), Value::Int(1))]),
        Value::record("U", vec![("a".into(), Value::Int(2))]),
        Value::Range(1, 3, false),
        Value::some(Value::Int(1)),
        Value::some(Value::Int(2)),
    ]
}

fn gia_tri_mau() -> Vec<Value> {
    vec![
        Value::Int(0),
        Value::Int(1),
        Value::Int(2),
        Value::Float(1.0),
        Value::Percent(0.5),
        Value::Str("x".into()),
        Value::Bytes(vec![1, 2]),
        Value::list(vec![Value::Int(1), Value::Int(2)]),
        Value::map(vec![(Value::Int(1), Value::Int(2))]),
        Value::record("R", vec![("f".into(), Value::Int(1))]),
        Value::Unit,
        Value::Bool(true),
        Value::Char('z'),
    ]
}

/// Đệm 120 khoá Int phân biệt (`10_000 + i`) — cùng bộ cho cả hai vế.
fn dem(mut kv: Vec<(Value, Value)>) -> Vec<(Value, Value)> {
    for i in 0..120 {
        kv.push((Value::Int(10_000 + i), Value::Int(7)));
    }
    kv
}

fn tao_map(lcg: &mut Lcg, ks: &[Value], vs: &[Value]) -> Value {
    let n = 1 + lcg.pick(6);
    let kv = (0..n)
        .map(|_| {
            (
                ks[lcg.pick(ks.len())].clone(),
                vs[lcg.pick(vs.len())].clone(),
            )
        })
        .collect();
    Value::map(dem(kv))
}

#[test]
fn duong_nhanh_khop_logic_cu_tren_5000_ca_ngau_nhien() {
    let ks = khoa_mau();
    let vs = gia_tri_mau();
    let mut lcg = Lcg(0x5eed_0270);
    let mut so_bang = 0usize;
    for _ in 0..5000 {
        let a = tao_map(&mut lcg, &ks, &vs);
        let mut b = if lcg.next().is_multiple_of(3) {
            a.clone()
        } else {
            tao_map(&mut lcg, &ks, &vs)
        };
        // Đảo thứ tự entry của một vế: ánh xạ không đổi, kết quả phải không đổi.
        if lcg.next().is_multiple_of(2) {
            if let Value::Map(kv) = &mut b {
                let mut entries = kv.as_slice().to_vec();
                entries.reverse();
                *kv = std::rc::Rc::new(vis_core::value::VisMap::from_pairs(entries));
            }
        }
        let cu = eq_map_cu(&a, &b);
        let moi = eq(&a, &b);
        assert_eq!(cu, moi, "lệch ngữ nghĩa: {a:?} vs {b:?}");
        assert_eq!(moi, eq(&b, &a), "không đối xứng: {a:?} vs {b:?}");
        if cu {
            so_bang += 1;
        }
    }
    assert!(so_bang > 0, "phải có ca bằng");
    assert!(so_bang < 5000, "phải có ca khác");
    println!(
        "5000 ca khớp logic cũ ({so_bang} bằng / {} khác)",
        5000 - so_bang
    );
}
