// SPDX-License-Identifier: MIT OR Apache-2.0
//! Sinh + kiểm **token handle** `db:<32 hex>` (128 bit từ OS, fallback không-crate).
//! Tách khỏi `mod.rs` ở `ST-317b` (thuần move, 0 đổi hành vi).

use std::sync::atomic::{AtomicU64, Ordering};

/// Token ngẫu nhiên 128 bit (`db:` + 32 hex thường). Hai nửa 64-bit lấy từ **nguồn OS**:
/// Unix đọc 16 byte `/dev/urandom`; nền tảng khác/không đọc được ⇒ fallback **không-crate**
/// dùng `RandomState` của std (cũng được seed từ OS) băm SipHash hai miền tách biệt, kèm bộ đếm.
/// Token chỉ là **handle mờ trong phiên**, không phải bí mật xác thực: host luôn khớp toàn bộ
/// token trong `bang` của chính phiên, tombstone chặn tái dùng; không dùng làm mật khẩu/capability
/// liên phiên. Không đoán được từ output, không tái sử dụng trong phiên.
pub(super) fn token_ngau_nhien() -> String {
    static BO_DEM: AtomicU64 = AtomicU64::new(0);
    let dem = BO_DEM.fetch_add(1, Ordering::Relaxed);
    let (a, b) = ngau_nhien_os().unwrap_or_else(|| ngau_nhien_du_phong(dem));
    let token128 = ((a as u128) << 64) | (b as u128);
    format!("db:{token128:032x}")
}

/// 16 byte entropy từ OS trên Unix (`/dev/urandom`). Không dùng `unsafe`/crate ngoài; lỗi mở/đọc
/// ⇒ `None` để caller dùng fallback (không panic, không chặn mở db).
#[cfg(unix)]
pub(super) fn ngau_nhien_os() -> Option<(u64, u64)> {
    use std::io::Read;
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .ok()?
        .read_exact(&mut b)
        .ok()?;
    Some((
        u64::from_le_bytes(b[0..8].try_into().ok()?),
        u64::from_le_bytes(b[8..16].try_into().ok()?),
    ))
}

/// Không có `/dev/urandom` (Windows/nền tảng khác) — fallback trong `token_ngau_nhien`.
#[cfg(not(unix))]
pub(super) fn ngau_nhien_os() -> Option<(u64, u64)> {
    None
}

/// Fallback **không thêm dependency**: `RandomState` của std được seed từ OS cho mỗi thread
/// (Unix: getrandom; Windows: OS entropy), rồi SipHash hai miền tách biệt lấy 2×64 bit. Không
/// phải CSPRNG và không hứa 128 bit entropy độc lập tuyệt đối — chấp nhận được vì token là
/// handle mờ trong phiên (xem [`token_ngau_nhien`]); bộ đếm bảo đảm không lặp trong cùng tiến
/// trình ngay cả khi nguồn OS lỗi.
pub(super) fn ngau_nhien_du_phong(dem: u64) -> (u64, u64) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    let chao: u64 = 0;
    let dia_chi = &chao as *const u64 as usize as u64;
    let rs = RandomState::new();
    let bam = |mien: u8| {
        let mut h = rs.build_hasher();
        h.write_u8(mien);
        h.write_u64(t);
        h.write_u64(pid);
        h.write_u64(dem);
        h.write_u64(dia_chi);
        h.finish()
    };
    (bam(0xA1), bam(0x5C))
}

/// Token đúng dạng? `db:` + đúng 32 ký tự hex thường — mọi dạng khác (`db:0`, `db:00`, `db:2`,
/// `db:1x`, `db:`…) đều bị từ chối **trước** khi tra bảng.
pub(super) fn token_dung_dang(h: &str) -> bool {
    match h.strip_prefix("db:") {
        Some(hex) => hex.len() == 32 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        None => false,
    }
}
