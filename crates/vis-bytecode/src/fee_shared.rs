// SPDX-License-Identifier: MIT OR Apache-2.0
// **ST-276 / VD-256 — MỘT nguồn định nghĩa sự kiện** cho `vis-bytecode` (compiler/verifier)
// và `vis-interp` (runtime). Tệp này được `include!` ở cả hai crate ⇒ không cần dependency mới,
// không đổi Cargo.toml. Không được tham chiếu item riêng của crate nào (chỉ `std`).

/// **Trục 2/4 — event-table version**: cấu trúc bảng sự kiện trong trailer `VFEE` = `4`.
/// `1`/`2` đã là `VERSION` của VBC1 (ST-246); `3` từng là *sentinel "version lạ"* trong test,
/// nhưng VD-257 chứng minh bằng `git log` rằng `3` **chưa từng** phát hành ⇒ VD-257 dùng `3` làm
/// `format::INSTRUMENTED_VERSION` (module format, trục 1). Trục 2 độc lập, giữ `4`.
pub const VERSION: u16 = 4;
/// Tên rõ nghĩa của [`VERSION`] — để mã/docs phân biệt **bốn trục phiên bản** (VD-257):
/// 1) `format::VERSION`/`format::INSTRUMENTED_VERSION` (module), 2) `EVENT_TABLE_VERSION` (this),
/// 3) [`WEIGHT_TABLE_VERSION`] (trọng số), 4) `vis-cli::cache::PHIEN_BAN` + các số trên (cache key).
pub const EVENT_TABLE_VERSION: u16 = VERSION;
/// **Trục 3/4 — weight-table version**: bảng trọng số chuẩn (D5) = `1`. Các số là **ĐỀ XUẤT,
/// CHƯA DUYỆT** (VD-256 D2); chỉ test-hook đo dùng, **không** phải chính sách mặc định.
/// Không đọc trọng số từ artifact — trailer chỉ chứa `pc/kind/site`, runtime lấy trọng số theo
/// phiên bản này. Khác với event-table version (4): có thể đổi trọng số **mà không** đổi cấu trúc
/// trailer, và ngược lại.
pub const WEIGHT_TABLE_VERSION: u16 = 1;

// ---------------------------------------------------------------------------
// VS3-1a — HỢP ĐỒNG NGÂN SÁCH / FUEL DÙNG CHUNG interp ↔ VM.
//
// `BUDGET_SIZING_CEILING` là **trần của miền hỗ trợ** dùng để định cỡ hàng rào an toàn của VM.
// **ST-290 (VD-273)**: từ `v0.4.0`, `BUDGET_DEFAULT = 40M` là ngân sách **mặc định** dùng chung
// interp ↔ VM (không còn chỉ test-hook); `--max-budget`/`--unlimited-budget` đổi theo phiên.
// Bất biến bắt buộc: một chương trình còn trong trần ngân sách **không** được trap fuel
// trước khi ngân sách cạn. Vì vậy `VM_SAFETY_FUEL` phải ≥ ceiling × tỷ lệ bước/lệnh trần.
// ---------------------------------------------------------------------------

/// Số vòng lặp/range tối đa mà interp chấp nhận cho **một** cấu trúc lặp (giữ nguyên giá trị
/// lịch sử `10^7`; tách hằng để một nguồn định nghĩa với định cỡ fuel).
pub const MAX_LOOP_ITERATIONS: u64 = 10_000_000;

/// **ST-290 (VD-273)** — ngân sách ngữ nghĩa **mặc định** của một lần chạy, dùng chung
/// interp ↔ VM. Từ `v0.4.0` ngân sách **bật mặc định** (không còn chỉ test-hook như VD-256);
/// vượt ngân sách báo `VS0510` (tách khỏi `VS0501` trap nội bộ và fuel VM). Người dùng có
/// workload hợp lệ dài hơn dùng `vis run --max-budget <N>` hoặc `--unlimited-budget`.
pub const BUDGET_DEFAULT: u64 = 40_000_000;

/// **VS3-1a** — trần ngân sách ngữ nghĩa dùng để **định cỡ** fuel, chưa phải `BUDGET` được A0
/// chốt (VD-257-B §3.6). Chọn 40M đơn vị `standard_weight`: cùng bậc với `MAX_LOOP_ITERATIONS`
/// vòng × 3–6 sự kiện/vòng (đo VS3-1a), và phủ trọn ca 2M với biên rộng. Nếu A0 chốt `BUDGET`
/// khác, phải tính lại [`VM_SAFETY_FUEL`] theo [`VM_STEPS_PER_COST_CEILING`] và chạy lại
/// `crates/vis-cli/tests/vs3_budget.rs`.
///
/// **ST-290 (VD-273)**: A0 đã chốt đúng giá trị này làm mặc định ⇒ trần định cỡ **trùng**
/// [`BUDGET_DEFAULT`]; fuel vẫn giữ `100M` (> 2× trần).
pub const BUDGET_SIZING_CEILING: u64 = 40_000_000;

/// Tỷ lệ `số lệnh VM / đơn vị ngân sách` lớn nhất trong miền hỗ trợ. Đo VS3-1a (debug):
/// `for` range 2M ≈ **1,17** (thân gán), **1,67** (thân literal); `while` 2M ≈ **1,0**; các
/// workload VD-257-B ≤ 1,0. Chọn trần **2** để có biên cho construct chưa đo; nếu đo thấy vượt,
/// phải tăng [`VM_SAFETY_FUEL`] tương ứng.
pub const VM_STEPS_PER_COST_CEILING: u64 = 2;

/// **VS3-1a** — fuel an toàn của VM, **một nguồn** cho `vis_vm::vm::SAFETY_FUEL` và test.
/// `100M = 40M (ceiling) × 2 (tỷ lệ trần) × 1,25 (biên)`. Fuel **không** phải ngân sách ngữ
/// nghĩa, **không** bật ngân sách mặc định; nó chỉ phải lớn hơn ngưỡng mà chương trình còn
/// trong ngân sách có thể chạm (bất biến kiểm ở `vs3_budget.rs`).
pub const VM_SAFETY_FUEL: u64 = 100_000_000;

/// Một loại sự kiện ngữ nghĩa — **một định nghĩa duy nhất** cho compiler và interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum FeeKind {
    /// Bắt đầu một câu lệnh nguồn (không tính lại khi resume).
    Stmt = 1,
    /// Một nút expression nguồn thực sự được đánh giá (nút có sự kiện riêng thì không cộng).
    Expr = 2,
    /// Ghi do **nguồn** quy định — không tính writeback VM/copy tham số/spill/temporary.
    Assign = 3,
    /// Mỗi lần thử điều kiện / tiến vòng (kể cả lần cuối sai).
    Loop = 4,
    /// Lời gọi hàm người dùng mở khung.
    Call = 5,
    /// Builtin / phương thức host (không cộng `E_CALL`).
    Builtin = 6,
    /// `spawn` tác vụ (internal `$spawn` chỉ mang sự kiện này).
    Spawn = 7,
    /// `await` **trong nguồn**; final join runtime không tạo `E_AWAIT` giả.
    Await = 8,
}

impl FeeKind {
    /// Tag ổn định trên đĩa / trong trace.
    pub fn tag(self) -> u8 {
        self as u8
    }
    pub fn from_tag(t: u8) -> Option<Self> {
        Some(match t {
            1 => FeeKind::Stmt,
            2 => FeeKind::Expr,
            3 => FeeKind::Assign,
            4 => FeeKind::Loop,
            5 => FeeKind::Call,
            6 => FeeKind::Builtin,
            7 => FeeKind::Spawn,
            8 => FeeKind::Await,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            FeeKind::Stmt => "E_STMT",
            FeeKind::Expr => "E_EXPR",
            FeeKind::Assign => "E_ASSIGN",
            FeeKind::Loop => "E_LOOP",
            FeeKind::Call => "E_CALL",
            FeeKind::Builtin => "E_BUILTIN",
            FeeKind::Spawn => "E_SPAWN",
            FeeKind::Await => "E_AWAIT",
        }
    }
    pub const ALL: [FeeKind; 8] = [
        FeeKind::Stmt,
        FeeKind::Expr,
        FeeKind::Assign,
        FeeKind::Loop,
        FeeKind::Call,
        FeeKind::Builtin,
        FeeKind::Spawn,
        FeeKind::Await,
    ];
}

/// Trọng số **đề xuất** (VD-256 D2: chưa chốt). Không đọc từ artifact (D5).
pub fn standard_weight(kind: FeeKind) -> u64 {
    match kind {
        FeeKind::Stmt | FeeKind::Expr | FeeKind::Assign | FeeKind::Loop => 1,
        FeeKind::Builtin | FeeKind::Await => 2,
        FeeKind::Call | FeeKind::Spawn => 4,
    }
}

/// Một sự kiện: tại `pc` (interp dùng `0`, bytecode = chỉ số lệnh), loại `kind`, vị trí `site`.
/// **Không** serialize trọng số (D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeEntry {
    pub pc: u32,
    pub kind: FeeKind,
    /// Byte offset trong nguồn đã NFC (khớp `format::Span::start`). `0` = node codegen tự sinh.
    pub site: u32,
}

impl FeeEntry {
    pub fn new(pc: usize, kind: FeeKind, site: u32) -> Self {
        Self {
            pc: pc as u32,
            kind,
            site,
        }
    }
    /// Nhãn trace ổn định: `E_CALL@12`.
    pub fn label(&self) -> String {
        format!("{}@{}", self.kind.name(), self.site)
    }
}
