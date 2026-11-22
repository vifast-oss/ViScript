// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277)** — đồng hồ của host WASI.
//!
//! Trên `wasm32-wasip1`, `SystemTime`/`Instant` của Rust std đi qua `clock_time_get` của WASI
//! (wall clock + monotonic), nên **không** cần `unsafe` hay dependency. Trên native, cùng hàm
//! dùng đồng hồ hệ điều hành — test chỉ kiểm tính chất (không so giá trị tuyệt đối).

use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Mili-giây kể từ epoch Unix (wall clock).
pub fn bay_gio_ms() -> Result<u64, String> {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("đồng hồ hệ thống trước epoch: {e}"))?;
    u64::try_from(d.as_millis()).map_err(|_| "đồng hồ vượt u64 mili-giây".to_string())
}

/// Giây kể từ epoch Unix (wall clock).
pub fn bay_gio_giay() -> Result<i64, String> {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("đồng hồ hệ thống trước epoch: {e}"))?;
    i64::try_from(d.as_secs()).map_err(|_| "đồng hồ vượt i64 giây".to_string())
}

/// Mili-giây **đơn điệu** kể từ lần gọi đầu tiên trong tiến trình.
pub fn don_dieu_ms() -> u64 {
    use std::sync::OnceLock;
    static GOC: OnceLock<Instant> = OnceLock::new();
    let goc = GOC.get_or_init(Instant::now);
    u64::try_from(goc.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dong_ho_tuong_hop_ly() {
        // 2020-01-01 > 1.57e12 ms; đồng hồ máy thật/CI luôn sau mốc này.
        assert!(bay_gio_ms().unwrap() > 1_577_836_800_000);
        assert!(bay_gio_giay().unwrap() > 1_577_836_800);
    }

    #[test]
    fn don_dieu_khong_giam() {
        let a = don_dieu_ms();
        let b = don_dieu_ms();
        assert!(b >= a, "đồng hồ đơn điệu lùi: {a} → {b}");
    }
}
