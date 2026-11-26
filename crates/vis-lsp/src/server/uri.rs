// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chuyển đổi URI `file:` ↔ đường dẫn + chọn `Mode` theo đuôi tệp (tách nguyên văn khỏi `lib.rs`
//! ở `ST-317b`). Giữ nguyên luật percent-decode/drive/UNC của ST-80/ST-297/VD-258.

use vis_syntax::Mode;

/// **ST-80**: `textDocument.uri` (`file://…`) → đường dẫn tệp; `None` cho URI không phải `file:`,
/// percent-decode cơ bản (`%20`, UTF-8), chấp nhận đường dẫn tuyệt đối (`file:///…`).
pub(crate) fn uri_thanh_duong(uri: &str) -> Option<String> {
    uri_thanh_duong_voi(uri, cfg!(windows))
}

/// Như [`uri_thanh_duong`] nhưng nhận cờ nền tảng (test thuần trên Linux vẫn kiểm được nhánh
/// Windows). **VD-258**: percent-decode **trước** khi nhận diện drive, nên `file:///C%3A/path`
/// (VS Code gửi drive đã mã hoá) hoạt động; trước đây `%3A` chưa được giải mã nên path còn
/// `/C%3A/path` và bị coi là tương đối/hỏng trên Windows.
pub(crate) fn uri_thanh_duong_voi(uri: &str, windows: bool) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    // Percent-decode trước (UTF-8), rồi mới chuẩn hoá đường dẫn theo nền tảng.
    let rest = percent_giai_ma(rest)?;
    // **ST-297**: trên Windows `\` là dấu phân cách; chuẩn hoá **trước** khi nhận diện
    // drive/UNC để mixed slash (`C:/a\b`, `%5C`) về cùng một dạng. Trên POSIX `\` là ký tự
    // thường nên KHÔNG đổi.
    let rest = if windows {
        rest.replace('\\', "/")
    } else {
        rest
    };
    // **VD-241 § fix(Windows/CRLF #psvn-A5)**: LSP trên Windows gửi `file:///c:/path` (3 slash)
    // hoặc `file:///c:\path`; chuẩn hoá về `c:/...` khi phát hiện drive letter.
    fn trim_win_drive(r: &str) -> Option<String> {
        let b = r.as_bytes();
        // `/c:/path` → `c:/path`
        if b.len() >= 3
            && b[0] == b'/'
            && b[1].is_ascii_alphabetic()
            && b[2] == b':'
            && (b.len() == 3 || b[3] == b'/')
        {
            return Some(r[1..].to_string());
        }
        // `c:/path` (no leading slash) → `c:/path`
        if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/' {
            return Some(r.to_string());
        }
        None
    }
    if windows {
        if let Some(r) = trim_win_drive(&rest) {
            return Some(r);
        }
        // `file:////server/share` (path `//server/share`) giữ nguyên.
        if rest.starts_with("//") {
            return Some(rest);
        }
        // `file://server/share/a` — authority là UNC server; LSP file URI không có
        // authority nghĩa là path tương đối, nhưng trên Windows đây là UNC hợp lệ.
        if !rest.starts_with('/') {
            if rest.is_empty() || !rest.contains('/') {
                return None;
            }
            return Some(format!("//{rest}"));
        }
        // `file:///etc/x` — đường dẫn tuyệt đối kiểu POSIX/Windows rooted.
        Some(rest)
    } else {
        // Non-Windows: authority (`file://server/...`) không phải path cục bộ; `file:////…`
        // cho path `//…` vẫn giữ.
        rest.starts_with('/').then_some(rest)
    }
}

/// **ST-297** — path → URI `file:` chuẩn (percent-encode). `windows` chỉ đổi cách nhận
/// drive/UNC, không phụ thuộc nền tảng host nên test được trên Linux.
pub(crate) fn duong_thanh_uri(p: &std::path::Path, windows: bool) -> String {
    fn ma_hoa(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for &b in s.as_bytes() {
            match b {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'/'
                | b':' => out.push(b as char),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    let s = p.to_string_lossy();
    if windows {
        let s = s.replace('\\', "/");
        if let Some(rest) = s.strip_prefix("//") {
            // UNC: `file://server/share/...` (authority), không phải `file:////`.
            return format!("file://{}", ma_hoa(rest));
        }
        if let Some(rest) = s.strip_prefix('/') {
            return format!("file:///{}", ma_hoa(rest));
        }
        // Drive `C:/...` hoặc path tương đối (LSP luôn gửi absolute; giữ an toàn).
        return format!("file:///{}", ma_hoa(&s));
    }
    if let Some(rest) = s.strip_prefix('/') {
        format!("file:///{}", ma_hoa(rest))
    } else {
        format!("file:///{}", ma_hoa(&s))
    }
}

/// Percent-decode cơ bản (UTF-8); `%` không theo sau 2 hex hợp lệ được giữ nguyên (hành vi cũ).
pub(crate) fn percent_giai_ma(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let h = std::str::from_utf8(&b[i + 1..i + 3]).ok()?;
            if let Ok(v) = u8::from_str_radix(h, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

/// `didOpen`: `params.textDocument.{uri,text}`.
/// `didChange` full sync: lấy `contentChanges[last].text`.
/// Chế độ phân tích theo đuôi URI (`.vish` ⇒ Command).
pub(crate) fn mode_cua(uri: &str) -> Mode {
    if uri.ends_with(".vish") {
        Mode::Command
    } else {
        Mode::Script
    }
}
