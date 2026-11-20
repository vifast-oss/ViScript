// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`--fs-mock`** (ST-42) — `FsHost` **tất định** đọc từ một tệp kịch bản JSON, để test `ls`/`open`
//! **hermetic** (không phụ thuộc đĩa). Định dạng:
//!
//! ```json
//! { "dirs": { "./photos": [
//!     {"name": "a.jpg", "size": "2mb", "modified": "3h", "is_file": true} ] },
//!   "files": { "./x.txt": "nội dung" } }
//! ```
//!
//! `size` = literal `size` (bảng của lexer: `2mb` = **2 000 000** byte, `2mib` = 2 097 152) hoặc số
//! byte; `modified` = literal `duration` **từ epoch** (tất định) ⇒ `sort_by modified --desc` = mới nhất trước.

use vis_interp::builtin::{FsHost, MucFs};
use vis_interp::value::Value;

/// FsHost giả lập theo kịch bản.
#[derive(Debug, Clone, Default)]
pub struct FsMock {
    dirs: Vec<(String, Vec<MucFs>)>,
    /// **VD-270 (fsbytes)**: nội dung tệp lưu **byte thô** (kịch bản JSON nạp chuỗi thành UTF-8
    /// byte) để `read_bytes`/`write_bytes` roundtrip được cả byte không phải UTF-8.
    files: Vec<(String, Vec<u8>)>,
}

fn lay(m: &Value, ten: &str) -> Option<Value> {
    if let Value::Map(entries) = m {
        entries
            .iter()
            .find(|(k, _)| matches!(k, Value::Str(s) if s == ten))
            .map(|(_, v)| v.clone())
    } else {
        None
    }
}

fn chuoi(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.clone()),
        _ => None,
    }
}

/// `rw-r--r--`/`-rw-r--r--` (hoặc `755` hệ 8) → bit POSIX; `-`/rỗng ⇒ `0`.
fn mode_tu_chuoi(s: &str) -> u32 {
    let t = s.trim().trim_start_matches('-');
    if t.is_empty() || t == "-" {
        return 0;
    }
    if t.chars().all(|c| ('0'..='7').contains(&c)) {
        return u32::from_str_radix(t, 8).unwrap_or(0);
    }
    let mut mode = 0u32;
    let bits = [
        0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
    ];
    for (i, c) in t.chars().take(9).enumerate() {
        if c != '-' {
            mode |= bits[i];
        }
    }
    mode
}

impl FsMock {
    /// Nạp kịch bản từ chuỗi JSON. `Err` nêu rõ lỗi định dạng.
    pub fn tu_json(s: &str) -> Result<Self, String> {
        let v = vis_interp::json::decode(s)
            .map_err(|e| format!("kịch bản `--fs-mock` không hợp lệ: {e}"))?;
        let mut mock = FsMock::default();
        if let Some(dirs) = lay(&v, "dirs") {
            let Value::Map(ds) = dirs else {
                return Err("`--fs-mock`: `dirs` phải là object".into());
            };
            for (k, dsach) in ds {
                let Some(p) = chuoi(&k) else { continue };
                let Value::List(items) = dsach else {
                    return Err(format!("`--fs-mock`: `dirs[{p}]` phải là mảng"));
                };
                let mut muc = Vec::new();
                for it in items {
                    let Some(ten) = lay(&it, "name").and_then(|x| chuoi(&x)) else {
                        return Err(format!("`--fs-mock`: mục trong `{p}` thiếu `name`"));
                    };
                    let size = match lay(&it, "size") {
                        Some(Value::Int(n)) => n,
                        Some(Value::Size(n)) => n,
                        Some(Value::Str(s)) => {
                            vis_interp::value::parse_size(&s).ok_or_else(|| {
                                format!("`--fs-mock`: `size` của `{ten}` không hợp lệ: {s}")
                            })?
                        }
                        _ => 0,
                    };
                    let modified = match lay(&it, "modified") {
                        Some(Value::Duration(d)) => d,
                        Some(Value::Int(n)) => n,
                        Some(Value::Str(s)) => {
                            vis_interp::value::parse_duration(&s).ok_or_else(|| {
                                format!("`--fs-mock`: `modified` của `{ten}` không hợp lệ: {s}")
                            })?
                        }
                        _ => 0,
                    };
                    let is_file = matches!(lay(&it, "is_file"), Some(Value::Bool(true)));
                    // ST-46: `mode` = chuỗi `rw-r--r--` (hoặc số), `mtime` = duration từ epoch.
                    let mode = match lay(&it, "mode") {
                        Some(Value::Int(n)) => n.max(0) as u32,
                        Some(Value::Str(s)) => mode_tu_chuoi(&s),
                        _ => 0,
                    };
                    let modified_ns = match lay(&it, "mtime") {
                        Some(Value::Duration(d)) => d,
                        Some(Value::Int(n)) => n,
                        Some(Value::Str(s)) => {
                            vis_interp::value::parse_duration(&s).ok_or_else(|| {
                                format!("`--fs-mock`: `mtime` của `{ten}` không hợp lệ: {s}")
                            })?
                        }
                        _ => modified,
                    };
                    muc.push(MucFs {
                        name: ten,
                        size: size.max(0) as u64,
                        modified_ns,
                        is_file,
                        mode,
                    });
                }
                muc.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
                mock.dirs.push((p, muc));
            }
        }
        if let Some(files) = lay(&v, "files") {
            let Value::Map(fs) = files else {
                return Err("`--fs-mock`: `files` phải là object".into());
            };
            for (k, v) in fs {
                let Some(p) = chuoi(&k) else { continue };
                let Some(nd) = chuoi(&v) else {
                    return Err(format!("`--fs-mock`: `files[{p}]` phải là chuỗi"));
                };
                mock.files.push((p, nd.into_bytes()));
            }
        }
        Ok(mock)
    }

    /// **VD-268**: đệ quy con của [`walk`](Self::walk) trên cây kịch bản. `goc` là khoá tuyệt đối
    /// trong `dirs`; `tuong_doi` là tiền tố trả về (rỗng ở root).
    fn duyet_walk(&self, goc: &str, tuong_doi: &str, ra: &mut Vec<String>) {
        let Some((_, muc)) = self.dirs.iter().find(|(k, _)| k == goc) else {
            return;
        };
        let mut muc = muc.clone();
        muc.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        for m in muc {
            let rel = if tuong_doi.is_empty() {
                m.name.clone()
            } else {
                format!("{tuong_doi}/{}", m.name)
            };
            if m.is_file {
                ra.push(rel);
            } else {
                ra.push(format!("{rel}/"));
                let con = format!("{}/{}", goc.trim_end_matches('/'), m.name);
                self.duyet_walk(&con, &rel, ra);
            }
        }
    }
}

impl FsHost for FsMock {
    fn read(&mut self, p: &str) -> Result<String, String> {
        let bytes = self
            .files
            .iter()
            .find(|(k, _)| k == p)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                format!("không đọc được tệp `{p}`: không có trong kịch bản `--fs-mock`")
            })?;
        String::from_utf8(bytes)
            .map_err(|e| format!("không đọc được `{p}`: nội dung không phải UTF-8 ({e})"))
    }

    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        self.write_bytes(p, s.as_bytes())
    }

    /// **VD-270 (fsbytes)**: đọc **byte thô** trong bộ nhớ (mọi giá trị `0..=255`).
    fn read_bytes(&mut self, p: &str) -> Result<Vec<u8>, String> {
        self.files
            .iter()
            .find(|(k, _)| k == p)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| format!("không đọc được tệp `{p}`: không có trong kịch bản `--fs-mock`"))
    }

    /// **VD-270 (fsbytes)**: ghi **byte thô** trong bộ nhớ; tạo mục mới nếu thiếu (cùng hợp đồng
    /// tạo/cắt tệp với host thật).
    fn write_bytes(&mut self, p: &str, data: &[u8]) -> Result<(), String> {
        match self.files.iter_mut().find(|(k, _)| k == p) {
            Some((_, v)) => *v = data.to_vec(),
            None => self.files.push((p.to_string(), data.to_vec())),
        }
        Ok(())
    }

    /// **VD-266**: cùng ngữ nghĩa dòng với host thật (dùng chung `chia_dong`).
    fn read_lines(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.read(p).map(|s| vis_interp::builtin::chia_dong(&s))
    }

    /// **VD-266**: thêm cuối "tệp" trong kịch bản (tạo mục mới nếu thiếu).
    fn append(&mut self, p: &str, s: &str) -> Result<(), String> {
        match self.files.iter_mut().find(|(k, _)| k == p) {
            Some((_, v)) => v.extend_from_slice(s.as_bytes()),
            None => self.files.push((p.to_string(), s.as_bytes().to_vec())),
        }
        Ok(())
    }

    /// **VD-266**: tạo thư mục **một cấp**; đã tồn tại ⇒ `Err` (khớp `create_dir`).
    fn mkdir(&mut self, p: &str) -> Result<(), String> {
        if self.exists(p) {
            return Err(format!("không tạo được thư mục `{p}`: đã tồn tại"));
        }
        self.dirs.push((p.to_string(), Vec::new()));
        Ok(())
    }

    /// **VD-270 (mkdir-all)**: tạo **đệ quy** trong bộ nhớ — mỗi tiền tố thiếu được thêm vào
    /// `dirs`; đích đã là thư mục ⇒ `Ok` (idempotent); tiền tố là **tệp** trong kịch bản ⇒ `Err`.
    /// Tách thành phần theo `/`, bỏ `.`/`/` lặp, `..` lùi **từ vựng** (kịch bản mock không có
    /// `cwd`/symlink để phân giải; symlink không biểu diễn được trong `--fs-mock`).
    fn mkdir_all(&mut self, p: &str) -> Result<(), String> {
        if p.is_empty() {
            return Err("không tạo được thư mục ``: đường dẫn rỗng".to_string());
        }
        let tuyet_doi = p.starts_with('/');
        let mut thanh: Vec<&str> = Vec::new();
        for x in p.split('/') {
            match x {
                "" | "." => {}
                ".." => {
                    thanh.pop();
                }
                _ => thanh.push(x),
            }
        }
        if thanh.is_empty() {
            // `/`, `.`, hoặc đường quy về chính gốc — coi như đã tồn tại.
            return Ok(());
        }
        let mut hien = if tuyet_doi {
            "/".to_string()
        } else {
            String::new()
        };
        for x in thanh {
            if hien.is_empty() || hien.ends_with('/') {
                hien.push_str(x);
            } else {
                hien.push('/');
                hien.push_str(x);
            }
            if self.files.iter().any(|(k, _)| k == &hien) {
                return Err(format!(
                    "không tạo được thư mục `{p}`: `{hien}` đã tồn tại và không phải thư mục"
                ));
            }
            if !self.dirs.iter().any(|(k, _)| k == &hien) {
                self.dirs.push((hien.clone(), Vec::new()));
            }
        }
        Ok(())
    }

    /// **VD-266**: xoá tệp trong kịch bản; **từ chối** thư mục.
    fn remove(&mut self, p: &str) -> Result<(), String> {
        if let Some(i) = self.files.iter().position(|(k, _)| k == p) {
            self.files.remove(i);
            return Ok(());
        }
        if self.dirs.iter().any(|(k, _)| k == p) {
            return Err(format!("không xoá được `{p}`: là thư mục (chỉ xoá tệp)"));
        }
        Err(format!(
            "không xoá được tệp `{p}`: không có trong kịch bản `--fs-mock`"
        ))
    }

    /// **VD-269**: chép nội dung **trong bộ nhớ**; nguồn phải là tệp, đích phải **chưa có** (cùng
    /// chính sách với host thật). Kịch bản mock không biểu diễn được symlink.
    fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        if self.dirs.iter().any(|(k, _)| k == from) {
            return Err(format!(
                "không chép được `{from}`: là thư mục (chỉ chép tệp)"
            ));
        }
        let noi_dung = match self.files.iter().find(|(k, _)| k == from) {
            Some((_, v)) => v.clone(),
            None => {
                return Err(format!(
                    "không chép được tệp `{from}`: không có trong kịch bản `--fs-mock`"
                ));
            }
        };
        if self.exists(to) {
            return Err(format!("không chép được `{to}`: đích đã tồn tại"));
        }
        self.files.push((to.to_string(), noi_dung));
        Ok(())
    }

    /// **VD-269**: đổi tên **trong bộ nhớ**; nguồn phải là tệp, đích phải **chưa có**.
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        if self.dirs.iter().any(|(k, _)| k == from) {
            return Err(format!(
                "không đổi tên được `{from}`: là thư mục (chỉ đổi tên tệp)"
            ));
        }
        let Some(i) = self.files.iter().position(|(k, _)| k == from) else {
            return Err(format!(
                "không đổi tên được tệp `{from}`: không có trong kịch bản `--fs-mock`"
            ));
        };
        if self.exists(to) {
            return Err(format!("không đổi tên được `{to}`: đích đã tồn tại"));
        }
        let (_, noi_dung) = self.files.remove(i);
        self.files.push((to.to_string(), noi_dung));
        Ok(())
    }

    /// **VD-270**: số **byte** của "tệp" trong kịch bản (UTF-8, không phải số ký tự); thư mục và
    /// mục thiếu ⇒ `Err` (cùng chính sách với host thật).
    fn size(&mut self, p: &str) -> Result<i64, String> {
        if let Some((_, v)) = self.files.iter().find(|(k, _)| k == p) {
            return Ok(v.len() as i64);
        }
        if self.dirs.iter().any(|(k, _)| k == p) {
            return Err(format!(
                "không lấy được kích thước `{p}`: không phải tệp thường"
            ));
        }
        Err(format!(
            "không lấy được kích thước `{p}`: không có trong kịch bản `--fs-mock`"
        ))
    }

    /// **VD-270**: `true` ⇔ khoá có trong `files`; mục thiếu ⇒ `false` (khớp host thật). Kịch bản
    /// mock **không** biểu diễn được symlink.
    fn is_file(&mut self, p: &str) -> Result<bool, String> {
        Ok(self.files.iter().any(|(k, _)| k == p))
    }

    /// **VD-270**: `true` ⇔ khoá có trong `dirs`; mục thiếu ⇒ `false` (khớp host thật).
    fn is_dir(&mut self, p: &str) -> Result<bool, String> {
        Ok(self.dirs.iter().any(|(k, _)| k == p))
    }

    /// **VD-268**: duyệt cây trong kịch bản, cùng hợp đồng với host thật: tương đối, thư mục kèm
    /// `/`, DFS sắp theo byte tên từng cấp. Thư mục con không khai trong `dirs` coi như **rỗng**
    /// (kịch bản không cần liệt kê mọi thư mục rỗng). Root không có ⇒ `Err` dữ liệu.
    fn walk(&mut self, p: &str) -> Result<Vec<String>, String> {
        if !self.dirs.iter().any(|(k, _)| k == p) {
            return Err(format!(
                "không duyệt được thư mục `{p}`: không có trong kịch bản `--fs-mock`"
            ));
        }
        let mut ra = Vec::new();
        self.duyet_walk(p, "", &mut ra);
        Ok(ra)
    }

    fn exists(&mut self, p: &str) -> bool {
        self.files.iter().any(|(k, _)| k == p) || self.dirs.iter().any(|(k, _)| k == p)
    }

    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.list_entries(p)
            .map(|m| m.into_iter().map(|x| x.name).collect())
    }

    fn list_entries(&mut self, p: &str) -> Result<Vec<MucFs>, String> {
        self.dirs
            .iter()
            .find(|(k, _)| k == p)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                format!("không đọc được thư mục `{p}`: không có trong kịch bản `--fs-mock`")
            })
    }
}
