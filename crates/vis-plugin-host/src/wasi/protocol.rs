// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307/ST-310c (VD-285)** — giao thức plugin WASI: manifest, kiểm ABI trên byte wasm,
//! codec văn bản `PluginValue` và phản hồi `ok`/`err`.
//!
//! Tách nguyên hành vi từ `wasi.rs` cũ; **không** đụng syscall/quyền (phần đó ở `sandbox.rs`
//! và `runner.rs`). Mọi lỗi dữ liệu người dùng là `VS0501`, không panic.

use vis_plugin_abi::{Error, PluginValue, Result, VIS_PLUGIN_ABI_V1};

/// Tiền tố export marker ABI trên byte wasm.
pub const ABI_EXPORT_PREFIX: &str = "vis_plugin_abi_v";

/// Dòng handshake mà plugin phải in ở chế độ `abi`.
pub const ABI_HANDSHAKE: &str = "VIS_PLUGIN_ABI_V1=";

/// Trần kích thước tệp wasm (64 MiB) — chặn tệp lạ nuốt bộ nhớ.
pub const MAX_WASM_BYTES: usize = 64 * 1024 * 1024;

/// Một hàm plugin do manifest khai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HamPlugin {
    pub name: String,
    pub params: Vec<String>,
    pub ret: String,
}

/// Manifest plugin đọc từ chế độ `manifest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub module: String,
    pub abi: u32,
    pub funcs: Vec<HamPlugin>,
    pub caps: Vec<(String, bool)>,
}

impl Manifest {
    /// Tìm hàm theo tên.
    pub fn tim(&self, ten: &str) -> Option<&HamPlugin> {
        self.funcs.iter().find(|f| f.name == ten)
    }
}

/// Kiểm **trên byte wasm** trước khi chạy: magic/version, export `_start`, đúng một marker ABI.
pub(crate) fn kiem_wasm_abi(wasm: &[u8]) -> Result<()> {
    let exports = doc_export(wasm)?;
    let mut abi: Option<u32> = None;
    let mut co_start = false;
    for (ten, kind) in exports {
        if ten == "_start" && kind == 0 {
            co_start = true;
        }
        if let Some(rest) = ten.strip_prefix(ABI_EXPORT_PREFIX) {
            if let Ok(n) = rest.parse::<u32>() {
                if abi.is_some() {
                    return Err(Error::new(
                        "VS0501",
                        format!("plugin có nhiều export `{ABI_EXPORT_PREFIX}*` — không rõ ABI"),
                    ));
                }
                abi = Some(n);
            }
        }
    }
    if !co_start {
        return Err(Error::new(
            "VS0501",
            "không phải plugin WASI: thiếu export `_start` (WASI command)",
        ));
    }
    let abi = abi.ok_or_else(|| {
        Error::new(
            "VS0501",
            format!("không phải plugin ViScript: thiếu export `{ABI_EXPORT_PREFIX}1`"),
        )
    })?;
    if abi != VIS_PLUGIN_ABI_V1 {
        return Err(Error::new(
            "VS0501",
            format!("ABI plugin v{abi} không khớp `VIS_PLUGIN_ABI_V1` ({VIS_PLUGIN_ABI_V1})"),
        ));
    }
    Ok(())
}

/// Đọc bảng export của wasm (section 7) — LEB128 có kiểm biên, không panic.
fn doc_export(wasm: &[u8]) -> Result<Vec<(String, u8)>> {
    if wasm.len() < 8 || &wasm[0..4] != b"\0asm" {
        return Err(Error::new(
            "VS0501",
            "không phải tệp wasm (thiếu magic `\\0asm`)",
        ));
    }
    let version = u32::from_le_bytes([wasm[4], wasm[5], wasm[6], wasm[7]]);
    if version != 1 {
        return Err(Error::new(
            "VS0501",
            format!("phiên bản wasm {version} không hỗ trợ (cần 1)"),
        ));
    }
    let mut i = 8usize;
    let mut ra = Vec::new();
    while i < wasm.len() {
        let id = wasm[i];
        i += 1;
        let (size, ni) = leb(wasm, i)?;
        i = ni;
        let end = i
            .checked_add(size)
            .ok_or_else(|| Error::new("VS0501", "wasm hỏng: kích thước section tràn"))?;
        if end > wasm.len() {
            return Err(Error::new("VS0501", "wasm hỏng: section vượt quá tệp"));
        }
        if id == 7 {
            let mut j = i;
            let (count, nj) = leb(wasm, j)?;
            j = nj;
            for _ in 0..count {
                let (nlen, nj2) = leb(wasm, j)?;
                j = nj2;
                let ten_end = j
                    .checked_add(nlen)
                    .ok_or_else(|| Error::new("VS0501", "wasm hỏng: tên export tràn"))?;
                if ten_end > end {
                    return Err(Error::new("VS0501", "wasm hỏng: tên export vượt section"));
                }
                let ten = std::str::from_utf8(&wasm[j..ten_end])
                    .map_err(|_| Error::new("VS0501", "wasm hỏng: tên export không UTF-8"))?
                    .to_string();
                j = ten_end;
                if j >= end {
                    return Err(Error::new("VS0501", "wasm hỏng: export thiếu kind"));
                }
                let kind = wasm[j];
                j += 1;
                let (_, nj3) = leb(wasm, j)?;
                j = nj3;
                ra.push((ten, kind));
            }
        }
        i = end;
    }
    Ok(ra)
}

fn leb(b: &[u8], mut i: usize) -> Result<(usize, usize)> {
    let mut ra = 0usize;
    let mut shift = 0u32;
    loop {
        let x = *b
            .get(i)
            .ok_or_else(|| Error::new("VS0501", "wasm hỏng: LEB128 cụt"))?;
        i += 1;
        ra |= ((x & 0x7f) as usize) << shift;
        if x & 0x80 == 0 {
            return Ok((ra, i));
        }
        shift += 7;
        if shift >= usize::BITS {
            return Err(Error::new("VS0501", "wasm hỏng: LEB128 quá dài"));
        }
    }
}

/// Phân tích manifest dạng dòng `module=/abi=/func=/cap=/ok`.
pub(crate) fn doc_manifest(ra: &str) -> Result<Manifest> {
    let mut module = None;
    let mut abi = None;
    let mut funcs = Vec::new();
    let mut caps = Vec::new();
    let mut ok = false;
    for dong in ra.lines().map(str::trim) {
        if dong.is_empty() {
            continue;
        }
        if dong == "ok" {
            ok = true;
            continue;
        }
        let (khoa, gia) = dong.split_once('=').ok_or_else(|| {
            Error::new(
                "VS0501",
                format!("manifest plugin sai: dòng `{dong}` không có `=`"),
            )
        })?;
        match khoa {
            "module" => module = Some(gia.to_string()),
            "abi" => {
                abi = Some(gia.parse::<u32>().map_err(|_| {
                    Error::new("VS0501", format!("manifest plugin sai abi `{gia}`"))
                })?)
            }
            "func" => {
                let mut phan = gia.split(';');
                let ten = phan.next().unwrap_or("").trim();
                let params = phan.next().unwrap_or("");
                let ret = phan.next().unwrap_or("unit").trim();
                if ten.is_empty() {
                    return Err(Error::new("VS0501", "manifest plugin: hàm không tên"));
                }
                funcs.push(HamPlugin {
                    name: ten.to_string(),
                    params: if params.trim().is_empty() {
                        Vec::new()
                    } else {
                        params.split(',').map(|s| s.trim().to_string()).collect()
                    },
                    ret: ret.to_string(),
                });
            }
            "cap" => {
                let (kind, quyen) = gia.split_once(';').unwrap_or((gia, "ro"));
                let write = match quyen.trim() {
                    "rw" | "write" => true,
                    "ro" | "read" => false,
                    khac => {
                        return Err(Error::new(
                            "VS0501",
                            format!("manifest plugin: quyền cap lạ `{khac}`"),
                        ))
                    }
                };
                caps.push((kind.trim().to_string(), write));
            }
            khac => {
                return Err(Error::new(
                    "VS0501",
                    format!("manifest plugin: khoá lạ `{khac}`"),
                ))
            }
        }
    }
    if !ok {
        return Err(Error::new("VS0501", "manifest plugin thiếu dòng kết `ok`"));
    }
    let module = module.ok_or_else(|| Error::new("VS0501", "manifest plugin thiếu `module=`"))?;
    let abi = abi.ok_or_else(|| Error::new("VS0501", "manifest plugin thiếu `abi=`"))?;
    Ok(Manifest {
        module,
        abi,
        funcs,
        caps,
    })
}

/// Phân tích phản hồi `ok\n<giá trị>` hoặc `err\n<VSxxxx>\n<hex>`.
pub(crate) fn doc_ket_qua(ra: &str) -> Result<PluginValue> {
    let mut dong = ra.lines();
    match dong.next().map(str::trim) {
        Some("ok") => {
            let gia = dong
                .next()
                .ok_or_else(|| Error::new("VS0501", "plugin trả `ok` thiếu giá trị"))?;
            giai_ma(gia.trim())
        }
        Some("err") => {
            let code = dong.next().unwrap_or("VS0501").trim();
            let ma = match code {
                "VS0604" => "VS0604",
                _ => "VS0501",
            };
            let hex_msg = dong.next().unwrap_or("").trim();
            let msg = giai_hex(hex_msg)
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_else(|| "plugin báo lỗi không rõ".to_string());
            Err(Error::new(ma, msg))
        }
        Some(khac) => Err(Error::new(
            "VS0501",
            format!("phản hồi plugin sai: `{khac}` (cần `ok`/`err`)"),
        )),
        None => Err(Error::new("VS0501", "plugin không trả gì")),
    }
}

/// Mã hoá một [`PluginValue`] sang codec văn bản của giao thức.
pub fn ma_hoa(v: &PluginValue) -> String {
    let mut s = String::new();
    ma_hoa_vao(v, &mut s);
    s
}

fn ma_hoa_vao(v: &PluginValue, s: &mut String) {
    match v {
        PluginValue::Unit => s.push_str("u:"),
        PluginValue::Bool(b) => s.push_str(if *b { "b:1:" } else { "b:0:" }),
        PluginValue::Int(i) => {
            s.push_str("i:");
            s.push_str(&i.to_string());
            s.push(':');
        }
        PluginValue::Float(f) => {
            s.push_str("f:");
            s.push_str(&f.to_string());
            s.push(':');
        }
        PluginValue::Str(x) => {
            s.push_str("s:");
            s.push_str(&x.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(x.as_bytes()));
        }
        PluginValue::Bytes(x) => {
            s.push_str("y:");
            s.push_str(&x.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(x));
        }
        PluginValue::List(xs) => {
            s.push_str("l:");
            s.push_str(&xs.len().to_string());
            s.push(':');
            for x in xs {
                ma_hoa_vao(x, s);
            }
        }
        PluginValue::Map(kv) => {
            s.push_str("m:");
            s.push_str(&kv.len().to_string());
            s.push(':');
            for (k, v) in kv {
                ma_hoa_vao(k, s);
                ma_hoa_vao(v, s);
            }
        }
        PluginValue::Option(None) => s.push_str("o:0:"),
        PluginValue::Option(Some(x)) => {
            s.push_str("o:1:");
            ma_hoa_vao(x, s);
        }
        PluginValue::Result(Ok(x)) => {
            s.push_str("r:1:");
            ma_hoa_vao(x, s);
        }
        PluginValue::Result(Err(e)) => {
            s.push_str("r:0:");
            s.push_str(&e.message.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(e.message.as_bytes()));
        }
        PluginValue::Cap { kind, root, write } => {
            s.push_str("c:");
            s.push_str(&kind.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(kind.as_bytes()));
            s.push_str(&root.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(root.as_bytes()));
            s.push(if *write { '1' } else { '0' });
            s.push(':');
        }
    }
}

/// Giải mã codec văn bản; lỗi ⇒ `VS0501`, không panic.
pub fn giai_ma(s: &str) -> Result<PluginValue> {
    let mut d = Doc {
        b: s.as_bytes(),
        i: 0,
        sau: 0,
    };
    let v = d.gia_tri()?;
    if d.i != d.b.len() {
        return Err(Error::new(
            "VS0501",
            format!("giá trị plugin còn thừa `{}`", &s[d.i..]),
        ));
    }
    Ok(v)
}

/// **ST-314a (W3 M1)**: trần độ sâu giá trị plugin — input `875 byte` lồng 200.000 cấp từng làm
/// host **stack overflow/SIGABRT**; vượt trần ⇒ `VS0501` có kiểm soát.
const MAX_SAU: usize = 64;

struct Doc<'a> {
    b: &'a [u8],
    i: usize,
    sau: usize,
}

impl Doc<'_> {
    fn ky_tu(&mut self) -> Result<u8> {
        let x = *self
            .b
            .get(self.i)
            .ok_or_else(|| Error::new("VS0501", "giá trị plugin cụt"))?;
        self.i += 1;
        Ok(x)
    }

    fn cham(&mut self) -> Result<()> {
        if self.ky_tu()? != b':' {
            return Err(Error::new("VS0501", "giá trị plugin thiếu `:`"));
        }
        Ok(())
    }

    fn so_den_cham(&mut self) -> Result<usize> {
        let bat_dau = self.i;
        while self.i < self.b.len() && self.b[self.i] != b':' {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[bat_dau..self.i])
            .map_err(|_| Error::new("VS0501", "giá trị plugin không UTF-8"))?;
        self.cham()?;
        s.parse::<usize>()
            .map_err(|_| Error::new("VS0501", format!("giá trị plugin: số `{s}` sai")))
    }

    fn hex_n(&mut self, n: usize) -> Result<Vec<u8>> {
        let can = n
            .checked_mul(2)
            .ok_or_else(|| Error::new("VS0501", "giá trị plugin: độ dài tràn"))?;
        let end = self
            .i
            .checked_add(can)
            .ok_or_else(|| Error::new("VS0501", "giá trị plugin: độ dài tràn"))?;
        if end > self.b.len() {
            return Err(Error::new("VS0501", "giá trị plugin: hex cụt"));
        }
        let s = std::str::from_utf8(&self.b[self.i..end])
            .map_err(|_| Error::new("VS0501", "giá trị plugin: hex không UTF-8"))?;
        let ra = giai_hex(s).ok_or_else(|| Error::new("VS0501", "giá trị plugin: hex sai"))?;
        self.i = end;
        Ok(ra)
    }

    fn chuoi(&mut self) -> Result<String> {
        let n = self.so_den_cham()?;
        let b = self.hex_n(n)?;
        String::from_utf8(b).map_err(|_| Error::new("VS0501", "giá trị plugin: chuỗi không UTF-8"))
    }

    fn gia_tri(&mut self) -> Result<PluginValue> {
        if self.sau >= MAX_SAU {
            return Err(Error::new(
                "VS0501",
                format!("giá trị plugin: lồng quá sâu (> {MAX_SAU} cấp)"),
            ));
        }
        self.sau += 1;
        let kq = self.gia_tri_sau();
        self.sau -= 1;
        kq
    }

    fn gia_tri_sau(&mut self) -> Result<PluginValue> {
        let tag = self.ky_tu()?;
        self.cham()?;
        match tag {
            b'u' => Ok(PluginValue::Unit),
            b'b' => {
                let x = self.ky_tu()?;
                self.cham()?;
                match x {
                    b'1' => Ok(PluginValue::Bool(true)),
                    b'0' => Ok(PluginValue::Bool(false)),
                    _ => Err(Error::new("VS0501", "giá trị plugin: bool sai")),
                }
            }
            b'i' => {
                let bat_dau = self.i;
                while self.i < self.b.len() && self.b[self.i] != b':' {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[bat_dau..self.i])
                    .map_err(|_| Error::new("VS0501", "giá trị plugin: int không UTF-8"))?;
                self.cham()?;
                s.parse::<i64>()
                    .map(PluginValue::Int)
                    .map_err(|_| Error::new("VS0501", format!("giá trị plugin: int `{s}` sai")))
            }
            b'f' => {
                let bat_dau = self.i;
                while self.i < self.b.len() && self.b[self.i] != b':' {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[bat_dau..self.i])
                    .map_err(|_| Error::new("VS0501", "giá trị plugin: float không UTF-8"))?;
                self.cham()?;
                s.parse::<f64>()
                    .map(PluginValue::Float)
                    .map_err(|_| Error::new("VS0501", format!("giá trị plugin: float `{s}` sai")))
            }
            b's' => self.chuoi().map(PluginValue::Str),
            b'y' => {
                let n = self.so_den_cham()?;
                self.hex_n(n).map(PluginValue::Bytes)
            }
            b'l' => {
                let n = self.so_den_cham()?;
                let mut xs = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    xs.push(self.gia_tri()?);
                }
                Ok(PluginValue::List(xs))
            }
            b'm' => {
                let n = self.so_den_cham()?;
                let mut kv = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    let k = self.gia_tri()?;
                    let v = self.gia_tri()?;
                    kv.push((k, v));
                }
                Ok(PluginValue::Map(kv))
            }
            b'o' => {
                let x = self.ky_tu()?;
                self.cham()?;
                match x {
                    b'0' => Ok(PluginValue::Option(None)),
                    b'1' => Ok(PluginValue::Option(Some(Box::new(self.gia_tri()?)))),
                    _ => Err(Error::new("VS0501", "giá trị plugin: option sai")),
                }
            }
            b'r' => {
                let x = self.ky_tu()?;
                self.cham()?;
                match x {
                    b'1' => Ok(PluginValue::Result(Ok(Box::new(self.gia_tri()?)))),
                    b'0' => {
                        let n = self.so_den_cham()?;
                        let b = self.hex_n(n)?;
                        let msg = String::from_utf8(b)
                            .map_err(|_| Error::new("VS0501", "giá trị plugin: lỗi không UTF-8"))?;
                        Ok(PluginValue::Result(Err(Box::new(Error::io(msg)))))
                    }
                    _ => Err(Error::new("VS0501", "giá trị plugin: result sai")),
                }
            }
            b'c' => {
                let kind = self.chuoi()?;
                let root = self.chuoi()?;
                let w = self.ky_tu()?;
                self.cham()?;
                let write = match w {
                    b'1' => true,
                    b'0' => false,
                    _ => return Err(Error::new("VS0501", "giá trị plugin: cap sai")),
                };
                Ok(PluginValue::Cap { kind, root, write })
            }
            _ => Err(Error::new(
                "VS0501",
                format!("giá trị plugin: tag lạ `{}`", tag as char),
            )),
        }
    }
}

fn ma_hex(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 0x0f) as usize] as char);
    }
    s
}

fn giai_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let b = s.as_bytes();
    let mut ra = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        ra.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(ra)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_tron_vong_va_sai_thi_loi() {
        let ds = vec![
            PluginValue::Unit,
            PluginValue::Bool(true),
            PluginValue::Int(-7),
            PluginValue::Float(1.5),
            PluginValue::Str("xin chào\nwasi".into()),
            PluginValue::Bytes(vec![0, 255, 16]),
            PluginValue::List(vec![PluginValue::Int(1), PluginValue::Str("a".into())]),
            PluginValue::Map(vec![(PluginValue::Str("k".into()), PluginValue::Int(9))]),
            PluginValue::Option(Some(Box::new(PluginValue::Int(3)))),
            PluginValue::Option(None),
            PluginValue::Result(Ok(Box::new(PluginValue::Str("ok".into())))),
            PluginValue::Cap {
                kind: "Dir".into(),
                root: "/tmp/x".into(),
                write: true,
            },
        ];
        for v in ds {
            let s = ma_hoa(&v);
            assert_eq!(giai_ma(&s).expect("giải mã"), v, "codec: {s}");
        }
        for sai in ["", "x:", "i:abc:", "s:2:61", "l:2:i:1:", "c:1:44"] {
            assert!(giai_ma(sai).is_err(), "phải lỗi: `{sai}`");
        }
    }

    /// **ST-314a (W3 M1)**: input lồng sâu không được làm host stack overflow.
    #[test]
    fn giai_ma_tu_choi_long_qua_sau() {
        let mut s = String::new();
        for _ in 0..1000 {
            s.push_str("l:1:");
        }
        s.push_str("u:");
        let e = giai_ma(&s).expect_err("phải lỗi");
        assert!(format!("{e}").contains("lồng quá sâu"), "thông điệp: {e}");
    }

    #[test]
    fn manifest_hop_le_va_thieu_ok() {
        let m = doc_manifest("module=mau\nabi=1\nfunc=cong;int,int;int\ncap=Dir;ro\nok\n")
            .expect("manifest");
        assert_eq!(m.module, "mau");
        assert_eq!(m.funcs[0].params, vec!["int", "int"]);
        assert_eq!(m.caps, vec![("Dir".to_string(), false)]);
        assert!(doc_manifest("module=mau\nabi=1\n").is_err());
    }
}
