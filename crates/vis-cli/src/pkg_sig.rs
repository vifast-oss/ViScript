// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5b (VD-271)** — chữ ký **Ed25519** cho registry HTTP của `vis pkg`.
//!
//! Registry cục bộ **không** bắt buộc chữ ký. Với registry HTTP, người dùng cấu hình **một hoặc
//! nhiều khoá công khai tin cậy** (`--registry-key <hex>` lặp lại / `VIS_REGISTRY_KEY` danh sách
//! phân tách phẩy hoặc khoảng trắng); khi có khoá, mọi phiên bản được chọn **phải** có
//! `signature_ed25519` khớp **một trong** các khoá, nếu không ⇒ từ chối cài (trước khi tải tarball).
//! Nhiều khoá hỗ trợ **key rotation**: khoá cũ vẫn verify bản cũ, khoá mới verify bản mới.
//! Mọi lỗi cấu hình khoá/xác minh chữ ký được báo bằng mã **`VS0708`** (ST-288/VD-273).
//!
//! # Thông điệp được ký (hợp đồng `vispkg-sig1`)
//!
//! Ký **byte UTF-8** của compact JSON do [`thong_diep_ky`] sinh, đúng thứ tự khoá:
//!
//! ```text
//! {"deps":[["<tên>","<ràng buộc>"],…],"entry":"…","files":[["<path>","<sha256>"],…],
//!  "format":"vispkg-sig1","name":"…","tarball_sha256":"…","version":"…"}
//! ```
//!
//! `deps` sắp theo `(tên, ràng buộc)`, `files` sắp theo `(path, sha256)` — tất định, không phụ thuộc
//! thứ tự trong index. Nhờ vậy chữ ký phủ **cả metadata** (entry/deps/files), không chỉ tarball.
//!
//! `ring` đã có trong cây (transitive của `rustls`, VS3-4a); feature `pkg-sig` chỉ bật trên nền không
//! phải wasm32. Khi tắt feature, [`xac_minh`] trả lỗi rõ — **không** bỏ qua kiểm tra.

use vis_interp::json::json;
use vis_interp::value::Value;

/// Giải chuỗi hex **đúng** `so_byte` byte; chấp nhận hoa/thường, bỏ khoảng trắng hai đầu.
pub fn hex_giai(s: &str, so_byte: usize) -> Result<Vec<u8>, String> {
    let s = s.trim();
    let can = so_byte.saturating_mul(2);
    if s.len() != can {
        return Err(format!(
            "cần {can} chữ số hex ({so_byte} byte), nhận {}",
            s.len()
        ));
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(so_byte);
    for i in (0..b.len()).step_by(2) {
        let hi = gia_tri(b[i])
            .ok_or_else(|| format!("ký tự hex lạ `{}` tại byte {i}", char::from(b[i])))?;
        let lo = gia_tri(b[i + 1])
            .ok_or_else(|| format!("ký tự hex lạ `{}` tại byte {}", char::from(b[i + 1]), i + 1))?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

/// Mã hoá byte thành hex thường (dùng cho test/chẩn đoán).
pub fn hex_ma(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn gia_tri(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Sinh **thông điệp chuẩn** để ký/kiểm (hợp đồng `vispkg-sig1` — xem module docs).
pub fn thong_diep_ky(
    ten: &str,
    version: &str,
    tar_sha: &str,
    entry: &str,
    deps: &[(String, String)],
    files: &[(String, String)],
) -> Vec<u8> {
    let mut ds = deps.to_vec();
    ds.sort();
    let mut fs = files.to_vec();
    fs.sort();
    let cap = |ds: Vec<(String, String)>| {
        Value::List(
            ds.into_iter()
                .map(|(a, b)| Value::List(vec![Value::Str(a), Value::Str(b)]))
                .collect(),
        )
    };
    let v = Value::Map(vec![
        (Value::Str("deps".into()), cap(ds)),
        (Value::Str("entry".into()), Value::Str(entry.into())),
        (Value::Str("files".into()), cap(fs)),
        (
            Value::Str("format".into()),
            Value::Str("vispkg-sig1".into()),
        ),
        (Value::Str("name".into()), Value::Str(ten.into())),
        (
            Value::Str("tarball_sha256".into()),
            Value::Str(tar_sha.into()),
        ),
        (Value::Str("version".into()), Value::Str(version.into())),
    ]);
    let mut s = String::new();
    json(&v, false, &mut s);
    s.into_bytes()
}

/// Xác minh chữ ký Ed25519 trên `thong_diep`; lỗi ⇒ thông điệp rõ (không panic).
#[cfg(all(feature = "pkg-sig", not(target_arch = "wasm32")))]
pub fn xac_minh(khoa: &[u8; 32], thong_diep: &[u8], chu_ky: &[u8]) -> Result<(), String> {
    use ring::signature::{UnparsedPublicKey, ED25519};
    if chu_ky.len() != 64 {
        return Err(format!(
            "chữ ký Ed25519 phải 64 byte, nhận {}",
            chu_ky.len()
        ));
    }
    UnparsedPublicKey::new(&ED25519, khoa.as_slice())
        .verify(thong_diep, chu_ky)
        .map_err(|_| "chữ ký Ed25519 không hợp lệ".to_string())
}

/// **VS3-5c**: xác minh chữ ký khớp **một trong** nhiều khoá công khai (key rotation).
///
/// Khoá cũ vẫn verify được bản cũ; khoá mới verify bản mới; chỉ cần **một** khoá khớp. Danh sách
/// rỗng hoặc chữ ký sai độ dài ⇒ lỗi rõ; không im lặng bỏ qua.
#[cfg(all(feature = "pkg-sig", not(target_arch = "wasm32")))]
pub fn xac_minh_nhieu(khoas: &[[u8; 32]], thong_diep: &[u8], chu_ky: &[u8]) -> Result<(), String> {
    if khoas.is_empty() {
        return Err("không có khoá công khai nào để xác minh chữ ký".to_string());
    }
    if chu_ky.len() != 64 {
        return Err(format!(
            "chữ ký Ed25519 phải 64 byte, nhận {}",
            chu_ky.len()
        ));
    }
    for khoa in khoas {
        if xac_minh(khoa, thong_diep, chu_ky).is_ok() {
            return Ok(());
        }
    }
    Err(format!(
        "chữ ký Ed25519 không khớp bất kỳ khoá công khai nào trong {} khoá",
        khoas.len()
    ))
}

/// Bản build không có `pkg-sig` (hoặc wasm32): **không** bỏ qua kiểm tra — báo lỗi rõ.
#[cfg(not(all(feature = "pkg-sig", not(target_arch = "wasm32"))))]
pub fn xac_minh_nhieu(
    _khoas: &[[u8; 32]],
    _thong_diep: &[u8],
    _chu_ky: &[u8],
) -> Result<(), String> {
    Err("xác minh chữ ký Ed25519 cần feature `pkg-sig` và nền không phải wasm32".to_string())
}

/// Bản build không có `pkg-sig` (hoặc wasm32): **không** bỏ qua kiểm tra — báo lỗi rõ.
#[cfg(not(all(feature = "pkg-sig", not(target_arch = "wasm32"))))]
pub fn xac_minh(_khoa: &[u8; 32], _thong_diep: &[u8], _chu_ky: &[u8]) -> Result<(), String> {
    Err("xác minh chữ ký Ed25519 cần feature `pkg-sig` và nền không phải wasm32".to_string())
}

#[cfg(test)]
mod tests {
    use super::{hex_giai, hex_ma, thong_diep_ky};

    /// Hợp đồng `vispkg-sig1`: pin **từng byte** — đổi thứ tự/khoá là đổi chữ ký của mọi registry.
    #[test]
    fn thong_diep_pin_va_sap_xep() {
        let sha = "a".repeat(64);
        let f1 = "1".repeat(64);
        let f2 = "2".repeat(64);
        let msg = thong_diep_ky(
            "util",
            "0.1.0",
            &sha,
            "src/util.vis",
            &[("z".into(), "^1".into()), ("a".into(), "~2".into())],
            &[("b.vis".into(), f2.clone()), ("a.vis".into(), f1.clone())],
        );
        let mong = format!(
            "{{\"deps\":[[\"a\",\"~2\"],[\"z\",\"^1\"]],\"entry\":\"src/util.vis\",\"files\":[[\"a.vis\",\"{f1}\"],[\"b.vis\",\"{f2}\"]],\"format\":\"vispkg-sig1\",\"name\":\"util\",\"tarball_sha256\":\"{sha}\",\"version\":\"0.1.0\"}}"
        );
        assert_eq!(String::from_utf8(msg).unwrap(), mong);
    }

    #[test]
    fn hex_dung_va_tu_choi_sai() {
        assert_eq!(hex_giai("00ff10", 3).unwrap(), vec![0, 0xff, 0x10]);
        assert_eq!(hex_ma(&[0, 0xff, 0x10]), "00ff10");
        assert!(hex_giai("00ff", 3).unwrap_err().contains("cần 6"));
        assert!(hex_giai("00fg", 2).unwrap_err().contains("ký tự hex lạ"));
    }

    /// Build **không** có `pkg-sig`: không được im lặng bỏ qua — phải báo lỗi rõ.
    #[cfg(not(all(feature = "pkg-sig", not(target_arch = "wasm32"))))]
    #[test]
    fn thieu_feature_thi_bao_loi_ro() {
        let e = super::xac_minh(&[0u8; 32], b"x", &[0u8; 64]).unwrap_err();
        assert!(e.contains("pkg-sig"), "{e}");
    }

    #[cfg(all(feature = "pkg-sig", not(target_arch = "wasm32")))]
    #[test]
    fn ky_va_xac_minh_that() {
        use ring::rand::SystemRandom;
        use ring::signature::{Ed25519KeyPair, KeyPair};

        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("sinh khoá");
        let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("đọc khoá");
        let khoa: [u8; 32] = kp.public_key().as_ref().try_into().expect("32 byte");
        let msg = b"vispkg-sig1\nthu nghiem";
        let sig = kp.sign(msg);
        assert_eq!(super::xac_minh(&khoa, msg, sig.as_ref()), Ok(()));
        assert!(super::xac_minh(&khoa, b"thong diep khac", sig.as_ref()).is_err());
        let mut hong = sig.as_ref().to_vec();
        hong[0] ^= 1;
        assert!(super::xac_minh(&khoa, msg, &hong).is_err());
        assert!(super::xac_minh(&khoa, msg, &sig.as_ref()[..63]).is_err());
    }

    #[cfg(all(feature = "pkg-sig", not(target_arch = "wasm32")))]
    #[test]
    fn nhieu_khoa_chi_can_mot_khoa_khop() {
        use ring::rand::SystemRandom;
        use ring::signature::{Ed25519KeyPair, KeyPair};

        let rng = SystemRandom::new();
        let kp_cu = Ed25519KeyPair::from_pkcs8(
            Ed25519KeyPair::generate_pkcs8(&rng)
                .expect("sinh khoá cũ")
                .as_ref(),
        )
        .expect("đọc khoá cũ");
        let kp_moi = Ed25519KeyPair::from_pkcs8(
            Ed25519KeyPair::generate_pkcs8(&rng)
                .expect("sinh khoá mới")
                .as_ref(),
        )
        .expect("đọc khoá mới");
        let cu: [u8; 32] = kp_cu.public_key().as_ref().try_into().expect("32 byte");
        let moi: [u8; 32] = kp_moi.public_key().as_ref().try_into().expect("32 byte");
        let msg = b"vispkg-sig1\nquay vong khoa";
        let sig_cu = kp_cu.sign(msg);
        let sig_moi = kp_moi.sign(msg);
        let ds = [cu, moi];
        assert_eq!(super::xac_minh_nhieu(&ds, msg, sig_cu.as_ref()), Ok(()));
        assert_eq!(super::xac_minh_nhieu(&ds, msg, sig_moi.as_ref()), Ok(()));
        assert!(super::xac_minh_nhieu(&[moi], msg, sig_cu.as_ref()).is_err());
        assert!(super::xac_minh_nhieu(&ds, b"khac", sig_cu.as_ref()).is_err());
        assert!(super::xac_minh_nhieu(&[], msg, sig_cu.as_ref()).is_err());
        assert!(super::xac_minh_nhieu(&ds, msg, &sig_cu.as_ref()[..63]).is_err());
    }
}
