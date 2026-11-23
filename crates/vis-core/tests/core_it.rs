// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — bộ kiểm thử tích hợp hợp đồng lõi của `vis-core`.
//!
//! Khẳng định bề mặt công khai tầng đáy: `Value` (so sánh/hiển thị/parse kích thước & thời
//! lượng), `json` (một nguồn `esc` duy nhất đóng `#112` + roundtrip), `CookieJar`, và
//! định vị `pos`/`Trap`.

use vis_core::http_client::CookieJar;
use vis_core::json::{decode, esc, json};
use vis_core::value::{eq, parse_duration, parse_size, show, Value};
use vis_core::{pos, Trap};
use vis_syntax::Span;

#[test]
fn json_roundtrip_va_escape_duy_nhat() {
    let mau = "xin chào \"ViScript\" \\ dòng1\ndòng2\t\x08\x0c\r\x01";
    let da_esc = esc(mau);
    let decoded = decode(&da_esc).expect("json::esc phải giải mã được bằng json::decode");
    assert!(eq(&decoded, &Value::Str(mau.to_string())));

    let obj = Value::Map(vec![
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
    assert_eq!(parse_size("5kb"), Some(5 * 1024));
    assert_eq!(parse_size("2mb"), Some(2 * 1024 * 1024));
    assert_eq!(parse_size("khong-hop-le"), None);

    assert_eq!(parse_duration("250ms"), Some(250));
    assert_eq!(parse_duration("3s"), Some(3_000));
    assert_eq!(parse_duration("sai"), None);

    assert_eq!(vis_core::exit_code(&Value::Unit), 0);
    assert_eq!(vis_core::exit_code(&Value::ok(Value::Int(1))), 0);
    assert_eq!(
        vis_core::exit_code(&Value::err(Value::Str("lỗi".into()))),
        1
    );
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
