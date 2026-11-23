// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — lịch sử **append-only** + tìm mờ (subsequence) cho ViShell.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct History {
    entries: Vec<String>,
    path: Option<PathBuf>,
}

impl History {
    /// Nạp lịch sử từ `path` (nếu có); file được **append**, không ghi đè.
    pub fn new(path: Option<PathBuf>) -> io::Result<Self> {
        let mut h = Self {
            entries: Vec::new(),
            path,
        };
        if let Some(p) = &h.path {
            if p.is_file() {
                let s = fs::read_to_string(p)?;
                h.entries = s
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect();
            }
        }
        Ok(h)
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Thêm một dòng vào bộ nhớ + **append** file (nếu có). Dòng rỗng bị bỏ qua.
    pub fn add(&mut self, line: &str) -> io::Result<()> {
        if line.is_empty() {
            return Ok(());
        }
        self.entries.push(line.to_string());
        if let Some(p) = &self.path {
            if let Some(d) = p.parent() {
                fs::create_dir_all(d)?;
            }
            let mut f = OpenOptions::new().create(true).append(true).open(p)?;
            writeln!(f, "{line}")?;
        }
        Ok(())
    }

    /// Tìm mờ: mọi ký tự của `needle` xuất hiện theo thứ tự; điểm ưu tiên khớp liền nhau và
    /// vị trí sớm. Kết quả mới nhất đứng trước khi điểm bằng nhau.
    pub fn fuzzy(&self, needle: &str) -> Vec<&str> {
        let n = needle.to_lowercase();
        let mut ds: Vec<(i32, usize, &str)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, s)| score(&s.to_lowercase(), &n).map(|sc| (sc, i, s.as_str())))
            .collect();
        ds.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        ds.into_iter().map(|(_, _, s)| s).collect()
    }
}

fn score(hay: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let mut sc = 0;
    let mut hi = 0usize;
    let mut prev: Option<usize> = None;
    for (ni, nc) in needle.chars().enumerate() {
        let mut found = None;
        for (i, c) in hay.char_indices().skip_while(|(i, _)| *i < hi) {
            if c == nc {
                found = Some(i);
                break;
            }
        }
        let i = found?;
        sc += 10;
        if prev.is_some_and(|p| p + 1 == i) {
            sc += 15;
        }
        if ni == 0 {
            sc += if i == 0 { 10 } else { 0 };
        }
        prev = Some(i);
        hi = i + nc.len_utf8();
    }
    Some(sc)
}
