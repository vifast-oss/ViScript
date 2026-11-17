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
// 610→614, 662→666, 737→745, 1412→1420. **Nội dung 7 khối không đổi** (vẫn đúng 7 khối
// `...`/`Entry`/`assert`); chỉ toạ độ đổi. Đây là hệ quả của việc sửa tài liệu, không phải
// nới ngưỡng: tổng số khối vẫn phải đúng 22 và mỗi khối vẫn phải `not ok` đúng 1 chẩn đoán.
// Lý do từng khối (bản đầy đủ): khối 305 `for i in 0..10: ...`; 432 `fn max_by[...] -> T?: ...`;
// 445 `fn show(self) -> str: ...`; 614 `pub cmd ls(...): ...`; 666 `extern cmd git(..., ...rest: str)`;
// 745 `fn main(...): ...`; 1420 `Entry(trường: …)` (constructor có trường tên không có trong §A.3)
// + `assert xs == [3, 2, 1]` (không có trong §A.1/A.2) — hai lỗ hổng tài liệu, không tự thêm cú pháp.
//
// `#[allow(dead_code)]`: mỗi tệp test chỉ dùng **một phần** danh sách (vis-syntax dùng bản 6 khối,
// ba tệp còn lại dùng bản 7 khối) nên mục không dùng ở tệp này là mục dùng ở tệp khác.
#[allow(dead_code)]
const DOC_EXCLUDED: [usize; 7] = [305, 432, 445, 614, 666, 745, 1420];

/// Vị trí khối `666` trong `DOC_EXCLUDED` — khối **duy nhất** không bị vis-syntax loại trừ.
#[allow(dead_code)]
const VI_TRI_KHOI_666: usize = 4;

/// 6 khối loại trừ theo nghĩa **vis-syntax** (22 − 16 khối đủ thân): khối 666
/// (`extern cmd git(…, ...rest: str)`) **parse sạch** từ ST-27 vì variadic đã có.
/// Dẫn xuất từ `DOC_EXCLUDED`, không chép số.
#[allow(dead_code)]
const DOC_EXCLUDED_SYNTAX: [usize; 6] = {
    let mut out = [0usize; 6];
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
