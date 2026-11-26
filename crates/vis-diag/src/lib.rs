// SPDX-License-Identifier: MIT OR Apache-2.0
//! `vis-diag` — khung chẩn đoán của dòng ViScript (ST-01).
//!
//! Vòng 1 mới có **định dạng mã lỗi** `VS` + 4 chữ số (sổ mã đầy đủ thuộc ST-02b).
//! Không `unsafe` (luật dòng ViScript), không phụ thuộc ngoài.
//!
//! **ST-318d**: nội dung tách theo nhóm — `codes` (kiểu nền), `books` (sổ mã), `tra_cuu`
//! (tra cứu), `explain` (trợ giúp i18n), `render` (chẩn đoán + in). Tệp này chỉ giữ doc, khai
//! báo module và `pub use`; đường dẫn `vis_diag::*` **không đổi**.

mod books;
mod codes;
mod explain;
mod render;
mod tra_cuu;

pub use books::{
    all_codes, ma_truong, BLOCKS_RESERVED, BOOKS, CODES, COMMAND_CODES, MODULE_CODES, NAME_CODES,
    RUNTIME_CODES, SYNTAX_CODES, TYPE_CODES,
};
pub use codes::{code, Code, Lang, Severity};
pub use explain::{explain_help, severity_label, ExplainHelp};
pub use render::{render, Diag, DiagLine};
pub use tra_cuu::{lookup, lookup_str, next_block};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ma_co_bon_chu_so() {
        assert_eq!(code(1), format!("VS{:04}", 1));
        assert_eq!(code(9999), "VS9999");
    }

    #[test]
    fn so_ma_lien_mach_va_dung_spec_lex() {
        let ids: Vec<u16> = CODES.iter().map(|c| c.id).collect();
        assert_eq!(ids.first(), Some(&101));
        // **R1a (VD-178)**: +**VS0111** (cảnh báo chữ hoa biến thể) ⇒ **11** mã.
        assert_eq!(ids.last(), Some(&111));
        assert_eq!(ids.len(), 11);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(CODES[i].id, 101 + i as u16, "sổ phải liền mạch 0101..0111");
            assert_eq!(CODES[i].short, format!("VS{id:04}"));
            assert!(!CODES[i].message_vi.is_empty());
        }
    }

    #[test]
    fn tra_ma_va_ma_la() {
        assert_eq!(lookup(103).map(|c| c.short), Some("VS0103"));
        // **R1a (VD-178)**: `VS0111` **đã** cấp ⇒ mã lạ là `VS0112`.
        assert!(lookup(112).is_none());
        assert_eq!(
            lookup_str("VS0107").map(|c| c.severity),
            Some(Severity::Warning)
        );
        assert_eq!(lookup_str("vs0107").map(|c| c.id), Some(107));
        assert!(lookup_str("VS010").is_none());
        assert!(lookup_str("XX0101").is_none());
    }

    #[test]
    fn khoi_ke_tiep_khong_tai_dung() {
        // 01xx (từ vựng) + 02xx (cú pháp) + 03xx (tên, VD-42) đã cấp, và 04xx/05xx đã **dành
        // trước** (VD-27) ⇒ khối kế là 06xx. Sửa kỳ vọng cũ 301 kèm lý do: VD-42 cấp thêm `03xx`
        // và dành trước `04xx`/`05xx`; `next_block` nay suy ra từ khối, không đếm mã.
        assert_eq!(
            next_block(),
            801,
            // **SỬA KÈM LÝ DO (ST-25)**: `07xx` đã cấp 3 mã module (`VS0701..0703`) ⇒ khối kế `08xx` (801).
            "01xx..07xx đã cấp/dành trước ⇒ khối kế 08xx (801)"
        );
    }

    #[test]
    fn so_runtime_05xx_khoa_theo_vd42() {
        let ids: Vec<u16> = RUNTIME_CODES.iter().map(|c| c.id).collect();
        // **SỬA KÈM LÝ DO (ST-60)**: thêm **VS0506** ở **cuối** (index 0..2 giữ nguyên) cho lỗi
        // trong thân tác vụ `async`. **SỬA KÈM LÝ DO (ST-61)**: thêm **VS0504** (gửi sau `close`)
        // và **VS0505** (deadlock/khoá) — vẫn ở cuối khối, index 0..2 giữ nguyên.
        assert_eq!(
            ids,
            vec![501, 502, 503, 504, 505, 506, 507, 508, 509, 510],
            "sổ runtime đúng thứ tự tăng dần (R9: +VS0507 tràn i64, +VS0508 không Sendable; \
             ST-290/VD-273: +VS0509 ghi chéo worker, +VS0510 vượt ngân sách ngữ nghĩa)"
        );
        for c in RUNTIME_CODES.iter() {
            assert_eq!(lookup(c.id).map(|x| x.short), Some(c.short));
            assert_eq!(lookup_str(c.short).map(|x| x.id), Some(c.id));
            for lang in [Lang::Vi, Lang::En, Lang::Ja] {
                assert!(!c.message(lang).is_empty(), "{} thiếu {:?}", c.short, lang);
            }
        }
        assert!(
            RUNTIME_CODES[2].message_vi.contains("ngoài phạm vi"),
            "VS0503 nói 'ngoài phạm vi'"
        );
    }

    #[test]
    fn so_kieu_04xx_khoa_theo_vd42() {
        let ids: Vec<u16> = TYPE_CODES.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![401, 402, 403, 405, 412, 413, 414, 415, 416],
            "sổ kiểu đúng thứ tự tăng dần (ST-50: VS0405 trước VS0412; R9: +VS0413..0416)"
        );
        // **SỬA KÈM LÝ DO (ST-50, Agent 0: VS0605 sai sổ — trường là lỗi kiểu)**: VS0605 đổi tên
        // thành **VS0405** và chuyển sang sổ này ⇒ 0412 **dời xuống index 4**.
        assert_eq!(TYPE_CODES[3].short, "VS0405");
        assert_eq!(
            TYPE_CODES[3].message_vi,
            "không có trường này trên kiểu bản ghi (trường không tồn tại)",
            "VS0405 nói rõ trường không có"
        );
        assert_eq!(TYPE_CODES[4].short, "VS0412");
        assert_eq!(
            TYPE_CODES[4].message_vi, "kiểu không khớp",
            "VS0412 giữ đúng nghĩa thiết kế §9"
        );
        for c in TYPE_CODES.iter() {
            assert_eq!(lookup(c.id).map(|x| x.short), Some(c.short));
            assert_eq!(lookup_str(c.short).map(|x| x.id), Some(c.id));
            for lang in [Lang::Vi, Lang::En, Lang::Ja] {
                assert!(!c.message(lang).is_empty(), "{} thiếu {:?}", c.short, lang);
            }
        }
        assert!(
            TYPE_CODES[1].message_vi.contains("xét hết"),
            "VS0402 phải nói 'xét hết'"
        );
    }

    #[test]
    fn so_ten_03xx_khoa_theo_vd42() {
        let ids: Vec<u16> = NAME_CODES.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![301, 302, 303, 304]);
        assert_eq!(NAME_CODES[0].short, "VS0301");
        assert_eq!(NAME_CODES[1].short, "VS0302");
        assert_eq!(NAME_CODES[3].short, "VS0304");
        // **SỬA KÈM LÝ DO (ST-18)**: ST-18 cấp sổ thứ **sáu** `06xx` (4 mã chế độ lệnh) ⇒
        // 10 + 2 + 2 + 4 + 3 + 4 = **25** mã, **6** sổ; `lookup(601)` nay **có**, mã chưa cấp là `701`.
        // **SỬA KÈM LÝ DO (ST-25)**: sổ thứ bảy `07xx` (module) đã cấp ⇒ 25 + 3 = **28** mã,
        // `BOOKS` = **7**; khối kế tiếp là `801` (xem `next_block` bên dưới).
        // **SỬA KÈM LÝ DO (ST-50, Agent 0: VS0605 sai sổ — trường là lỗi kiểu)**: VS0605 đổi tên
        // thành **VS0405** và **chuyển sang `TYPE_CODES`** ⇒ `06xx` 5 → **4**, `04xx` 4 → **5**,
        // `all_codes()` **vẫn 29**; `next_block()` vẫn **801**.
        // (ST-40a cũ: thêm VS0605 `06xx` trường không có trên `it` ⇒ 28 + 1 = **29** mã.)
        // **SỬA KÈM LÝ DO (ST-60)**: +**VS0506** ⇒ **30** mã (sổ runtime 3 → 4).
        // **SỬA KÈM LÝ DO (ST-61)**: +**VS0504**/**VS0505** (kênh: gửi sau close, deadlock) ⇒ **32**
        // mã (sổ runtime 4 → **6**, thêm **ở cuối** nên index 0..3 không đổi).
        // **SỬA KÈM LÝ DO (ST-128, VD-125)**: +**VS0704** ("không giải được ràng buộc gói", khối
        // `07xx`, số **kế tiếp** của khối) ⇒ **33** mã (sổ module 3 → **4**, thêm **ở cuối** nên
        // index 0..2 không đổi); `BOOKS` vẫn **7**; `next_block()` vẫn **801** (không đổi).
        // **R9 (VD-178)**: +16 mã (VS0203..0206, VS0303, VS0413..0416, VS0507/0508,
        // VS0605/0606, VS0705..0707) ⇒ **49** mã; `BOOKS` vẫn **7**; `next_block()` vẫn **801**.
        // **R1a (VD-178)**: +**VS0111** (cảnh báo chữ hoa biến thể) ⇒ **50** mã.
        // **VD-215 (2026-11-18)**: +**VS0207**/**VS0208** ⇒ **52** mã.
        // **VD-223 (proxy A0)**: +**VS0211** ⇒ 53; **VD-224 (proxy A0)**: huỷ VS0209.
        // **ST-288 (VD-273)**: +**VS0708** (chữ ký/khoá tin cậy registry) ⇒ **54** mã.
        // **ST-290 (VD-273)**: +**VS0509**/**VS0510** ⇒ **56** mã.
        // **ST-315d (VD-291)**: +**VS0304** (chu trình `const`) ⇒ **57** mã.
        assert_eq!(all_codes().count(), 57);
        assert_eq!(BOOKS.len(), 7);
        for id in [601u16, 602, 603, 604] {
            assert!(
                lookup(id).is_some(),
                "mã {id} phải có ở ST-18 (khối `06xx`)"
            );
        }
        // **SỬA KÈM LÝ DO (ST-25)**: `07xx` nay đã cấp cho module (`VS0701/0702/0703`).
        for id in [701u16, 702, 703, 704] {
            assert!(
                lookup(id).is_some(),
                "mã {id} phải có ở ST-25/ST-128 (khối `07xx` module)"
            );
        }
        assert!(
            lookup(801).is_none(),
            "mã 801 chưa cấp — khối `08xx` là của vòng sau"
        );
        // Khối đã dành trước vẫn chặn `next_block` rơi vào 01xx..07xx.
        assert_eq!(BLOCKS_RESERVED, [1, 2, 3, 4, 5, 6, 7]);
        for c in NAME_CODES.iter() {
            for lang in [Lang::Vi, Lang::En, Lang::Ja] {
                assert!(!c.message(lang).is_empty(), "{} thiếu {:?}", c.short, lang);
            }
        }
    }

    #[test]
    fn so_cu_phap_02xx_khoa_theo_vd35() {
        // Sổ cú pháp riêng: 6 mã (R9: +VS0203..0206), đúng thứ tự, đúng nghĩa, `CODES` KHÔNG bị đụng.
        let ids: Vec<u16> = SYNTAX_CODES.iter().map(|c| c.id).collect();
        // **VD-224 (uỷ quyền A0-proxy)**: huỷ VS0209 ⇒ ids [201..208, 211].
        assert_eq!(ids, vec![201, 202, 203, 204, 205, 206, 207, 208, 211]);
        assert_eq!(SYNTAX_CODES[0].short, "VS0201");
        assert_eq!(SYNTAX_CODES[1].short, "VS0202");
        assert_eq!(SYNTAX_CODES[2].short, "VS0203");
        assert_eq!(SYNTAX_CODES[3].short, "VS0204");
        assert_eq!(SYNTAX_CODES[4].short, "VS0205");
        assert_eq!(SYNTAX_CODES[5].short, "VS0206");
        assert_eq!(SYNTAX_CODES[6].short, "VS0207");
        assert_eq!(SYNTAX_CODES[7].short, "VS0208");
        assert_eq!(SYNTAX_CODES[8].short, "VS0211");
        assert_eq!(SYNTAX_CODES[7].severity, Severity::Warning);
        assert_eq!(SYNTAX_CODES[8].severity, Severity::Warning);
        assert!(
            SYNTAX_CODES[1].message_vi.contains("50pct"),
            "VS0202 phải gợi ý `50pct` (L3)"
        );
        // **R1a (VD-178)**: +**VS0111** ⇒ **11** mã từ vựng.
        assert_eq!(CODES.len(), 11, "sổ từ vựng đúng 11 mã của §7");
        // Tra xuyên sổ: `vis explain VS0201` phải chạy.
        // **VD-224**: ids **không liên tục** (209 bị huỷ) — tra theo đúng `id` của từng mã.
        for c in SYNTAX_CODES.iter() {
            assert_eq!(lookup(c.id).map(|x| x.short), Some(c.short));
            assert_eq!(lookup_str(c.short).map(|x| x.id), Some(c.id));
        }
    }

    #[test]
    fn so_khop_spec_lex_that() {
        // Đọc thẳng bảng §7 của SPEC-lex: sổ trong code không được lệch nguồn chuẩn.
        let spec = std::fs::read_to_string("../../docs/SPEC-lex.md")
            .expect("phải đọc được docs/SPEC-lex.md");
        let mut rows: Vec<(String, String)> = Vec::new();
        for line in spec.lines() {
            let t = line.trim();
            if t.starts_with("| VS") {
                let cells: Vec<&str> = t.trim_matches('|').split('|').map(|c| c.trim()).collect();
                if cells.len() >= 2 {
                    rows.push((cells[0].to_string(), cells[1].to_string()));
                }
            }
        }
        assert_eq!(
            rows.len(),
            CODES.len(),
            "SPEC-lex §7 phải có {} hàng",
            CODES.len()
        );
        for (row, code) in rows.iter().zip(CODES.iter()) {
            assert_eq!(row.0, code.short);
            assert_eq!(row.1, code.message_vi);
        }
    }

    #[test]
    fn chan_doan_giu_vi_tri() {
        let d = Diag::new(&CODES[2], "token lạ", 7);
        assert_eq!(d.msg, "token lạ");
        assert_eq!(d.offset, 7);
    }

    /// **ST-316b (`#47`)** — trợ giúp `vis explain` vi/en/ja: **một nguồn** trong `vis-diag`;
    /// chuỗi khớp **nguyên văn** bản cũ ở `vis-cli/src/main.rs` (CLI chỉ in).
    #[test]
    fn explain_help_khop_chuoi_cu() {
        let khoi = BLOCKS_RESERVED.len();
        let ma = all_codes().count();
        let vi = explain_help(Lang::Vi);
        assert_eq!(
            vi.usage,
            "dùng: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]"
        );
        assert_eq!(
            vi.book,
            format!(
                "sổ mã: {khoi} khối, {ma} mã — `vis explain --list` in toàn bộ (mã + câu ngắn); `--json` in một dòng máy đọc"
            )
        );
        let en = explain_help(Lang::En);
        assert_eq!(
            en.usage,
            "usage: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]"
        );
        assert_eq!(
            en.book,
            format!(
                "code book: {khoi} blocks, {ma} codes — `vis explain --list` prints all (code + short text); `--json` prints one machine-readable line"
            )
        );
        let ja = explain_help(Lang::Ja);
        assert_eq!(
            ja.usage,
            "使い方: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]"
        );
        assert_eq!(
            ja.book,
            format!(
                "コード帳: {khoi} ブロック、{ma} コード — `vis explain --list` で全件表示（コード + 短い説明）、`--json` は機械可読な1行"
            )
        );
    }
}
