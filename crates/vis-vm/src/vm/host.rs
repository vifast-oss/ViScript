// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-284 (VD-272)** — mặt **host** của VM: cửa vào `run_with_*`, gắn host và `impl Host for Vm`.
//!
//! Tách nguyên văn khỏi `vm.rs` (R0 refactor, 0 đổi hành vi/ABI): tệp lõi chỉ còn máy ảo +
//! `Th::exec`; mọi cầu nối host nằm ở đây.

use std::rc::Rc;

use vis_bytecode::Module;
use vis_interp::builtin::{self, Host};
use vis_interp::interp::{Abort, Trap, R, RT};
use vis_interp::value::{Value, VmFunc};
use vis_interp::RunOutput;
use vis_syntax::Span;

use super::Vm;

/// Như [`run`] nhưng gắn **host hệ thống tệp** (ST-33). `None` ⇒ `std.fs.*` ⇒ `VS0604`.
pub fn run_with_fs<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
) -> RunOutput {
    run_with_hosts(m, text, args, fs, None)
}

/// **ST-63 (VD-105)**: như [`run_with_fs`] nhưng gắn **cả host mạng** (`std.net.*`); `None` ở host
/// nào ⇒ `VS0604`. Đây là cửa CLI dùng để hai đường (interp/VM) nhận **cùng** host.
pub fn run_with_hosts<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
) -> RunOutput {
    run_with_hosts_caps(m, text, args, fs, net, &[])
}

/// **ST-72b (VD-103/VD-106)**: như [`run_with_hosts`] nhưng gắn giá trị quyền `cap` cho `main`.
/// **ST-89**: giữ **nguyên chữ ký cũ** — không có host `Ngoai` (wrapper của
/// [`run_with_full_hosts_caps`]).
pub fn run_with_hosts_caps<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
    caps: &[(u32, Value)],
) -> RunOutput {
    run_with_full_hosts_caps(m, text, args, None, fs, net, None, caps)
}

/// **ST-89 (VD-110)**: như [`run_with_hosts_caps`] nhưng nhận **cả** host chương trình ngoài — cùng
/// trait `vis_interp::builtin::Ngoai` mà interp dùng; VM **không** gọi `std::process`.
/// **VD-241 (Gate 1)**: nhận thêm **host cơ sở dữ liệu** (`None` ⇒ `std.sqlite.*` ⇒ `VS0604`).
#[allow(clippy::too_many_arguments)]
pub fn run_with_full_hosts_caps<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    ngoai: Option<&'a mut dyn vis_interp::builtin::Ngoai>,
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
    db: Option<&'a mut dyn vis_interp::builtin::DbHost>,
    caps: &[(u32, Value)],
) -> RunOutput {
    let mut vm = Vm::new(m, text);
    if let Some(n) = ngoai {
        vm.set_ngoai(n);
    }
    if let Some(f) = fs {
        vm.set_fs(f);
    }
    if let Some(g) = net {
        vm.set_net(g);
    }
    if let Some(d) = db {
        vm.set_db(d);
    }
    if !caps.is_empty() {
        vm.caps = caps.to_vec();
    }
    vm.run(args)
}

impl<'a> Vm<'a> {
    /// Gắn host hệ thống tệp (ST-33) — chỉ dùng qua [`crate::run_with_fs`].
    pub fn set_fs(&mut self, f: &'a mut dyn vis_interp::builtin::FsHost) {
        self.fs = Some(f);
    }

    /// **ST-63 (VD-105)**: gắn host mạng — chỉ dùng qua [`crate::run_with_hosts`].
    pub fn set_net(&mut self, n: &'a mut dyn vis_interp::builtin::NetHost) {
        self.net = Some(n);
    }

    /// **VD-241 (Gate 1)**: gắn host cơ sở dữ liệu — VM chỉ gọi qua trait, không biết native.
    pub fn set_db(&mut self, d: &'a mut dyn vis_interp::builtin::DbHost) {
        self.db = Some(d);
    }

    /// **ST-89 (VD-110)**: gắn host chương trình ngoài — VM chỉ gọi qua trait, không chạm OS.
    pub fn set_ngoai(&mut self, n: &'a mut dyn vis_interp::builtin::Ngoai) {
        self.ngoai = Some(n);
    }
}

impl Host for Vm<'_> {
    fn out(&mut self) -> &mut String {
        &mut self.out
    }

    fn fs(&mut self) -> Option<&mut dyn vis_interp::builtin::FsHost> {
        self.fs
            .as_mut()
            .map(|f| &mut **f as &mut dyn vis_interp::builtin::FsHost)
    }

    /// **ST-63 (VD-105)**: host mạng; `None` ⇒ `std.net.*` ⇒ `VS0604`.
    fn net(&mut self) -> Option<&mut dyn vis_interp::builtin::NetHost> {
        self.net
            .as_mut()
            .map(|n| &mut **n as &mut dyn vis_interp::builtin::NetHost)
    }

    /// **VD-241 (Gate 1)**: host cơ sở dữ liệu; `None` ⇒ `std.sqlite.*` ⇒ `VS0604`.
    fn db(&mut self) -> Option<&mut dyn vis_interp::builtin::DbHost> {
        self.db
            .as_mut()
            .map(|d| &mut **d as &mut dyn vis_interp::builtin::DbHost)
    }

    /// **ST-89 (VD-110)**: chương trình ngoài; `None` ⇒ `VS0604`.
    fn ngoai(
        &mut self,
        ten: &str,
        args: &[String],
        raw: bool,
        stdin: &str,
        span: Span,
    ) -> Option<R<Value>> {
        self.ngoai
            .as_mut()
            .map(|n| (**n).chay(ten, args, raw, stdin, span))
    }

    /// **ST-89 (VD-110)**: `| complete` — đủ `stdout/stderr/code` từ host.
    fn ngoai_day_du(
        &mut self,
        ten: &str,
        args: &[String],
        stdin: &str,
        span: Span,
    ) -> Option<Result<vis_interp::builtin::KetQua, vis_interp::interp::Abort>> {
        self.ngoai
            .as_mut()
            .map(|n| (**n).chay_day_du(ten, args, stdin, span))
    }

    fn sched(&self) -> &std::cell::RefCell<vis_interp::rt::Scheduler> {
        &self.sched
    }

    /// **ST-60**: handle của VM là **chỉ số hàm bytecode** — chạy bằng `goi_ham_bytecode`.
    fn goi_than(&mut self, fi: u32, args: Vec<Value>, span: Span) -> R<Value> {
        let f = Rc::new(VmFunc {
            idx: fi,
            captured: Vec::new(),
        });
        self.goi_ham_bytecode(&f, args, span)
    }

    fn ten_than(&self, fi: u32) -> String {
        self.m
            .funcs
            .get(fi as usize)
            .map(|f| f.name.clone())
            .unwrap_or_default()
    }

    fn call(&mut self, f: &Value, args: Vec<Value>, span: Span) -> R<Value> {
        match f {
            Value::Vm(v) => self.goi_ham_bytecode(v, args, span),
            Value::Builtin(n) => builtin::call(self, n, args, span),
            other => Err(Abort::Trap(Trap::new(
                RT,
                format!("`{}` không phải hàm", other.show()),
                span,
            ))),
        }
    }

    fn spawn_task(&mut self, fi: u32, args: Vec<Value>, ten: String, span: Span) -> R<Value> {
        crate::vm::worker::spawn_task(self, fi, args, ten, span)
    }

    fn await_task(&mut self, t: &std::rc::Rc<vis_interp::rt::Task>, span: Span) -> R<Value> {
        crate::vm::worker::await_task(self, t, span)
    }

    fn join_tasks(&mut self) -> R<()> {
        crate::vm::worker::join_tasks(self)
    }
}
