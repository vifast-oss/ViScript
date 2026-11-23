// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Snapshot `Show`** (VD-47 #2/#3) — khoá **đúng chuỗi** của `str(x)` cho mọi dạng giá trị.
//!
//! Đây là test **khoá** (không phải đỏ-trước): Agent 0 chốt các dạng ở VD-47; đổi bất kỳ dòng nào
//! dưới đây là **thay đổi ngữ nghĩa** ⇒ phải có VD mới.

use vis_interp::Value;

fn f(v: Value) -> String {
    v.show()
}

#[test]
fn snapshot_show_scalar() {
    assert_eq!(f(Value::Int(-7)), "-7");
    assert_eq!(f(Value::Int(i64::MIN)), "-9223372036854775808");
    assert_eq!(f(Value::Float(1.5)), "1.5");
    assert_eq!(f(Value::Float(1.0)), "1");
    assert_eq!(f(Value::Float(f64::NAN)), "NaN");
    assert_eq!(f(Value::Bool(true)), "true");
    assert_eq!(f(Value::Char('ố')), "'ố'");
    assert_eq!(f(Value::Str("héllo".into())), "héllo");
    assert_eq!(f(Value::Path("./a.txt".into())), "./a.txt");
    assert_eq!(f(Value::Bytes(vec![b'a', 0x00, 0xff])), "b\"a\\x00\\xff\"");
    assert_eq!(f(Value::Unit), "()");
}

/// `size`: **R10 (VD-178/VD-267)** — thang thập phân, đơn vị lớn nhất có phần nguyên ≥ 1,
/// tối đa 2 chữ số lẻ (**làm tròn nửa lên**), bỏ 0 thừa; âm giữ dấu; `0` ⇒ `0b`.
#[test]
fn snapshot_show_size() {
    assert_eq!(f(Value::Size(0)), "0b");
    assert_eq!(f(Value::Size(999)), "999b");
    assert_eq!(f(Value::Size(1000)), "1kb");
    assert_eq!(f(Value::Size(1024)), "1.02kb");
    assert_eq!(f(Value::Size(1200)), "1.2kb");
    assert_eq!(f(Value::Size(1234)), "1.23kb");
    assert_eq!(f(Value::Size(1235)), "1.24kb");
    assert_eq!(f(Value::Size(1500)), "1.5kb");
    assert_eq!(f(Value::Size(5000)), "5kb");
    assert_eq!(f(Value::Size(999_994)), "999.99kb");
    assert_eq!(f(Value::Size(999_995)), "1000kb");
    assert_eq!(f(Value::Size(999_999)), "1000kb");
    assert_eq!(f(Value::Size(1_000_000)), "1mb");
    assert_eq!(f(Value::Size(1_500_000)), "1.5mb");
    assert_eq!(f(Value::Size(5_000_000)), "5mb");
    assert_eq!(f(Value::Size(1_000_000_000)), "1gb");
    assert_eq!(f(Value::Size(1_005_000_000)), "1.01gb");
    assert_eq!(f(Value::Size(1_234_567_890)), "1.23gb");
    assert_eq!(f(Value::Size(999_999_999_999)), "1000gb");
    assert_eq!(f(Value::Size(1_000_000_000_000)), "1tb");
    assert_eq!(f(Value::Size(1_234_567_890_123)), "1.23tb");
    // Biên `i64`: dùng `unsigned_abs`, không tràn với `i64::MIN`.
    assert_eq!(f(Value::Size(i64::MAX)), "9223372.04tb");
    assert_eq!(f(Value::Size(i64::MIN)), "-9223372.04tb");
    // Âm: chọn đơn vị theo **độ lớn**, giữ dấu.
    assert_eq!(f(Value::Size(-999)), "-999b");
    assert_eq!(f(Value::Size(-1000)), "-1kb");
    assert_eq!(f(Value::Size(-1500)), "-1.5kb");
}

/// `duration`: **R10 (VD-178/VD-267)** — ghép từ lớn đến nhỏ, bỏ thành phần 0, không cách;
/// phân rã nguyên chính xác; âm giữ dấu; `0` ⇒ `0ns`.
#[test]
fn snapshot_show_duration() {
    assert_eq!(f(Value::Duration(0)), "0ns");
    assert_eq!(f(Value::Duration(1)), "1ns");
    assert_eq!(f(Value::Duration(999)), "999ns");
    assert_eq!(f(Value::Duration(1_000)), "1us");
    assert_eq!(f(Value::Duration(1_500)), "1us500ns");
    assert_eq!(f(Value::Duration(1_000_000)), "1ms");
    assert_eq!(f(Value::Duration(150_000_000)), "150ms");
    assert_eq!(f(Value::Duration(250_000_000)), "250ms");
    assert_eq!(f(Value::Duration(1_000_000_000)), "1s");
    assert_eq!(f(Value::Duration(1_500_000_000)), "1s500ms");
    assert_eq!(f(Value::Duration(1_000_000_001)), "1s1ns");
    assert_eq!(f(Value::Duration(90_000_000_000)), "1m30s");
    assert_eq!(f(Value::Duration(5_400_000_000_000)), "1h30m");
    assert_eq!(f(Value::Duration(86_400_000_000_000)), "1d");
    assert_eq!(f(Value::Duration(604_800_000_000_000)), "1w");
    assert_eq!(f(Value::Duration(691_200_000_000_000)), "1w1d");
    // Mốc epoch của `mtime` (VD-267): trước in `1700000000s`, nay ghép đầy đủ.
    assert_eq!(
        f(Value::Duration(1_700_000_000_000_000_000)),
        "2810w5d22h13m20s"
    );
    // Biên `i64`: dùng `unsigned_abs`, không tràn với `i64::MIN`.
    assert_eq!(
        f(Value::Duration(i64::MAX)),
        "15250w1d23h47m16s854ms775us807ns"
    );
    assert_eq!(
        f(Value::Duration(i64::MIN)),
        "-15250w1d23h47m16s854ms775us808ns"
    );
    assert_eq!(f(Value::Duration(-1_500_000_000)), "-1s500ms");
}

/// `percent`: `ratio*100` + `pct`, bỏ `.0` (VD-47 #2).
#[test]
fn snapshot_show_percent() {
    assert_eq!(f(Value::Percent(0.5)), "50pct");
    assert_eq!(f(Value::Percent(0.125)), "12.5pct");
    assert_eq!(f(Value::Percent(1.0)), "100pct");
}

/// Tập hợp + kiểu người dùng (VD-47 #3).
#[test]
fn snapshot_show_hop_va_kieu_nguoi_dung() {
    assert_eq!(f(Value::List(vec![Value::Int(1), Value::Int(2)])), "[1, 2]");
    assert_eq!(
        f(Value::Map(vec![
            (Value::Str("a".into()), Value::Int(1)),
            (Value::Str("b".into()), Value::Int(2)),
        ])),
        "{\"a\": 1, \"b\": 2}"
    );
    assert_eq!(
        f(Value::Record(
            "User".into(),
            vec![
                ("name".into(), Value::Str("An".into())),
                ("age".into(), Value::Int(20)),
            ]
        )),
        "User(name: An, age: 20)"
    );
    assert_eq!(
        f(Value::Enum(
            "Shape".into(),
            "Circle".into(),
            vec![Value::Float(2.0)]
        )),
        "Circle(2)"
    );
    assert_eq!(
        f(Value::Enum("Shape".into(), "Empty".into(), vec![])),
        "Empty"
    );
    // **VD-204 (1)**: `Option`/`Result` là enum std — hiển thị qua cùng luật `Enum`.
    assert_eq!(f(Value::none()), "None");
    assert_eq!(f(Value::some(Value::Int(5))), "Some(5)");
    assert_eq!(f(Value::ok(Value::Int(1))), "Ok(1)");
    assert_eq!(f(Value::err(Value::Str("âm".into()))), "Err(âm)");
}
