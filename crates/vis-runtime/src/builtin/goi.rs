// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bộ **điều phối lời gọi** builtin ([`call`]) và helper `exec` (ST-34).

use vis_syntax::Span;

use crate::codes::RT;
use vis_core::host::{KetQua, MucFs};
use vis_core::value::Value;

use super::{chuoi_quyen, sap_xep_theo_khoa, thoi_gian_utc, trap, Host, KieuKhoa};

/// Tách `stdout` thành `list[str]` — **bỏ** newline cuối, không sinh dòng rỗng thừa (ST-34).
fn dong_cua(s: &str) -> Value {
    let mut t = s;
    if let Some(x) = t.strip_suffix('\n') {
        t = x;
    }
    let mut dong: Vec<Value> = Vec::new();
    if !t.is_empty() {
        for d in t.split('\n') {
            dong.push(Value::Str(d.to_string()));
        }
    }
    Value::list(dong)
}

/// Record `{stdout: list[str], stderr: list[str], code: int}` cho `| complete` (ST-34).
fn nha_ket_qua(kq: KetQua) -> Value {
    Value::record(
        String::new(),
        vec![
            ("stdout".into(), dong_cua(&kq.stdout)),
            ("stderr".into(), dong_cua(&kq.stderr)),
            ("code".into(), Value::Int(kq.code as i64)),
        ],
    )
}

pub fn call(
    host: &mut dyn Host,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    // **ST-302 (#8)**: runtime `derives Json` — descriptor do lower sinh, người dùng không thấy.
    if name == "$json:to" || name == "$json:from" {
        if args.len() != 2 {
            return Err(trap(RT, "derives Json: sai số đối số nội bộ", span));
        }
        let Value::Str(desc) = &args[1] else {
            return Err(trap(RT, "derives Json: thiếu descriptor", span));
        };
        if name == "$json:to" {
            return vis_core::json::derive::to_json(&args[0], desc)
                .map_err(|e| trap(RT, format!("to_json: {e}"), span));
        }
        return match vis_core::json::derive::from_json(&args[0], desc) {
            Ok(v) => Ok(Value::ok(v)),
            Err(e) => Ok(Value::err(Value::ban_ghi_error(e, None))),
        };
    }
    // **VD-208 (2)**: `$value:<tên>` — dựng **giá trị dựng sẵn** (dùng được như hàm ở cả hai
    // đường). `:` không hợp lệ trong tên ⇒ người dùng không viết được; `vis_fmt`/`check` không đụng.
    // **VD-218 (2026-11-18)**: tên `$value:…` **không bao giờ** xuất hiện ở thông báo lỗi/stack trace.
    if let Some(ten) = name.strip_prefix("$value:") {
        return super::value::value_builtin(ten)
            .ok_or_else(|| trap(RT, "giá trị dựng sẵn không xác định", span));
    }
    // **ST-293 (VD-274/VD-280)**: select nhiều kênh — logic/wait-set ở `rt/select.rs`.
    if matches!(
        name,
        "select_recv" | "select_recv_timeout" | "select_send" | "select_send_timeout"
    ) {
        return crate::rt::select::call(host, name, args, span);
    }
    // **ST-60 (VD-95)**: ba builtin **nội bộ** của async — tên có `$` ⇒ người dùng **không** viết
    // được. `Int` của chúng là **handle của host** (interp: `sym`; VM: chỉ số hàm bytecode),
    // **không** phải giá trị chương trình. Phòng thủ: bytecode đột biến ⇒ `VS0501`, **không** panic.
    if let Some(kieu) = name.strip_prefix('$') {
        // **ST-61 (VD-96)**: `$kenh(n)` — tạo kênh đệm `n` (`n ≥ 0`; âm ⇒ `VS0501`).
        if kieu == "kenh" {
            return match args.into_iter().next() {
                Some(Value::Int(cap)) => crate::rt::tao_kenh(host, cap, span),
                _ => Err(trap(
                    crate::codes::RT,
                    "`chan[T](n)` cần dung lượng `int`",
                    span,
                )),
            };
        }
        // **VD-241 §3** — nội bộ: nâng giá trị tới đủ `depth` lớp `Opt` (đối số 2).
        if kieu == "lift-opt" {
            return Ok(crate::lift::lift_opt_args(args));
        }
        if kieu == "await" {
            return match args.into_iter().next() {
                Some(Value::Task(t)) => host.await_task(&t, span),
                _ => Err(trap(crate::codes::RT, "`await` cần một `Task[T]`", span)),
            };
        }
        let fi = match args.first() {
            Some(Value::Int(i)) if *i >= 0 && *i <= u32::MAX as i64 => *i as u32,
            _ => {
                return Err(trap(
                    crate::codes::RT,
                    format!("`{name}`: handle tác vụ không hợp lệ"),
                    span,
                ))
            }
        };
        let con: Vec<Value> = args.into_iter().skip(1).collect();
        return match kieu {
            "tac_vu" => {
                let t = host
                    .sched()
                    .borrow_mut()
                    .tao(fi, con, host.ten_than(fi), span);
                Ok(Value::Task(t))
            }
            "spawn" => {
                let ten = host.ten_than(fi);
                host.spawn_task(fi, con, ten, span)
            }
            _ => Err(trap(
                crate::codes::RT,
                format!("builtin nội bộ `{name}` chưa hỗ trợ"),
                span,
            )),
        };
    }
    // ST-27/ST-34: `exec.[complete.][raw.]<ten>` — chương trình ngoài, do **host** chạy.
    // - đối số 0: `argv` (`list[str]`); đối số 1 (ST-34): **stdin** (`str` hoặc `list[str]`);
    // - tiền tố `complete.` ⇒ trả **record** `{stdout, stderr, code}`, exit≠0 **không** là lỗi.
    if let Some(mut ten) = name.strip_prefix("exec.") {
        let mut complete = false;
        if let Some(t) = ten.strip_prefix("complete.") {
            ten = t;
            complete = true;
        }
        let mut raw = false;
        if let Some(t) = ten.strip_prefix("raw.") {
            ten = t;
            raw = true;
        }
        let ds: Vec<String> = match args.first() {
            Some(Value::List(xs)) => xs
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => other.show(),
                })
                .collect(),
            _ => Vec::new(),
        };
        // **Một** quy tắc cho cả hai đường (ST-34): `list[str]` ⇒ nối `\n` **+ newline cuối**;
        // `str` ⇒ **nguyên văn** (không thêm newline); khác ⇒ `show()`.
        let stdin = match args.get(1) {
            Some(Value::Str(s)) => s.clone(),
            Some(Value::List(xs)) => {
                let mut t = xs
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => s.clone(),
                        other => other.show(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !t.is_empty() {
                    t.push('\n');
                }
                t
            }
            Some(other) => other.show(),
            None => String::new(),
        };
        if complete {
            return match host.ngoai_day_du(ten, &ds, &stdin, span) {
                Some(Ok(kq)) => Ok(nha_ket_qua(kq)),
                Some(Err(e)) => Err(e),
                None => Err(trap(
                    &vis_diag::COMMAND_CODES[3],
                    format!(
                        "chương trình ngoài `{ten}` chưa hỗ trợ ở môi trường này (không có host)"
                    ),
                    span,
                )),
            };
        }
        return match host.ngoai(ten, &ds, raw, &stdin, span) {
            Some(kq) => kq,
            None => Err(trap(
                &vis_diag::COMMAND_CODES[3],
                format!("chương trình ngoài `{ten}` chưa hỗ trợ ở môi trường này (không có host)"),
                span,
            )),
        };
    }
    // ST-26: `std.*` — một hiện thực dùng chung cho interp và VM (VD-55).
    if name.starts_with("std.") {
        return crate::stdlib::call(host, name.trim_start_matches("std."), args, span);
    }
    match name {
        "print" => {
            // **R6d (VD-178)**: nhiều đối số cách nhau **1 dấu cách**; `print()` ⇒ chỉ `\n`.
            let dong = args.iter().map(Value::show).collect::<Vec<_>>().join(" ");
            host.out().push_str(&dong);
            host.out().push('\n');
            Ok(Value::Unit)
        }
        "str" => {
            if args.len() != 1 {
                return Err(trap(RT, "`str` cần 1 đối số", span));
            }
            Ok(Value::Str(args[0].show()))
        }
        // **ST-79 (VD-108)**: `assert(điều_kiện)` cho `test fn`; sai ⇒ trap `VS0501` để
        // `vis test` in `not ok` (không thêm mã mới — yêu cầu ST-79).
        "assert" => super::assert::assert_builtin(&args, span),
        // **VD-267**: `panic(msg)` — trap `VS0501` (exit 101), không Rust panic/abort.
        "panic" => super::panic::panic_builtin(&args, span),
        // ST-18 — 3 lệnh builtin chế độ lệnh (`echo`/`lines`/`take`), thuần, không FS.
        "echo" => {
            if args.len() > 4 {
                return Err(trap(RT, "`echo` nhận tối đa 4 đối số", span));
            }
            let upper = match args.get(3) {
                Some(Value::Bool(b)) => *b,
                None => false,
                Some(_) => return Err(trap(RT, "`--upper` cần `bool`", span)),
            };
            let mut phan: Vec<String> = Vec::new();
            for a in args.iter().take(3) {
                match a {
                    Value::Str(s) => {
                        if !s.is_empty() {
                            phan.push(s.clone());
                        }
                    }
                    _ => return Err(trap(RT, "`echo` cần đối số `str`", span)),
                }
            }
            let kq = phan.join(" ");
            Ok(Value::Str(if upper { kq.to_uppercase() } else { kq }))
        }
        "lines" => {
            if args.len() != 1 {
                return Err(trap(RT, "`lines` cần 1 đối số", span));
            }
            let Value::Str(s) = &args[0] else {
                return Err(trap(RT, "`lines` cần `str`", span));
            };
            let mut ds: Vec<Value> = s.split('\n').map(|x| Value::Str(x.to_string())).collect();
            if matches!(ds.last(), Some(Value::Str(x)) if x.is_empty()) {
                ds.pop();
            }
            Ok(Value::list(ds))
        }
        "take" => {
            if args.len() != 2 {
                return Err(trap(RT, "`take` cần 2 đối số", span));
            }
            let (Value::List(xs), Value::Int(n)) = (&args[0], &args[1]) else {
                return Err(trap(RT, "`take` cần `list[str]` và `int`", span));
            };
            let n = (*n).max(0) as usize;
            Ok(Value::list(xs.iter().take(n).cloned().collect()))
        }
        // ST-42: `ls`/`open` qua `FsHost` (thật hoặc `--fs-mock`).
        "ls" => {
            let p = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                None => ".".to_string(),
                Some(other) => {
                    return Err(trap(
                        RT,
                        format!("`ls`: đường dẫn cần `str`, được `{}`", other.show()),
                        span,
                    ))
                }
            };
            let all = matches!(args.get(1), Some(Value::Bool(true)));
            let host = host.fs().ok_or_else(|| {
                trap(
                    RT,
                    "`ls` chưa hỗ trợ ở môi trường này (không có host hệ tệp)",
                    span,
                )
            })?;
            let muc = host
                .list_entries(&p)
                .map_err(|e| trap(RT, format!("`ls {p}`: {e}"), span))?;
            let mut ds: Vec<MucFs> = muc
                .into_iter()
                .filter(|m| all || !m.name.starts_with('.'))
                .collect();
            // **Sắp theo tên (byte)** — ổn định giữa hệ tệp.
            ds.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
            let long = matches!(args.get(2), Some(Value::Bool(true)));
            Ok(Value::list(
                ds.into_iter()
                    .map(|m| {
                        // **ST-240 / VD-242 §4 (nợ #171)**: host ép `SystemTime` (u128 ns) sang
                        // `i64`; mtime > 2262 tràn thành số âm ⇒ `thoi_gian_utc` in năm 1861.
                        // Thời gian trước epoch là bất biến sai ở đây ⇒ thay bằng `–` (được VD-242
                        // cho phép) và kẹp `modified` về biên, không in số âm vô nghĩa.
                        let ns = m.modified_ns.max(0);
                        let hien_thoi = m.modified_ns >= 0;
                        let mut truong = vec![
                            ("name".to_string(), Value::Str(m.name)),
                            ("size".to_string(), Value::Size(m.size as i64)),
                            ("modified".to_string(), Value::Duration(ns)),
                            ("is_file".to_string(), Value::Bool(m.is_file)),
                        ];
                        let ten = if long {
                            truong.push(("mode".to_string(), Value::Str(chuoi_quyen(m.mode))));
                            truong.push((
                                "modified_time".to_string(),
                                Value::Str(if hien_thoi {
                                    thoi_gian_utc(ns)
                                } else {
                                    "–".to_string()
                                }),
                            ));
                            "FileEntryLong"
                        } else {
                            "FileEntry"
                        };
                        Value::record(ten, truong)
                    })
                    .collect(),
            ))
        }
        "open" => {
            // ST-56 (phòng thủ): bytecode bị **đột biến** (`fuzz_mutate`) có thể gọi với `argc`
            // sai ⇒ `args[0]` panic. Front end **đã** kiểm số đối số ⇒ mã hợp lệ không đổi.
            let Some(Value::Str(p)) = args.first() else {
                return Err(trap(RT, "`open` cần `str`", span));
            };
            let host = host.fs().ok_or_else(|| {
                trap(
                    RT,
                    "`open` chưa hỗ trợ ở môi trường này (không có host hệ tệp)",
                    span,
                )
            })?;
            let s = host
                .read(p)
                .map_err(|e| trap(RT, format!("`open {p}`: {e}"), span))?;
            Ok(Value::Str(s))
        }
        // ST-41: `Render` — **chỉ** cho giá trị cuối pipeline (`boc_render`); `print` giữ nguyên.
        "render" => {
            // ST-56 (phòng thủ): `argc = 0` từ bytecode **đột biến** từng panic ở đây.
            let Some(v) = args.first() else {
                return Err(trap(RT, "`render` cần 1 đối số", span));
            };
            let co_ban_ghi = matches!(args.get(1), Some(Value::Bool(true)));
            match crate::render::in_gia_tri(v, co_ban_ghi) {
                Some(s) => {
                    host.out().push_str(&s);
                    Ok(Value::Unit)
                }
                None => {
                    host.out().push_str(&v.show());
                    host.out().push('\n');
                    Ok(Value::Unit)
                }
            }
        }
        // ST-40b: `get`/`select` — chiếu từng phần tử qua closure (cùng một hiện thực cho hai đường).
        "get" | "select" => {
            // ST-56 (phòng thủ): số đối số — xem ghi chú ở `open`.
            let (Some(Value::List(xs)), Some(f)) = (args.first(), args.get(1)) else {
                return Err(trap(RT, format!("`{name}` cần `list` và hàm"), span));
            };
            let f = f.clone();
            let mut out: Vec<Value> = Vec::with_capacity(xs.len());
            for x in xs.iter().cloned() {
                out.push(host.call(&f, vec![x], span)?);
            }
            Ok(Value::list(out))
        }
        // ST-39: `sort_by` — khoá `int/size/duration/percent/str`; **ổn định** cả hai chiều (đảo
        // comparator, `sort_by` của Rust ổn định), `--desc` mặc định `false`.
        "sort_by" => {
            // ST-56 (phòng thủ): số đối số — xem ghi chú ở `open`.
            let (Some(Value::List(xs)), Some(f)) = (args.first(), args.get(1)) else {
                return Err(trap(RT, "`sort_by` cần `list` và hàm", span));
            };
            let desc = match args.get(2) {
                None => false,
                Some(Value::Bool(b)) => *b,
                Some(other) => {
                    return Err(trap(
                        RT,
                        format!("`sort_by`: `--desc` cần `bool`, được `{}`", other.show()),
                        span,
                    ))
                }
            };
            // ST-53: cùng lõi với `std.list.sort_by` (khoá rộng hơn, `--desc`).
            sap_xep_theo_khoa(host, xs, f, desc, KieuKhoa::Moi, "sort_by", span)
        }
        // ST-38: `where`/`each` — closure-arg; **một** hiện thực cho cả hai đường (`Host::call`).
        "where" => {
            if args.len() != 2 {
                return Err(trap(RT, "`where` cần 2 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`where` cần `list`", span));
            };
            let f = args[1].clone();
            let mut out: Vec<Value> = Vec::new();
            for x in xs.iter().cloned() {
                match host.call(&f, vec![x.clone()], span)? {
                    Value::Bool(true) => out.push(x),
                    Value::Bool(false) => {}
                    other => {
                        return Err(trap(
                            RT,
                            format!("`where`: hàm lọc phải trả `bool`, được `{}`", other.show()),
                            span,
                        ))
                    }
                }
            }
            Ok(Value::list(out))
        }
        "each" => {
            if args.len() != 2 {
                return Err(trap(RT, "`each` cần 2 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`each` cần `list`", span));
            };
            let f = args[1].clone();
            for x in xs.iter().cloned() {
                host.call(&f, vec![x], span)?;
            }
            Ok(Value::Unit)
        }
        "first" => {
            if args.len() != 1 {
                return Err(trap(RT, "`first` cần 1 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`first` cần `list`", span));
            };
            Ok(Value::opt(xs.first().cloned()))
        }
        "count" => {
            if args.len() != 1 {
                return Err(trap(RT, "`count` cần 1 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`count` cần `list`", span));
            };
            Ok(Value::Int(xs.len() as i64))
        }
        "len" => {
            if args.len() != 1 {
                return Err(trap(RT, "`len` cần 1 đối số", span));
            }
            match &args[0] {
                Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                Value::List(xs) => Ok(Value::Int(xs.len() as i64)),
                Value::Map(kv) => Ok(Value::Int(kv.len() as i64)),
                Value::Bytes(b) => Ok(Value::Int(b.len() as i64)),
                other => Err(trap(
                    RT,
                    format!("`len` chưa hỗ trợ cho `{}` ở VS0", other.show()),
                    span,
                )),
            }
        }
        "Some" => {
            if args.len() != 1 {
                return Err(trap(RT, "`Some` cần 1 đối số", span));
            }
            let mut it = args.into_iter();
            let v = it.next().unwrap_or(Value::Unit);
            Ok(Value::some(v))
        }
        "None" => Ok(Value::none()),
        "Ok" => {
            if args.len() != 1 {
                return Err(trap(RT, "`Ok` cần 1 đối số", span));
            }
            let mut it = args.into_iter();
            let v = it.next().unwrap_or(Value::Unit);
            Ok(Value::ok(v))
        }
        "Err" => {
            if args.len() != 1 {
                return Err(trap(RT, "`Err` cần 1 đối số", span));
            }
            let mut it = args.into_iter();
            let v = it.next().unwrap_or(Value::Unit);
            Ok(Value::err(v))
        }
        // ST-56 (VD-92): `ref(x)` là **biểu thức** hạ identity — cái tên `ref` **không** phải hàm.
        "ref" => Err(trap(
            RT,
            "`ref` chỉ là biểu thức `ref(x)`, không phải hàm",
            span,
        )),
        other => Err(trap(RT, format!("tên `{other}` chưa hỗ trợ ở VS0"), span)),
    }
}
