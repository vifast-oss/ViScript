// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`std.*` lõi** (ST-26): ngữ nghĩa dùng **chung** cho interp và VM (VD-55) — một hiện thực duy nhất.
//!
//! Tên đầy đủ có tiền tố module (`str.split`, `list.sort`, `time.now_ms`) — `tyck` kiểm chữ ký theo
//! [`vis_types::stdlib`], tầng này kiểm **kiểu lúc chạy** (sai ⇒ `VS0501` nêu hàm + kiểu nhận được).
//!
//! Mọi hàm **thuần** trừ `std.time.*` (đọc đồng hồ trong Rust, **không** thêm method vào [`Host`]);
//! `std.fs` **không** ở đây (ST-27), `std.json.decode` **chưa** có (VS1b — `VS0301` ở tầng hạ).

use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use vis_diag::COMMAND_CODES;
use vis_syntax::Span;

use crate::builtin::Host;
use crate::builtin_ls::{
    basename_voi, dirname_voi, ext_voi, la_tuyet_doi, noi_duong_voi, stem_voi,
};
use crate::interp::{Abort, Trap, R};
use crate::rt::ReactorOp;
use crate::value::{eq, Value};

mod bisect;
mod chung;
mod flatten;
mod frame_net;
mod fsbytes;
mod fscanonicalize;
mod fsissymlink;
mod fsmkdirall;
mod fsreadlink;
mod indexof;
mod listchunksexact;
mod listcontains;
mod listmath;
mod listmean;
mod listmedian;
mod listminmaxfloat;
mod listsumfloat;
mod mapcontainskey;
mod netconst;
mod pathancestors;
mod pathcomponents;
mod pathisabs;
mod pathjoinall;
mod pathnorm;
mod pathrel;
mod pathrelbool;
mod pathstripprefix;
mod pathwithext;
mod pathwithfilename;
mod pathwithstem;
mod sqlite_explain;
mod sqliteappid;
mod sqlitecols;
mod sqlitedataver;
mod sqliteexists;
mod sqliteidx;
mod sqliteidxcols;
mod sqliteidxexists;
mod sqlitejournal;
mod sqlitepk;
mod sqliteschema;
mod sqliteschemaver;
mod sqlitetables;
mod sqliteuserver;
mod strascii;
mod strasciipreds;
mod strcharat;
mod strfind;
mod strisblank;
mod strlen;
mod strpad;
mod strparse;
mod strparsefloat;
mod strreplace;
mod strreplacelast;
mod strreplacen;
mod strrsplit;
mod strrsplitonce;
mod strsplitincl;
mod strsplitn;
mod strsplitonce;
mod strsplitterm;
mod strstrip;
mod strtitle;
mod strtrim;
pub use crate::json::json;
pub use chung::DAU_QUYEN;
use chung::{ban_do, chuoi, danh_sach, hop_nhat_fs, khong_host, kiem, loi, rowset, so};
use frame_net::frame_unit_result;
pub use frame_net::{
    frame_net_set_write, frame_net_set_write_timeout, frame_net_take_write, frame_net_write_timeout,
};
use netconst::gioi_han_net_read;
pub use netconst::{FRAME_NET_CONNECT, FRAME_NET_WRITE, FRAME_WRITE_TIMEOUT_MS, NET_READ_CAP};

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
        // ----- std.path (thuần chuỗi, không chạm đĩa) -----
        // **VD-258**: Windows nhận ổ đĩa (`C:\`, `C:/`, UNC) và cả `\`/`/`; POSIX giữ nguyên (`builtin_ls`).
        "path.join" => {
            let a = chuoi(name, &args[0], 0, span)?;
            let b = chuoi(name, &args[1], 1, span)?;
            Ok(Value::Str(noi_duong_voi(a, b, cfg!(windows))))
        }
        // **VD-270 (path.join_all)**: nối `list[str]` trái→phải bằng **đúng** quy tắc
        // `path.join` (gấp trái, `[]` ⇒ `""`).
        "path.join_all" => pathjoinall::goi(&args, span),
        // **VD-270 (path.ancestors)**: chuỗi **tổ tiên từ vựng** POSIX gồm chính `p`; chỉ bỏ
        // `/` cuối, giữ `.`/`..`, gốc `/` là cuối (không thêm `""`) — thuần chuỗi.
        "path.ancestors" => pathancestors::goi(&args, span),
        // **VD-270 (path.components)**: tách **từ vựng** POSIX thành thành phần; tuyệt đối ⇒
        // phần tử đầu `"/"`, bỏ thành phần rỗng, giữ nguyên `.`/`..` (không chuẩn hoá).
        "path.components" => pathcomponents::goi(&args, span),
        "path.basename" | "path.dirname" | "path.ext" | "path.stem" => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(match name {
                "path.basename" => basename_voi(p, cfg!(windows)),
                "path.dirname" => dirname_voi(p, cfg!(windows)),
                "path.ext" => ext_voi(p, cfg!(windows)),
                _ => stem_voi(p, cfg!(windows)),
            }))
        }
        "path.is_abs" => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Bool(la_tuyet_doi(p, cfg!(windows))))
        }
        // **VD-270 (path.is_absolute)**: cùng quy tắc `path.is_abs` (không đĩa/`cwd`).
        "path.is_absolute" => pathisabs::goi(&args, span),
        // **VD-270 (path.is_relative)**: phần bù **từ vựng** của `path.is_abs` (không đĩa/`cwd`).
        "path.is_relative" => pathrelbool::goi(&args, span),
        // **VD-270 (path.normalize)**: chuẩn hoá **từ vựng** POSIX (thuần chuỗi, không chạm đĩa).
        "path.normalize" => pathnorm::goi(&args, span),
        // **VD-270 (path.relative)**: đường dẫn tương đối **thuần từ vựng** (không `cwd`/đĩa).
        "path.relative" => pathrel::goi(&args, span),
        // **VD-270 (path.strip_prefix)**: bỏ tiền tố **theo thành phần** (thuần từ vựng POSIX;
        // phần còn lại **nguyên văn**, bỏ đúng một `/`; không chuẩn hoá `.`/`..`).
        "path.strip_prefix" => pathstripprefix::goi(&args, span),
        // **VD-270 (path.with_extension)**: thay/đặt/bỏ phần mở rộng của thành phần cuối
        // (thuần chuỗi; dùng **đúng** `ext_voi`/`stem_voi` của `path.ext`/`path.stem`).
        "path.with_extension" => pathwithext::goi(&args, span),
        // **VD-270 (path.with_file_name)**: thay **toàn bộ** thành phần cuối (tên tệp kể cả phần
        // mở rộng) bằng `name` **nguyên văn**, giữ tiền tố nguyên văn (thuần chuỗi).
        "path.with_file_name" => pathwithfilename::goi(&args, span),
        // **VD-270 (path.with_stem)**: thay **phần thân** (file stem) của thành phần cuối, giữ
        // nguyên phần mở rộng hiện có (thuần chuỗi; dùng **đúng** `stem_voi`/`ext_voi`).
        "path.with_stem" => pathwithstem::goi(&args, span),
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
            // của `crate::json::decode`.
            if let Some(off) = crate::json::zero_dau(s) {
                let e = format!("JSON không hợp lệ ở byte {off}: số có số 0 đầu");
                return Ok(Value::err(Value::ban_ghi_error(e, None)));
            }
            match crate::json::decode(s) {
                // **VD-222 (điểm 4)**: trả `Result[Json, Error]` — bọc cây JSON vào enum `Json`.
                Ok(v) => Ok(Value::ok(crate::json::boc(v))),
                Err(e) => Ok(Value::err(Value::ban_ghi_error(e, None))),
            }
        }
        "json.encode" | "json.encode_pretty" => {
            // **ST-238 / VD-242 (nợ #169, Q4b, §3.5)**: NaN/vô cực (kể cả `Percent`) ⇒ **lỗi**,
            // không xuất token `inf`/`NaN` và không tự đổi `null`. Kiểm đệ thuỳ trước khi mã hoá.
            if crate::json::co_so_phi_huu_han(&args[0]) {
                return Err(loi(
                    "json.encode: giá trị số không hữu hạn (NaN/inf) không được mã hoá",
                    span,
                ));
            }
            let mut s = String::new();
            json(&args[0], name.ends_with("_pretty"), &mut s);
            Ok(Value::Str(s))
        }
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
        "fs.mkdir" | "fs.mkdir_all" => fsmkdirall::goi(host, name, &args, span),
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
            match crate::builtin_fs::loc_glob(&muc, mau) {
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
            hop_nhat_fs(crate::builtin_fs::goi_metadata(host, name, p), span)
        }
        // **VD-270 (fs-is-symlink)**: `true` ⇔ **chính** `p` là symlink (theo `symlink_metadata`,
        // **không** đi theo): symlink tới tệp/thư mục/**đích thiếu** ⇒ `true` (khác `exists`);
        // tệp/thư mục thường, path thiếu ⇒ `false`; lỗi metadata khác ⇒ `Err(Error)`.
        "fs.is_symlink" => fsissymlink::goi(host, name, &args, span),

        // **VD-270 (fs-read-link)**: đọc **đích** symlink, **không** đi theo link (dangling vẫn
        // `Ok`); tệp/thư mục/thiếu/lỗi khác ⇒ `Err(Error)` dữ liệu, không trap.
        "fs.read_link" => fsreadlink::goi(host, name, &args, span),
        // **VD-270 (fs-canonicalize)**: đường dẫn tuyệt đối đã phân giải symlink/`..` bằng
        // `std::fs::canonicalize` (**chạm đĩa** + đọc `cwd`, khác `path.normalize`); `p` phải tồn
        // tại, thiếu/rỗng/lỗi ⇒ `Err(Error)` dữ liệu, không trap; ngoài cap ⇒ `VS0604`.
        "fs.canonicalize" => fscanonicalize::goi(host, name, &args, span),
        // **VD-270 (fsbytes)**: byte thô `list[int]` — helper riêng (R1), cùng một ngữ nghĩa cho
        // interp ↔ VM; lỗi dữ liệu ⇒ `Err(Error)`, ngoài cap ⇒ `VS0604` qua `FsGuard`.
        "fs.read_bytes" | "fs.write_bytes" => fsbytes::goi(host, name, &args, span),
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
mod st238_tests {
    use super::*;

    #[test]
    fn net_read_ky_so_vao_khong_duoc_cap_phat() {
        assert!(gioi_han_net_read(-1).is_err(), "âm bị từ chối");
        assert!(gioi_han_net_read(i64::MAX).is_err(), "i64::MAX bị từ chối");
        assert!(
            gioi_han_net_read(NET_READ_CAP + 1).is_err(),
            "vượt trần bị từ chối"
        );
        assert_eq!(
            gioi_han_net_read(NET_READ_CAP).unwrap(),
            NET_READ_CAP as usize
        );
        assert_eq!(gioi_han_net_read(0).unwrap(), 0);
        assert_eq!(gioi_han_net_read(1024).unwrap(), 1024);
    }

    #[test]
    fn json_non_finite_bi_phat_hien() {
        assert!(crate::json::co_so_phi_huu_han(&Value::Float(f64::NAN)));
        assert!(crate::json::co_so_phi_huu_han(&Value::Float(f64::INFINITY)));
        assert!(crate::json::co_so_phi_huu_han(&Value::Percent(
            f64::NEG_INFINITY
        )));
        assert!(crate::json::co_so_phi_huu_han(&Value::List(vec![
            Value::Int(1),
            Value::Float(f64::NAN)
        ])));
        assert!(!crate::json::co_so_phi_huu_han(&Value::Float(1.5)));
        assert!(!crate::json::co_so_phi_huu_han(&Value::Int(1)));
    }

    // ------------------------------------------------------------------
    // VD-258 — `json.encode` giữ dấu float cho **cả** `Json.Num(Float)`.
    // ------------------------------------------------------------------

    fn json_num(f: f64) -> Value {
        Value::Enum("Json".into(), "Num".into(), vec![Value::Float(f)])
    }

    fn ma_hoa(v: &Value) -> String {
        let mut s = String::new();
        json(v, false, &mut s);
        s
    }

    #[test]
    fn json_num_float_giu_dau_va_roundtrip() {
        // 1.0 không được in "1" (decoder sẽ thành Int) — cả cây thường lẫn `Json.Num`.
        assert_eq!(ma_hoa(&json_num(1.0)), "1.0");
        assert_eq!(ma_hoa(&json_num(1.5)), "1.5");
        for f in [1.0_f64, 1.5, 1e20, 1e300, f64::MAX, 5e-324, 1e-320] {
            let s = ma_hoa(&json_num(f));
            let v = crate::json::decode(&s)
                .unwrap_or_else(|e| panic!("encode({f}) = {s:?} không decode lại được: {e}"));
            match v {
                Value::Float(g) => {
                    assert_eq!(g.to_bits(), f.to_bits(), "roundtrip {f} qua {s:?} cho {g}")
                }
                other => panic!("encode({f}) = {s:?} decode thành {other:?}, không phải Float"),
            }
        }
        // 1e20 trước đây in `100000000000000000000` (không dấu) ⇒ decode thành int tràn i64.
        let s = ma_hoa(&json_num(1e20));
        assert!(
            s.contains('.') || s.contains('e') || s.contains('E'),
            "phải giữ dấu float: {s}"
        );
    }
}
