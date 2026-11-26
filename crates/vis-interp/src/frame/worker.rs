// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-284 (VD-272)** — driver frame (fallback/idle + vòng lặp scheduler) tách khỏi `frame.rs`.
//!
//! Tách nguyên văn (R0 refactor, 0 đổi hành vi): `frame.rs` chỉ còn khung dữ liệu + work stack;
//! driver chạy scheduler, giữ chỗ cho worker frame `P4` (ST-292). Cùng module path
//! `vis_interp::frame::worker`, không đổi API (`run_frame_driver` được re-export ở `frame`).

use super::params::env_for_params;
use super::proc::purge_terminal_frames;
use super::*;
use crate::builtin::Host;
use crate::rt::Waiter;

/// **VS3-1d** — driver frame dùng chung cho test và runtime thật. `it` đã có host/caps do
/// caller gắn; hàm chỉ chạy scheduler + trả `FrameOutput`.
pub(crate) fn run_frame_driver<'a>(
    it: &mut Interp<'a>,
    text: &str,
    hir: &'a vis_hir::hir::Hir,
    symbols: &'a vis_hir::Symbols,
    _types: &'a vis_hir::TypeTable,
    args: &[String],
    config: FrameConfig,
) -> FrameOutput {
    let _ = symbols;

    // **VD-255 / Group B**: hằng cấp tệp được đánh giá trước main (khớp `interp.rs:229`).
    // Dùng chung một `Env` để const sau thấy const trước; mỗi frame con clone env làm gốc.
    let globals: Env = Rc::new(RefCell::new(HashMap::new()));
    let mutable: Rc<RefCell<HashSet<u32>>> = Rc::new(RefCell::new(HashSet::new()));
    let consts: Vec<(u32, &'a HExpr)> = hir
        .items
        .iter()
        .filter_map(|item| match item {
            HItem::Const(c) => Some((c.sym, &c.value)),
            _ => None,
        })
        .collect();
    for (sym, e) in consts {
        match eval_expr_in(it, e, &globals, &mutable) {
            Ok(v) => {
                globals.borrow_mut().insert(sym, v);
            }
            Err(Abort::Trap(t)) => return frame_output_early(it, text, None, Some(t)),
            Err(_) => return frame_output_early(it, text, None, None),
        }
    }

    let mut frames: HashMap<u32, IFrame<'a>> = HashMap::new();
    let main_fn = hir.items.iter().find_map(|i| match i {
        HItem::Fn(f) if f.name == "main" => Some(f),
        _ => None,
    });
    // **VD-262**: biến cấp tệp là global — khai báo `let`/`var` chạy trước `main`; không có
    // `main` thì để thân cấp tệp tự chạy như cũ (tránh thi hành hai lần).
    if main_fn.is_some() {
        if let Some(t) = khoi_tao_bien_cap_tep_frame(it, hir, &globals, &mutable) {
            return frame_output_early(it, text, None, Some(t));
        }
    }
    let body: &'a [HStmt] = main_fn.map(|f| f.body.as_slice()).unwrap_or(&hir.stmts);
    let argv = Value::list(args.iter().map(|s| Value::Str(s.clone())).collect());
    // **VD-259**: `main` dùng **cùng** helper tham số/default với `dispatch`. Bản cũ chỉ đưa
    // argv cho tham số 1 và `Unit` cho mọi tham số còn lại ⇒ `b: int = 7` bị `Unit`.
    let main_env = match main_fn {
        Some(f) => {
            // **VS3-1d**: nối `caps` của CLI như runtime mặc định — tham số `cap[...]` nhận
            // `Value::Cap`; tham số khác nhận `argv` (tương thích đường cũ).
            let pos = if f.params.is_empty() {
                Vec::new()
            } else if it.caps().is_empty() {
                vec![argv]
            } else {
                (0..f.params.len())
                    .map(|i| {
                        it.caps()
                            .iter()
                            .find(|(pos, _)| *pos as usize == i)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| argv.clone())
                    })
                    .collect()
            };
            match env_for_params(
                it,
                &f.params,
                &pos,
                &[],
                &globals,
                &mutable,
                &f.name,
                f.span,
            ) {
                Ok(env) => env,
                Err(Abort::Trap(t)) => return frame_output_early(it, text, None, Some(t)),
                Err(_) => return frame_output_early(it, text, None, None),
            }
        }
        // **VD-259**: không có `main` ⇒ chạy câu lệnh cấp tệp như runtime mặc định, KHÔNG
        // gắn argv vào một ô `u32::MAX` (fallback cũ khiến mọi tên lạ nhận danh sách args).
        // **ST-318a (#203)**: thân cấp tệp chạy **trực tiếp** trên `globals` (không clone) để
        // biến top-level là ô chung với task frame — cùng ngữ nghĩa interp non-frame/VM.
        None => {
            it.env = globals.clone();
            globals.clone()
        }
    };
    frames.insert(0, IFrame::new(body, main_env, &globals, mutable.clone()));
    let mut dong_bo = DongBo::default();
    dong_bo.tao(0);

    it.sched
        .borrow_mut()
        .frame_enable_for_test(config.fake_sources, config.events);
    it.sched
        .borrow_mut()
        .frame_reactor_load_for_test(config.reactors, config.timer_real_wait);
    if let Some(b) = config.backend {
        it.sched.borrow_mut().frame_install_backend_for_test(b);
    }

    let mut current: u32 = 0;
    let mut final_value: Option<Value> = None;
    let mut trap: Option<Trap> = None;
    let mut guard = 0usize;
    let max_turns = crate::rt::frame_max_turns_for_test().unwrap_or(200_000);
    loop {
        guard += 1;
        // **VS3-1d**: `RunOutput.steps` phải >0 như runtime mặc định (chống xanh-giả).
        it.steps += 1;
        if guard > max_turns {
            trap = Some(Trap::new(
                RT,
                format!(
                    "vòng lặp scheduler quá giới hạn ({max_turns} turn) — kiểm tra await vòng tròn/deadlock"
                ),
                Span::new(0, 0),
            ));
            break;
        }
        // **ST-287 (VD-273)**: task đã bị huỷ tại safe point trước đó **không** được chạy
        // lại/cache kết quả (kể cả khi frame id còn sót trong hàng đợi do race nền).
        if current != 0 && it.sched.borrow().frame.cancelled.contains(&current) {
            frames.remove(&current);
            dong_bo.xoa(current);
            continue;
        }
        it.sched.borrow_mut().frame.current = current;
        // **ST-292 (P4b)**: chuyển cancel của scheduler tới handle worker (idempotent).
        let huy: Vec<u32> = it.sched.borrow().frame.cancelled.iter().copied().collect();
        for id in huy {
            it.worker_cancel(id);
        }
        // **VD-258**: task đang chạy phải mang `DangChay` (trước đây không bao giờ đặt)
        // ⇒ `await` chính nó bị phát hiện thay vì treo im lặng.
        if current != 0 {
            if let Some(t) = it
                .sched
                .borrow()
                .tasks
                .iter()
                .find(|t| t.id == current)
                .cloned()
            {
                t.trang_thai.set(rt::TrangThai::DangChay);
            }
        }
        // VD-257/D — reset hạn ngạch ĐẦU MỖI TURN (không đụng ngân sách tổng).
        crate::rt::fee::quota_reset_for_test();
        let outcome = step_frame(it, current, &mut frames, &mut dong_bo);
        match outcome {
            Ok(v) => {
                // **VD-259**: frame đã terminal ⇒ bỏ khỏi map ngay (không giữ `IFrame`/env
                // của nó tới cuối chương trình).
                frames.remove(&current);
                dong_bo.xoa(current);
                if current == 0 {
                    final_value = v;
                    // Không `break`: main xong ⇒ final join các task không ai await.
                } else {
                    let id = current;
                    let val = v.unwrap_or(Value::Unit);
                    if let Some(t) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(val));
                        t.trang_thai.set(rt::TrangThai::Xong);
                    }
                    it.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
                }
            }
            Err(Abort::Trap(t)) => {
                if current == 0 {
                    frames.remove(&current);
                    dong_bo.xoa(0);
                    trap = Some(t);
                    break;
                }
                let id = current;
                frames.remove(&id);
                dong_bo.xoa(id);
                let wrapped = rt::wrap_task_error_for_test(
                    &it.sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.ten.clone())
                        .unwrap_or_default(),
                    t,
                    it.sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.span)
                        .unwrap_or_else(|| Span::new(0, 0)),
                );
                if let Some(tk) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                    *tk.kq.borrow_mut() = Some(Err(wrapped.clone()));
                    tk.trang_thai.set(rt::TrangThai::Xong);
                }
                it.sched.borrow_mut().frame_task_done(id, Err(wrapped));
            }
            Err(Abort::Ret(v)) => {
                frames.remove(&current);
                dong_bo.xoa(current);
                if current == 0 {
                    // `return`/`?` ở `main` vẫn phải final-join các task còn lại (không bỏ).
                    final_value = Some(v);
                } else {
                    let id = current;
                    if let Some(t) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(v));
                        t.trang_thai.set(rt::TrangThai::Xong);
                    }
                    it.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
                }
            }
            Err(Abort::Pending(p)) => {
                // **VD-258**: wait không có task/wait = yield quota; frame vẫn sẵn sàng.
                let quota_yield = p.task.is_none() && p.wait.is_none();
                // **VD-258**: `await` chính tác vụ đang chạy (hoặc một task `DangChay`)
                // không bao giờ có event đánh thức ⇒ trap rõ thay vì treo/exit 0.
                if let Some(t) = &p.task {
                    if t.id == current || t.trang_thai.get() == rt::TrangThai::DangChay {
                        trap = Some(Trap::new(
                            crate::interp::DEADLOCK,
                            "deadlock: tác vụ đang chờ chính nó (await chính tác vụ đang chạy)",
                            p.span,
                        ));
                        break;
                    }
                }
                if current != 0 {
                    if let Some(t) = it
                        .sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == current)
                        .cloned()
                    {
                        t.trang_thai.set(rt::TrangThai::Pending);
                    }
                }
                let w: Option<Waiter> = if let Some(w) = p.wait {
                    let gen = w.gen_of(it.sched.borrow().frame.gen);
                    Some(Waiter {
                        frame: current,
                        wait: w,
                        gen,
                    })
                } else {
                    p.task
                        .as_ref()
                        .and_then(|t| it.sched.borrow().frame_await(current, t))
                };
                if let Some(w) = w {
                    it.sched.borrow_mut().frame_register(w);
                } else {
                    it.sched.borrow_mut().frame.ready.push_back(current);
                }
                if quota_yield {
                    // Yield quota là tiến độ hợp lệ, không phải scheduler quay vô ích.
                    guard = guard.saturating_sub(1);
                }
            }
        }

        // Lên lịch task `Moi` khi không còn frame sẵn sàng — kể cả task tạo bằng `tao`
        // (gọi `async fn` trực tiếp, không `spawn`) đang được `await`. Khớp VM; nếu không,
        // main treo khi await một task thường. **VD-258**: CHỈ `Moi` (chưa chạy) — task
        // `Pending` đang chờ event không được đẩy lại ready (quay vô ích tới guard).
        let need_join = true;
        let mut next = it.sched.borrow_mut().frame_next();
        if it.sched.borrow().frame.selector_error.is_some() {
            break;
        }
        // **ST-292 (P4)**: không có frame sẵn sàng nhưng pool còn in-flight ⇒ pump outcome
        // (50 ms/lần) và thử `frame_next` lại; timer/reactor vẫn có cơ hội fire giữa các lần.
        if next.is_none() && it.worker_in_flight() {
            for _ in 0..400 {
                match it.worker_pump(true) {
                    Ok(_) => {}
                    Err(crate::interp::Abort::Trap(t)) => {
                        trap = Some(t);
                        break;
                    }
                    Err(_) => {}
                }
                next = it.sched.borrow_mut().frame_next();
                if next.is_some() || !it.worker_in_flight() {
                    break;
                }
            }
        }
        let next = match next {
            Some(n) => Some(n),
            None if need_join => {
                let pending: Vec<u32> = {
                    let s = it.sched.borrow();
                    s.tasks
                        .iter()
                        .filter(|t| t.trang_thai.get() == rt::TrangThai::Moi)
                        .filter(|t| !s.frame.external.contains_key(&t.id))
                        .filter(|t| !it.worker_is_task(t.id))
                        .map(|t| t.id)
                        .collect()
                };
                for id in pending {
                    if !it.sched.borrow().frame.ready.contains(&id) {
                        it.sched.borrow_mut().frame.ready.push_back(id);
                    }
                }
                it.sched.borrow_mut().frame_next()
            }
            None => None,
        };
        // **VD-260**: bỏ frame của task đã `Xong`/`Huy` ngay tại safe point kế tiếp —
        // không giữ `IFrame` + env tới cuối chương trình. `next` (nếu có) luôn còn sống
        // nên được giữ lại; frame mồ côi (task đã bị thu hồi) cũng bị bỏ.
        let purged = purge_terminal_frames(&mut frames, &it.sched.borrow(), next);
        if !purged.is_empty() {
            let mut s = it.sched.borrow_mut();
            for id in &purged {
                s.frame.log.push(format!("purge:{}", id));
            }
            s.frame.live_frames = frames.len();
            drop(s);
            for id in &purged {
                dong_bo.xoa(*id);
            }
        }
        match next {
            Some(n) => {
                match frames.entry(n) {
                    std::collections::hash_map::Entry::Occupied(_) => {}
                    std::collections::hash_map::Entry::Vacant(e) => {
                        match build_task_frame(it, n, &globals, &mutable) {
                            Ok(Some((body, env))) => {
                                e.insert(IFrame::new(body, env, &globals, mutable.clone()));
                                dong_bo.tao(n);
                            }
                            Ok(None) => {
                                // Nguồn giả / task ngoài — không có thân để dựng frame. Bỏ qua,
                                // GIỮ NGUYÊN `current` (không gán rồi `continue` như cũ, vốn
                                // gây panic `step_frame` ở vòng sau vì frame không tồn tại).
                                continue;
                            }
                            Err(Abort::Trap(t)) => {
                                trap = Some(t);
                                break;
                            }
                            Err(e) => {
                                // Default tham số cần suspend (hoặc `return`) — chưa có
                                // continuation cho bước dựng frame; báo rõ thay vì chạy sai.
                                let span = match &e {
                                    Abort::Pending(p) => p.span,
                                    _ => Span::new(0, 0),
                                };
                                trap = Some(Trap::new(
                                    RT,
                                    "frame test: không dựng được tham số/default của task",
                                    span,
                                ));
                                break;
                            }
                        }
                    }
                }
                current = n;
            }
            None => break,
        }
    }

    if trap.is_none() {
        if let Some(e) = it.sched.borrow_mut().frame_take_selector_error() {
            trap = Some(Trap::new(RT, e, Span::new(0, 0)));
        }
    }
    // **VD-259/VD-260**: frame của task đã `Xong`/`Huy` không còn là trạng thái treo — bỏ
    // trước khi đo `live_frames` (và đã bỏ ngay tại safe point trong vòng lặp). Chỉ frame
    // thật sự Pending mới còn trong `frames`.
    let purged = purge_terminal_frames(&mut frames, &it.sched.borrow(), None);
    if !purged.is_empty() {
        let mut s = it.sched.borrow_mut();
        for id in &purged {
            // Hậu tố `-end`: frame chỉ được dọn ở cuối (không kịp safe point) — khác với
            // `purge:<id>` giữa vòng lặp, để test phân biệt được retention.
            s.frame.log.push(format!("purge-end:{}", id));
        }
    }
    it.sched.borrow_mut().frame.live_frames = frames.len();
    frames.clear();
    dong_bo.frame_seq.clear();
    dong_bo.dirty.clear();
    it.sched.borrow_mut().frame_drain_events();
    // **ST-278 (VD-256)**: lỗi task cache ở final join phải surfaced (cùng hợp đồng VS0506).
    // **ST-292 (P4)**: chờ worker pool, cache task chưa await, flush output theo spawn order.
    if trap.is_none() {
        if let Err(crate::interp::Abort::Trap(t)) = it.worker_final_join() {
            trap = Some(t);
        }
    }
    let final_join_err = it.sched.borrow_mut().frame_final_join();
    if trap.is_none() {
        trap = final_join_err;
    }
    // **VD-258**: không còn frame sẵn sàng mà vẫn có waiter/task chưa terminal ⇒ deadlock
    // thật (event không bao giờ tới), KHÔNG được trả exit 0 như thành công.
    if trap.is_none() && !it.worker_in_flight() {
        trap = it.sched.borrow().frame_stuck_trap();
    }
    // **VD-259**: đo **trước** `frame_reactor_shutdown` — nếu đo sau, regs/timers luôn 0 và
    // snapshot không còn là bằng chứng vòng đời.
    let snapshot = it.sched.borrow().diag_snapshot();
    it.sched.borrow_mut().frame_reactor_shutdown();
    let mut log = it.sched.borrow().frame.log.clone();
    log.extend(it.sched.borrow().frame.reactor.log.clone());
    let stdout = it.take_stdout();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(text)],
        None => Vec::new(),
    };
    // **VD-258**: exit theo giá trị cuối của `main` (`int`/`Result.Err`), không luôn 0.
    let exit = match (&trap, &final_value) {
        (Some(_), _) => 1,
        (None, Some(v)) => crate::value::exit_code(v),
        (None, None) => 0,
    };
    FrameOutput {
        stdout,
        diagnostics,
        value: final_value,
        exit,
        log,
        snapshot,
    }
}

/// Trả `FrameOutput` sớm khi đánh giá hằng cấp tệp lỗi (trước khi scheduler chạy).
fn frame_output_early(
    it: &mut Interp<'_>,
    text: &str,
    value: Option<Value>,
    trap: Option<Trap>,
) -> FrameOutput {
    let stdout = it.take_stdout();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(text)],
        None => Vec::new(),
    };
    FrameOutput {
        stdout,
        diagnostics,
        value,
        exit: if trap.is_some() { 1 } else { 0 },
        log: Vec::new(),
        snapshot: Default::default(),
    }
}

/// Dựng frame cho task `id` từ HIR thật (thân `async fn`).
#[allow(clippy::type_complexity)]
fn build_task_frame<'a>(
    it: &mut Interp<'a>,
    id: u32,
    globals: &Env,
    mutable: &Rc<RefCell<HashSet<u32>>>,
) -> R<Option<(&'a [HStmt], Env)>> {
    let (f, args, span, ten) = {
        let s = it.sched.borrow();
        let Some(t) = s.tasks.iter().find(|t| t.id == id) else {
            return Ok(None);
        };
        if s.frame.external.contains_key(&id) {
            return Ok(None);
        }
        let Some(f) = it.fns.get(&t.than).copied() else {
            return Ok(None);
        };
        (f, t.args.clone(), t.span, t.ten.clone())
    };
    // **VD-259**: cùng helper tham số/default với `dispatch` — task `async` có default không
    // còn nhận `Unit` khi gọi thiếu đối số.
    let env = env_for_params(it, &f.params, &args, &[], globals, mutable, &ten, span)?;
    Ok(Some((&f.body, env)))
}

/// Một bước chạy frame: tiêu thụ work cho tới khi frame xong, treo (`Pending`), hoặc lỗi.
fn step_frame<'a>(
    it: &mut Interp<'a>,
    frame_id: u32,
    frames: &mut HashMap<u32, IFrame<'a>>,
    dong_bo: &mut DongBo,
) -> R<Option<Value>> {
    let mut frame = frames.remove(&frame_id).expect("frame hiện tại");
    dong_bo.nap(it, &mut frame, frame_id);
    let res = buoc_frame_dong_bo(it, &mut frame, frame_id, dong_bo);
    frames.insert(frame_id, frame);
    res
}

// ---------------------------------------------------------------------------
// **ST-318a (#203)** — biến cấp tệp (`let`/`var` top-level) là **một ô chung** giữa các
// frame, khớp interp non-frame (`env` chung) và VM (mảng `globals`): frame W=1 trước đây giữ
// `Env` **riêng** (clone lúc dựng) nên `var` ghi trong task không thấy ở main — interp in `0`
// còn VM in `1` trên cùng chương trình. `it.env` (globals, do `khoi_tao_bien_cap_tep_frame`
// gắn) là nguồn chung cho các `sym` **đã có** trong đó; tham số/`let` trong thân (sym riêng,
// không có trong globals) vẫn ở `Env` của frame.
// ---------------------------------------------------------------------------

/// **ST-318a F2 (P2-C)** — sổ **dirty-set** cho biến cấp tệp.
///
/// Chỉ đánh dấu **sym thực sự bị ghi** (sau mỗi work-item ghi) kèm số thứ tự tăng đơn điệu;
/// mỗi frame nhớ `seq` đã đồng bộ. Nhờ vậy `nap` chỉ chép các sym đổi kể từ lần chạy trước của
/// frame — **không** quét `frame.mutable` (20k `var` khai báo nhưng không ghi gần như miễn phí).
#[derive(Default)]
struct DongBo {
    /// Số thứ tự lần ghi gần nhất (tăng đơn điệu).
    seq: u64,
    /// `sym → seq` lần ghi gần nhất; giá trị thật luôn ở `it.env` (globals).
    dirty: HashMap<u32, u64>,
    /// `frame id → seq` frame đã đồng bộ xong.
    frame_seq: HashMap<u32, u64>,
}

impl DongBo {
    /// Frame mới: env vừa clone globals hiện tại ⇒ coi như đã thấy tới `seq`.
    fn tao(&mut self, id: u32) {
        self.frame_seq.insert(id, self.seq);
    }

    /// Frame terminal bị bỏ: quên mốc seq của nó (dọn `dirty` sẽ làm ở `nap` kế tiếp).
    fn xoa(&mut self, id: u32) {
        self.frame_seq.remove(&id);
    }

    /// Nạp các sym dirty kể từ lần chạy trước của frame `id` vào mọi khung gọi của nó.
    fn nap(&mut self, it: &Interp<'_>, frame: &mut IFrame<'_>, id: u32) {
        let fs = self.frame_seq.get(&id).copied().unwrap_or(0);
        if !self.dirty.is_empty() {
            let mut ds: Vec<u32> = self
                .dirty
                .iter()
                .filter(|(_, s)| **s > fs)
                .map(|(k, _)| *k)
                .collect();
            if !ds.is_empty() {
                ds.sort_unstable();
                let globals = it.env.clone();
                let cap: Vec<(u32, Value)> = {
                    let g = globals.borrow();
                    ds.into_iter()
                        .filter_map(|s| g.get(&s).map(|v| (s, v.clone())))
                        .collect()
                };
                if !cap.is_empty() {
                    nap_vao_env(frame, &globals, &cap);
                }
            }
        }
        self.frame_seq.insert(id, self.seq);
        // **F3 (P2-1)**: dọn **mọi** nhánh — kể cả khi `ds` rỗng (frame đã thấy hết) —
        // nếu không, `dirty` do chính frame vừa ghi 20k sym sẽ không bao giờ co lại và bị
        // duyệt O(#sym) mỗi turn. `prune` tự no-op khi sổ < 64 mục.
        self.prune();
    }

    /// Ghi biến cấp tệp vừa đổi (giá trị đã nằm trong globals) và đánh dấu dirty.
    ///
    /// **ST-319c2 (#206)**: trong thân closure bắt biến **theo giá trị** (SPEC §4.7) biến cấp
    /// tệp chỉ là bản snapshot của `env` khung gọi ⇒ lệnh ghi là **cục bộ**, không đụng
    /// `globals`, không đánh dirty, không đồng bộ sang frame khác. Khung **hàm tên** (kể cả
    /// hàm được closure gọi) vẫn xuyên như `#203` nhờ `by_value = false`.
    fn ghi(&mut self, it: &Interp<'_>, frame: &mut IFrame<'_>, sym: u32, id: u32) {
        // Chỉ khung **đang thực thi** mới quyết định: hàm tên được closure gọi vẫn là khung
        // hàm tên (ghi xuyên như F2/F3, `#203`), còn thân closure là ô snapshot cục bộ.
        if frame.cur().by_value {
            return;
        }
        if !ghi_bien_cap_tep(it, frame, sym) {
            return;
        }
        self.seq += 1;
        self.dirty.insert(sym, self.seq);
        // Env của chính frame vừa được cập nhật đủ ⇒ nó đã "thấy" mốc này.
        self.frame_seq.insert(id, self.seq);
    }

    /// Bỏ mục mà **mọi** frame sống đã đồng bộ — chỉ chạy khi sổ ≥ 64 mục để amortize O(#frame).
    fn prune(&mut self) {
        if self.dirty.len() < 64 {
            return;
        }
        if self.frame_seq.is_empty() {
            self.dirty.clear();
            return;
        }
        if let Some(min) = self.frame_seq.values().copied().min() {
            self.dirty.retain(|_, s| *s > min);
        }
    }
}

/// Chép `cap` (sym → giá trị mới) vào các khung gọi **không cô lập** của `frame` (bỏ qua env
/// chính là globals).
///
/// **ST-319c2 (#206)**: chỉ khung **closure bắt biến theo giá trị** (`by_value`) sống trên
/// snapshot nên **không** nhận ghi cấp tệp của frame khác. Khung hàm tên/thường — kể cả khung
/// hàm tên nằm **trên** closure trong stack và env dựng từ `globals` — vẫn đồng bộ F2/F3.
fn nap_vao_env(frame: &mut IFrame<'_>, globals: &Env, cap: &[(u32, Value)]) {
    let envs: Vec<Env> = frame
        .calls
        .iter()
        .filter(|cf| !cf.by_value)
        .map(|cf| cf.env.clone())
        .collect();
    for env in envs {
        if Rc::ptr_eq(&env, globals) {
            continue;
        }
        let mut e = env.borrow_mut();
        for (k, v) in cap {
            // Chỉ cập nhật ô mà frame đã có — không kéo biến top-level vào env vốn không thấy nó.
            if e.contains_key(k) {
                e.insert(*k, v.clone());
            }
        }
    }
}

/// `sym` mà work-item **có thể ghi** vào env hiện tại (gán biến, gán đường, writeback `push`).
///
/// Chỉ ba đường này ghi được biến cấp tệp: `let`/`var` top-level đã chạy trước `main` (và khi
/// không có `main` thì env chính **là** globals), còn bind pattern trong thân là sym cục bộ.
///
/// **F3 (P3-1/P3-2)**: `ApplyMethod`/`RedispatchMethod` chỉ **writeback** với `push`
/// (`method_writeback`), nên method chỉ đọc (`len`/`get`/`keys`/`first`/…) không đánh dirty.
fn sym_ghi_cua(w: &Work<'_>) -> Option<u32> {
    match w {
        Work::ApplyAssignVar { sym, .. } => Some(*sym),
        Work::ApplyAssignPath(a) => Some(a.sym),
        Work::ApplyMethod {
            name,
            base_var: Some(sym),
            ..
        }
        | Work::RedispatchMethod {
            name,
            base_var: Some(sym),
            ..
        } if name.as_str() == "push" => Some(*sym),
        _ => None,
    }
}

/// Ghi **một** biến cấp tệp vừa đổi về `it.env` (globals) rồi cập nhật mọi khung gọi đang treo
/// của cùng frame — nhờ vậy `print(g)` ngay sau `tang()` (cùng một step) vẫn thấy giá trị mới,
/// và frame khác (task/main) thấy được khi resume.
///
/// Trả `true` nếu đúng là biến cấp tệp (đã ghi globals và cần đánh dấu dirty); `false` với tham
/// số/`let` cục bộ. Khi env chính **là** globals (chương trình không có `main`) thì lệnh ghi đã
/// vào globals từ trước, nhưng vẫn phải trả `true` để task frame khác nhận được ở lần resume.
fn ghi_bien_cap_tep(it: &Interp<'_>, frame: &mut IFrame<'_>, sym: u32) -> bool {
    let globals = it.env.clone();
    let env = frame.cur().env.clone();
    let Some(v) = env.borrow().get(&sym).cloned() else {
        return false;
    };
    // Sym cục bộ (tham số/`let` trong thân) không có trong globals ⇒ không đụng.
    if !globals.borrow().contains_key(&sym) {
        return false;
    }
    if !Rc::ptr_eq(&env, &globals) {
        globals.borrow_mut().insert(sym, v.clone());
        let cap = [(sym, v)];
        nap_vao_env(frame, &globals, &cap);
    }
    true
}

/// **ST-318a (#203)** — bản sao `step_frame_owned` (`frame.rs`) có đồng bộ biến cấp tệp sau
/// từng work-item. Mọi nhánh còn lại giữ nguyên hành vi bản gốc (ngân sách ⇒ `Pending`, pop
/// khung ⇒ merge closure/hoàn nguyên tham số, hết work ⇒ trả `vals`).
///
/// ⚠️ **Giữ khớp từng nhánh** với `frame.rs::step_frame_owned`: loop gốc đổi thứ tự/thêm nhánh
/// thì phải sửa tệp này ngay (đây là bản sao **có chủ đích** để chèn đồng bộ giữa các item).
fn buoc_frame_dong_bo<'a>(
    it: &mut Interp<'a>,
    frame: &mut IFrame<'a>,
    frame_id: u32,
    dong_bo: &mut DongBo,
) -> R<Option<Value>> {
    loop {
        if frame.done() {
            return Ok(frame.vals.pop());
        }
        // **VD-257/D**: hết hạn ngạch — yield về đuôi ready, không trap, không chạy lại item.
        if crate::rt::fee::quota_take_hit() {
            let cur = it.sched.borrow().frame.current;
            it.sched.borrow_mut().frame.log.push(format!("quota:{cur}"));
            return Err(Abort::Pending(crate::rt::Pending {
                task: None,
                wait: None,
                span: Span::new(0, 0),
            }));
        }
        let w = frame.cur().work.pop();
        match w {
            Some(w) => {
                let sym_ghi = sym_ghi_cua(&w);
                // Đồng bộ **kể cả khi item lỗi** — side-effect đã xảy ra trước trap vẫn phải
                // thấy được ở frame khác (final join/báo lỗi), đúng như env chung non-frame.
                let res = super::process_work(it, frame, w);
                if let Some(sym) = sym_ghi {
                    dong_bo.ghi(it, frame, sym, frame_id);
                }
                res?;
            }
            None => {
                // Hết work ⇒ kết thúc khung con: hoàn nguyên tham số, nâng ret, đẩy giá trị.
                let cf = frame.calls.pop().expect("khung");
                let mut v = cf.last.unwrap_or(Value::Unit);
                if cf.ret_depth > 0 {
                    v = crate::lift::lift_opt(v, cf.ret_depth);
                }
                // **VD-258**: closure có env riêng cho lần gọi này — gộp các biến bắt
                // về env gốc của closure (tham số bị loại ⇒ lần gọi khác không bị đè).
                if let Some(dich) = &cf.merge {
                    let params: Vec<u32> = cf.restore.iter().map(|(s, _)| *s).collect();
                    let snapshot = cf.env.borrow().clone();
                    let mut den = dich.borrow_mut();
                    for (sym, val) in snapshot {
                        if !params.contains(&sym) {
                            den.insert(sym, val);
                        }
                    }
                }
                for (sym, old) in cf.restore {
                    match old {
                        Some(old) => {
                            cf.env.borrow_mut().insert(sym, old);
                        }
                        None => {
                            cf.env.borrow_mut().remove(&sym);
                        }
                    }
                }
                frame.vals.push(v);
            }
        }
    }
}
