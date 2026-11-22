// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-101b (VD-113)** — escape chuỗi JSON **một nguồn** cho `check --json` + `test --json`:
//! `"`→`\"`, `\`→`\\`, `\b`/`\f`/`\n`/`\r`/`\t`, ký tự `< 0x20` ⇒ `\u00XX`; **giữ UTF-8 thô**
//! (tiếng Việt/emoji) — hợp lệ JSON, không đổi giá trị sau parse. Không dùng serde.

/// Escape một chuỗi thành **nội dung** trong dấu `"` của JSON (không thêm dấu bao).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use vis_interp::json;
    use vis_interp::value::Value;

    fn giai_ma(s: &str) -> String {
        let body = format!("{{\"m\":\"{}\"}}", esc(s));
        let v = json::decode(&body).expect("JSON phải hợp lệ");
        let Value::Map(kv) = v else {
            panic!("không phải map")
        };
        kv.iter()
            .find_map(|(k, v)| match (k, v) {
                (Value::Str(k), Value::Str(v)) if k == "m" => Some(v.clone()),
                _ => None,
            })
            .expect("thiếu m")
    }

    /// (a) `"` + `\` + `\b` + `\f` ⇒ parse được, khôi phục **đúng từng ký tự**.
    #[test]
    fn quote_va_backslash_roundtrip() {
        let goc = "a\"b\\c\u{8}\u{c}";
        assert_eq!(giai_ma(goc), goc);
    }

    /// (b) newline/tab/CR/control `< 0x20` ⇒ parse được, khôi phục đúng.
    #[test]
    fn newline_tab_control_roundtrip() {
        let goc = "dòng1\n\t\r\u{1}\u{1f}";
        assert_eq!(giai_ma(goc), goc);
    }

    /// (c) UTF-8 **thô**: tiếng Việt + emoji không bị escape, parse khôi phục đúng.
    #[test]
    fn utf8_tho_giu_nguyen() {
        let goc = "Việt 😀 日本語";
        assert_eq!(giai_ma(goc), goc);
        assert!(!esc(goc).contains("\\u"), "không được escape UTF-8");
    }
}

/// **ST-125 (VD-130e)**: fuzz **100 000** chuỗi cho [`esc`], đối chiếu **ba phía** — chuỗi gốc,
/// `json.decode(esc(s))` (`vis_interp::json::decode`, đúng hàm `std.json.decode` gọi), và
/// `python3 json.loads` trên cùng chuỗi đã escape. Lệch nào cũng là **lỗi của `esc`** ⇒ `lech = 0`.
///
/// Seed **cố định** [`SEED`]; bộ sinh **tất định** (xorshift64*) nên chạy lại cho cùng kết quả.
#[cfg(test)]
mod fuzz_100k {
    use super::*;
    use vis_interp::json;
    use vis_interp::value::Value;

    /// Seed cố định của bộ sinh (ghi vào log của lần chạy).
    pub const SEED: u64 = 20261115;
    /// Số chuỗi mỗi lần chạy (VD-130e: 100 000).
    pub const SO_CA: usize = 100_000;

    struct Rng(u64);

    impl Rng {
        fn moi(seed: u64) -> Self {
            Rng(seed | 1)
        }
        fn tiep(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn duoi(&mut self, n: u64) -> u64 {
            self.tiep() % n
        }
        fn chon(&mut self, ds: &[char]) -> char {
            ds[self.duoi(ds.len() as u64) as usize]
        }
    }

    /// Bảng ký tự "khó": điều khiển, `U+0000`, `U+2028`/`U+2029`, ngoài BMP, `"`, `\`, và Unicode thường.
    fn bang() -> Vec<char> {
        let mut v: Vec<char> = Vec::new();
        for c in 0u32..0x20 {
            if let Some(c) = char::from_u32(c) {
                v.push(c);
            }
        }
        v.push(char::from_u32(0x7f).expect("DEL"));
        v.extend(['"', '\\', '/', '\u{8}', '\u{c}', '\u{2028}', '\u{2029}']);
        v.extend(['a', 'Z', '0', ' ', 'ế', 'ữ', '日', '本']);
        // Ngoài BMP: emoji + chữ tượng hình (surrogate pair trong UTF-16, 4 byte UTF-8).
        v.extend([
            '\u{1F600}',
            '\u{1F1FB}',
            '\u{1F1F3}',
            '\u{20BB7}',
            '\u{10FFFF}',
        ]);
        v
    }

    /// Khôi phục chuỗi từ **nội dung đã escape** qua `json.decode` (đúng đường của `std.json.decode`).
    fn giai_ma(e: &str) -> String {
        let body = format!("{{\"m\":\"{e}\"}}");
        let v = json::decode(&body).expect("JSON phải hợp lệ");
        let Value::Map(kv) = v else {
            panic!("không phải map")
        };
        kv.iter()
            .find_map(|(k, v)| match (k, v) {
                (Value::Str(k), Value::Str(v)) if k == "m" => Some(v.clone()),
                _ => None,
            })
            .expect("thiếu m")
    }

    /// Sinh ca thứ `i` theo **7 họ** phủ đủ yêu cầu VD-130e + chuỗi ngẫu nhiên.
    fn sinh(rng: &mut Rng, i: usize, bang: &[char]) -> String {
        let dieu_khien: Vec<char> = bang
            .iter()
            .copied()
            .filter(|c| (*c as u32) < 0x20)
            .collect();
        let ngoai_bmp: Vec<char> = bang
            .iter()
            .copied()
            .filter(|c| (*c as u32) >= 0x1_0000)
            .collect();
        match i % 7 {
            // (1) ký tự điều khiển
            0 => (0..1 + rng.duoi(8))
                .map(|_| rng.chon(&dieu_khien))
                .collect(),
            // (2) `U+0000` lẫn trong chuỗi thường
            1 => {
                let mut s = String::new();
                for _ in 0..1 + rng.duoi(6) {
                    if rng.duoi(3) == 0 {
                        s.push('\u{0}');
                    } else {
                        s.push(rng.chon(bang));
                    }
                }
                s
            }
            // (3) `U+2028`/`U+2029` (dấu phân cách dòng Unicode)
            2 => {
                let mut s = String::new();
                for _ in 0..1 + rng.duoi(6) {
                    match rng.duoi(3) {
                        0 => s.push('\u{2028}'),
                        1 => s.push('\u{2029}'),
                        _ => s.push_str("ab"),
                    }
                }
                s
            }
            // (4) ngoài BMP (emoji) + chữ thường
            3 => {
                let mut s = String::new();
                for _ in 0..1 + rng.duoi(6) {
                    if rng.duoi(2) == 0 {
                        s.push(rng.chon(&ngoai_bmp));
                    } else {
                        s.push(rng.chon(bang));
                    }
                }
                s
            }
            // (5) `"` và `\` dày đặc (đúng chỗ escape dễ sai)
            4 => (0..1 + rng.duoi(10))
                .map(|_| if rng.duoi(2) == 0 { '"' } else { '\\' })
                .collect(),
            // (6) chuỗi ngẫu nhiên hoàn toàn
            5 => {
                let n = rng.duoi(200) as usize;
                (0..n).map(|_| rng.chon(bang)).collect()
            }
            // (7) biên: rỗng, một ký tự, chuỗi dài
            _ => match rng.duoi(3) {
                0 => String::new(),
                1 => rng.chon(bang).to_string(),
                _ => (0..300 + rng.duoi(300)).map(|_| rng.chon(bang)).collect(),
            },
        }
    }

    /// Escaper **cố ý hỏng** (chỉ dùng trong ca đối chứng): quên escape `U+0000`..`U+001F` còn lại và
    /// quên `\\` ⇒ phép so ba phía **phải** phát hiện. Chứng minh bộ fuzz **không** "xanh vô nghĩa".
    fn esc_hong(s: &str) -> String {
        let mut o = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\n' => o.push_str("\\n"),
                c => o.push(c),
            }
        }
        o
    }

    /// **Đối chứng (ca phá)**: cùng bộ sinh, escaper hỏng ⇒ **phải** có ≥1 ca lệch/bị từ chối.
    #[test]
    fn doi_chung_escaper_hong_bi_phat_hien() {
        let mut rng = Rng::moi(SEED);
        let bang = bang();
        let mut phat_hien = 0usize;
        for i in 0..SO_CA {
            let goc = sinh(&mut rng, i, &bang);
            let e = esc_hong(&goc);
            let body = format!("{{\"m\":\"{e}\"}}");
            let hong = match json::decode(&body) {
                Err(_) => true,
                Ok(v) => {
                    let Value::Map(kv) = v else {
                        panic!("không phải map")
                    };
                    let lai = kv
                        .iter()
                        .find_map(|(k, v)| match (k, v) {
                            (Value::Str(k), Value::Str(v)) if k == "m" => Some(v.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    lai != goc
                }
            };
            if hong {
                phat_hien += 1;
            }
        }
        assert!(
            phat_hien > 0,
            "bộ fuzz KHÔNG phát hiện escaper hỏng ⇒ fuzz vô nghĩa"
        );
    }

    /// **Ca chính**: 100 000 chuỗi × 2 đường (`json.decode` + `python3 json.loads`) ⇒ `lech = 0`.
    #[test]
    fn fuzz_100k_khop_ba_phia() {
        let mut rng = Rng::moi(SEED);
        let bang = bang();
        let mut ca: Vec<(String, String)> = Vec::with_capacity(SO_CA);
        let mut lech = 0usize;
        let mut vi_du = String::new();
        for i in 0..SO_CA {
            let goc = sinh(&mut rng, i, &bang);
            let e = esc(&goc);
            let lai = giai_ma(&e);
            if lai != goc {
                lech += 1;
                if vi_du.is_empty() {
                    vi_du = format!("ca {i}: goc={goc:?} esc={e:?} giai={lai:?}");
                }
            }
            ca.push((goc, e));
        }
        assert_eq!(
            lech, 0,
            "json.decode lệch {lech}/{SO_CA} ca — ví dụ: {vi_du}"
        );

        // ---- Phía thứ ba: `python3 json.loads` (một lần gọi cho cả 100 000 ca) ----
        let d = std::env::temp_dir().join(format!("vis-jsonesc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        let tep_esc = d.join("escaped.txt");
        let tep_goc = d.join("goc.bin");
        let mut s_esc = String::new();
        let mut goc_bin: Vec<u8> = Vec::new();
        for (goc, e) in &ca {
            s_esc.push('"');
            s_esc.push_str(e);
            s_esc.push_str("\"\n");
            goc_bin.extend_from_slice(&(goc.len() as u32).to_le_bytes());
            goc_bin.extend_from_slice(goc.as_bytes());
        }
        std::fs::write(&tep_esc, &s_esc).expect("ghi escaped");
        std::fs::write(&tep_goc, &goc_bin).expect("ghi goc");

        // ---- Đường thứ hai của bộ escape (`to json` dùng `json_tai`): cũng phải khớp `python3`. ----
        let mut s_enc = String::new();
        let mut lech_enc = 0usize;
        for (goc, _) in &ca {
            let mut out = String::new();
            vis_interp::stdlib::json(&Value::Str(goc.clone()), false, &mut out);
            let khop = match json::decode(&out) {
                Ok(Value::Str(x)) => x == *goc,
                _ => false,
            };
            if !khop {
                lech_enc += 1;
            }
            s_enc.push_str(&out);
            s_enc.push('\n');
        }
        assert_eq!(
            lech_enc, 0,
            "json.encode (json_tai) lệch {lech_enc}/{SO_CA} ca"
        );
        let tep_enc = d.join("encoded.txt");
        std::fs::write(&tep_enc, &s_enc).expect("ghi encoded");

        let py = std::process::Command::new("python3")
            .arg("-c")
            .arg(
                "import json,struct,sys\n\
                 esc,goc,enc=sys.argv[1],sys.argv[2],sys.argv[3]\n\
                 b=open(goc,'rb').read()\n\
                 pos=0;n=0;lech=0\n\
                 with open(esc,encoding='utf-8') as f:\n\
                 \x20   for line in f:\n\
                 \x20       k=struct.unpack_from('<I',b,pos)[0];pos+=4\n\
                 \x20       s=b[pos:pos+k].decode('utf-8');pos+=k\n\
                 \x20       if json.loads(line.rstrip('\\n'))!=s: lech+=1\n\
                 \x20       n+=1\n\
                 with open(enc,encoding='utf-8') as f2:\n\
                 \x20   pos=0;m=0;lech2=0\n\
                 \x20   for line in f2:\n\
                 \x20       k=struct.unpack_from('<I',b,pos)[0];pos+=4\n\
                 \x20       s=b[pos:pos+k].decode('utf-8');pos+=k\n\
                 \x20       if json.loads(line.rstrip('\\n'))!=s: lech2+=1\n\
                 \x20       m+=1\n\
                 print('python3 ca=%d lech=%d ca_encode=%d lech_encode=%d'%(n,lech,m,lech2))\n",
            )
            .arg(&tep_esc)
            .arg(&tep_goc)
            .arg(&tep_enc)
            .output()
            .expect("gọi python3 (cần python3 trong PATH)");
        let out = String::from_utf8_lossy(&py.stdout).to_string();
        let loi = String::from_utf8_lossy(&py.stderr).to_string();
        eprintln!("ST-125 fuzz: seed={SEED} ca={SO_CA} json_decode_lech=0 · {out}");
        assert!(
            py.status.success(),
            "python3 lỗi ({}): {loi}",
            py.status.code().unwrap_or(-1)
        );
        assert_eq!(
            out.trim(),
            format!("python3 ca={SO_CA} lech=0 ca_encode={SO_CA} lech_encode=0"),
            "python3 json.loads lệch: {out}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
