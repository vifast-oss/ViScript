// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-317c** — **một nguồn** cho `std.*`: `StdId` + tên + chữ ký + metadata (module).
//!
//! Bảng `STD_REGISTRY` (macro `std_registry!`) là nơi **duy nhất** khai báo hàm `std.*`.
//! Macro sinh tại compile-time: enum `StdId` (mỗi tên đúng một biến thể, thứ tự khai = `index()`),
//! `COUNT`, `ALL`, `as_str`/`from_name`, metadata `module`, và bảng chữ ký [`FUNCS`].
//!
//! **Thêm một hàm `std.*` = đúng 2 chỗ**:
//! 1. một hàng trong bảng dưới đây (biến thể + tên + chữ ký);
//! 2. một nhánh `StdId::<BiếnThể>` trong `vis_runtime::stdlib::call_by_id`.
//!
//! Không có `_ =>` ở dispatcher nên rustc bắt **thiếu nhánh** tại compile-time (exhaustive 100 %).
//! `MODULES` là metadata cấp module; test khoá `FUNCS` ↔ `MODULES` để không lệch.

use super::dinh_nghia::StdFn;
use super::ty::Ty;

/// Sinh `StdId` + `ALL` + `COUNT` + tra cứu tên + `FUNCS` từ **một** bảng khai báo.
///
/// Cú pháp một hàng: `<BiếnThể> => "<module.tên>": [<tham số>] -> <kiểu trả>,`
macro_rules! std_registry {
    ($( $id:ident => $ten:literal : [ $( $p:expr ),* $(,)? ] -> $ret:expr, )+) => {
        /// Mã định danh một hàm `std.*` — sinh từ bảng `STD_REGISTRY`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum StdId {
            $( $id, )+
        }

        impl StdId {
            /// Tổng số hàm `std.*` — bằng `FUNCS.len()` (test khoá song ánh).
            pub const COUNT: usize = [$( stringify!($id) ),+].len();

            /// Toàn bộ id theo thứ tự khai (dùng cho test song ánh).
            pub const ALL: [StdId; Self::COUNT] = [ $( StdId::$id, )+ ];

            /// Tên đầy đủ `module.ham` tương ứng.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( StdId::$id => $ten, )+
                }
            }

            /// Metadata: module `std` của hàm (`"fs"`, `"str"`, …).
            pub fn module(self) -> &'static str {
                match self.as_str().split_once('.') {
                    Some((m, _)) => m,
                    None => "",
                }
            }

            /// Tra id theo tên đầy đủ — `None` nếu không phải hàm `std.*`.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $( $ten => Some(Self::$id), )+
                    _ => None,
                }
            }

            /// Chỉ số khai báo (`0..COUNT`) — dùng làm chỉ số mảng dispatch.
            pub fn index(self) -> usize {
                self as usize
            }
        }

        /// Toàn bộ hàm `std.*` (tên + chữ ký) — sinh từ **cùng** bảng registry, giữ thứ tự khai.
        pub const FUNCS: [StdFn; StdId::COUNT] = [
            $( StdFn {
                name: $ten,
                params: &[ $( $p ),* ],
                ret: $ret,
            }, )+
        ];
    };
}

// Bảng khai báo **duy nhất** cho hàm `std.*` — thứ tự hàng = thứ tự `FUNCS` (giữ nguyên từ
// ST-296c) và = thứ tự biến thể `StdId`.
//
// `STD_REGISTRY` là tên tài liệu cho khối macro dưới; dữ liệu sống trong lời gọi
// `std_registry!` ngay bên dưới.
std_registry! {
    // ----- std.path -----
    PathJoin => "path.join": [Ty::Str, Ty::Str] -> Ty::Str,
    PathJoinAll => "path.join_all": [Ty::ListStr] -> Ty::Str,
    PathAncestors => "path.ancestors": [Ty::Str] -> Ty::ListStr,
    PathComponents => "path.components": [Ty::Str] -> Ty::ListStr,
    PathBasename => "path.basename": [Ty::Str] -> Ty::Str,
    PathDirname => "path.dirname": [Ty::Str] -> Ty::Str,
    PathExt => "path.ext": [Ty::Str] -> Ty::Str,
    PathStem => "path.stem": [Ty::Str] -> Ty::Str,
    PathIsAbs => "path.is_abs": [Ty::Str] -> Ty::Bool,
    PathIsAbsolute => "path.is_absolute": [Ty::Str] -> Ty::Bool,
    PathIsRelative => "path.is_relative": [Ty::Str] -> Ty::Bool,
    PathNormalize => "path.normalize": [Ty::Str] -> Ty::Str,
    PathRelative => "path.relative": [Ty::Str, Ty::Str] -> Ty::Str,
    PathStripPrefix => "path.strip_prefix": [Ty::Str, Ty::Str] -> Ty::OptStr,
    PathWithExtension => "path.with_extension": [Ty::Str, Ty::Str] -> Ty::Str,
    PathWithFileName => "path.with_file_name": [Ty::Str, Ty::Str] -> Ty::Str,
    PathWithStem => "path.with_stem": [Ty::Str, Ty::Str] -> Ty::Str,
    // ----- std.fs -----
    FsRead => "fs.read": [Ty::Str] -> Ty::ResultStrStr,
    FsWrite => "fs.write": [Ty::Str, Ty::Str] -> Ty::ResultUnitStr,
    FsExists => "fs.exists": [Ty::Str] -> Ty::Bool,
    FsList => "fs.list": [Ty::Str] -> Ty::ResultListStrStr,
    FsReadLines => "fs.read_lines": [Ty::Str] -> Ty::ResultListStrStr,
    FsAppend => "fs.append": [Ty::Str, Ty::Str] -> Ty::ResultUnitStr,
    FsMkdir => "fs.mkdir": [Ty::Str] -> Ty::ResultUnitStr,
    FsMkdirAll => "fs.mkdir_all": [Ty::Str] -> Ty::ResultUnitStr,
    FsRemove => "fs.remove": [Ty::Str] -> Ty::ResultUnitStr,
    FsCopy => "fs.copy": [Ty::Str, Ty::Str] -> Ty::ResultUnitStr,
    FsCurrentDir => "fs.current_dir": [] -> Ty::ResultStrErr,
    FsRename => "fs.rename": [Ty::Str, Ty::Str] -> Ty::ResultUnitStr,
    FsWalk => "fs.walk": [Ty::Str] -> Ty::ResultListStrStr,
    FsGlob => "fs.glob": [Ty::Str, Ty::Str] -> Ty::ResultListStrStr,
    FsSize => "fs.size": [Ty::Str] -> Ty::ResultIntErr,
    FsIsFile => "fs.is_file": [Ty::Str] -> Ty::ResultBoolErr,
    FsIsDir => "fs.is_dir": [Ty::Str] -> Ty::ResultBoolErr,
    FsIsSymlink => "fs.is_symlink": [Ty::Str] -> Ty::ResultBoolErr,
    FsReadLink => "fs.read_link": [Ty::Str] -> Ty::ResultStrStr,
    FsCanonicalize => "fs.canonicalize": [Ty::Str] -> Ty::ResultStrStr,
    FsReadBytes => "fs.read_bytes": [Ty::Str] -> Ty::ResultListIntErr,
    FsWriteBytes => "fs.write_bytes": [Ty::Str, Ty::List(&Ty::Int)] -> Ty::ResultUnitStr,
    // ----- std.list -----
    ListIsEmpty => "list.is_empty": [Ty::List(&Ty::Var(0))] -> Ty::Bool,
    ListSort => "list.sort": [Ty::List(&Ty::Var(0))] -> Ty::List(&Ty::Var(0)),
    ListSortBy => "list.sort_by": [Ty::List(&Ty::Var(0)), Ty::FnKhoa] -> Ty::List(&Ty::Var(0)),
    ListReverse => "list.reverse": [Ty::List(&Ty::Var(0))] -> Ty::List(&Ty::Var(0)),
    ListUnique => "list.unique": [Ty::List(&Ty::Var(0))] -> Ty::List(&Ty::Var(0)),
    ListConcat => "list.concat": [Ty::List(&Ty::Var(0)), Ty::List(&Ty::Var(0))] -> Ty::List(&Ty::Var(0)),
    ListFlatten => "list.flatten": [Ty::List(&Ty::List(&Ty::Var(0)))] -> Ty::List(&Ty::Var(0)),
    ListChunk => "list.chunk": [Ty::List(&Ty::Var(0)), Ty::Int] -> Ty::List(&Ty::List(&Ty::Var(0))),
    ListChunksExact => "list.chunks_exact": [Ty::List(&Ty::Var(0)), Ty::Int] -> Ty::List(&Ty::List(&Ty::Var(0))),
    ListWindows => "list.windows": [Ty::List(&Ty::Var(0)), Ty::Int] -> Ty::List(&Ty::List(&Ty::Var(0))),
    ListIndexOf => "list.index_of": [Ty::List(&Ty::Var(0)), Ty::Var(0)] -> Ty::OptInt,
    ListLastIndexOf => "list.last_index_of": [Ty::List(&Ty::Var(0)), Ty::Var(0)] -> Ty::OptInt,
    ListContains => "list.contains": [Ty::List(&Ty::Var(0)), Ty::Var(0)] -> Ty::Bool,
    ListBinarySearch => "list.binary_search": [Ty::List(&Ty::Var(0)), Ty::Var(0)] -> Ty::OptInt,
    ListTake => "list.take": [Ty::List(&Ty::Var(0)), Ty::Int] -> Ty::List(&Ty::Var(0)),
    ListDrop => "list.drop": [Ty::List(&Ty::Var(0)), Ty::Int] -> Ty::List(&Ty::Var(0)),
    ListZip => "list.zip": [Ty::List(&Ty::Var(0)), Ty::List(&Ty::Var(0))] -> Ty::List(&Ty::List(&Ty::Var(0))),
    ListMin => "list.min": [Ty::List(&Ty::Var(0))] -> Ty::Var(0),
    ListMax => "list.max": [Ty::List(&Ty::Var(0))] -> Ty::Var(0),
    ListSum => "list.sum": [Ty::List(&Ty::Int)] -> Ty::Int,
    ListProduct => "list.product": [Ty::List(&Ty::Int)] -> Ty::Int,
    ListMean => "list.mean": [Ty::List(&Ty::Float)] -> Ty::OptFloat,
    ListMedian => "list.median": [Ty::List(&Ty::Float)] -> Ty::OptFloat,
    ListSumFloat => "list.sum_float": [Ty::List(&Ty::Float)] -> Ty::Float,
    ListMinFloat => "list.min_float": [Ty::List(&Ty::Float)] -> Ty::OptFloat,
    ListMaxFloat => "list.max_float": [Ty::List(&Ty::Float)] -> Ty::OptFloat,
    // ----- std.map -----
    MapIsEmpty => "map.is_empty": [Ty::Map(&Ty::Var(1), &Ty::Var(2))] -> Ty::Bool,
    MapContainsKey => "map.contains_key": [Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Var(1)] -> Ty::Bool,
    MapKeys => "map.keys": [Ty::Map(&Ty::Var(1), &Ty::Var(2))] -> Ty::List(&Ty::Var(1)),
    MapValues => "map.values": [Ty::Map(&Ty::Var(1), &Ty::Var(2))] -> Ty::List(&Ty::Var(2)),
    MapMerge => "map.merge": [Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Map(&Ty::Var(1), &Ty::Var(2)),] -> Ty::Map(&Ty::Var(1), &Ty::Var(2)),
    MapGetOr => "map.get_or": [Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Str, Ty::Var(2)] -> Ty::Var(2),
    MapRemove => "map.remove": [Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Str] -> Ty::Map(&Ty::Var(1), &Ty::Var(2)),
    // ----- std.time -----
    TimeNowMs => "time.now_ms": [] -> Ty::Int,
    TimeMonotonicMs => "time.monotonic_ms": [] -> Ty::Int,
    // ----- std.json -----
    JsonEncode => "json.encode": [Ty::Var(0)] -> Ty::Str,
    JsonEncodePretty => "json.encode_pretty": [Ty::Var(0)] -> Ty::Str,
    JsonDecode => "json.decode": [Ty::Str] -> Ty::ResultJsonErr,
    // ----- std.net -----
    NetConnect => "net.connect": [Ty::Str] -> Ty::ResultStrStr,
    NetConnectTimeout => "net.connect_timeout": [Ty::Str, Ty::Int] -> Ty::ResultStrStr,
    NetRead => "net.read": [Ty::Str, Ty::Int, Ty::Int] -> Ty::ResultStrStr,
    NetWrite => "net.write": [Ty::Str, Ty::Str] -> Ty::ResultUnitStr,
    NetClose => "net.close": [Ty::Str] -> Ty::ResultUnitStr,
    NetListen => "net.listen": [Ty::Str] -> Ty::ResultStrStr,
    NetLocalAddr => "net.local_addr": [Ty::Str] -> Ty::ResultStrStr,
    NetAccept => "net.accept": [Ty::Str] -> Ty::ResultStrStr,
    NetAcceptTimeout => "net.accept_timeout": [Ty::Str, Ty::Int] -> Ty::ResultStrStr,
    NetCloseListen => "net.close_listen": [Ty::Str] -> Ty::ResultUnitStr,
    // ----- std.http -----
    HttpGet => "http.get": [Ty::Str] -> Ty::ResultStrStr,
    HttpGetTimeout => "http.get_timeout": [Ty::Str, Ty::Int] -> Ty::ResultStrStr,
    HttpHead => "http.head": [Ty::Str] -> Ty::ResultHead,
    HttpHeadTimeout => "http.head_timeout": [Ty::Str, Ty::Int] -> Ty::ResultHead,
    // ----- std.sqlite -----
    SqliteOpen => "sqlite.open": [Ty::Str] -> Ty::ResultStrStr,
    SqliteOpenWith => "sqlite.open_with": [Ty::Str, Ty::Bool, Ty::Int] -> Ty::ResultStrStr,
    SqliteClose => "sqlite.close": [Ty::Str] -> Ty::ResultUnitStr,
    SqliteExecute => "sqlite.execute": [Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)] -> Ty::ResultIntErr,
    SqliteExecuteBatch => "sqlite.execute_batch": [Ty::Str, Ty::Str] -> Ty::ResultIntErr,
    SqliteQuery => "sqlite.query": [Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)] -> Ty::ResultRowSetErr,
    SqliteQueryOne => "sqlite.query_one": [Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)] -> Ty::ResultRowSetErr,
    SqliteExplain => "sqlite.explain": [Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)] -> Ty::ResultRowSetErr,
    SqliteLastInsertRowid => "sqlite.last_insert_rowid": [Ty::Str] -> Ty::ResultIntErr,
    SqliteChanges => "sqlite.changes": [Ty::Str] -> Ty::ResultIntErr,
    SqliteBegin => "sqlite.begin": [Ty::Str] -> Ty::ResultUnitStr,
    SqliteCommit => "sqlite.commit": [Ty::Str] -> Ty::ResultUnitStr,
    SqliteRollback => "sqlite.rollback": [Ty::Str] -> Ty::ResultUnitStr,
    SqliteInTransaction => "sqlite.in_transaction": [Ty::Str] -> Ty::ResultBoolErr,
    SqliteTables => "sqlite.tables": [Ty::Str] -> Ty::ResultListStrStr,
    SqliteTableExists => "sqlite.table_exists": [Ty::Str, Ty::Str] -> Ty::ResultBoolErr,
    SqliteIndexExists => "sqlite.index_exists": [Ty::Str, Ty::Str] -> Ty::ResultBoolErr,
    SqliteColumns => "sqlite.columns": [Ty::Str, Ty::Str] -> Ty::ResultListStrStr,
    SqliteIndexes => "sqlite.indexes": [Ty::Str, Ty::Str] -> Ty::ResultListStrStr,
    SqliteSchemaSql => "sqlite.schema_sql": [Ty::Str, Ty::Str] -> Ty::ResultOptStrErr,
    SqliteIndexColumns => "sqlite.index_columns": [Ty::Str, Ty::Str] -> Ty::ResultListStrStr,
    SqlitePrimaryKeys => "sqlite.primary_keys": [Ty::Str, Ty::Str] -> Ty::ResultListStrStr,
    SqliteSchemaVersion => "sqlite.schema_version": [Ty::Str] -> Ty::ResultIntErr,
    SqliteUserVersion => "sqlite.user_version": [Ty::Str] -> Ty::ResultIntErr,
    SqliteApplicationId => "sqlite.application_id": [Ty::Str] -> Ty::ResultIntErr,
    SqliteDataVersion => "sqlite.data_version": [Ty::Str] -> Ty::ResultIntErr,
    SqliteJournalMode => "sqlite.journal_mode": [Ty::Str] -> Ty::ResultStrErr,
    SqliteDatabaseList => "sqlite.database_list": [Ty::Str] -> Ty::ResultListRowSetErr,
    // ----- std.str -----
    StrSplit => "str.split": [Ty::Str, Ty::Str] -> Ty::ListStr,
    StrSplitOnce => "str.split_once": [Ty::Str, Ty::Str] -> Ty::OptListStr,
    StrRsplitOnce => "str.rsplit_once": [Ty::Str, Ty::Str] -> Ty::OptListStr,
    StrSplitInclusive => "str.split_inclusive": [Ty::Str, Ty::Str] -> Ty::ListStr,
    StrSplitTerminator => "str.split_terminator": [Ty::Str, Ty::Str] -> Ty::ListStr,
    StrSplitn => "str.splitn": [Ty::Str, Ty::Str, Ty::Int] -> Ty::ListStr,
    StrRsplit => "str.rsplit": [Ty::Str, Ty::Str] -> Ty::ListStr,
    StrLines => "str.lines": [Ty::Str] -> Ty::ListStr,
    StrSplitWhitespace => "str.split_whitespace": [Ty::Str] -> Ty::ListStr,
    StrJoin => "str.join": [Ty::ListStr, Ty::Str] -> Ty::Str,
    StrTrim => "str.trim": [Ty::Str] -> Ty::Str,
    StrUpper => "str.upper": [Ty::Str] -> Ty::Str,
    StrLower => "str.lower": [Ty::Str] -> Ty::Str,
    StrTitle => "str.title": [Ty::Str] -> Ty::Str,
    StrStartsWith => "str.starts_with": [Ty::Str, Ty::Str] -> Ty::Bool,
    StrEndsWith => "str.ends_with": [Ty::Str, Ty::Str] -> Ty::Bool,
    StrContains => "str.contains": [Ty::Str, Ty::Str] -> Ty::Bool,
    StrFind => "str.find": [Ty::Str, Ty::Str] -> Ty::OptInt,
    StrRfind => "str.rfind": [Ty::Str, Ty::Str] -> Ty::OptInt,
    StrCount => "str.count": [Ty::Str, Ty::Str] -> Ty::Int,
    StrMatches => "str.matches": [Ty::Str, Ty::Str] -> Ty::ListStr,
    StrStripPrefix => "str.strip_prefix": [Ty::Str, Ty::Str] -> Ty::OptStr,
    StrStripSuffix => "str.strip_suffix": [Ty::Str, Ty::Str] -> Ty::OptStr,
    StrRepeat => "str.repeat": [Ty::Str, Ty::Int] -> Ty::Str,
    StrReplace => "str.replace": [Ty::Str, Ty::Str, Ty::Str] -> Ty::Str,
    StrReplaceFirst => "str.replace_first": [Ty::Str, Ty::Str, Ty::Str] -> Ty::Str,
    StrReplaceLast => "str.replace_last": [Ty::Str, Ty::Str, Ty::Str] -> Ty::Str,
    StrReplaceN => "str.replace_n": [Ty::Str, Ty::Str, Ty::Str, Ty::Int] -> Ty::Str,
    StrTrimStart => "str.trim_start": [Ty::Str] -> Ty::Str,
    StrTrimEnd => "str.trim_end": [Ty::Str] -> Ty::Str,
    StrTrimMatches => "str.trim_matches": [Ty::Str, Ty::Str] -> Ty::Str,
    StrTrimStartMatches => "str.trim_start_matches": [Ty::Str, Ty::Str] -> Ty::Str,
    StrTrimEndMatches => "str.trim_end_matches": [Ty::Str, Ty::Str] -> Ty::Str,
    StrPadLeft => "str.pad_left": [Ty::Str, Ty::Int, Ty::Str] -> Ty::Str,
    StrPadRight => "str.pad_right": [Ty::Str, Ty::Int, Ty::Str] -> Ty::Str,
    StrCenter => "str.center": [Ty::Str, Ty::Int, Ty::Str] -> Ty::Str,
    StrParseInt => "str.parse_int": [Ty::Str] -> Ty::OptInt,
    StrParseFloat => "str.parse_float": [Ty::Str] -> Ty::OptFloat,
    StrByteLen => "str.byte_len": [Ty::Str] -> Ty::Int,
    StrCharCount => "str.char_count": [Ty::Str] -> Ty::Int,
    StrCharAt => "str.char_at": [Ty::Str, Ty::Int] -> Ty::OptStr,
    StrIsEmpty => "str.is_empty": [Ty::Str] -> Ty::Bool,
    StrIsAscii => "str.is_ascii": [Ty::Str] -> Ty::Bool,
    StrIsAsciiDigit => "str.is_ascii_digit": [Ty::Str] -> Ty::Bool,
    StrIsAsciiAlphabetic => "str.is_ascii_alphabetic": [Ty::Str] -> Ty::Bool,
    StrIsAsciiAlphanumeric => "str.is_ascii_alphanumeric": [Ty::Str] -> Ty::Bool,
    StrIsAsciiWhitespace => "str.is_ascii_whitespace": [Ty::Str] -> Ty::Bool,
    StrIsBlank => "str.is_blank": [Ty::Str] -> Ty::Bool,
}

/// Module `std` được hỗ trợ (metadata cấp module, mỗi phần tử là tên ngắn).
///
/// **VD-241 Gate 1** — `std.sqlite` MVP (prototype nội bộ, chưa freeze API).
pub const MODULES: [&str; 10] = [
    "str", "list", "map", "path", "time", "json", "fs", "net", "http", "sqlite",
];

/// `std.io` **chưa** làm — giữ tên để thông điệp rõ (ST-33: `std.fs` **đã có**).
pub const MODULES_HOAN: [&str; 1] = ["io"];

#[cfg(test)]
mod tests {
    use super::{StdId, FUNCS, MODULES};
    use std::collections::BTreeSet;

    /// Bảng là **một nguồn**: `StdId` ↔ `FUNCS` phủ khít, tên duy nhất, metadata module khớp.
    #[test]
    fn registry_mot_nguon_va_metadata_khop() {
        assert_eq!(StdId::COUNT, FUNCS.len(), "COUNT phải bằng FUNCS.len()");
        assert_eq!(StdId::ALL.len(), StdId::COUNT);
        let mut ten = BTreeSet::new();
        let mut mo_dun = BTreeSet::new();
        for (i, id) in StdId::ALL.iter().enumerate() {
            assert_eq!(id.index(), i, "ALL[{i}] phải có index {i}");
            assert!(ten.insert(id.as_str()), "tên trùng: {}", id.as_str());
            assert_eq!(StdId::from_name(id.as_str()), Some(*id));
            assert_eq!(FUNCS[i].name, id.as_str(), "FUNCS lệch thứ tự registry");
            assert!(MODULES.contains(&id.module()), "module lạ: {}", id.module());
            mo_dun.insert(id.module());
        }
        // Mọi module khai trong `MODULES` phải có ít nhất một hàm — không metadata chết.
        let tu_funcs: BTreeSet<&str> = FUNCS
            .iter()
            .filter_map(|f| f.name.split_once('.').map(|(m, _)| m))
            .collect();
        let tu_bang: BTreeSet<&str> = MODULES.iter().copied().collect();
        assert_eq!(
            tu_funcs, tu_bang,
            "MODULES lệch module xuất hiện trong FUNCS"
        );
        assert_eq!(mo_dun, tu_bang);
    }
}
