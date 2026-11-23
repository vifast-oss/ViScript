// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — tiện ích `vis test`: PRNG SplitMix64, Fisher–Yates, `--shard`,
//! tên đơn vị (`[k/N]`), định dạng thời gian và sắp `--slowest`.
//!
//! Tách **nguyên văn** khỏi `vistest.rs` để giữ mỗi tệp ≤ 1.150 dòng.

use super::*;

/// **VD-270**: SplitMix64 — PRNG nhỏ, tất định, không phụ thuộc (trạng thái chỉ là `u64`).
pub(super) fn splitmix64(s: &mut u64) -> u64 {
    *s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// **VD-270**: Fisher–Yates từ cuối, nguồn ngẫu nhiên là `splitmix64(seed)` ⇒ cùng seed cho
/// **cùng hoán vị** trên mọi máy; không dùng crate `rand` nên không thêm phụ thuộc.
pub(super) fn xao_tron<T>(v: &mut [T], seed: u64) {
    let mut s = seed;
    for i in (1..v.len()).rev() {
        let j = (splitmix64(&mut s) % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// **VD-270**: đọc `--shard <I>/<N>` — đúng một `/`, hai số nguyên dương, `1 ≤ I ≤ N`.
pub(super) fn parse_shard(s: &str) -> Option<(usize, usize)> {
    let (a, b) = s.split_once('/')?;
    if b.contains('/') {
        return None;
    }
    let i = a.parse::<usize>().ok()?;
    let n = b.parse::<usize>().ok()?;
    if n == 0 || i == 0 || i > n {
        return None;
    }
    Some((i, n))
}

/// **VD-270**: kết quả từng **đơn vị** theo đúng thứ tự gốc (chỉ số ca, rồi k) — tên, kết quả,
/// **thời gian tường** của lần chạy; `None` = chưa chạy (`--fail-fast` bỏ phần đuôi chưa lập lịch).
pub(super) type KetQuaDonVi = Vec<Option<(String, Kq, std::time::Duration)>>;

/// **VD-270**: tên gốc của ca (không hậu tố `[k/N]`) — `test fn` dùng **tên test**, fixture dùng
/// đường dẫn hiển thị như trước.
pub(super) fn ten_goc_cua(goc: &Path, p: &Path, bai_test: Option<&(String, String)>) -> String {
    match bai_test {
        Some((ten_test, _)) => ten_test.clone(),
        None => p
            .strip_prefix(goc)
            .unwrap_or(p)
            .to_string_lossy()
            .to_string(),
    }
}

/// **VD-270**: tên in ra của **một đơn vị**; hậu tố `[k/N]` chỉ thêm khi `--repeat > 1`.
pub(super) fn ten_don_vi(ten_goc: &str, k: usize, repeat: usize) -> String {
    if repeat > 1 {
        format!("{ten_goc} [{k}/{repeat}]")
    } else {
        ten_goc.to_string()
    }
}

/// **VD-270**: định dạng thời gian cho `--slowest` — `≥ 1 ms` dùng `"<ms> ms"`, dưới 1 ms dùng
/// `"<us> us"` (micro giây; cắt xuống theo đơn vị, không in phần lẻ).
pub(super) fn dinh_dang_thoi_gian(d: std::time::Duration) -> String {
    if d.as_millis() >= 1 {
        format!("{} ms", d.as_millis())
    } else {
        format!("{} us", d.as_micros())
    }
}

/// **VD-270**: sắp **giảm dần** theo thời gian rồi lấy tối đa `n` mục; hoà giữ **thứ tự gốc**
/// (`sort_by_key` của Rust là **ổn định**). Tách khỏi vòng chạy để test phần “hoà” độc lập đồng hồ.
pub(super) fn sap_xep_slowest(
    mut ds: Vec<(String, std::time::Duration)>,
    n: usize,
) -> Vec<(String, std::time::Duration)> {
    ds.sort_by_key(|a| std::cmp::Reverse(a.1));
    ds.truncate(n);
    ds
}
