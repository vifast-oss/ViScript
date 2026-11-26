// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318h (VD-298)** — work stack của frame evaluator: `Work`, `MethodKind`,
//! `Callee`, `LoopKindInit`.
//!
//! Tách **nguyên văn** từ `frame.rs` (R0, 0 đổi hành vi); dùng chung qua `super::*`.

use super::*;

/// Một work-item của IFrame phải làm tiếp. `Eval`/`Exec` đẩy continuation,
/// các biến thể `Apply*` là continuation tiêu thụ giá trị trên `vals`.
pub(super) enum Work<'a> {
    // ---- nền tảng (Lát 1) ----
    Eval(&'a HExpr),
    Exec(&'a HStmt),
    /// Sau khi operand của `await` thành `Value::Task`, gọi `await_task` thật.
    ApplyAwait(Span),
    /// `lhs`/`rhs` đã trên `vals` (theo thứ tự); áp dụng phép toán thật.
    ApplyBin {
        op: BinOp,
        span: Span,
    },
    /// Đánh giá tuần tự đối số (vị trí + tên) rồi chạy `cont`.
    EvalArgsCont {
        args: &'a [HExpr],
        named: &'a [(String, HExpr)],
        idx: usize,
        span: Span,
        cont: Vec<Work<'a>>,
    },
    /// Gán giá trị trên `vals` vào pattern của `let` (kèm độ sâu `Opt` của annot).
    Bind {
        pat: &'a HPat,
        opt_depth: usize,
        /// VD-257/D: span của câu `let` — dùng làm `site` cho E_ASSIGN.
        span: Span,
    },
    /// `let … else:` — tên trần bóc `Ok`/`Some`; pattern bác được khớp trên toàn giá trị;
    /// không khớp ⇒ hoàn nguyên binding rồi chạy khối `else` (VD-263/VD-265).
    ApplyLetElse {
        pat: &'a HPat,
        else_body: &'a [HStmt],
        bind_sym: Option<u32>,
        opt_depth: usize,
        span: Span,
    },
    /// Gán giá trị trên `vals` vào ô `var` (sym), kèm độ sâu `Opt` của annot.
    SetVar {
        sym: u32,
        opt_depth: usize,
        span: Span,
    },
    /// Đặt `last` = giá trị trên `vals`.
    SetLast,
    /// Đặt `last = None` (đầu mỗi câu lệnh / kết thúc khối không có giá trị).
    ResetLast,
    /// Đẩy `last` (hoặc `Unit`) lên `vals` — cuối khối biểu thức.
    PushLast,
    /// `return <value>`: đặt `last` rồi kết thúc thân khung hiện tại.
    DoReturn,

    // ---- Lát 2: retry builtin mà reactor báo Pending ----
    Redispatch {
        callee: Callee<'a>,
        args: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    },

    // ---- VD-255 / Group A: điều khiển ----
    /// Vào khối câu lệnh lồng (nhánh if/arm match/thân hàm/scope).
    ExecBlock(&'a [HStmt]),
    /// Đánh giá khối biểu thức rồi đẩy giá trị (`last`) lên `vals`.
    EvalBlock(&'a [HStmt]),
    /// Đánh giá `HCond`, chạy `on_true` nếu đúng, `on_false` nếu sai.
    EvalCond {
        cond: &'a HCond,
        on_true: Vec<Work<'a>>,
        on_false: Vec<Work<'a>>,
        span: Span,
    },
    /// Áp dụng kết quả bool của `HCond::Expr` (đã nằm trên `vals`).
    ApplyCond {
        on_true: Vec<Work<'a>>,
        on_false: Vec<Work<'a>>,
        span: Span,
    },
    /// Áp dụng `HCond::Let`: khớp pattern, giữ binding nếu khớp, hoàn nguyên nếu không.
    ApplyCondLet {
        pat: &'a HPat,
        on_true: Vec<Work<'a>>,
        on_false: Vec<Work<'a>>,
    },
    /// Khởi tạo vòng lặp: tạo `LoopState`, đẩy `LoopMark` + bước đầu.
    LoopInit {
        kind: LoopKindInit<'a>,
        body: &'a [HStmt],
        span: Span,
    },
    /// Đáywork của một lần lặp: thân chạy xong ⇒ bước tiếp theo.
    LoopMark {
        id: usize,
    },
    /// Vào thân một lần lặp (đẩy LoopMark + các câu lệnh thân).
    LoopEnter {
        id: usize,
    },
    /// Một bước tiến vòng (thử điều kiện / lấy phần tử / vào thân).
    LoopStep {
        id: usize,
    },
    /// Kết thúc vòng lặp (last = Unit, pop LoopState).
    LoopExit {
        id: usize,
    },
    /// `break` — thoát vòng trong cùng.
    DoBreak {
        span: Span,
    },
    /// `continue` — sang lần lặp tiếp của vòng trong cùng.
    DoContinue {
        span: Span,
    },

    // ---- VD-255 / Group B: biểu thức & gọi ----
    ApplyUn {
        op: UnOp,
        span: Span,
    },
    ApplyField {
        base: &'a HExpr,
        name: String,
        optional: bool,
        span: Span,
    },
    ApplyIndex {
        span: Span,
    },
    BuildList {
        n: usize,
    },
    BuildSet {
        n: usize,
    },
    BuildTuple {
        n: usize,
    },
    BuildMap {
        n: usize,
    },
    /// Đánh giá từng giá trị trường của record literal theo thứ tự nguồn.
    EvalRecordFields {
        fields: &'a [(String, HExpr)],
        idx: usize,
        span: Span,
    },
    BuildRecord {
        name: String,
        fields: &'a [(String, HExpr)],
        span: Span,
    },
    BuildEnum {
        name: String,
        variant: String,
        n: usize,
        span: Span,
    },
    ApplySome,
    InterpParts {
        parts: &'a [InterpPart],
        idx: usize,
        out: String,
        span: Span,
    },
    InterpPartVal {
        parts: &'a [InterpPart],
        idx: usize,
        out: String,
        spec: Option<String>,
        span: Span,
    },
    ApplyShortCircuit {
        op: BinOp,
        rhs: &'a HExpr,
        span: Span,
    },
    ApplyBoolRhs {
        span: Span,
    },
    ApplyCoalesce {
        rhs: &'a HExpr,
    },
    ApplyPipe {
        rhs: &'a HExpr,
        span: Span,
    },
    ApplyPipeCall {
        lhs: Value,
        span: Span,
    },
    /// Chọn nhánh `match`: thử từng arm (pattern + guard).
    MatchArm {
        arms: &'a [HArm],
        idx: usize,
        subj: Value,
        stmt_mode: bool,
        span: Span,
    },
    MatchGuard {
        arms: &'a [HArm],
        idx: usize,
        subj: Value,
        stmt_mode: bool,
        snapshot: HashMap<u32, Value>,
        span: Span,
    },
    /// Gọi hàm/builtin tên sau khi đối số đã eval (`names` = tên đối số theo thứ tự).
    ApplyNamedCall {
        callee: Callee<'a>,
        npos: usize,
        names: Vec<String>,
        span: Span,
    },
    /// Gọi một **giá trị hàm** (closure/builtin value) sau khi callee + đối số đã eval.
    ApplyValueCall {
        npos: usize,
        names: Vec<String>,
        span: Span,
    },
    /// Retry một giá trị hàm đã eval đối số mà builtin báo `Pending`.
    RedispatchValue {
        f: Value,
        args: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    },
    /// Gán `subject` đã eval vào work `MatchArm` tương ứng.
    MatchSubj {
        arms: &'a [HArm],
        stmt_mode: bool,
        span: Span,
    },
    /// Thu base + các giá trị trường của `with` rồi dựng bản ghi mới.
    WithCollect {
        names: Vec<String>,
        n: usize,
        span: Span,
    },
    /// Base của `callee.field(...)`: nhận diện method vs field-holding-closure lúc chạy.
    ApplyMethodOrField {
        name: String,
        args: &'a [HExpr],
        named: &'a [(String, HExpr)],
        base: &'a HExpr,
        span: Span,
    },
    /// Gọi phương thức lõi trên list/str/map/chan (recv đã nằm trong work).
    ApplyMethod {
        name: String,
        recv: Value,
        nargs: usize,
        base_var: Option<u32>,
        span: Span,
    },
    /// Tạo closure: chụp **env chia sẻ** hiện tại.
    MakeLambda {
        params: Vec<u32>,
        /// **ST-301**: số lớp `Opt` của annot từng tham số (0 = không nâng).
        param_depths: Vec<usize>,
        ret_depth: usize,
        body: HBody,
    },
    /// Gán biến `var` (đã eval rhs) — dùng cho closure mutation/writeback.
    ApplyAssignVar {
        sym: u32,
        op: AssignOp,
        span: Span,
    },
    /// **VD-270** — đích gán **đường** (`a.b.f`): đi xuống từng bước / ghi sau vế phải.
    AssignPathStep(expr::AssignPathStep<'a>),
    ApplyAssignPath(expr::ApplyAssignPath),

    // ---- VD-257 / stream A: `?`, kênh, cmd-block ----
    /// `?` trên `Option`/`Result`: bóc `Some`/`Ok`, hoặc **trả sớm** `None`/`Err`
    /// từ khung hiện tại (short-circuit — mọi work còn lại của khung bị bỏ).
    ApplyTry {
        span: Span,
    },
    /// `xs[i]?` — chỉ số **có kiểm tra** ⇒ `T?` (không trap, VD-178 R8).
    ApplyTryIndex {
        span: Span,
    },
    /// `chan[T](n)` — tạo kênh sau khi dung lượng đã eval (`$kenh` thật).
    BuildChan {
        span: Span,
    },
    /// Retry phương thức host (`send`/`recv`/…) sau `Abort::Pending`: đối số đã eval
    /// được giữ lại trong work ⇒ resume **không eval lại** (không lặp side-effect).
    /// **VD-258**: phương thức không resume được (Pending lần nữa) ⇒ trap rõ, không
    /// chạy lại toàn bộ phương thức (tránh lặp side-effect của callback).
    RedispatchMethod {
        name: String,
        recv: Value,
        args: Vec<Value>,
        base_var: Option<u32>,
        span: Span,
    },
    /// **VD-258**: vòng `list.map/filter/each` **resume được** — giữ `idx` + `acc`
    /// qua suspend, gọi callback tuần tự; `awaiting` ⇒ giá trị callback nằm trên `vals`.
    MethodLoop {
        kind: MethodKind,
        f: Value,
        items: Vec<Value>,
        idx: usize,
        acc: Vec<Value>,
        awaiting: bool,
        span: Span,
    },
}

/// **VD-258**: ba phương thức list nhận callback (trước đây chạy qua `host.call`
/// trong một lần gọi Rust ⇒ không suspend/resume được).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MethodKind {
    Map,
    Filter,
    Each,
}

#[derive(Clone, Copy)]
pub(super) enum Callee<'a> {
    /// Hàm khai trong tệp — `(sym, là_async, tên)`.
    Named(u32, bool, &'a str),
    /// `spawn f(...)` — tạo + **xếp vào hàng đợi sẵn sàng**.
    Spawn(u32, &'a str),
    /// Builtin (tên dựng sẵn).
    Builtin(&'a str),
}

/// Dữ liệu khởi tạo của một vòng lặp.
pub(super) enum LoopKindInit<'a> {
    While { cond: &'a HCond },
    For { pat: &'a HPat, iter: &'a HExpr },
    Loop,
}
