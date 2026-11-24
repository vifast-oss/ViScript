// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — test đơn vị của `vis test` (`vistest.rs`), tách ra cùng thư mục.

use super::*;
use vis_core::json;
use vis_core::value::Value;

/// Đo đỏ ST-101b: newline/tab/control trong `message` làm JSON **không parse được**.
#[test]
fn esc_json_newline_tab_control_parse_duoc() {
    let goc = "a\"b\\c\n\t\r\u{1}";
    let body = format!("{{\"m\":\"{}\"}}", esc_json(goc));
    let v = json::decode(&body).expect("JSON phải hợp lệ");
    let Value::Map(kv) = v else {
        panic!("không phải map")
    };
    let m = kv.iter().find_map(|(k, v)| match (k, v) {
        (Value::Str(k), Value::Str(v)) if k == "m" => Some(v.clone()),
        _ => None,
    });
    assert_eq!(m.as_deref(), Some(goc));
}

/// `sh -c <script>` — `test fn` ViScript không in được ra stderr, cũng không chạy được lệnh
/// ngoài, nên hai ca dưới gọi thẳng `chay_co_han` với một con `sh`.
///
/// **ST-139 (Agent 0)**: chỉ Unix — `/bin/sh` không có trên Windows. `test_timeout.rs` cũng đã
/// dùng `ps` (Unix) từ ST-105, nhưng ca **unit** này chạy trong `cargo test --workspace` của
/// cổng trên mọi nền, nên phải tự chặn để `gate --fast` không đỏ trên Windows (ST-136).
#[cfg(unix)]
fn sh(script: &str) -> Command {
    let mut c = Command::new("/bin/sh");
    c.arg("-c").arg(script);
    c
}

/// Nợ #89 (issue #23): con in ~200 KB ra **stderr** (> buffer pipe ~64 KiB) rồi thoát ngay ⇒
/// không được báo quá hạn oan, stderr phải về **đủ**.
#[cfg(unix)]
#[test]
fn stderr_nhieu_khong_bi_qua_han_oan() {
    let t0 = std::time::Instant::now();
    let (ma, _out, err, qua_han) = chay_co_han(
        &mut sh("i=0; while [ $i -lt 20000 ]; do echo 0123456789 >&2; i=$((i+1)); done"),
        3_000,
    );
    assert!(!qua_han, "báo quá hạn oan sau {:?}", t0.elapsed());
    assert_eq!(ma, 0);
    assert_eq!(err.len(), 20_000 * 11, "stderr phải về đủ");
    assert!(t0.elapsed().as_millis() < 3_000);
}

/// Quá hạn **không bao giờ treo**: con bị kill nhưng **cháu** (`sleep 5`, nợ #88 chưa kill
/// cây cháu) còn giữ đầu ghi stdout/stderr ⇒ vẫn phải trả `quá hạn` trong < 3 s.
#[cfg(unix)]
#[test]
fn chau_giu_ong_khong_lam_treo() {
    let t0 = std::time::Instant::now();
    let (_ma, _out, _err, qua_han) = chay_co_han(&mut sh("sleep 5 & sleep 10"), 300);
    let dt = t0.elapsed();
    assert!(qua_han);
    assert!(
        dt.as_millis() < 3_000,
        "{dt:?} ≥ 3 s ⇒ đang chờ cháu đóng ống"
    );
}

/// **VD-270**: `--slowest` in `<ms> ms` khi ≥ 1 ms, `<us> us` khi dưới 1 ms (cắt xuống).
#[test]
fn slowest_dinh_dang_thoi_gian() {
    use std::time::Duration;
    assert_eq!(dinh_dang_thoi_gian(Duration::from_micros(0)), "0 us");
    assert_eq!(dinh_dang_thoi_gian(Duration::from_micros(999)), "999 us");
    assert_eq!(dinh_dang_thoi_gian(Duration::from_micros(1_500)), "1 ms");
    assert_eq!(dinh_dang_thoi_gian(Duration::from_millis(2_500)), "2500 ms");
}

/// **VD-270**: `--slowest` sắp **giảm dần**; thời gian **bằng nhau** giữ **thứ tự gốc**;
/// lấy tối đa N mục.
#[test]
fn slowest_sap_giam_dan_hoa_giu_thu_tu_goc() {
    use std::time::Duration;
    let ds = vec![
        ("a".to_string(), Duration::from_millis(5)),
        ("b".to_string(), Duration::from_millis(9)),
        ("c".to_string(), Duration::from_millis(5)),
        ("d".to_string(), Duration::from_millis(1)),
    ];
    let ra = sap_xep_slowest(ds.clone(), 3);
    let tens: Vec<&str> = ra.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        tens,
        ["b", "a", "c"],
        "hoà 5 ms phải giữ thứ tự gốc (a rồi c)"
    );
    assert_eq!(sap_xep_slowest(ds, 99).len(), 4, "N > số mục ⇒ in hết");
}

/// **ST-230**: PATH cho tiến trình con có thư mục binary ở **đầu**, phần PATH cũ giữ nguyên
/// thứ tự phía sau; thư mục rỗng ⇒ không đổi PATH (nhánh `current_exe()` fallback `"vis"`).
#[test]
fn path_voi_muc_dau_them_dau_va_giu_phan_con_lai() {
    let p = super::path_voi_muc_dau(Path::new("/tmp/muc-vis"))
        .expect("PATH phải join được (không chứa ký tự cấm)");
    let ds: Vec<PathBuf> = std::env::split_paths(&p).collect();
    assert_eq!(ds.first(), Some(&PathBuf::from("/tmp/muc-vis")));
    let cu: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    assert_eq!(&ds[1..], &cu[..], "PATH cũ phải giữ nguyên thứ tự");
    assert!(
        super::path_voi_muc_dau(Path::new("")).is_none(),
        "thư mục rỗng ⇒ không đổi PATH"
    );
}
