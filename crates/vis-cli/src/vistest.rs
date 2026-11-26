// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`vis test`** (ST-30) — chạy bộ `test/sem/**` (và mọi thư mục được trỏ) in **TAP**.
//!
//! Quy ước bộ ca (giống harness `crates/vis-cli/tests/sem.rs`):
//! - `<tên>.vis`/`.vish` **không** có `<tên>.err` ⇒ phải chạy **exit 0** và `stdout` **byte-khớp** `.out`
//!   (bỏ dòng SPDX đầu của `.out`).
//! - có `<tên>.err` ⇒ phải **đỏ** đúng mã trong `.err` (`VS05xx` ⇒ exit **101**, còn lại ⇒ exit 2).
//!
//! `--vm` chạy **đường VM** (`--no-cache`); không có `--vm` thì chạy interp (mặc định `vis run`, VD-56).
//!
//! **VD-267**:
//! - `--filter <substr>` — chỉ chọn ca có **tên `test fn`** hoặc **đường dẫn hiển thị** của fixture
//!   chứa `<substr>` (phân biệt hoa/thường). Lọc **trước khi chạy** nên ca bị loại không có tác dụng
//!   phụ. Chuỗi rỗng hoặc không ca nào khớp ⇒ TAP hợp lệ `1..0`, exit **0**.
//! - `--fail-fast` — dừng ngay sau ca `not ok` **đầu tiên**. Kế hoạch TAP đếm **ca đã chạy**
//!   (`1..<đã chạy>`), không phải số ca đã chọn; `--json` cũng đếm ca đã chạy. Exit vẫn **1**.
//!
//! **VD-270**:
//! - `--repeat <N>` — chạy mỗi ca **đã chọn** N lần (`N ≥ 1`, mặc định **1**), thứ tự
//!   ca-trước-lặp-sau. `N = 1` giữ **nguyên byte** TAP/JSON cũ; `N > 1` ⇒ mỗi lần lặp là một ca
//!   riêng, tên `"<tên> [k/N]"` (k 1-based), đánh số TAP liên tục, plan `1..<số lần đã chạy>`.
//! - Đỏ ở **bất kỳ** lần lặp nào ⇒ exit **1**; không `--fail-fast` thì giữ **mọi** kết quả lặp.
//!   `--fail-fast` dừng ngay sau lần lặp đỏ đầu tiên (`--json` đếm cùng tập đã chạy).
//! - `--list` bỏ qua `--repeat` (liệt kê ca đã chọn **một lần**) nhưng vẫn kiểm N; `--timeout`
//!   áp **mỗi lần lặp** của **mọi ca** — `test fn` **và** fixture `.out/.err` (mở rộng ST-105:
//!   fixture quá hạn ⇒ kill, `not ok` + `# <tên>: quá hạn <ms> ms`, đếm là đỏ).
//! - `--shuffle [--seed <u64>]` — xáo thứ tự ca **đã chọn**, **sau `--filter`** và **trước**
//!   `--repeat` (vẫn ca-trước-lặp-sau) lẫn `--list` (liệt kê theo thứ tự đã xáo). Cùng seed +
//!   cùng tập chọn ⇒ cùng thứ tự; thiếu `--seed` ⇒ seed **0**; `--seed` không kèm `--shuffle`
//!   ⇒ exit **2**. PRNG **SplitMix64** + Fisher–Yates, không thêm phụ thuộc; không cờ ⇒
//!   nguyên byte thứ tự/TAP/JSON cũ.
//! - `--shard <I>/<N>` — chia ca **đã chọn** (sau `--filter`, sau `--shuffle`) thành `N` phần
//!   rời nhau theo chỉ số 0-based: phần `I` giữ các ca có `chỉ_số % N == I-1` (1 ≤ I ≤ N).
//!   Hợp `N` phần = đúng tập đã chọn, không trùng; `--list`/`--repeat`/`--json` áp **sau** shard.
//!   Cú pháp sai (`0/2`, `3/2`, `1/0`, thiếu `/`, thừa `/`, không nguyên, thiếu giá trị, lặp cờ)
//!   ⇒ exit **2** + usage nêu `--shard`.
//! - `--jobs <N>` — chạy **song song** `N` **đơn vị** (ca đã chọn × lần lặp `k` của `--repeat`;
//!   `N ≥ 1`, mặc định **1**). Kết quả in theo **đúng thứ tự cũ** (chỉ số ca, rồi k) dù worker
//!   xong lệch thứ tự; `--json`/`--json-out` cùng shape/thứ tự. `N = 1` giữ **nguyên byte**
//!   TAP/JSON. `--list` bỏ qua `--jobs` (vẫn kiểm N). Thiếu/`0`/âm/không nguyên/lặp cờ ⇒ exit
//!   **2** + usage nêu `--jobs`.
//! - `--fail-fast` + `--jobs > 1`: ngừng **lập lịch** đơn vị mới ngay sau lần đỏ đầu tiên; đơn
//!   vị **đang chạy** được hoàn tất; in theo thứ tự gốc; plan/cases đếm **đơn vị đã chạy**.
//!   Khác bản tuần tự: vài ca có chỉ số **sau** ca đỏ có thể đã kịp chạy (đã lập lịch trước khi
//!   thấy đỏ) nên plan có thể lớn hơn vị trí ca đỏ.
//! - `--slowest <N>` — đo **thời gian tường** từng **đơn vị đã chạy** (ca × lần lặp; `--fail-fast`
//!   chỉ đơn vị đã chạy), sau TAP/JSON (stdout **không đổi**) in tối đa `N` dòng ra **stderr**,
//!   **giảm dần**, hoà giữ **thứ tự gốc**: `# slowest: <ms> ms - <tên>`, dưới 1 ms dùng
//!   `# slowest: <us> us - <tên>`. `N` nguyên ≥ 1; thiếu/`0`/âm/không nguyên/lặp cờ ⇒ exit **2**
//!   và usage nêu `--slowest`; `--slowest` + `--list` ⇒ exit **2** (không có thời gian chạy).
//!   `--jobs`/`--repeat`/`--shard`/`--filter`/`--shuffle` compose; `--slowest` áp **sau** khi đã
//!   có danh sách đơn vị.

use std::path::{Path, PathBuf};

use vis_syntax::{Kind, Mode, NodeKind, Span};

mod ca;
mod run;
mod ten;
mod tien_trinh;

use ca::*;
use ten::*;
use tien_trinh::*;

pub use run::run;

/// **ST-101b (tạm)**: escape JSON hiện tại — tách để đo đỏ (sau thay bằng `json_esc`).
fn esc_json(s: &str) -> String {
    crate::json_esc::esc(s)
}

/// **ST-101**: thông điệp JSON **một dòng**, cắt ≤200 **ký tự** (char, không byte).
fn mot_dong(m: &str) -> String {
    let s = m.replace(['\n', '\r', '\t'], " ");
    // ≤200 **kể cả** dấu “…” khi cắt.
    if s.chars().count() <= 200 {
        return s;
    }
    let mut o: String = s.chars().take(199).collect();
    o.push('…');
    o
}

/// **ST-101**: ca cho `--json`.
struct CaJson {
    ten: String,
    ok: bool,
    msg: String,
}

/// Bỏ dòng SPDX đầu của tệp kỳ vọng (QD §9.3 — fixture `.out` có header).
fn bo_spdx(s: &str) -> &str {
    match s.split_once('\n') {
        Some((d, rest)) if d.trim_start().starts_with('#') && d.contains("SPDX") => rest,
        _ => s,
    }
}

/// Tìm `test/sem` từ cwd leo lên (chạy được cả trong `crates/vis-cli` và ở gốc repo).
fn tim_mac_dinh() -> Option<PathBuf> {
    let mut d = std::env::current_dir().ok()?;
    loop {
        for ten in ["test/sem", "viscript/test/sem"] {
            let p = d.join(ten);
            if p.is_dir() {
                return Some(p);
            }
        }
        if !d.pop() {
            return None;
        }
    }
}

fn gom(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ds: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    ds.sort();
    for p in ds {
        if p.is_dir() {
            // **VD-258** — cấu hình core không có host `std.sqlite`; bỏ nhóm này thay vì
            // chạy 38 ca chắc chắn đỏ. Cấu hình mặc định (`sqlite`) không đổi.
            #[cfg(not(feature = "sqlite"))]
            if p.file_name().is_some_and(|n| n == "sqlite") {
                continue;
            }
            gom(&p, out);
        } else if matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("vis") | Some("vish")
        ) {
            out.push(p);
        }
    }
}

/// **ST-79 (VD-108)**: tên + span của mọi `test fn` trong tệp, theo thứ tự khai báo (CST).
fn ds_test(text: &str) -> Vec<(String, Span, Span)> {
    let out = vis_syntax::parse(text, Mode::Script);
    let cst = &out.cst;
    let mut ds = Vec::new();
    for n in cst.root.children() {
        if n.kind != NodeKind::TestDecl {
            continue;
        }
        let toks: Vec<&vis_syntax::Token> = n.tokens(&cst.tokens).collect();
        let mut idents = toks.iter().filter(|t| t.kind == Kind::Ident);
        let Some(test_tok) = idents.next() else {
            continue;
        };
        let Some(name_tok) = idents.next() else {
            continue;
        };
        ds.push((name_tok.text.clone(), test_tok.span, name_tok.span));
    }
    ds
}

/// Đổi `test fn <tên>` thành `fn main` (chỉ tên đang chọn); giữ nguyên phần còn lại.
fn thanh_main(text: &str, test_span: Span, name_span: Span) -> String {
    // **VD-241 § fix(Windows/CRLF #psvn-A1)**: span trỏ vào văn bản chuẩn hoá CRLF→LF, slice theo đó.
    let norm = vis_syntax::lex::normalize_source(text);
    let mut s = String::with_capacity(norm.len());
    s.push_str(&norm[..test_span.start]);
    s.push_str("fn main");
    s.push_str(&norm[name_span.end..]);
    s
}

/// **VD-270**: object JSON của `--json`/`--json-out` (một dòng + `\n`).
fn json_body(cases: &[CaJson], hong: usize) -> String {
    let mut ds = String::new();
    for (i, c) in cases.iter().enumerate() {
        if i > 0 {
            ds.push(',');
        }
        ds.push_str(&format!(
            "{{\"name\":\"{}\",\"ok\":{},\"message\":\"{}\"}}",
            esc_json(&c.ten),
            c.ok,
            esc_json(&c.msg)
        ));
    }
    format!(
        "{{\"total\":{},\"ok\":{},\"failed\":{},\"cases\":[{ds}]}}\n",
        cases.len(),
        cases.len().saturating_sub(hong),
        hong
    )
}

/// `vis test` chỉ dùng ở chế độ script/lệnh — giữ tham chiếu để `Mode` không bị bỏ (tài liệu hoá).
#[allow(dead_code)]
fn _mode(_: Mode) {}

#[cfg(test)]
mod tests;
