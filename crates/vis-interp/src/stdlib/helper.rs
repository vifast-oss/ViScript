// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `helper` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod bisect {
    //! **VD-270 (binary-search)** — tìm kiếm nhị phân trong danh sách **đã sắp tăng**:
    //! `std.list.binary_search`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - **Điều kiện tiên quyết**: `xs` đã sắp **tăng dần** theo **cùng** thứ tự `list.sort`/
    //!   `list.min`/`list.max` (so bằng [`Value::cmp_value`]). Hàm **không** kiểm lại thứ tự:
    //!   danh sách chưa sắp là **hành vi không xác định** — vẫn chạy, không trap, nhưng kết quả có
    //!   thể `None` dù phần tử có mặt hoặc trả một vị trí bất kỳ.
    //! - Trả **`Some(i)`** với `i` là **biên dưới** (lower-bound): vị trí **đầu tiên** trong dãy
    //!   phần tử bằng `value`. Có trùng lặp ⇒ luôn là vị trí **đầu** của cụm bằng nhau; không có
    //!   ⇒ `None`; danh sách rỗng ⇒ `None` (không trap).
    //! - **Thuật toán tất định**: vòng `while lo < hi`, `mid = lo + (hi - lo) / 2`; nếu
    //!   `xs[mid] < value` thì `lo = mid + 1`, ngược lại (bằng **hoặc** lớn hơn) `hi = mid`.
    //!   Kết thúc kiểm `xs[lo] == value` theo **thứ tự** (`cmp_value == Equal`), không dùng `eq`
    //!   cấu trúc — nhờ vậy `int` ↔ `float` so được như `list.sort` (`1` và `1.0` là cùng một giá
    //!   trị theo thứ tự).
    //! - Phần tử **không so được** (NaN hoặc kiểu chưa có thứ tự như list/bản ghi) tại một điểm
    //!   giữa ⇒ `None` ngay, **không** trap: điều kiện "sắp theo thứ tự" đã không thể đúng, nên
    //!   không có vị trí đáng tin để trả.
    //! - Chi phí `O(log n)` phép so, không cấp phát.

    use crate::value::Value;

    /// Vị trí **biên dưới** của `value` trong `xs` (đã sắp tăng theo `cmp_value`); `None` nếu
    /// không có hoặc gặp phần tử không so được.
    pub(crate) fn tim(xs: &[Value], value: &Value) -> Option<usize> {
        let mut lo = 0usize;
        let mut hi = xs.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match xs[mid].cmp_value(value)? {
                std::cmp::Ordering::Less => lo = mid + 1,
                // Bằng **hoặc** lớn hơn đều thu hẹp về nửa trái để tìm **biên dưới**.
                std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => hi = mid,
            }
        }
        // `lo` là vị trí chèn đầu tiên; chỉ là khớp khi phần tử tại đó **bằng** `value`.
        (lo < xs.len() && xs[lo].cmp_value(value) == Some(std::cmp::Ordering::Equal)).then_some(lo)
    }

    /// Dispatch chung: `xs` đã sắp tăng, trả `Some(vị trí biên dưới)` hoặc `None`.
    pub(crate) fn goi(xs: &[Value], value: &Value) -> Value {
        Value::opt(tim(xs, value).map(|i| Value::Int(i as i64)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[i64]) -> Vec<Value> {
            xs.iter().copied().map(Value::Int).collect()
        }

        #[test]
        fn dau_giua_cuoi() {
            let xs = ds(&[10, 20, 30, 40, 50]);
            assert_eq!(tim(&xs, &Value::Int(10)), Some(0));
            assert_eq!(tim(&xs, &Value::Int(30)), Some(2));
            assert_eq!(tim(&xs, &Value::Int(50)), Some(4));
        }

        #[test]
        fn khong_co_va_rong() {
            let xs = ds(&[10, 20, 30]);
            assert_eq!(tim(&xs, &Value::Int(5)), None);
            assert_eq!(tim(&xs, &Value::Int(35)), None);
            assert_eq!(tim(&xs, &Value::Int(25)), None);
            assert_eq!(tim(&[], &Value::Int(1)), None);
        }

        #[test]
        fn trung_lap_tra_bien_duoi() {
            let xs = ds(&[1, 2, 2, 2, 3]);
            assert_eq!(tim(&xs, &Value::Int(2)), Some(1));
            assert_eq!(tim(&ds(&[2, 2, 2]), &Value::Int(2)), Some(0));
            assert_eq!(tim(&ds(&[1, 2, 2]), &Value::Int(3)), None);
        }

        #[test]
        fn so_am_va_str() {
            let am = ds(&[-9, -5, -5, -1, 0]);
            assert_eq!(tim(&am, &Value::Int(-5)), Some(1));
            assert_eq!(tim(&am, &Value::Int(-6)), None);
            let chu = vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("b".into()),
            ];
            assert_eq!(tim(&chu, &Value::Str("b".into())), Some(1));
            assert_eq!(tim(&chu, &Value::Str("c".into())), None);
        }

        #[test]
        fn khong_so_duoc_tra_none_khong_trap() {
            // List lồng nhau không có thứ tự trong `cmp_value` ⇒ `None`, không panic.
            let xs = vec![Value::List(vec![Value::Int(1)])];
            assert_eq!(tim(&xs, &Value::List(vec![Value::Int(1)])), None);
        }

        #[test]
        fn ket_qua_la_option_that() {
            let v = goi(&ds(&[5]), &Value::Int(5));
            assert!(v.is_some());
            assert!(matches!(v.opt_payload(), Some(Value::Int(0))));
            let n = goi(&ds(&[5]), &Value::Int(6));
            assert!(n.is_none());
        }
    }
}

pub(crate) mod chung {
    //! Helper dùng chung cho `stdlib` (tách khỏi `stdlib.rs` để giữ R1).

    use vis_diag::{COMMAND_CODES, RUNTIME_CODES};
    use vis_syntax::Span;
    use vis_types::stdlib::{sig_of, Ty};

    use crate::interp::{Abort, Trap};
    use crate::value::Value;

    /// `Err(String)` của host ⇒ `Result` của VS0 (`Err(msg)`), **không** phải trap.
    /// **ST-72b (VD-103/VD-106)**: dấu **cưỡng chế quyền** trong `Err` của host — `hop_nhat_fs` đổi
    /// thành **trap `VS0604`** (không bắt được), thay vì `Err(str)` dữ liệu (vượt quyền là lỗi **môi
    /// trường**, không phải dữ liệu chương trình).
    pub const DAU_QUYEN: &str = "\u{1}VS0604:";

    pub(crate) fn hop_nhat_fs(r: Result<Value, String>, span: Span) -> Result<Value, Abort> {
        match r {
            Ok(v) => Ok(Value::ok(v)),
            Err(e) => match e.strip_prefix(DAU_QUYEN) {
                Some(msg) => Err(Abort::Trap(Trap::new(
                    &COMMAND_CODES[3],
                    msg.to_string(),
                    span,
                ))),
                // Mọi lỗi host khác ⇒ `Err(Error)` dữ liệu (VD-241 pre-merge §1).
                None => Ok(Value::err(Value::ban_ghi_error(e, None))),
            },
        }
    }

    /// **VD-266** — `(columns, rows)` từ host → `Ok(RowSet { columns, rows })` (dùng chung
    /// `sqlite.query`/`query_one`).
    pub(crate) fn rowset(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Value {
        Value::ok(rowset_record(columns, rows))
    }

    /// **VD-270 (sqlite-database-list)** — dựng **một** bản ghi `RowSet` (không bọc `Ok`) để dùng
    /// trong danh sách `list[RowSet]`; cùng hình dạng `columns` + `rows` với [`rowset`].
    pub(crate) fn rowset_record(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Value {
        let columns = Value::List(columns.into_iter().map(Value::Str).collect());
        let rows = Value::List(rows.into_iter().map(Value::List).collect());
        Value::Record(
            "RowSet".into(),
            vec![("columns".into(), columns), ("rows".into(), rows)],
        )
    }

    /// Không có host ⇒ `VS0604` (cùng câu với chương trình ngoài, VD-60 (2)).
    pub(crate) fn khong_host(name: &str, span: Span) -> Abort {
        Abort::Trap(Trap::new(
            &COMMAND_CODES[3],
            format!("`std.{name}` chưa hỗ trợ ở môi trường này (không có host)"),
            span,
        ))
    }

    pub(crate) fn loi(msg: impl Into<String>, span: Span) -> Abort {
        Abort::Trap(Trap::new(&RUNTIME_CODES[0], msg, span))
    }

    /// Kiểm số đối số + kiểu theo bảng chữ ký; trả `Err(Abort)` nếu sai.
    pub(crate) fn kiem(name: &str, args: &[Value], span: Span) -> Result<(), Abort> {
        let Some(sig) = sig_of(name) else {
            return Err(loi(
                format!("hàm `std.{name}` không có trong bảng chữ ký"),
                span,
            ));
        };
        if args.len() != sig.so_doi() {
            return Err(loi(
                format!("`{name}` cần {} đối số, được {}", sig.so_doi(), args.len()),
                span,
            ));
        }
        for (i, (a, t)) in args.iter().zip(sig.params.iter()).enumerate() {
            let ok = match t {
            // **ST-58 (VD-94/VD-99)**: biến kiểu / `list[T]` / `map[K,V]` — thân hàm tự kiểm như
            // trước với `Any` (tyck đã chặn **tĩnh** phần lớn; dữ liệu không kiểu vẫn tới đây).
            Ty::Var(_) | Ty::Map(..) => true,
            // **VD-241**: `list[SqlValue]` (params của `std.sqlite`) — kiểm **phần tử** là `SqlValue`.
            Ty::List(inner) if **inner == Ty::SqlValue => {
                matches!(a, Value::List(xs) if xs.iter().all(|v| matches!(v, Value::Enum(e, _, _) if e == "SqlValue")))
            }
            Ty::List(_) => true,
            Ty::Int => matches!(a, Value::Int(_)),
            Ty::Str => matches!(a, Value::Str(_)),
            Ty::Bool => matches!(a, Value::Bool(_)),
            // **VD-270 (list-mean)**: phần tử `list[float]` — kiểm runtime khi dữ liệu không kiểu.
            Ty::Float => matches!(a, Value::Float(_)),
            Ty::ListStr => {
                matches!(a, Value::List(xs) if xs.iter().all(|v| matches!(v, Value::Str(_))))
            }
            Ty::FnKhoa => matches!(a, Value::Fn(_) | Value::Vm(_) | Value::Builtin(_)),
            // **ST-33: `std.fs` trả `Result[..]` — kiểu **trả**, không dùng để kiểm đối số.
            // **VD-222**: `Error`/`Json`/`Result[Json, Error]` — kiểu **trả** của `std.*` (lỗi/JSON),
            // không dùng để kiểm đối số người dùng; runtime giữ nguyên **tín hiệu** như trước.
            Ty::ResultStrStr
            | Ty::ResultUnitStr
            | Ty::ResultListStrStr
            | Ty::ResultHead
            | Ty::ResultJsonErr
            | Ty::Error
            | Ty::Json
            // **VD-241 (Gate 1)**: `Result[RowSet, Error]` cũng là kiểu **trả** của `std.sqlite.query`.
            | Ty::ResultIntErr
            // **VD-270 (sqlite-journal-mode)**: `Result[str, Error]` là kiểu **trả** của
            // `sqlite.journal_mode` (biến thể riêng, không phải kiểu đối số).
            | Ty::ResultStrErr
            | Ty::ResultBoolErr
            | Ty::ResultRowSetErr
            // **VD-270 (sqlite-database-list)**: `list[RowSet]` / `Result[list[RowSet], Error]`
            // là kiểu **trả** của `sqlite.database_list`.
            | Ty::ListRowSet
            | Ty::ResultListRowSetErr
            // **VD-270 (fsbytes)**: `Result[list[int], Error]` là kiểu **trả** của `fs.read_bytes`.
            | Ty::ResultListIntErr
            // **VD-270 (strfind)**: `int?` là kiểu **trả** của `str.find`/`str.rfind`.
            | Ty::OptInt
            // **VD-270 (strstrip)**: `str?` là kiểu **trả** của `str.strip_prefix`/`strip_suffix`.
            | Ty::OptStr
            // **VD-270 (parse-float)**: `float?` là kiểu **trả** của `str.parse_float`.
            | Ty::OptFloat
            // **VD-270 (split-once)**: `list[str]?` là kiểu **trả** của `str.split_once`.
            | Ty::OptListStr
            // **VD-270 (sqlite-schema-sql)**: `Result[str?, Error]` là kiểu **trả** của
            // `sqlite.schema_sql`.
            | Ty::ResultOptStrErr => true,
            // **VD-241**: `SqlValue` tham số (chưa dùng ở vị trí đối số)/`RowSet` — kiểm runtime nếu đi qua.
            Ty::SqlValue => matches!(a, Value::Enum(e, _, _) if e == "SqlValue"),
            Ty::RowSet => matches!(a, Value::Record(n, _) if n == "RowSet"),
            // **VD-267**: `never` chỉ là kiểu **trả** của builtin trần — không kiểm đối số.
            Ty::Never => true,
        };
            if !ok {
                return Err(loi(
                    format!(
                        "`{name}`: đối số {} cần `{}`, được `{}`",
                        i + 1,
                        t.ten(),
                        a.show()
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn chuoi<'a>(
        name: &str,
        v: &'a Value,
        i: usize,
        span: Span,
    ) -> Result<&'a str, Abort> {
        match v {
            Value::Str(s) => Ok(s),
            other => Err(loi(
                format!(
                    "`{name}`: đối số {} cần `str`, được `{}`",
                    i + 1,
                    other.show()
                ),
                span,
            )),
        }
    }

    pub(crate) fn so(name: &str, v: &Value, i: usize, span: Span) -> Result<i64, Abort> {
        match v {
            Value::Int(n) => Ok(*n),
            other => Err(loi(
                format!(
                    "`{name}`: đối số {} cần `int`, được `{}`",
                    i + 1,
                    other.show()
                ),
                span,
            )),
        }
    }

    pub(crate) fn danh_sach<'a>(
        name: &str,
        v: &'a Value,
        i: usize,
        span: Span,
    ) -> Result<&'a [Value], Abort> {
        match v {
            Value::List(xs) => Ok(xs),
            other => Err(loi(
                format!(
                    "`{name}`: đối số {} cần `list`, được `{}`",
                    i + 1,
                    other.show()
                ),
                span,
            )),
        }
    }

    pub(crate) fn ban_do<'a>(
        name: &str,
        v: &'a Value,
        i: usize,
        span: Span,
    ) -> Result<&'a [(Value, Value)], Abort> {
        match v {
            Value::Map(kv) => Ok(kv),
            other => Err(loi(
                format!(
                    "`{name}`: đối số {} cần `map`, được `{}`",
                    i + 1,
                    other.show()
                ),
                span,
            )),
        }
    }
}

pub(crate) mod flatten {
    //! **VD-270 (flatten)** — nối các list con thành **một** list: `std.list.flatten`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `xs: list[list[T]]` ⇒ `list[T]`: nối các list con **theo thứ tự**, **một cấp** duy nhất.
    //! - `xs` rỗng ⇒ `[]`; list con rỗng không đóng góp phần tử nào (không sinh phần tử giả).
    //! - Phần tử của list con **vốn là list** được giữ **nguyên** làm phần tử — **không** làm phẳng
    //!   đệ quy: `[[[1], [2]], [[3]]]` ⇒ `[[1], [2], [3]]`.
    //! - Không có trần riêng: một lượt quét, cấp phát đúng `tổng độ dài các list con`.
    //!
    //! `xs` do checker buộc là `list[list[T]]`; tầng chạy vẫn kiểm **động** từng phần tử phải là
    //! `list` (dữ liệu không kiểu tới được đây) — sai ⇒ `Err(msg)` cho [`super::super::loi`] dựng trap
    //! `VS0501`, không panic Rust.

    use crate::value::Value;

    /// Nối **một cấp** các list con của `xs`; `name` là tên đầy đủ `"list.flatten"`.
    ///
    /// `Err` là câu lỗi (chưa kèm span) khi một phần tử không phải `Value::List`; caller bọc thành
    /// trap `VS0501` với span lời gọi.
    pub(crate) fn goi(name: &str, xs: &[Value]) -> Result<Value, String> {
        let mut out: Vec<Value> = Vec::with_capacity(xs.iter().map(do_dai).sum());
        for x in xs {
            match x {
                Value::List(inner) => out.extend_from_slice(inner),
                other => {
                    return Err(format!(
                        "`{name}`: mỗi phần tử cần `list`, được `{}`",
                        other.show()
                    ));
                }
            }
        }
        Ok(Value::List(out))
    }

    /// Độ dài dự phòng cho cấp phát: list con ⇒ số phần tử, còn lại ⇒ 0 (sẽ báo lỗi sau).
    fn do_dai(v: &Value) -> usize {
        match v {
            Value::List(inner) => inner.len(),
            _ => 0,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[i64]) -> Value {
            Value::List(xs.iter().copied().map(Value::Int).collect())
        }

        fn phang(xs: Vec<Value>) -> Vec<Value> {
            match goi("list.flatten", &xs).expect("flatten hợp lệ") {
                Value::List(out) => out,
                other => panic!("phải trả list, được {other:?}"),
            }
        }

        /// `Value` không có `PartialEq` (so bằng [`crate::value::eq`] như `==` của ngôn ngữ).
        fn bang(out: &[Value], mong: &[Value]) {
            assert_eq!(out.len(), mong.len(), "độ dài lệch: {out:?} vs {mong:?}");
            for (i, (a, b)) in out.iter().zip(mong).enumerate() {
                assert!(crate::value::eq(a, b), "phần tử {i} lệch: {a:?} vs {b:?}");
            }
        }

        #[test]
        fn noi_theo_thu_tu_va_bo_list_con_rong() {
            let out = phang(vec![ds(&[1, 2]), ds(&[]), ds(&[3]), ds(&[4, 5])]);
            bang(
                &out,
                &[
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                    Value::Int(4),
                    Value::Int(5),
                ],
            );
        }

        #[test]
        fn xs_rong_va_toan_list_con_rong() {
            bang(&phang(Vec::new()), &[]);
            bang(&phang(vec![ds(&[]), ds(&[])]), &[]);
        }

        #[test]
        fn chi_mot_cap_khong_de_quy() {
            // `[[[1], [2]], [[3]]]` ⇒ `[[1], [2], [3]]`: phần tử list được giữ nguyên.
            let out = phang(vec![
                Value::List(vec![ds(&[1]), ds(&[2])]),
                Value::List(vec![ds(&[3])]),
            ]);
            bang(&out, &[ds(&[1]), ds(&[2]), ds(&[3])]);
        }

        #[test]
        fn phan_tu_khong_phai_list_la_loi() {
            let e = goi("list.flatten", &[ds(&[1]), Value::Int(2)]).expect_err("phải báo lỗi");
            assert!(e.contains("list.flatten"), "câu lỗi thiếu tên hàm: {e}");
            assert!(e.contains("list"), "câu lỗi thiếu kiểu cần: {e}");
        }
    }
}

pub(crate) mod indexof {
    //! **VD-270 (indexof)** — vị trí phần tử trong danh sách: `std.list.index_of` /
    //! `std.list.last_index_of`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Chỉ số **0-based**; so bằng [`eq`] — **cùng** ngữ nghĩa `==` (cấu trúc: list/map/record/
    //!   enum lồng nhau, `int` ↔ `float` theo tỉ lệ như `==`).
    //! - `index_of` trả vị trí **đầu tiên**, `last_index_of` vị trí **cuối cùng**; không có phần
    //!   tử bằng `value` ⇒ `None`; danh sách rỗng ⇒ `None` (không trap).
    //! - Quét tuyến tính, không cấp phát danh sách kết quả; không có trần riêng.
    //!
    //! `value` **không** bị ép kiểu ở tầng chạy: checker đã buộc `T` từ `xs`; lời gọi thủ công vẫn
    //! so **cấu trúc** và trả `None` khi không bằng.

    use crate::value::{eq, Value};

    /// Vị trí **đầu tiên** (`cuoi == false`) hoặc **cuối cùng** (`cuoi == true`) có phần tử `==`
    /// `value`; `None` nếu không có (kể cả danh sách rỗng).
    pub(crate) fn tim(xs: &[Value], value: &Value, cuoi: bool) -> Option<usize> {
        if cuoi {
            xs.iter().rposition(|x| eq(x, value))
        } else {
            xs.iter().position(|x| eq(x, value))
        }
    }

    /// Dispatch chung; `name` là `"list.index_of"` hoặc `"list.last_index_of"`.
    pub(crate) fn goi(name: &str, xs: &[Value], value: &Value) -> Value {
        let i = tim(xs, value, name == "list.last_index_of");
        Value::opt(i.map(|i| Value::Int(i as i64)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[i64]) -> Vec<Value> {
            xs.iter().copied().map(Value::Int).collect()
        }

        #[test]
        fn dau_cuoi_va_trung_lap() {
            let xs = ds(&[10, 20, 30, 20]);
            assert_eq!(tim(&xs, &Value::Int(20), false), Some(1));
            assert_eq!(tim(&xs, &Value::Int(20), true), Some(3));
            assert_eq!(tim(&xs, &Value::Int(10), true), Some(0));
            assert_eq!(tim(&xs, &Value::Int(30), false), Some(2));
        }

        #[test]
        fn khong_co_va_danh_sach_rong() {
            assert_eq!(tim(&ds(&[1, 2, 3]), &Value::Int(9), false), None);
            assert_eq!(tim(&ds(&[1, 2, 3]), &Value::Int(9), true), None);
            assert_eq!(tim(&[], &Value::Int(1), false), None);
            assert_eq!(tim(&[], &Value::Int(1), true), None);
        }

        #[test]
        fn so_cau_truc_long_nhau() {
            // Cùng `eq_value` với `==`: list lồng nhau và record so theo **trường**, không theo
            // thứ tự khai báo.
            let xs = vec![
                Value::List(vec![Value::Int(1), Value::Int(2)]),
                Value::List(vec![Value::Int(3), Value::Int(4)]),
                Value::List(vec![Value::Int(1), Value::Int(2)]),
            ];
            let can = Value::List(vec![Value::Int(1), Value::Int(2)]);
            assert_eq!(tim(&xs, &can, false), Some(0));
            assert_eq!(tim(&xs, &can, true), Some(2));

            let a = Value::Record(
                "P".into(),
                vec![("x".into(), Value::Int(1)), ("y".into(), Value::Int(2))],
            );
            let b = Value::Record(
                "P".into(),
                vec![("y".into(), Value::Int(2)), ("x".into(), Value::Int(1))],
            );
            assert_eq!(tim(&[a], &b, false), Some(0));
        }

        #[test]
        fn ket_qua_la_option_that() {
            let v = goi("list.index_of", &ds(&[5]), &Value::Int(5));
            assert!(v.is_some());
            assert!(matches!(v.opt_payload(), Some(Value::Int(0))));
            let n = goi("list.last_index_of", &ds(&[5]), &Value::Int(6));
            assert!(n.is_none());
        }
    }
}
