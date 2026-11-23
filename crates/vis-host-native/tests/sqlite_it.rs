// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: SQLite/host DB (feature `sqlite`).
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "sqlite_it/sqlite.rs"]
mod sqlite;
#[path = "sqlite_it/vd266_db.rs"]
mod vd266_db;
#[path = "sqlite_it/vd268_dbcache.rs"]
mod vd268_dbcache;
#[path = "sqlite_it/vd270_batch_trigger.rs"]
mod vd270_batch_trigger;
#[path = "sqlite_it/vd270_db_batch.rs"]
mod vd270_db_batch;
#[path = "sqlite_it/vd270_db_explain.rs"]
mod vd270_db_explain;
#[path = "sqlite_it/vd270_sqlite_application_id.rs"]
mod vd270_sqlite_application_id;
#[path = "sqlite_it/vd270_sqlite_columns.rs"]
mod vd270_sqlite_columns;
#[path = "sqlite_it/vd270_sqlite_data_version.rs"]
mod vd270_sqlite_data_version;
#[path = "sqlite_it/vd270_sqlite_database_list.rs"]
mod vd270_sqlite_database_list;
#[path = "sqlite_it/vd270_sqlite_index_columns.rs"]
mod vd270_sqlite_index_columns;
#[path = "sqlite_it/vd270_sqlite_index_exists.rs"]
mod vd270_sqlite_index_exists;
#[path = "sqlite_it/vd270_sqlite_indexes.rs"]
mod vd270_sqlite_indexes;
#[path = "sqlite_it/vd270_sqlite_journal_mode.rs"]
mod vd270_sqlite_journal_mode;
#[path = "sqlite_it/vd270_sqlite_primary_keys.rs"]
mod vd270_sqlite_primary_keys;
#[path = "sqlite_it/vd270_sqlite_schema_sql.rs"]
mod vd270_sqlite_schema_sql;
#[path = "sqlite_it/vd270_sqlite_schema_version.rs"]
mod vd270_sqlite_schema_version;
#[path = "sqlite_it/vd270_sqlite_table_exists.rs"]
mod vd270_sqlite_table_exists;
#[path = "sqlite_it/vd270_sqlite_tables.rs"]
mod vd270_sqlite_tables;
#[path = "sqlite_it/vd270_sqlite_user_version.rs"]
mod vd270_sqlite_user_version;
