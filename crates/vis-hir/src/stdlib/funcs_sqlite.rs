// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — bảng chữ ký `std.*` nhóm `funcs_sqlite` (tách khỏi `stdlib.rs`, 0 đổi hành vi).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub(super) const FUNCS_SQLITE: [StdFn; 28] = [
    StdFn {
        name: "sqlite.open",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "sqlite.open_with",
        params: &[Ty::Str, Ty::Bool, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "sqlite.close",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.execute",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.execute_batch",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.query",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
    },
    StdFn {
        name: "sqlite.query_one",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
    },
    StdFn {
        name: "sqlite.explain",
        params: &[Ty::Str, Ty::Str, Ty::List(&Ty::SqlValue)],
        ret: Ty::ResultRowSetErr,
    },
    StdFn {
        name: "sqlite.last_insert_rowid",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.changes",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.begin",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.commit",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.rollback",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "sqlite.in_transaction",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "sqlite.tables",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "sqlite.table_exists",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "sqlite.index_exists",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "sqlite.columns",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "sqlite.indexes",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "sqlite.schema_sql",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultOptStrErr,
    },
    StdFn {
        name: "sqlite.index_columns",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "sqlite.primary_keys",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "sqlite.schema_version",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.user_version",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.application_id",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.data_version",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "sqlite.journal_mode",
        params: &[Ty::Str],
        ret: Ty::ResultStrErr,
    },
    StdFn {
        name: "sqlite.database_list",
        params: &[Ty::Str],
        ret: Ty::ResultListRowSetErr,
    },
];
