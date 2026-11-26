// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — bộ kiểm thử tích hợp hợp đồng lõi của `vis-core`.
//!
//! Khẳng định bề mặt công khai tầng đáy: `Value` (so sánh/hiển thị/parse kích thước & thời
//! lượng), `json` (một nguồn `esc` duy nhất đóng `#112` + roundtrip), `CookieJar`, và
//! định vị `pos`/`Trap`.

use vis_core::cookie::CookieJar;
use vis_core::json::{decode, esc, json};
use vis_core::value::{eq, parse_duration, parse_size, show, Value};
use vis_core::{pos, Trap};
use vis_syntax::Span;

#[test]
fn json_roundtrip_va_escape_duy_nhat() {
    let mau = "xin chào \"ViScript\" \\ dòng1\ndòng2\t\x08\x0c\r\x01";
    // `esc` trả **nội dung** trong dấu `"` (không thêm dấu bao) — bọc dấu `"` để `decode` nhận
    // đúng một giá trị chuỗi JSON hoàn chỉnh (ST-311b: vá test lệch hợp đồng `esc`/`decode`).
    let da_esc = format!("\"{}\"", esc(mau));
    let decoded = decode(&da_esc).expect("json::esc phải giải mã được bằng json::decode");
    assert!(eq(&decoded, &Value::Str(mau.to_string())));

    let obj = Value::map(vec![
        (Value::Str("ten".into()), Value::Str(mau.into())),
        (Value::Str("so".into()), Value::Int(42)),
        (Value::Str("ok".into()), Value::Bool(true)),
        (
            Value::Str("ds".into()),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::none()]),
        ),
    ]);
    let mut encoded = String::new();
    json(&obj, false, &mut encoded);
    let back = decode(&encoded).expect("roundtrip JSON phải thành công");
    assert_eq!(show(&back), show(&obj));
}

#[test]
fn value_parse_size_duration_va_exit_code() {
    // **ST-311b (vá test ST-311c lệch hợp đồng)**: `parse_size` phân biệt SI (`kb`=1000) và IEC
    // (`kib`=1024); `parse_duration` trả **nanosecond**; chuỗi không có cặp `<số><đơn vị>` ⇒ `None` (#196).
    assert_eq!(parse_size("5kb"), Some(5 * 1000));
    assert_eq!(parse_size("5kib"), Some(5 * 1024));
    assert_eq!(parse_size("2mb"), Some(2 * 1_000_000));
    assert_eq!(parse_size("khong-hop-le"), None);

    assert_eq!(parse_duration("250ms"), Some(250_000_000));
    assert_eq!(parse_duration("3s"), Some(3_000_000_000));
    assert_eq!(parse_duration("sai"), None);
    assert_eq!(parse_duration(""), None);
    assert_eq!(parse_duration("250"), None);

    assert_eq!(vis_core::exit_code(&Value::Unit), 0);
    assert_eq!(vis_core::exit_code(&Value::ok(Value::Int(1))), 0);
    assert_eq!(
        vis_core::exit_code(&Value::err(Value::Str("lỗi".into()))),
        1
    );
}

#[test]
fn value_kich_thuoc_va_map_cow() {
    // **ST-315b (VD-291 §3.2 / VD-03)**: khoá kích thước `Value` — `Map` chỉ giữ một `Rc`
    // (8 B), không nhúng `Vec` (24 B) vào biến thể.
    assert!(
        std::mem::size_of::<Value>() <= 64,
        "Value phình: {} B",
        std::mem::size_of::<Value>()
    );
    #[cfg(target_pointer_width = "64")]
    assert_eq!(
        std::mem::size_of::<Value>(),
        64,
        "ST-315b: trên 64-bit Value phải đúng 64 B (VD-03)"
    );

    // CoW: `Value::clone` chỉ tăng refcount; ghi qua `Rc::make_mut` không đụng bản gốc.
    let a = Value::map(vec![
        (Value::Str("a".into()), Value::Int(1)),
        (Value::Str("b".into()), Value::Int(2)),
    ]);
    let b = a.clone();
    match (&a, &b) {
        (Value::Map(ra), Value::Map(rb)) => assert!(std::rc::Rc::ptr_eq(ra, rb)),
        _ => panic!("phải là map"),
    }
    let mut bm = b;
    if let Value::Map(rc) = &mut bm {
        std::rc::Rc::make_mut(rc).insert(Value::Str("a".into()), Value::Int(9));
    }
    assert_eq!(show(&a), "{\"a\": 1, \"b\": 2}");
    assert_eq!(show(&bm), "{\"a\": 9, \"b\": 2}");
}

#[test]
fn cookie_jar_va_trap_pos() {
    let mut jar = CookieJar::new();
    jar.nhan_set_cookie(
        "example.com",
        "/api/v1",
        &["sid=abc123; Path=/api; Secure".into()],
    );
    let hdr = jar.header("example.com", "/api/v1/users", true);
    assert_eq!(hdr.as_deref(), Some("sid=abc123"));
    // Secure cookie không gửi qua HTTP thường.
    assert_eq!(jar.header("example.com", "/api/v1/users", false), None);

    let text = "dong 1\ndong 2";
    let (line, col) = pos(text, Span::new(7, 11));
    assert_eq!((line, col), (2, 1));
    let t = Trap::new(&vis_diag::RUNTIME_CODES[0], "mẫu lỗi", Span::new(0, 3));
    assert_eq!(t.code.short, "VS0501");
}
