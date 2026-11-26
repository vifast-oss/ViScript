// SPDX-License-Identifier: MIT OR Apache-2.0
//! Parser **chế độ lệnh** (`Mode::Command`) — ST-06, Phụ lục A.7 + VD-40.
//!
//! Sinh **CmdCST chưa định kiểu** (giống `vis-syntax` giai đoạn vòng 1): từ trần, cờ, biến, glob
//! còn nguyên dạng; `cmd_lower` (vòng 2, VD-22) mới quyết kiểu theo chữ ký lệnh. Cây node dùng cho
//! CST lossless; sexp (quy ước ở `viscript/test/cmd/golden.cmd`) dựng song song để golden đối chiếu.
//!
//! # Chốt theo VD-40
//!
//! 1. Một dòng = `(items <pipeline> …)` khi có `;`; một pipeline trần giữ nguyên `(pipe …)`.
//!    Tệp nhiều dòng = `(shell <item>…)`; tệp **một** dòng ⇒ sexp của dòng đó.
//! 2. `-5` là **từ trần** `(word -5)`; `-l`/`--oneline`/`--` là cờ. `cmd_lower` phân biệt số/cờ.
//! 3. `[ab]` (lexer cắt `[`, `ab`, `]`) được **ghép** thành một `(word [ab])` khi các span **liền
//!    kề**; có khoảng trắng bên trong ⇒ lỗi. List literal chỉ trong `( … )`.
//! 4. `--` là `(flag --)`; `cmd_lower` hiểu là kết thúc cờ.
//! 5. `|>` ở chế độ lệnh ⇒ lỗi (không có luật sinh).
//!
//! Biểu thức trong `( … )` và thân closure được **cắt theo span** rồi parse lại bằng
//! `Mode::Script` (lexer chế độ lệnh không sinh token biểu thức như `+`, `>=`…), nên sexp bên
//! trong dùng đúng quy ước ST-05 (`(field x size)`, `(> a b)`…).
//!
//! **ST-318f** — tách **nguyên văn** theo nhóm (R1 mềm): `phan_loai` (luật let/khai),
//! `file` (toàn tệp + khối khai báo), `dong` (một dòng), `stage` (pipeline/stage),
//! `lenh_let`, `nhom` (`$`/`( … )`), `tu` (từ/cờ/glob), `closure`. Tệp này chỉ còn
//! khai báo module + `pub use`; sexp/CST/API không đổi.

mod closure;
mod dong;
mod file;
mod lenh_let;
mod nhom;
mod phan_loai;
mod stage;
mod tu;

pub use phan_loai::{
    dinh_danh_chua_bind, let_la_khai, let_la_khai_voi, ten_khai_bao, ten_mac_dinh, tham_so_cua,
    than_khai_lenh, than_khai_lenh_voi,
};
