// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`std.*` lõi** (ST-26): ngữ nghĩa dùng **chung** cho interp và VM (VD-55) — một hiện thực duy nhất.
//!
//! Tên đầy đủ có tiền tố module (`str.split`, `list.sort`, `time.now_ms`) — `tyck` kiểm chữ ký theo
//! [`vis_hir::stdlib`], tầng này kiểm **kiểu lúc chạy** (sai ⇒ `VS0501` nêu hàm + kiểu nhận được).
//!
//! Mọi hàm **thuần** trừ `std.time.*` (đọc đồng hồ trong Rust, **không** thêm method vào [`Host`]);
//! `std.fs` **không** ở đây (ST-27), `std.json.decode` **chưa** có (VS1b — `VS0301` ở tầng hạ).

use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use vis_diag::COMMAND_CODES;
use vis_syntax::Span;

use crate::builtin::Host;
use crate::rt::ReactorOp;
use vis_core::trap::{Abort, Trap, R};
use vis_core::value::{eq, Value};

mod fs_net;
mod fs_net_d;
mod helper;
mod list_a;
mod list_b;
mod path_a;
mod path_b;
mod path_c;
mod path_d;
mod sqlite_a;
mod sqlite_b;
mod sqlite_c;
mod sqlite_d;
mod sqlite_e;
mod str_a;
mod str_b;
mod str_c;
mod str_d;
mod str_e;
pub(crate) use fs_net::{
    fsbytes, fscanonicalize, fscurrentdir, fsissymlink, fsmkdirall, fsreadlink,
};
pub(crate) use helper::{bisect, chung, flatten, indexof};
pub(crate) use list_a::{listchunksexact, listcontains, listmath, listmean};
pub(crate) use list_b::{listmedian, listminmaxfloat, listsumfloat, mapcontainskey};
pub(crate) use path_a::{pathancestors, pathcomponents, pathisabs, pathjoinall};
pub(crate) use path_b::{pathnorm, pathrel, pathrelbool, pathstripprefix};
pub(crate) use path_c::{pathwithext, pathwithfilename, pathwithstem};
pub(crate) use sqlite_a::{sqliteappid, sqlitecols, sqlitedataver};
pub(crate) use sqlite_b::{sqlitedblist, sqliteexists, sqliteidx};
pub(crate) use sqlite_c::sqliteidxcols;
pub(crate) use sqlite_d::{sqliteidxexists, sqlitejournal, sqlitepk, sqliteschema};
pub(crate) use sqlite_e::{sqlite_explain, sqliteschemaver, sqlitetables, sqliteuserver};
pub(crate) use str_a::{strascii, strasciipreds, strcharat, strfind, strisblank};
pub(crate) use str_b::{strlen, strmatches, strpad, strparse, strparsefloat};
pub(crate) use str_c::{strreplace, strreplacelast, strreplacen, strrsplit, strrsplitonce};
pub(crate) use str_d::{strsplitincl, strsplitn, strsplitonce};
pub(crate) use str_e::{strsplitterm, strstrip, strtitle, strtrim};

pub(crate) use chung::DAU_QUYEN;
use chung::{ban_do, chuoi, danh_sach, hop_nhat_fs, khong_host, kiem, loi, rowset, so};
use vis_core::json::json;
pub(crate) use vis_core::reactor::{
    frame_net_set_write, frame_net_take_write, frame_net_write_timeout, FRAME_NET_CONNECT,
    FRAME_NET_WRITE,
};
use vis_core::reactor::{frame_unit_result, gioi_han_net_read};

// ===========================================================================
// ST-277 / VD-256 — connect/write nonblocking trên ĐƯỜNG FRAME TEST (chỉ test).
//
// `rt.rs`/trait `FrameReactorBackend` KHÔNG đổi (zero-dep, cross-platform). Adapter
// thật nằm trong integration test Linux; core chỉ cần (a) ghi dữ liệu `net.write` vào
// một kênh chung và (b) đi qua `frame_reactor_poll_io(ReactorOp::Read, ...)` với handle
// được mã hoá `N:<addr>` / `W:<tay>`. Adapter đọc kênh này, tự làm nonblocking
// connect (kiểm tra SO_ERROR khi readiness) và partial write (lưu offset). Mọi cơ chế
// deadline/token/generation/cleanup đã có trong `frame_real_poll` được TÁI DÙNG nguyên
// văn — không helper mới trong `rt.rs`. Runtime mặc định (`frame.enabled == false`)
// và API ngôn ngữ (`net.connect`/`net.write`, chữ ký, giá trị trả) không đổi.
// ===========================================================================

// **VD-241 pre-merge §1 (2026-11-19)**: `DAU_LOI_GOI` (VS0501) **đã bị bỏ** — mọi lỗi host
// (handle giả/đã đóng/sai loại, SQL quá dài, params thiếu/thừa/quá, vượt rows/bytes/connections,
// timeout, authorizer/policy từ chối, lỗi SQLite…) đều là **lỗi dữ liệu/runtime** ⇒ `Result.Err`
// (`Err(Error)`), KHÔNG trap `VS0501`. Chỉ còn **hai** lớp ngoài `Result`: (a) lỗi tĩnh/checker-time
// (sai kiểu/sai arity — tầng `kiem`, không đổi phạm vi), (b) bất biến nội bộ bị phá (bug nội bộ —
// cơ chế chẩn đoán nội bộ hiện hành). Lỗi data **không** mang dấu prefix — cùng mẫu `DAU_QUYEN`
// (lọc bằng `strip_prefix`), khác ở chỗ **không** cần dấu: mọi `Err` host không có `DAU_QUYEN`
// ⇒ `Err(Error)` dữ liệu.

/// Gọi một hàm `std.*` theo tên đầy đủ.
pub fn call(host: &mut dyn Host, name: &str, args: Vec<Value>, span: Span) -> R<Value> {
    kiem(name, &args, span)?;
    match name {
        // ----- std.str -----
        "str.split" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let sep = chuoi(name, &args[1], 1, span)?;
            let ds: Vec<Value> = if sep.is_empty() {
                // Tách theo **ký tự** (Unicode scalar) khi dấu phân cách rỗng.
                s.chars().map(|c| Value::Str(c.to_string())).collect()
            } else {
                s.split(sep).map(|x| Value::Str(x.to_string())).collect()
            };
            Ok(Value::List(ds))
        }
        // **VD-270 (split-once)**: tách tại **lần xuất hiện đầu** của `sep`, đúng nghĩa Rust
        // `str::split_once`: `split_once("a,b,c", ",")` ⇒ `Some(["a", "b,c"])`; không khớp ⇒
        // `None`; `sep` rỗng ⇒ `Some(["", s])`; `s` rỗng + `sep` khác rỗng ⇒ `None`. `Some`
        // luôn có **đúng 2** phần, không trap; lõi ở `stdlib/strsplitonce.rs`.
        "str.split_once" => strsplitonce::goi(name, &args, span),
        // **VD-270 (rsplit)**: tách **phải → trái**, trả các phần theo **thứ tự ngược** (phần
        // cuối trước), đúng nghĩa Rust `str::rsplit`: `rsplit("a,b,c", ",")` ⇒ `["c", "b", "a"]`.
        // Không khớp ⇒ `[s]`; `s` rỗng + `sep` khác rỗng ⇒ `[""]`; dấu cuối sinh phần rỗng
        // **đầu**, dấu đầu sinh phần rỗng **cuối**; chồng lấn không khớp lại (`"aaa"` cắt `"aa"`
        // ⇒ `["", "a"]`); `sep` rỗng theo **đúng** nghĩa Rust (khác `str.split` của VS); lõi ở
        // `stdlib/strrsplit.rs`.
        "str.rsplit" => strrsplit::goi(name, &args, span),
        // **VD-270 (rsplit-once)**: tách tại **lần xuất hiện cuối** của `sep`, đúng nghĩa Rust
        // `str::rsplit_once`: `rsplit_once("a,b,c", ",")` ⇒ `Some(["a,b", "c"])`; không khớp ⇒
        // `None`; `sep` rỗng ⇒ `Some([s, ""])`; `s` rỗng + `sep` khác rỗng ⇒ `None`. `Some`
        // luôn có **đúng 2** phần, không trap; lõi ở `stdlib/strrsplitonce.rs`.
        "str.rsplit_once" => strrsplitonce::goi(name, &args, span),
        // **VD-270 (split-inclusive)**: tách tại mỗi lần xuất hiện **không chồng** của `sep`,
        // **giữ** dấu phân cách ở cuối mỗi phần: `split_inclusive("a,b,c", ",")` ⇒
        // `["a,", "b,", "c"]`; không khớp ⇒ `[s]`; dấu cuối **không** sinh phần rỗng thêm; `sep`
        // rỗng theo **đúng** nghĩa Rust (`"abc"` ⇒ `["", "a", "b", "c"]`); `s` rỗng + `sep` khác
        // rỗng ⇒ `[]` (probe Rust 1.100.0, khác `str.split` của VS); lõi ở
        // `stdlib/strsplitincl.rs`.
        "str.split_inclusive" => strsplitincl::goi(name, &args, span),
        // **VD-270 (split-terminator)**: như `str.split` nhưng **không** sinh phần rỗng cuối
        // khi `s` kết thúc bằng `sep`: `split_terminator("a,b,", ",")` ⇒ `["a", "b"]`;
        // chỉ bỏ **một** phần rỗng cuối (`","` ⇒ `[""]`, `",,"` ⇒ `["", ""]`); `s` rỗng +
        // `sep` khác rỗng ⇒ `[]` (probe Rust 1.100.0); `sep` rỗng theo **đúng** nghĩa Rust
        // (`"abc"` ⇒ `["", "a", "b", "c"]`, `""` ⇒ `[""]`); không đụng
        // `str.split`/`str.splitn`/`str.split_inclusive`; lõi ở `stdlib/strsplitterm.rs`.
        "str.split_terminator" => strsplitterm::goi(name, &args, span),
        // **VD-270 (splitn)**: tối đa `n` phần, tách trái→phải; **phần cuối** giữ nguyên phần
        // còn lại (kể cả dấu phân cách): `splitn("a,b,c", ",", 2)` ⇒ `["a", "b,c"]`.
        // `n == 0` ⇒ `[]`, `n == 1` ⇒ `[s]`, `n > số phần` ⇒ như `str.split`; `sep` rỗng theo
        // **đúng** nghĩa Rust `str::splitn` (khác `str.split` của VS: rỗng ⇒ tách theo scalar,
        // không sinh phần rỗng đầu/cuối). `n < 0` ⇒ trap `VS0501`; lõi ở `stdlib/strsplitn.rs`.
        "str.splitn" => strsplitn::goi(name, &args, span),
        // **VD-265**: `lines` theo nghĩa Rust — tách ở `\n`, bỏ `\r` ngay trước `\n`; dấu
        // xuống dòng cuối **không** sinh dòng rỗng. `split_whitespace` tách theo
        // `char::is_whitespace` (Unicode), bỏ khoảng trắng đầu/cuối và gộp nhiều khoảng liền.
        "str.lines" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(Value::List(
                s.lines().map(|x| Value::Str(x.to_string())).collect(),
            ))
        }
        "str.split_whitespace" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(Value::List(
                s.split_whitespace()
                    .map(|x| Value::Str(x.to_string()))
                    .collect(),
            ))
        }
        "str.join" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let sep = chuoi(name, &args[1], 1, span)?;
            let mut phan: Vec<String> = Vec::with_capacity(xs.len());
            for (i, v) in xs.iter().enumerate() {
                match v {
                    Value::Str(s) => phan.push(s.clone()),
                    other => {
                        return Err(loi(
                            format!("`{name}`: phần tử {} cần `str`, được `{}`", i, other.show()),
                            span,
                        ))
                    }
                }
            }
            Ok(Value::Str(phan.join(sep)))
        }
        "str.trim" | "str.trim_start" | "str.trim_end" | "str.upper" | "str.lower" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(match name {
                "str.trim" => s.trim().to_string(),
                "str.trim_start" => s.trim_start().to_string(),
                "str.trim_end" => s.trim_end().to_string(),
                "str.upper" => s.to_uppercase(),
                _ => s.to_lowercase(),
            }))
        }
        // **VD-270 (str-title)**: hoa **scalar đầu** mỗi từ, thường phần còn lại; biên từ là
        // **khoảng trắng Unicode** (giữ nguyên từng scalar), dấu câu không mở từ mới; `ß` ⇒ `SS`.
        "str.title" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(strtitle::title(s)))
        }
        // **VD-270 (strtrim)**: cắt theo **tập** scalar `chars`; rỗng ⇒ giữ nguyên, hết ⇒ `""`.
        "str.trim_matches" | "str.trim_start_matches" | "str.trim_end_matches" => {
            strtrim::goi(name, &args, span)
        }
        "str.starts_with" | "str.ends_with" | "str.contains" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let p = chuoi(name, &args[1], 1, span)?;
            let ok = match name {
                "str.starts_with" => s.starts_with(p),
                "str.ends_with" => s.ends_with(p),
                _ => s.contains(p),
            };
            Ok(Value::Bool(ok))
        }
        // **VD-270 (strstrip)**: bỏ **đúng một lần** tiền tố/hậu tố ở **đầu/cuối**; khớp ⇒
        // `Some(phần còn lại)`, không khớp ⇒ `None`; affix rỗng ⇒ `Some(s)` (nghĩa Rust
        // `strip_prefix`/`strip_suffix`). Đơn vị khớp theo byte nhưng biên luôn là ranh giới
        // scalar Unicode; lõi ở `stdlib/strstrip.rs`.
        "str.strip_prefix" | "str.strip_suffix" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let moc = chuoi(name, &args[1], 1, span)?;
            Ok(strstrip::goi(name, s, moc))
        }
        "str.repeat" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            if n < 0 {
                return Err(loi(
                    format!("`{name}`: số lần lặp phải ≥ 0, được {n}"),
                    span,
                ));
            }
            // Cap an toàn: 10^7 ký tự (như cap range) — tránh cấp phát khổng lồ.
            let tong = s.len().saturating_mul(n as usize);
            if tong > 10_000_000 {
                return Err(loi(
                    format!("`{name}`: kết quả quá lớn ({tong} byte > 10^7)"),
                    span,
                ));
            }
            Ok(Value::Str(s.repeat(n as usize)))
        }
        "str.replace" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let tu = chuoi(name, &args[1], 1, span)?;
            let den = chuoi(name, &args[2], 2, span)?;
            Ok(Value::Str(s.replace(tu, den)))
        }
        // **VD-270 (replace-first)**: chỉ thay **lần đầu** (Rust `replacen(…, 1)`), cùng đơn vị
        // scalar Unicode với `str.replace`/`find`; `from` rỗng ⇒ chèn `to` một lần ở đầu;
        // không khớp ⇒ `s` nguyên vẹn. Lõi ở `stdlib/strreplace.rs`.
        "str.replace_first" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let tu = chuoi(name, &args[1], 1, span)?;
            let den = chuoi(name, &args[2], 2, span)?;
            Ok(Value::Str(strreplace::replace_first(s, tu, den)))
        }
        // **VD-270 (replace-last)**: chỉ thay **lần cuối** (Rust `rfind` + ghép), cùng đơn vị
        // scalar Unicode với `str.replace`/`replace_first`/`rfind`; `from` rỗng ⇒ nối `to`
        // một lần ở cuối; không khớp ⇒ `s` nguyên vẹn. Lõi ở `stdlib/strreplacelast.rs`.
        "str.replace_last" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let tu = chuoi(name, &args[1], 1, span)?;
            let den = chuoi(name, &args[2], 2, span)?;
            Ok(Value::Str(strreplacelast::replace_last(s, tu, den)))
        }
        // **VD-270 (replace-n)**: thay tối đa `n` lần khớp **không chồng** đầu tiên (trái →
        // phải), đúng nghĩa Rust `replacen`; `n == 0` ⇒ `s` nguyên vẹn, `n` quá số lần ⇒ thay
        // tất cả; `from` rỗng ⇒ chèn tại mọi biên scalar theo `n`; `n < 0` ⇒ trap `VS0501`
        // (kiểm **trước** khi thay, cùng mẫu `str.splitn`/`str.repeat`). Lõi ở
        // `stdlib/strreplacen.rs`.
        "str.replace_n" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let tu = chuoi(name, &args[1], 1, span)?;
            let den = chuoi(name, &args[2], 2, span)?;
            let n = so(name, &args[3], 3, span)?;
            if n < 0 {
                return Err(loi(
                    format!("`{name}`: số lần `n` phải ≥ 0, được {n}"),
                    span,
                ));
            }
            // Trên nền 32-bit, `i64` vượt `usize` ⇒ coi như vô hạn (mọi lần khớp).
            let n = usize::try_from(n).unwrap_or(usize::MAX);
            Ok(Value::Str(strreplacen::replace_n(s, tu, den, n)))
        }
        // **VD-270 (parse-int)**: chuỗi thập phân **chặt** → `int?`; sai cú pháp/tràn ⇒ `None`.
        "str.parse_int" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(strparse::goi(s))
        }
        // **VD-270 (parse-float)**: cú pháp thập phân Rust (`f64::from_str`: dấu, `.5`/`5.`,
        // mũ) rồi **từ chối non-finite** (`inf`/`NaN`/tràn) ⇒ `None`; underflow `±0.0` vẫn
        // `Some`; sai cú pháp ⇒ `None`, không trap.
        "str.parse_float" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(strparsefloat::goi(s))
        }
        // **VD-270 (strfind)**: offset scalar Unicode; `find`/`rfind` ⇒ `int?`, `count` ⇒ `int`.
        "str.find" | "str.rfind" | "str.count" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let needle = chuoi(name, &args[1], 1, span)?;
            Ok(strfind::goi(name, s, needle))
        }
        // **VD-270 (matches)**: **các chuỗi con** khớp `pat` theo đúng `str::matches` của Rust —
        // khớp **không chồng**, trái → phải, `pat` rỗng khớp tại mọi biên scalar; không trap;
        // lõi ở `stdlib/strmatches.rs`.
        "str.matches" => strmatches::goi(name, &args, span),
        // **VD-269**: đệm theo **scalar** Unicode; `fill` rỗng ⇒ trap; `center` dư bên phải.
        "str.pad_left" | "str.pad_right" | "str.center" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let rong = so(name, &args[1], 1, span)?;
            let fill = chuoi(name, &args[2], 2, span)?;
            Ok(Value::Str(strpad::dem(name, s, rong, fill, span)?))
        }
        // **VD-270 (strlen)**: độ dài **byte UTF-8** (`str::len`), số **scalar Unicode**
        // (`chars().count()` — không phải grapheme), và rỗng. Không cấp phát.
        "str.byte_len" | "str.char_count" | "str.is_empty" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(strlen::goi(name, s))
        }
        // **VD-270 (str-is-ascii)**: `true` ⇔ **mọi byte** thuộc `0x00..=0x7F`; rỗng ⇒
        // `true`, scalar non-ASCII ⇒ `false`; không trap, O(độ dài byte). Lõi ở
        // `stdlib/strascii.rs`.
        "str.is_ascii" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(strascii::goi(s))
        }
        // **VD-270 (ascii-preds)**: bốn vị từ ASCII — `true` ⇔ `s` **không rỗng** và **mọi
        // scalar** là ASCII thuộc nhóm (`char::is_ascii_*`); rỗng ⇒ `false` (khác
        // `str.is_ascii("")` ⇒ `true`); không trap, O(số scalar). Lõi ở
        // `stdlib/strasciipreds.rs`.
        "str.is_ascii_digit" => Ok(strasciipreds::goi(
            strasciipreds::Nhom::Digit,
            chuoi(name, &args[0], 0, span)?,
        )),
        "str.is_ascii_alphabetic" => Ok(strasciipreds::goi(
            strasciipreds::Nhom::Alphabetic,
            chuoi(name, &args[0], 0, span)?,
        )),
        "str.is_ascii_alphanumeric" => Ok(strasciipreds::goi(
            strasciipreds::Nhom::Alphanumeric,
            chuoi(name, &args[0], 0, span)?,
        )),
        "str.is_ascii_whitespace" => Ok(strasciipreds::goi(
            strasciipreds::Nhom::Whitespace,
            chuoi(name, &args[0], 0, span)?,
        )),
        // **VD-270 (char-at)**: scalar Unicode thứ `i` (zero-based); âm/quá dài/rỗng ⇒ `None`.
        "str.char_at" => strcharat::goi(name, &args, span),
        // **VD-270 (is-blank)**: rỗng hoặc **mọi scalar** `char::is_whitespace` ⇒ `true`.
        "str.is_blank" => Ok(strisblank::goi(chuoi(name, &args[0], 0, span)?)),
        // ----- std.list -----
        // **VD-270 (strlen)**: danh sách rỗng (0 phần tử) ⇒ `bool`.
        "list.is_empty" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            Ok(Value::Bool(xs.is_empty()))
        }
        "list.sort" => {
            let xs = danh_sach(name, &args[0], 0, span)?.to_vec();
            let mut ds = xs.clone();
            // Phần tử đơn cũng phải kiểm: `sort_by` không gọi comparator cho danh sách 1 phần tử.
            if xs.iter().any(|x| x.cmp_value(x).is_none()) {
                return Err(loi(
                    "`list.sort`: không so được các phần tử (NaN hoặc kiểu chưa có thứ tự)",
                    span,
                ));
            }
            // **ST-240 / VD-242 §3.5 (nợ #168)**: không được `unwrap_or(Equal)` — đó là im lặng
            // giữ nguyên thứ tự khi gặp `NaN` (hoặc kiểu chưa có thứ tự). Báo lỗi theo cơ chế
            // hiện hành (`VS0501`, cùng `sort_by`). Kết quả sắp một phần bị **huỷ hoàn toàn**
            // (trả `Err`, không trả danh sách), nên không có "sắp nửa rồi mới báo lỗi".
            let mut khong_so_duoc = false;
            ds.sort_by(|a, b| match a.cmp_value(b) {
                Some(o) => o,
                None => {
                    khong_so_duoc = true;
                    std::cmp::Ordering::Equal
                }
            });
            if khong_so_duoc {
                return Err(loi(
                    "`list.sort`: không so được các phần tử (NaN hoặc kiểu chưa có thứ tự)",
                    span,
                ));
            }
            Ok(Value::List(ds))
        }
        // ST-53: **cùng lõi** `sap_xep_theo_khoa` với shell `sort_by`; ST-56: khoá nhận **`int`
        // hoặc `str`** (một kiểu trong cả lần gọi) — ca `int` cũ **không đổi**.
        "list.sort_by" => {
            let xs = danh_sach(name, &args[0], 0, span)?.to_vec();
            crate::builtin::sap_xep_theo_khoa(
                host,
                &xs,
                &args[1],
                false,
                crate::builtin::KieuKhoa::ChiIntHoacStr,
                name,
                span,
            )
        }
        "list.reverse" => {
            let mut xs = danh_sach(name, &args[0], 0, span)?.to_vec();
            xs.reverse();
            Ok(Value::List(xs))
        }
        "list.unique" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let mut out: Vec<Value> = Vec::new();
            for x in xs {
                if !out.iter().any(|y| eq(y, x)) {
                    out.push(x.clone());
                }
            }
            Ok(Value::List(out))
        }
        "list.concat" => {
            let a = danh_sach(name, &args[0], 0, span)?.to_vec();
            let b = danh_sach(name, &args[1], 1, span)?;
            let mut out = a;
            out.extend_from_slice(b);
            Ok(Value::List(out))
        }
        // **VD-270 (flatten)**: nối một cấp, giữ thứ tự; phần tử vốn là list giữ nguyên. Lõi ở `stdlib/flatten.rs`.
        "list.flatten" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            flatten::goi(name, xs).map_err(|m| loi(m, span))
        }
        "list.chunk" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            if n <= 0 {
                return Err(loi(
                    format!("`{name}`: kích thước khối phải ≥ 1, được {n}"),
                    span,
                ));
            }
            let n = n as usize;
            Ok(Value::List(
                xs.chunks(n).map(|c| Value::List(c.to_vec())).collect(),
            ))
        }
        // **VD-270 (chunks-exact)**: nhóm **đúng `n`** phần tử, **bỏ phần dư** cuối (khác
        // `list.chunk` giữ nhóm cuối ngắn). `n <= 0` ⇒ trap `VS0501` kiểm **trước khi chia**
        // (kể cả `xs` rỗng); `n > len`/rỗng ⇒ `[]`. Lõi ở `stdlib/listchunksexact.rs`.
        "list.chunks_exact" => listchunksexact::goi(name, &args, span),
        // **VD-270**: cửa sổ **trượt** độ dài `n`, bước 1 (chồng nhau `n-1` phần tử), theo thứ
        // tự. Khác `list.chunk` (nhóm rời). `n <= 0` ⇒ trap `VS0501` — kiểm **trước** dữ liệu
        // nên `list.windows([], 0)` cũng trap; `n > len(xs)` ⇒ `[]`; `n == len(xs)` ⇒ một cửa
        // sổ; `xs` rỗng (với `n > 0`) ⇒ `[]`.
        "list.windows" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            if n <= 0 {
                return Err(loi(
                    format!("`{name}`: kích thước cửa sổ phải ≥ 1, được {n}"),
                    span,
                ));
            }
            let n = n as usize;
            if n > xs.len() {
                return Ok(Value::List(Vec::new()));
            }
            Ok(Value::List(
                xs.windows(n).map(|c| Value::List(c.to_vec())).collect(),
            ))
        }
        // **VD-270 (indexof)**: vị trí **0-based** đầu/cuối theo `eq` cấu trúc (như `==`);
        // không có/`xs` rỗng ⇒ `None`.
        "list.index_of" | "list.last_index_of" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            Ok(indexof::goi(name, xs, &args[1]))
        }
        // **VD-270 (list-contains)**: `true` iff có phần tử `==` `value` — **đúng** phép so
        // cấu trúc của `index_of` (`Value::eq_value`); rỗng/không có ⇒ `false`, không trap.
        // Lõi ở `stdlib/listcontains.rs`.
        "list.contains" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            Ok(listcontains::goi(xs, &args[1]))
        }
        // **VD-270 (binary-search)**: `xs` **đã sắp tăng** theo `cmp_value` (cùng thứ tự
        // `list.sort`/`min`); trả **biên dưới** `Some(vị trí đầu của cụm bằng)` / `None`;
        // không kiểm lại thứ tự, không trap. Lõi ở `stdlib/bisect.rs`.
        "list.binary_search" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            Ok(bisect::goi(xs, &args[1]))
        }
        "list.take" | "list.drop" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            let n = n.max(0) as usize;
            let out: Vec<Value> = if name == "list.take" {
                xs.iter().take(n).cloned().collect()
            } else {
                xs.iter().skip(n).cloned().collect()
            };
            Ok(Value::List(out))
        }
        // **VD-265**: ghép cặp theo vị trí, **dừng ở danh sách ngắn hơn** (zip của Python/
        // Rust); danh sách rỗng ở một bên ⇒ `[]`. Cặp là **list 2 phần tử** (cùng biểu diễn
        // với `enumerate`) để không cần kiểu tuple mới.
        "list.zip" => {
            let a = danh_sach(name, &args[0], 0, span)?;
            let b = danh_sach(name, &args[1], 1, span)?;
            Ok(Value::List(
                a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| Value::List(vec![x.clone(), y.clone()]))
                    .collect(),
            ))
        }
        // **VD-265**: `min`/`max` gộp theo `cmp_value` (cùng thứ tự với `list.sort`); danh sách
        // **rỗng** là trap `VS0501` (dữ liệu không có phần tử), không panic; phần tử không so
        // được (NaN/kiểu lạ) cũng là trap như `sort` — không trả kết quả nửa vời.
        "list.min" | "list.max" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let Some(dau) = xs.first() else {
                return Err(loi(
                    format!("`{name}`: danh sách rỗng không có phần tử để gộp"),
                    span,
                ));
            };
            if xs.iter().any(|x| x.cmp_value(x).is_none()) {
                return Err(loi(
                    format!("`{name}`: không so được các phần tử (NaN hoặc kiểu chưa có thứ tự)"),
                    span,
                ));
            }
            let mut acc = dau.clone();
            for x in &xs[1..] {
                let Some(ord) = x.cmp_value(&acc) else {
                    return Err(loi(
                        format!(
                            "`{name}`: không so được các phần tử (NaN hoặc kiểu chưa có thứ tự)"
                        ),
                        span,
                    ));
                };
                let chon = if name == "list.min" {
                    ord == std::cmp::Ordering::Less
                } else {
                    ord == std::cmp::Ordering::Greater
                };
                if chon {
                    acc = x.clone();
                }
            }
            Ok(acc)
        }
        // **VD-270 (list-sum-product)**: gộp `list[int]` **trái → phải** bằng `+`/`*` checked;
        // rỗng ⇒ `0`/`1`; tràn ở bất kỳ bước nào ⇒ `VS0507` (không wrap). Lõi ở
        // `stdlib/listmath.rs` — **một** hiện thực cho interp + VM. `xs.sum()` vẫn đi phương
        // thức lõi (đứng trước R5c); với `list[int]` ngữ nghĩa trùng, `list[float]` giữ nguyên.
        "list.sum" | "list.product" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            listmath::goi(name, xs, span)
        }
        // **VD-270 (list-mean)**: trung bình `list[float]`; rỗng ⇒ `None`; gộp trái→phải
        // `f64` (IEEE: NaN/inf lan truyền, không trap). Lõi `stdlib/listmean.rs`.
        "list.mean" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            listmean::goi(xs, span)
        }
        // **VD-270 (list-median)**: trung vị `list[float]`; rỗng ⇒ `None`; sao chép rồi sắp
        // tăng (`-inf` < hữu hạn < `+inf`), lẻ ⇒ giữa, chẵn ⇒ `(a + b) / 2.0`; có `NaN` ⇒
        // `None` tất định, không trap. Lõi `stdlib/listmedian.rs`.
        "list.median" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            listmedian::goi(xs, span)
        }
        // **VD-270 (list-minmax-float)**: nhỏ nhất/lớn nhất `list[float]`; rỗng ⇒ `None`; `NaN`
        // ở bất kỳ vị trí ⇒ `None` tất định (cùng chính sách `median`); `±inf` so bình thường;
        // phần tử bằng nhau (kể cả `±0.0`) giữ **đầu tiên** theo thứ tự danh sách. Lõi
        // `stdlib/listminmaxfloat.rs`; sugar R5c `xs.min_float()`/`xs.max_float()`.
        "list.min_float" | "list.max_float" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            listminmaxfloat::goi(name, xs, span)
        }
        // **VD-270 (list-sum-float)**: tổng `list[float]`, hạt giống `+0.0` (rỗng ⇒ `0`); cộng
        // trái→phải `f64` (IEEE: NaN/±inf lan truyền, tràn ⇒ ±inf, không trap — kể cả mọi tổng
        // bằng 0 đều `+0.0`). Lõi `stdlib/listsumfloat.rs`; sugar R5c `xs.sum_float()`.
        "list.sum_float" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            listsumfloat::goi(xs, span)
        }
        // ----- std.map -----
        // **VD-270 (strlen)**: map rỗng (0 cặp) ⇒ `bool`.
        "map.is_empty" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            Ok(Value::Bool(kv.is_empty()))
        }
        // **VD-270 (map-contains-key)**: khoá có mặt theo `eq` cấu trúc như `map.has`.
        "map.contains_key" => mapcontainskey::goi(name, &args, span),
        "map.keys" | "map.values" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            let out: Vec<Value> = if name == "map.keys" {
                kv.iter().map(|(k, _)| k.clone()).collect()
            } else {
                kv.iter().map(|(_, v)| v.clone()).collect()
            };
            Ok(Value::List(out))
        }
        "map.merge" => {
            let a = ban_do(name, &args[0], 0, span)?.to_vec();
            let b = ban_do(name, &args[1], 1, span)?;
            let mut out = a;
            for (k, v) in b {
                match out.iter_mut().find(|(k0, _)| eq(k0, k)) {
                    Some(slot) => slot.1 = v.clone(),
                    None => out.push((k.clone(), v.clone())),
                }
            }
            Ok(Value::Map(out))
        }
        "map.get_or" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            let k = chuoi(name, &args[1], 1, span)?;
            let mac_dinh = args[2].clone();
            let kvk = Value::Str(k.to_string());
            Ok(kv
                .iter()
                .find(|(k0, _)| eq(k0, &kvk))
                .map(|(_, v)| v.clone())
                .unwrap_or(mac_dinh))
        }
        "map.remove" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            let k = chuoi(name, &args[1], 1, span)?;
            let kvk = Value::Str(k.to_string());
            Ok(Value::Map(
                kv.iter().filter(|(k0, _)| !eq(k0, &kvk)).cloned().collect(),
            ))
        }
        name if name.starts_with("path.") => path_d::goi(name, &args, span),
        // ----- std.time (đọc đồng hồ trong Rust; **không** qua Host) -----
        "time.now_ms" => {
            let ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            Ok(Value::Int(ms))
        }
        "time.monotonic_ms" => {
            static GOC: OnceLock<Instant> = OnceLock::new();
            let goc = GOC.get_or_init(Instant::now);
            Ok(Value::Int(goc.elapsed().as_millis() as i64))
        }
        // ----- std.json (encode + decode — ST-33) -----
        "json.decode" => {
            let s = chuoi(name, &args[0], 0, span)?;
            // **ST-240 / VD-242 §3.5 (nợ #175)**: `01` phải bị từ chối. Lỗi là **dữ liệu**
            // (`Result.Err` của chương trình), không phải trap — cùng mẫu với lỗi cú pháp JSON
            // của `vis_core::json::decode`.
            if let Some(off) = vis_core::json::zero_dau(s) {
                let e = format!("JSON không hợp lệ ở byte {off}: số có số 0 đầu");
                return Ok(Value::err(Value::ban_ghi_error(e, None)));
            }
            match vis_core::json::decode(s) {
                // **VD-222 (điểm 4)**: trả `Result[Json, Error]` — bọc cây JSON vào enum `Json`.
                Ok(v) => Ok(Value::ok(vis_core::json::boc(v))),
                Err(e) => Ok(Value::err(Value::ban_ghi_error(e, None))),
            }
        }
        "json.encode" | "json.encode_pretty" => {
            // **ST-238 / VD-242 (nợ #169, Q4b, §3.5)**: NaN/vô cực (kể cả `Percent`) ⇒ **lỗi**,
            // không xuất token `inf`/`NaN` và không tự đổi `null`. Kiểm đệ thuỳ trước khi mã hoá.
            if vis_core::json::co_so_phi_huu_han(&args[0]) {
                return Err(loi(
                    "json.encode: giá trị số không hữu hạn (NaN/inf) không được mã hoá",
                    span,
                ));
            }
            let mut s = String::new();
            json(&args[0], name.ends_with("_pretty"), &mut s);
            Ok(Value::Str(s))
        }
        name if name.starts_with("fs.")
            || name.starts_with("net.")
            || name.starts_with("http.") =>
        {
            fs_net_d::goi(host, name, &args, span)
        }
        // ----- std.sqlite (VD-241, Gate 1) — **chỉ qua `Host::db`**, không backend trong crate này.
        // Thiếu host ⇒ `VS0604`; mọi lỗi host (handle/SQL/params/giới hạn/authorizer/timeout/lỗi
        // SQLite) ⇒ `Err(Error)` dữ liệu (VD-241 pre-merge §1). -----
        "sqlite.open" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            // Mặc định `create = true` (A0 I); busy mặc định 5 s (hữu hạn ≤ 60 s host kẹp).
            hop_nhat_fs(host.db_open(p, true, 5_000).map(Value::Str), span)
        }
        "sqlite.open_with" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let Value::Bool(create) = &args[1] else {
                return Err(loi(
                    format!("`{name}`: đối số 2 cần `bool`, được `{}`", args[1].show()),
                    span,
                ));
            };
            let busy = so(name, &args[2], 2, span)?;
            let create = *create;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_open(p, create, busy).map(Value::Str), span)
        }
        "sqlite.close" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_close(h).map(|()| Value::Unit), span)
        }
        "sqlite.execute" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let params = danh_sach(name, &args[2], 2, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute(h, sql, params).map(Value::Int), span)
        }
        // **VD-270 (db-batch)** — nhiều câu DDL/DML, **không** params; host tách an toàn rồi chạy
        // từng câu qua đúng đường `db_execute` (authorizer/limit/timeout/cap/map lỗi).
        "sqlite.execute_batch" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute_batch(h, sql).map(Value::Int), span)
        }
        "sqlite.query" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let params = danh_sach(name, &args[2], 2, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            match host.db_query(h, sql, params) {
                Ok((columns, rows)) => Ok(rowset(columns, rows)),
                Err(e) => hop_nhat_fs(Err(e), span),
            }
        }
        // **VD-266** — cùng chính sách `query` nhưng tối đa **một** hàng (host dừng sau hàng đầu).
        "sqlite.query_one" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let params = danh_sach(name, &args[2], 2, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            match host.db_query_one(h, sql, params) {
                Ok((columns, rows)) => Ok(rowset(columns, rows)),
                Err(e) => hop_nhat_fs(Err(e), span),
            }
        }
        // **VD-270 (db-explain)** — chỉ nhận `SELECT`/`WITH` sau khoảng trắng/comment đầu; chạy
        // `EXPLAIN QUERY PLAN <sql>` qua **đúng** đường `db_query` (cùng authorizer/chỉ đọc/
        // trần/timeout/cache/bind params) rồi trả `RowSet` kế hoạch.
        "sqlite.explain" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let params = danh_sach(name, &args[2], 2, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            match sqlite_explain::chay(host, h, sql, params) {
                Ok((columns, rows)) => Ok(rowset(columns, rows)),
                Err(e) => hop_nhat_fs(Err(e), span),
            }
        }
        // **VD-270 (sqlite-tables)** — tên bảng/view schema chính, sắp byte, bỏ `sqlite_%`; qua
        // `db_query`; lỗi handle/host/kết quả ⇒ `Err(Error)` dữ liệu, DB rỗng ⇒ `Ok([])`.
        "sqlite.tables" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                sqlitetables::chay(host, h)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-270 (sqlite-table-exists)** — bảng/view tên đúng trong schema chính; tên bind;
        // thiếu ⇒ `Ok(false)`, lỗi handle/host/kết quả sai hình dạng ⇒ `Err(Error)` dữ liệu.
        "sqlite.table_exists" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let ten = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqliteexists::chay(host, h, ten).map(Value::Bool), span)
        }
        // **VD-270 (sqlite-index-exists)** — index tên đúng trong schema chính; tên bind;
        // thiếu/rỗng ⇒ `Ok(false)`, lỗi handle/host/kết quả sai hình dạng ⇒ `Err(Error)` dữ liệu.
        "sqlite.index_exists" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let ten = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqliteidxexists::chay(host, h, ten).map(Value::Bool), span)
        }
        // **VD-270 (sqlite-columns)** — tên cột bảng/view schema chính theo **thứ tự khai báo**
        // (`cid`); tên đối tượng bind; bảng/view thiếu ⇒ `Ok([])`, lỗi ⇒ `Err(Error)` dữ liệu.
        "sqlite.columns" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let table = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                sqlitecols::chay(host, h, table)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-270 (sqlite-indexes)** — tên index người dùng tạo của bảng schema chính, sắp byte,
        // bỏ autoindex; tên bảng bind; bảng/view thiếu ⇒ `Ok([])`, lỗi ⇒ `Err(Error)` dữ liệu.
        "sqlite.indexes" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let table = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                sqliteidx::chay(host, h, table)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-270 (sqlite-schema-sql)** — văn bản SQL khai báo của đối tượng schema chính tra
        // theo tên bind, phân biệt hoa/thường; thiếu/`sql` NULL ⇒ `Ok(None)`, lỗi ⇒ `Err`.
        "sqlite.schema_sql" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let ten = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                sqliteschema::chay(host, h, ten).map(|x| Value::opt(x.map(Value::Str))),
                span,
            )
        }
        // **VD-270 (sqlite-index-columns)** — tên cột index schema chính theo thứ tự index
        // (`seqno`), tra theo tên bind qua `sqlite_master.sql`; thiếu/autoindex ⇒ `Ok([])`,
        // mục biểu thức bị bỏ qua; handle/host/kết quả lỗi ⇒ `Err(Error)` dữ liệu.
        "sqlite.index_columns" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let index = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                sqliteidxcols::chay(host, h, index)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-270 (sqlite-primary-keys)** — tên cột khoá chính của bảng schema chính theo **thứ
        // tự khoá** (`pk` 1..N); tên bảng bind; không PK/view/thiếu ⇒ `Ok([])`, lỗi ⇒ `Err`.
        "sqlite.primary_keys" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let table = chuoi(name, &args[1], 1, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                sqlitepk::chay(host, h, table)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // **VD-270 (sqlite-data-version)** — `data_version` (bộ đếm theo connection), chỉ đọc qua
        // `PRAGMA data_version`; tăng khi connection **khác** commit vào **cùng** DB, không đổi
        // vì ghi/DDL của chính connection; `:memory:` mỗi connection một DB riêng; ngoài
        // `0..=i32::MAX`, lỗi handle/host/kết quả sai hình dạng ⇒ `Err(Error)` dữ liệu, không trap.
        "sqlite.data_version" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqlitedataver::chay(host, h).map(Value::Int), span)
        }
        // **VD-270 (sqlite-schema-version)** — `schema_version` (schema cookie) của **schema
        // chính** (`main`), chỉ đọc qua `PRAGMA schema_version`; mỗi lời gọi đọc lại nên DDL trên
        // cùng connection làm **tăng** giá trị; lỗi handle/host/kết quả sai hình dạng ⇒
        // `Err(Error)` dữ liệu, không trap.
        "sqlite.schema_version" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqliteschemaver::chay(host, h).map(Value::Int), span)
        }
        // **VD-270 (sqlite-user-version)** — `user_version` (số ứng dụng tự định nghĩa trong
        // header tệp DB), chỉ đọc qua `PRAGMA user_version`; DDL/ghi dữ liệu **không** đổi, giá
        // trị lưu theo tệp; ngoài `0..=i32::MAX`, lỗi handle/host/kết quả sai hình dạng ⇒
        // `Err(Error)` dữ liệu, không trap.
        "sqlite.user_version" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqliteuserver::chay(host, h).map(Value::Int), span)
        }
        // **VD-270 (sqlite-application-id)** — `application_id` (định danh ứng dụng trong header
        // tệp DB), chỉ đọc qua `PRAGMA application_id`; DDL/ghi dữ liệu **không** đổi, giá trị
        // lưu theo tệp; ngoài `0..=i32::MAX`, lỗi handle/host/kết quả sai hình dạng ⇒
        // `Err(Error)` dữ liệu, không trap.
        "sqlite.application_id" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqliteappid::chay(host, h).map(Value::Int), span)
        }
        // **VD-270 (sqlite-journal-mode)** — chế độ journal hiện tại của connection, chỉ đọc qua
        // `PRAGMA journal_mode` (không tham số); `:memory:` mới ⇒ `memory`, tệp mới ⇒ `delete`,
        // đọc lặp **không** đổi mode; ghi `PRAGMA journal_mode = …` bị authorizer chối; lỗi
        // handle/host/kết quả sai hình dạng ⇒ `Err(Error)` dữ liệu, không trap.
        "sqlite.journal_mode" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqlitejournal::chay(host, h).map(Value::Str), span)
        }
        // **VD-270 (sqlite-database-list)** — danh sách database gắn với connection, chỉ đọc qua
        // `PRAGMA database_list` (câu hằng, không bind); **mỗi hàng** thành một `RowSet`
        // `seq,name,file`, sắp theo `seq`; lỗi handle/host/kết quả sai hình dạng ⇒ `Err(Error)`
        // dữ liệu, không trap. ATTACH/DETACH vẫn bị authorizer chặn.
        "sqlite.database_list" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(sqlitedblist::chay(host, h).map(Value::List), span)
        }
        // **VD-266** — metadata chỉ đọc của connection; lỗi host ⇒ `Err(Error)` dữ liệu.
        "sqlite.last_insert_rowid" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_last_insert_rowid(h).map(Value::Int), span)
        }
        "sqlite.changes" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_changes(h).map(Value::Int), span)
        }
        "sqlite.begin" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute(h, "BEGIN", &[]).map(|_| Value::Unit), span)
        }
        "sqlite.commit" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute(h, "COMMIT", &[]).map(|_| Value::Unit), span)
        }
        "sqlite.rollback" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.db_execute(h, "ROLLBACK", &[]).map(|_| Value::Unit),
                span,
            )
        }
        "sqlite.in_transaction" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_in_transaction(h).map(Value::Bool), span)
        }
        other => Err(loi(
            format!("`std.{other}` chưa cài (bảng chữ ký có nhưng tầng chạy thiếu)"),
            span,
        )),
    }
}

#[cfg(test)]
mod st238_tests;
