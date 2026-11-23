// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — dispatch `std.fs.*` + `std.net.*` (chỉ qua `Host`), tách khỏi
//! `stdlib.rs` để giữ tệp dưới 1.150 dòng. Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use super::*;

/// Gọi một hàm `fs.*`/`net.*` theo tên đầy đủ.
pub(crate) fn goi(host: &mut dyn Host, name: &str, args: &[Value], span: Span) -> R<Value> {
    match name {
        // ----- std.fs (ST-33) — **chỉ qua `Host`**, không OS trong crate này -----
        "fs.read" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.read(p).map(Value::Str), span)
        }
        "fs.write" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let v = chuoi(name, &args[1], 1, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.write(p, v).map(|()| Value::Unit), span)
        }
        "fs.exists" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            // **ST-83**: `exists` trả **bool** (không phải Result) nhưng vẫn **trap `VS0604`** khi
            // ngoài quyền — không còn `false` im lặng.
            match host.exists_quyen(p) {
                Ok(b) => Ok(Value::Bool(b)),
                Err(e) => match e.strip_prefix(DAU_QUYEN) {
                    Some(msg) => Err(Abort::Trap(Trap::new(
                        &COMMAND_CODES[3],
                        msg.to_string(),
                        span,
                    ))),
                    None => Ok(Value::Bool(false)),
                },
            }
        }
        "fs.list" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.list(p)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-266**: bốn helper thực dụng — cùng mẫu `std.fs` (host ⇒ `VS0604`; lỗi dữ liệu ⇒
        // `Err(Error)`), không OS trong crate này.
        "fs.read_lines" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.read_lines(p)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        "fs.append" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let v = chuoi(name, &args[1], 1, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.append(p, v).map(|()| Value::Unit), span)
        }
        "fs.mkdir" | "fs.mkdir_all" => fsmkdirall::goi(host, name, args, span),
        "fs.remove" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.remove(p).map(|()| Value::Unit), span)
        }
        // **VD-269**: sao chép/đổi tên tệp — thiếu host ⇒ `VS0604`; lỗi dữ liệu (nguồn thiếu,
        // đích đã tồn tại, thư mục/symlink) ⇒ `Err(Error)`, không trap.
        "fs.copy" => {
            let tu = chuoi(name, &args[0], 0, span)?;
            let den = chuoi(name, &args[1], 1, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.copy(tu, den).map(|()| Value::Unit), span)
        }
        "fs.rename" => {
            let tu = chuoi(name, &args[0], 0, span)?;
            let den = chuoi(name, &args[1], 1, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.rename(tu, den).map(|()| Value::Unit), span)
        }
        // **VD-268**: duyệt đệ quy — đường dẫn tương đối, thư mục kèm `/`; thiếu host ⇒ `VS0604`,
        // lỗi dữ liệu ⇒ `Err(Error)` (cùng `hop_nhat_fs`).
        "fs.walk" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.walk(p)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-268**: `glob` = **lọc** kết quả `walk` bằng bộ khớp **thuần** (không thêm method
        // host ⇒ host thật, `--fs-mock`, interp và VM khớp byte). `walk` vẫn đi qua `FsGuard`
        // trước, nên root ngoài `cap[Dir]` ⇒ `VS0604` và symlink vẫn bị bỏ. Mẫu sai ⇒
        // `Err(Error)` dữ liệu, không trap.
        "fs.glob" => {
            let goc = chuoi(name, &args[0], 0, span)?;
            let mau = chuoi(name, &args[1], 1, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            let muc = match host.walk(goc) {
                Ok(xs) => xs,
                Err(e) => return hop_nhat_fs(Err(e), span),
            };
            match vis_core::host::fs::loc_glob(&muc, mau) {
                Ok(xs) => Ok(Value::ok(Value::List(
                    xs.into_iter().map(Value::Str).collect(),
                ))),
                Err(e) => Ok(Value::err(Value::ban_ghi_error(e, None))),
            }
        }
        // **VD-270**: metadata **chỉ đọc** — `symlink_metadata` (không theo symlink); thiếu path ⇒
        // `size` `Err(Error)`, predicate `Ok(false)`; ngoài cap ⇒ `VS0604` qua `FsGuard`.
        "fs.size" | "fs.is_file" | "fs.is_dir" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(vis_core::host::goi_metadata(host, name, p), span)
        }
        // **VD-270 (fs-is-symlink)**: `true` ⇔ **chính** `p` là symlink (theo `symlink_metadata`,
        // **không** đi theo): symlink tới tệp/thư mục/**đích thiếu** ⇒ `true` (khác `exists`);
        // tệp/thư mục thường, path thiếu ⇒ `false`; lỗi metadata khác ⇒ `Err(Error)`.
        "fs.is_symlink" => fsissymlink::goi(host, name, args, span),

        // **VD-270 (fs-read-link)**: đọc **đích** symlink, **không** đi theo link (dangling vẫn
        // `Ok`); tệp/thư mục/thiếu/lỗi khác ⇒ `Err(Error)` dữ liệu, không trap.
        "fs.read_link" => fsreadlink::goi(host, name, args, span),
        // **VD-270 (fs-canonicalize)**: đường dẫn tuyệt đối đã phân giải symlink/`..` bằng
        // `std::fs::canonicalize` (**chạm đĩa** + đọc `cwd`, khác `path.normalize`); `p` phải tồn
        // tại, thiếu/rỗng/lỗi ⇒ `Err(Error)` dữ liệu, không trap; ngoài cap ⇒ `VS0604`.
        "fs.canonicalize" => fscanonicalize::goi(host, name, args, span),
        // **VD-270 (fs-current-dir)**: `() -> Result[str, Error]` — cwd **tuyệt đối** + UTF-8,
        // đọc môi trường; **không** có path người dùng nên **không** kiểm `cap`/`FsGuard`; cwd
        // xoá/không truy cập/không UTF-8 ⇒ `Err(Error)` dữ liệu, không trap.
        "fs.current_dir" => fscurrentdir::goi(host, name, args, span),
        // **VD-270 (fsbytes)**: byte thô `list[int]` — helper riêng (R1), cùng một ngữ nghĩa cho
        // interp ↔ VM; lỗi dữ liệu ⇒ `Err(Error)`, ngoài cap ⇒ `VS0604` qua `FsGuard`.
        "fs.read_bytes" | "fs.write_bytes" => fsbytes::goi(host, name, args, span),
        // ----- std.net (ST-63, VD-100/VD-105) — **chỉ qua `Host::net`**, không OS trong crate này.
        // Thiếu host ⇒ `VS0604`; lỗi mạng ⇒ `Err(str)` (dữ liệu, **không** trap). -----
        "net.connect" | "net.connect_timeout" => {
            let dia_chi = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                // Mặc định 1 s — đủ rộng cho loopback, không treo chương trình.
                1_000
            };
            // ST-277/VD-256: đường frame có adapter thật ⇒ connect nonblocking. Handle
            // `N:<addr>` chỉ là khoá core; adapter khởi tạo connect, đăng ký WRITABLE,
            // và CHỈ trả tay khi readiness kèm kiểm tra kết quả nối thực tế (SO_ERROR).
            // Deadline/token/gen/cleanup là của `frame_reactor_poll_io` (không đổi rt.rs).
            if host.sched().borrow().frame.enabled && host.sched().borrow().frame.backend.is_some()
            {
                let khoa = format!("{FRAME_NET_CONNECT}{dia_chi}");
                return host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Read,
                    &khoa,
                    0,
                    timeout,
                    span,
                );
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            let r = host.connect(dia_chi, timeout.max(0) as u64);
            hop_nhat_fs(r.map(Value::Str), span)
        }
        "net.read" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let toi_da = so(name, &args[1], 1, span)?;
            let timeout = so(name, &args[2], 2, span)?;
            // ST-268/VD-252 (Lát 2): đường frame test dùng reactor giả — KHÔNG đọc socket
            // thật, KHÔNG busy-poll. Runtime mặc định (frame.enabled == false) đi nguyên văn.
            if host.sched().borrow().frame.enabled {
                let n = match gioi_han_net_read(toi_da) {
                    Ok(n) => n,
                    Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
                };
                let _ = n; // trần `NET_READ_CAP` đã kiểm; reactor giả không cấp phát byte.
                return host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Read,
                    tay,
                    n,
                    timeout,
                    span,
                );
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            // **ST-238 / VD-242 (nợ #167)**: `so()` đã chặn sai kiểu; ở đây chặn âm/overflow/cap
            // trước khi host cấp phát. Mọi trường hợp invalid ⇒ `Result.Err`, không abort.
            let n = match gioi_han_net_read(toi_da) {
                Ok(n) => n,
                Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
            };
            let r = host.read(tay, n, timeout.max(0) as u64);
            // Body là **byte**; ngôn ngữ chỉ có `str` ⇒ giải mã **lossy** (không cắt giữa ký tự).
            // `Ok(vec![])` = EOF ⇒ `""` (không phải lỗi). Nợ: API byte thật cho tới khi có `buf`.
            hop_nhat_fs(
                r.map(|b| Value::Str(String::from_utf8_lossy(&b).into_owned())),
                span,
            )
        }
        "net.write" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let du = chuoi(name, &args[1], 1, span)?;
            // ST-277/VD-256: đường frame có adapter thật ⇒ partial-write có offset.
            // Resume gọi lại builtin với ĐỐI SỐ ĐÃ CACHE (xem `Redispatch`/`frame.rs`),
            // nên bytes được ghi lại đúng từ registry/offset của adapter — KHÔNG gửi lại
            // phần đã gửi. Hạn write là một deadline monotonic duy nhất, không reset.
            if host.sched().borrow().frame.enabled && host.sched().borrow().frame.backend.is_some()
            {
                let khoa = format!("{FRAME_NET_WRITE}{tay}");
                frame_net_set_write(&khoa, du.as_bytes());
                let han = frame_net_write_timeout();
                let r = host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Read,
                    &khoa,
                    du.len(),
                    han,
                    span,
                );
                frame_net_take_write(&khoa); // dọn phần adapter chưa kịp take
                return r.map(frame_unit_result);
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.write(tay, du.as_bytes()).map(|()| Value::Unit), span)
        }
        "net.close" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            // Lát 3: trên đường frame có adapter thật ⇒ đóng socket + vô hiệu mọi
            // đăng ký/stale event của tay này, không cần host mạng mặc định.
            let backend = host.sched().borrow().frame.backend.clone();
            if let Some(b) = backend {
                host.sched().borrow_mut().frame_reactor_cancel_handle(tay);
                // ST-277: một `net.write` đang treo đăng ký dưới khoá `W:<tay>` — phải
                // huỷ cả khoá đó thì lần retry mới thấy tay đã đóng (Err đúng một lần),
                // không đợi tới deadline rồi trả timeout muộn.
                let wk = format!("{FRAME_NET_WRITE}{tay}");
                host.sched().borrow_mut().frame_reactor_cancel_handle(&wk);
                return hop_nhat_fs(b.borrow_mut().close(tay).map(|()| Value::Unit), span);
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.close(tay).map(|()| Value::Unit), span)
        }
        // **ST-65 (VD-100/VD-105)**: server TCP — `listen`/`local_addr`/`accept(_timeout)`/
        // `close_listen`; host giữ listener thật; thiếu host ⇒ `VS0604`; `accept` không timeout = chặn.
        "net.listen" => {
            let dia_chi = chuoi(name, &args[0], 0, span)?;
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.listen(dia_chi).map(Value::Str), span)
        }
        "net.local_addr" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.local_addr(tay).map(Value::Str), span)
        }
        "net.accept" | "net.accept_timeout" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                0
            };
            // ST-268/VD-252 (Lát 2): accept trên đường frame test = reactor giả.
            if host.sched().borrow().frame.enabled {
                return host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Accept,
                    tay,
                    0,
                    timeout,
                    span,
                );
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.accept(tay, timeout.max(0) as u64).map(Value::Str),
                span,
            )
        }
        "net.close_listen" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.close(tay).map(|()| Value::Unit), span)
        }
        // **ST-64 (VD-100/VD-105)**: giao thức HTTP **trong crate này** (`net.rs`), socket qua host.
        // Lỗi HTTP/mạng là `Err(str)` (dữ liệu); thiếu host ⇒ `VS0604`. Mặc định 5000 ms.
        "http.get" | "http.get_timeout" => {
            let url = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                5_000
            };
            // **VS3-1d/W1f-HTTP**: chương trình async chạy state machine reactor, không gọi
            // host blocking giữa request. Không backend ⇒ lỗi rõ, không fallback chặn.
            if host.sched().borrow().frame.enabled {
                if host.sched().borrow().frame.backend.is_some() {
                    return crate::net::http_get_async(host, url, timeout.max(0) as u64, span);
                }
                return Ok(Value::err(Value::ban_ghi_error(
                    "async HTTP cần reactor backend thật (không fallback blocking)",
                    None,
                )));
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                crate::net::http_get(host, url, timeout.max(0) as u64).map(Value::Str),
                span,
            )
        }
        // **ST-109**: `http.head` — chỉ header, không đọc thân; không theo redirect.
        "http.head" | "http.head_timeout" => {
            let url = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                5_000
            };
            if host.sched().borrow().frame.enabled {
                if host.sched().borrow().frame.backend.is_some() {
                    return crate::net::http_head_async(host, url, timeout.max(0) as u64, span);
                }
                return Ok(Value::err(Value::ban_ghi_error(
                    "async HTTP cần reactor backend thật (không fallback blocking)",
                    None,
                )));
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            let kq = crate::net::http_head(host, url, timeout.max(0) as u64).map(|(st, hs)| {
                let headers = Value::Map(
                    hs.into_iter()
                        .map(|(k, v)| (Value::Str(k), Value::Str(v)))
                        .collect(),
                );
                Value::Map(vec![
                    (Value::Str("status".to_string()), Value::Int(st as i64)),
                    (Value::Str("headers".to_string()), headers),
                ])
            });
            hop_nhat_fs(kq, span)
        }
        _ => unreachable!("fs_net_d::goi chỉ nhận tên fs.*/net.*"),
    }
}
