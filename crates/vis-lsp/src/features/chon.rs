// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/selectionRange` (chuỗi chọn tối thiểu, không smart-select CST).
//!
//! Mỗi vị trí (0-based UTF-16) trả một chuỗi lồng nhau, trong → ngoài:
//! token định danh (`vis_syntax::lex`, không có ⇒ range rỗng) → dòng vật lý → khai báo **nhỏ
//! nhất** trong `KetQua.ky_hieu` bao vị trí → toàn tệp. Range trùng **liền nhau** được gộp;
//! `parent` luôn bao `range` con (nên khai báo một dòng nằm trong dòng vật lý sẽ đứng trước dòng).

use vis_core::value::Value;
use vis_syntax::{lex, Kind, Mode, Token};

use super::jsonrpc::{m, phan_hoi};
use super::ky_hieu::KyHieu;
use super::phan_tich::KetQua;
use super::vi_tri::{byte_tu_vi_tri, vi_tri_utf16};

/// Một range LSP `(line, character)` 0-based, hai đầu **bao gồm** theo nghĩa LSP.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Khoang {
    dau: (u32, u32),
    cuoi: (u32, u32),
}

impl Khoang {
    fn moi(dau: (u32, u32), cuoi: (u32, u32)) -> Self {
        Self { dau, cuoi }
    }

    /// Range rỗng tại một điểm.
    fn diem(p: (u32, u32)) -> Self {
        Self { dau: p, cuoi: p }
    }

    /// `true` nếu `self` bao `khac` (range rỗng ở biên vẫn được bao).
    fn bao(&self, khac: &Self) -> bool {
        self.dau <= khac.dau && khac.cuoi <= self.cuoi
    }

    /// Độ dài tuyến tính để chọn range nhỏ nhất (dòng << 32 | cột).
    fn do_dai(&self) -> u128 {
        let a = ((self.dau.0 as u128) << 32) | self.dau.1 as u128;
        let b = ((self.cuoi.0 as u128) << 32) | self.cuoi.1 as u128;
        b - a
    }

    fn json(&self) -> Value {
        let diem = |p: (u32, u32)| {
            m(vec![
                ("line", Value::Int(p.0 as i64)),
                ("character", Value::Int(p.1 as i64)),
            ])
        };
        m(vec![("start", diem(self.dau)), ("end", diem(self.cuoi))])
    }
}

/// Token `Ident` tại `byte` (hoặc kết thúc đúng tại `byte`, để con trỏ ngay sau từ vẫn chọn từ).
fn token_tai(tokens: &[Token], text: &str, byte: usize) -> Option<Khoang> {
    let mut sat = None;
    for t in tokens {
        if t.kind != Kind::Ident {
            continue;
        }
        if t.span.start <= byte && byte < t.span.end {
            return Some(Khoang::moi(
                vi_tri_utf16(text, t.span.start),
                vi_tri_utf16(text, t.span.end),
            ));
        }
        if t.span.start < byte && t.span.end == byte {
            sat = Some(t);
        }
    }
    sat.map(|t| {
        Khoang::moi(
            vi_tri_utf16(text, t.span.start),
            vi_tri_utf16(text, t.span.end),
        )
    })
}

/// Dòng vật lý chứa `byte`: `(0, …)` tới hết dòng (không tính `\n`).
fn khoang_dong(text: &str, byte: usize) -> Khoang {
    let (line, _) = vi_tri_utf16(text, byte);
    let dau = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    let cuoi = text[byte..].find('\n').map_or(text.len(), |i| byte + i);
    let rong = text[dau..cuoi]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum::<u32>();
    Khoang::moi((line, 0), (line, rong))
}

/// Khai báo **nhỏ nhất** bao `byte`; duyệt con trước để trường/variant thắng type/enum.
fn khai_bao_nho_nhat(ds: &[KyHieu], byte: usize) -> Option<&KyHieu> {
    fn duyet<'a>(k: &'a KyHieu, byte: usize, sau: usize, tot: &mut Option<(&'a KyHieu, usize)>) {
        if k.span.start <= byte && byte <= k.span.end {
            let thay = match tot {
                None => true,
                Some((cu, sau_cu)) => {
                    k.span.len() < cu.span.len() || (k.span.len() == cu.span.len() && sau > *sau_cu)
                }
            };
            if thay {
                *tot = Some((k, sau));
            }
        }
        for c in &k.con {
            duyet(c, byte, sau + 1, tot);
        }
    }
    let mut tot = None;
    for k in ds {
        duyet(k, byte, 0, &mut tot);
    }
    tot.map(|(k, _)| k)
}

/// Sắp các ứng viên thành chuỗi bao hàm: mỗi bước chọn range **nhỏ nhất** còn lại bao range hiện
/// tại (bỏ range trùng liền nhau). Nhờ vậy `parent` luôn thật sự bao `range` con.
fn xep_chuoi(mut ds: Vec<Khoang>) -> Vec<Khoang> {
    ds.dedup();
    if ds.is_empty() {
        return Vec::new();
    }
    let mut con = vec![ds[0]];
    let mut con_lai = ds[1..].to_vec();
    while let Some(&hien) = con.last() {
        let mut chon: Option<(usize, Khoang)> = None;
        for (i, k) in con_lai.iter().enumerate() {
            if *k != hien && k.bao(&hien) && chon.is_none_or(|(_, cu)| k.do_dai() < cu.do_dai()) {
                chon = Some((i, *k));
            }
        }
        let Some((i, k)) = chon else { break };
        con.push(k);
        con_lai.remove(i);
    }
    con
}

fn nut_json(k: &Khoang, parent: Option<Value>) -> Value {
    let mut kv = vec![("range", k.json())];
    if let Some(p) = parent {
        kv.push(("parent", p));
    }
    m(kv)
}

/// Chuỗi ngoài→trong thành JSON: node ngoài cùng không có `parent`.
fn chuoi_json(ds: &[Khoang]) -> Value {
    let Some((ngoai, trong)) = ds.split_last() else {
        return Value::none();
    };
    let mut v = nut_json(ngoai, None);
    for k in trong.iter().rev() {
        v = nut_json(k, Some(v));
    }
    v
}

/// Phản hồi `textDocument/selectionRange` — đúng thứ tự `positions` đầu vào.
pub fn tra_loi_selection(kq: &KetQua, mode: Mode, vi_tris: &[(u32, u32)], id: &Value) -> Vec<u8> {
    let lexed = lex(&kq.text, mode);
    let tai_lieu = Khoang::moi((0, 0), vi_tri_utf16(&kq.text, kq.text.len()));
    let ds: Vec<Value> = vi_tris
        .iter()
        .map(|&(line, ch)| {
            let byte = byte_tu_vi_tri(&kq.text, line, ch);
            let token = token_tai(&lexed.tokens, &kq.text, byte)
                .unwrap_or_else(|| Khoang::diem(vi_tri_utf16(&kq.text, byte)));
            let mut ung = vec![token, khoang_dong(&kq.text, byte)];
            if let Some(k) = khai_bao_nho_nhat(&kq.ky_hieu, byte) {
                ung.push(Khoang::moi(
                    vi_tri_utf16(&kq.text, k.span.start),
                    vi_tri_utf16(&kq.text, k.span.end),
                ));
            }
            ung.push(tai_lieu);
            chuoi_json(&xep_chuoi(ung))
        })
        .collect();
    phan_hoi(id, Value::List(ds))
}
