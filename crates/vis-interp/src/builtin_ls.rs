// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-46 (VD-42)** — `ls --long`: quyền POSIX + thời gian UTC — tách khỏi `builtin.rs`
//! để giữ luật **R1** (≤ 1.500 dòng/tệp).

/// `ls --long` (ST-46): quyền `rw-r--r--` từ bit POSIX; `0` ⇒ `-`.
pub fn chuoi_quyen(mode: u32) -> String {
    if mode == 0 {
        return "-".to_string();
    }
    let mut s = String::with_capacity(9);
    for (bit, ch) in [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ] {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    s
}

/// `ls --long` (ST-46): `YYYY-MM-DD HH:MM` theo **UTC** (civil-from-days, không thêm crate).
pub fn thoi_gian_utc(ns: i64) -> String {
    if ns == 0 {
        return "-".to_string();
    }
    let mut giay = ns.div_euclid(1_000_000_000);
    let ngay = giay.div_euclid(86_400);
    giay = giay.rem_euclid(86_400);
    // Howard Hinnant: days since 1970-01-01 → (y, m, d).
    let z = ngay + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        giay / 3600,
        (giay % 3600) / 60
    )
}
