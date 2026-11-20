// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hằng số `std.*` tách khỏi `stdlib.rs` để giữ R1 (không đổi API công khai).

/// **ST-238 / VD-242 (nợ #167)**: trần `max_bytes` cho `net.read`. Input người dùng (kể cả
/// `i64::MAX`, âm, hay giá trị chuyển kiểu overflow) **không** được phép ép host cấp phát bộ nhớ
/// tùy ý (OOM ⇒ abort). Vượt trần ⇒ `Result.Err` dữ liệu, không trap, không panic.
pub const NET_READ_CAP: i64 = 8 * 1024 * 1024;

/// Tiền tố handle cho một lần `net.connect` trên đường frame test (`N:<dia_chi>`).
#[doc(hidden)]
pub const FRAME_NET_CONNECT: &str = "N:";
/// Tiền tố handle cho một lần `net.write` trên đường frame test (`W:<tay>`).
#[doc(hidden)]
pub const FRAME_NET_WRITE: &str = "W:";
/// Hạn mặc định của `net.write` trên đường frame test (ngôn ngữ không có đối số timeout).
/// Chọn rộng cho loopback; vẫn là hạn monotonic DUY NHẤT, không reset theo readiness.
#[doc(hidden)]
pub const FRAME_WRITE_TIMEOUT_MS: i64 = 5_000;

/// **ST-238 / VD-242 (nợ #167)**: validate `max_bytes` từ người dùng. `i64::MAX` (overflow khi
/// chuyển `usize`), âm, và giá trị vượt trần đều bị từ chối **trước** khi host cấp phát.
pub(super) fn gioi_han_net_read(toi_da: i64) -> Result<usize, String> {
    if toi_da < 0 {
        return Err("net.read: `max_bytes` không được âm".to_string());
    }
    if toi_da > NET_READ_CAP {
        return Err(format!(
            "net.read: `max_bytes` {toi_da} vượt giới hạn {NET_READ_CAP} byte"
        ));
    }
    Ok(toi_da as usize)
}
