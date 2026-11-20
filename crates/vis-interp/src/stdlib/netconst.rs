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
