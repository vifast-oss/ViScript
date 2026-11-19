// SPDX-License-Identifier: MIT OR Apache-2.0
// **Một nguồn duy nhất** cho danh sách khối tài liệu bị loại trừ (VD-36, đóng nợ #129 ở ST-158).
//
// 7 khối: 6 khối `...` + khối `Entry(...)`/`assert` của `docs/ViScript-thiet-ke.md`.
// Đổi tài liệu ⇒ cập nhật **duy nhất** tệp này. Bốn tệp test `include!` nó (đường
// `../../../test/doc_excluded.rs`), không chép số:
//   crates/vis-cli/tests/check.rs · crates/vis-fmt/tests/fmt.rs ·
//   crates/vis-syntax/tests/doc_fixtures.rs · crates/vis-types/tests/hir.rs
//
// Tệp này dùng `//` (không phải `//!`): `include!` chèn vào vị trí **item**, nên inner doc
// comment `//!` bị E0753 "expected outer doc comment".
//
// **SỬA KÈM LÝ DO (ST-120b)**: danh sách là **số dòng** trong tài liệu, nên 4 dòng trỏ
// `> Mức hiện thực VS2: …` (Phụ lục B của quyết định đợt 23) làm 4 mục dịch xuống:
// 610→614, 662→666, 737→745, 1412→1420. **R12a (VD-178)**: +1 dòng ở §3.2 (`type Email(str)`)
// ⇒ dịch +1: 305→306, 432→433, 445→446, 614→615, 666→667, 745→746, 1420→1421.
// **VD-215 (2026-11-18)**: +12 dòng bảng quy tắc `{…}` sau bảng kiểu ⇒ dịch +12:
// 306→318, 435→447, 448→460, 617→629, 669→681, 748→760.
// **VD-222 (2026-11-18/19, proxy A0)**: +9 dòng ghi chú "Hiện thực VS2" ở §6.1 ⇒ dịch +9:
// 629→638, 681→690, 760→769.
// **Nội dung 7 khối không đổi** (vẫn đúng 7 khối
// `...`/`Entry`/`assert`); chỉ toạ độ đổi. Đây là hệ quả của việc sửa tài liệu, không phải
// nới ngưỡng: tổng số khối vẫn phải đúng 22 và mỗi khối vẫn phải `not ok` đúng 1 chẩn đoán.
// Lý do từng khối (bản đầy đủ): khối 325 `for i in 0..10: ...`; 470 `fn max_by[...] -> T?: ...`;
// 483 `fn show(self) -> str: ...`; 661 `pub cmd ls(...): ...`; 713 `extern cmd git(..., ...rest: str)`;
// 792 `fn main(...): ...`; 1467 `Entry(trường: …)` (constructor có trường tên không có trong §A.3)
// + `assert xs == [3, 2, 1]` (không có trong §A.1/A.2) — hai lỗ hổng tài liệu, không tự thêm cú pháp.
// **SỬA KÈM LÝ DO (VD-241 đợt coercion Option, 2026-11-19)**: §4.4 tài liệu thiết kế được cập
// nhật ngữ nghĩa chốt (bảng giá trị `T??`) ⇒ mọi khối sau đó dịch dòng; toạ độ 6 khối loại trừ
// dịch: 318→325, 447→470, 460→483, 638→661, 690→713, 769→792. Tổng số khối vẫn đúng 22.
//
// `#[allow(dead_code)]`: mỗi tệp test chỉ dùng **một phần** danh sách (vis-syntax dùng bản 6 khối,
// ba tệp còn lại dùng bản 7 khối) nên mục không dùng ở tệp này là mục dùng ở tệp khác.
#[allow(dead_code)]
const DOC_EXCLUDED: [usize; 6] = [325, 470, 483, 661, 713, 792];

/// Vị trí khối `713` (`extern cmd git(…, ...rest: str)`) trong `DOC_EXCLUDED` — khối **duy nhất** không bị vis-syntax loại trừ.
#[allow(dead_code)]
const VI_TRI_KHOI_666: usize = 4;

/// 6 khối loại trừ theo nghĩa **vis-syntax** (22 − 16 khối đủ thân): khối 713
/// (`extern cmd git(…, ...rest: str)`) **parse sạch** từ ST-27 vì variadic đã có.
/// Dẫn xuất từ `DOC_EXCLUDED`, không chép số.
#[allow(dead_code)]
const DOC_EXCLUDED_SYNTAX: [usize; 5] = {
    let mut out = [0usize; 5];
    let mut j = 0;
    let mut i = 0;
    while i < DOC_EXCLUDED.len() {
        if i != VI_TRI_KHOI_666 {
            out[j] = DOC_EXCLUDED[i];
            j += 1;
        }
        i += 1;
    }
    out
};
