// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — phím logic cho raw mode; không phụ thuộc terminal.

use std::io::{self, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    /// `Ctrl-J`: chèn xuống dòng (khác Enter gửi dòng).
    Newline,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    WordLeft,
    WordRight,
    CtrlC,
    CtrlR,
    Esc,
}

/// Giải mã một buffer byte (dùng cho test/PTY giả).
pub fn parse_keys(input: &[u8]) -> Vec<Key> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b == 0x1b {
            if i + 2 < input.len() && (input[i + 1] == b'[' || input[i + 1] == b'O') {
                let (key, used) = escape(&input[i..]);
                out.push(key);
                i += used;
            } else {
                out.push(Key::Esc);
                i += 1;
            }
            continue;
        }
        let key = match b {
            0x03 => Some(Key::CtrlC),
            0x12 => Some(Key::CtrlR),
            0x01 => Some(Key::Home),
            0x05 => Some(Key::End),
            0x7f | 0x08 => Some(Key::Backspace),
            b'\r' => Some(Key::Enter),
            b'\n' => Some(Key::Newline),
            b'\t' => Some(Key::Tab),
            0x00..=0x1f => None,
            _ => {
                let n = utf8_len(b);
                if i + n <= input.len() {
                    if let Ok(s) = std::str::from_utf8(&input[i..i + n]) {
                        if let Some(c) = s.chars().next() {
                            out.push(Key::Char(c));
                        }
                        i += n;
                        continue;
                    }
                }
                i += 1;
                continue;
            }
        };
        if let Some(k) = key {
            out.push(k);
        }
        i += 1;
    }
    out
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

fn escape(s: &[u8]) -> (Key, usize) {
    // `s` bắt đầu bằng ESC.
    if s.len() >= 3 && (s[1] == b'[' || s[1] == b'O') {
        match s[2] {
            b'A' => return (Key::Up, 3),
            b'B' => return (Key::Down, 3),
            b'C' => return (Key::Right, 3),
            b'D' => return (Key::Left, 3),
            b'H' => return (Key::Home, 3),
            b'F' => return (Key::End, 3),
            _ => {}
        }
    }
    if s.starts_with(b"\x1b[3~") {
        return (Key::Delete, 4);
    }
    if s.starts_with(b"\x1b[1;5D") {
        return (Key::WordLeft, 6);
    }
    if s.starts_with(b"\x1b[1;5C") {
        return (Key::WordRight, 6);
    }
    (Key::Esc, 1)
}

/// Đọc một phím từ terminal đã raw. `None` = EOF.
pub fn read_key<R: Read>(r: &mut R) -> io::Result<Option<Key>> {
    let mut b = [0u8; 1];
    if r.read(&mut b)? == 0 {
        return Ok(None);
    }
    if b[0] == 0x1b {
        let mut seq = vec![0x1b];
        let mut x = [0u8; 1];
        if r.read(&mut x)? == 0 {
            return Ok(Some(Key::Esc));
        }
        seq.push(x[0]);
        if seq[1] == b'[' || seq[1] == b'O' {
            // Đọc tới ký tự kết thúc CSI (`A`..`Z`, `~`) — tối đa 6 byte.
            while seq.len() < 6 {
                if r.read(&mut x)? == 0 {
                    break;
                }
                seq.push(x[0]);
                if seq.len() >= 3 && (x[0].is_ascii_alphabetic() || x[0] == b'~') {
                    break;
                }
            }
        }
        let keys = parse_keys(&seq);
        return Ok(keys.into_iter().next());
    }
    let n = utf8_len(b[0]);
    let mut buf = vec![b[0]];
    for _ in 1..n {
        let mut x = [0u8; 1];
        if r.read(&mut x)? == 0 {
            break;
        }
        buf.push(x[0]);
    }
    Ok(parse_keys(&buf).into_iter().next())
}
