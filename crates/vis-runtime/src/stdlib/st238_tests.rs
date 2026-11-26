// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a** — test R1 tách khỏi `stdlib.rs` (nguyên văn).

use super::*;
use vis_core::reactor::NET_READ_CAP;

#[test]
fn net_read_ky_so_vao_khong_duoc_cap_phat() {
    assert!(gioi_han_net_read(-1).is_err(), "âm bị từ chối");
    assert!(gioi_han_net_read(i64::MAX).is_err(), "i64::MAX bị từ chối");
    assert!(
        gioi_han_net_read(NET_READ_CAP + 1).is_err(),
        "vượt trần bị từ chối"
    );
    assert_eq!(
        gioi_han_net_read(NET_READ_CAP).unwrap(),
        NET_READ_CAP as usize
    );
    assert_eq!(gioi_han_net_read(0).unwrap(), 0);
    assert_eq!(gioi_han_net_read(1024).unwrap(), 1024);
}

#[test]
fn json_non_finite_bi_phat_hien() {
    assert!(vis_core::json::co_so_phi_huu_han(&Value::Float(f64::NAN)));
    assert!(vis_core::json::co_so_phi_huu_han(&Value::Float(
        f64::INFINITY
    )));
    assert!(vis_core::json::co_so_phi_huu_han(&Value::Percent(
        f64::NEG_INFINITY
    )));
    assert!(vis_core::json::co_so_phi_huu_han(&Value::list(vec![
        Value::Int(1),
        Value::Float(f64::NAN)
    ])));
    assert!(!vis_core::json::co_so_phi_huu_han(&Value::Float(1.5)));
    assert!(!vis_core::json::co_so_phi_huu_han(&Value::Int(1)));
}

// ------------------------------------------------------------------
// VD-258 — `json.encode` giữ dấu float cho **cả** `Json.Num(Float)`.
// ------------------------------------------------------------------

fn json_num(f: f64) -> Value {
    Value::enum_val("Json", "Num", vec![Value::Float(f)])
}

fn ma_hoa(v: &Value) -> String {
    let mut s = String::new();
    json(v, false, &mut s);
    s
}

#[test]
fn json_num_float_giu_dau_va_roundtrip() {
    // 1.0 không được in "1" (decoder sẽ thành Int) — cả cây thường lẫn `Json.Num`.
    assert_eq!(ma_hoa(&json_num(1.0)), "1.0");
    assert_eq!(ma_hoa(&json_num(1.5)), "1.5");
    for f in [1.0_f64, 1.5, 1e20, 1e300, f64::MAX, 5e-324, 1e-320] {
        let s = ma_hoa(&json_num(f));
        let v = vis_core::json::decode(&s)
            .unwrap_or_else(|e| panic!("encode({f}) = {s:?} không decode lại được: {e}"));
        match v {
            Value::Float(g) => {
                assert_eq!(g.to_bits(), f.to_bits(), "roundtrip {f} qua {s:?} cho {g}")
            }
            other => panic!("encode({f}) = {s:?} decode thành {other:?}, không phải Float"),
        }
    }
    // 1e20 trước đây in `100000000000000000000` (không dấu) ⇒ decode thành int tràn i64.
    let s = ma_hoa(&json_num(1e20));
    assert!(
        s.contains('.') || s.contains('e') || s.contains('E'),
        "phải giữ dấu float: {s}"
    );
}
