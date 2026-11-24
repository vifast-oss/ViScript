// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277)** — từ chối **rõ ràng** `net`/`proc`/`sqlite` trên WASI.
//!
//! Ba khả năng không có trong sandbox `wasm32-wasip1` của ST-299a. Thay vì để lỗi mơ hồ
//! (`VS0604` chung) hoặc panic, mỗi host dưới đây trả **một thông điệp ổn định** nêu đúng
//! khả năng bị từ chối. Không `unsafe`, không dependency, không chạm OS.

use vis_diag::COMMAND_CODES;
use vis_core::host::{DbHost, KetQua, NetHost, Ngoai};
use vis_core::trap::{Abort, Trap, R};
use vis_core::value::Value;
use vis_syntax::Span;

/// Thông điệp từ chối mạng (`std.net.*`, HTTP, TLS).
pub const LY_DO_MANG: &str =
    "wasm32-wasip1: host từ chối `net` (không có socket trong sandbox WASI — ST-299a)";
/// Thông điệp từ chối tiến trình ngoài (`cmd`/`extern`).
pub const LY_DO_PROC: &str =
    "wasm32-wasip1: host từ chối `proc` (không chạy tiến trình ngoài trong sandbox WASI — ST-299a)";
/// Thông điệp từ chối SQLite.
pub const LY_DO_SQLITE: &str =
    "wasm32-wasip1: host từ chối `sqlite` (không có backend SQLite trong sandbox WASI — ST-299a)";

fn trap_proc(span: Span) -> Abort {
    Abort::Trap(Trap::new(&COMMAND_CODES[3], LY_DO_PROC, span))
}

/// Host mạng **luôn từ chối** — mọi phương thức trả `Err`/trap, không panic.
#[derive(Debug, Default, Clone, Copy)]
pub struct TuChoiNet;

impl NetHost for TuChoiNet {
    fn connect(&mut self, _dia_chi: &str, _timeout_ms: u64) -> Result<String, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn connect_tls(&mut self, _dia_chi: &str, _timeout_ms: u64) -> Result<String, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn check_net(&mut self, _dia_chi: &str) -> Result<(), String> {
        Err(LY_DO_MANG.to_string())
    }

    fn http2_get(
        &mut self,
        _dia_chi: &str,
        _target: &str,
        _method: &str,
        _headers: &[(String, String)],
        _timeout_ms: u64,
    ) -> Result<Option<vis_core::host::KetQuaHttp2>, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn listen(&mut self, _dia_chi: &str) -> Result<String, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn local_addr(&mut self, _tay: &str) -> Result<String, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn accept(&mut self, _tay: &str, _timeout_ms: u64) -> Result<String, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn read(&mut self, _tay: &str, _toi_da: usize, _timeout_ms: u64) -> Result<Vec<u8>, String> {
        Err(LY_DO_MANG.to_string())
    }

    fn write(&mut self, _tay: &str, _du_lieu: &[u8]) -> Result<(), String> {
        Err(LY_DO_MANG.to_string())
    }

    fn close(&mut self, _tay: &str) -> Result<(), String> {
        Err(LY_DO_MANG.to_string())
    }
}

/// Host chương trình ngoài **luôn từ chối** bằng trap `VS0604` (không bắt được).
#[derive(Debug, Default, Clone, Copy)]
pub struct TuChoiProc;

impl Ngoai for TuChoiProc {
    fn chay(
        &mut self,
        _ten: &str,
        _args: &[String],
        _raw: bool,
        _stdin: &str,
        span: Span,
    ) -> R<Value> {
        Err(trap_proc(span))
    }

    fn chay_day_du(
        &mut self,
        _ten: &str,
        _args: &[String],
        _stdin: &str,
        span: Span,
    ) -> Result<KetQua, Abort> {
        Err(trap_proc(span))
    }
}

/// Host SQLite **luôn từ chối** — mọi thao tác `Err` dữ liệu rõ ràng, không panic.
#[derive(Debug, Default, Clone, Copy)]
pub struct TuChoiDb;

impl DbHost for TuChoiDb {
    fn db_open(&mut self, _path: &str, _create: bool, _busy_ms: i64) -> Result<String, String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_close(&mut self, _h: &str) -> Result<(), String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_execute(&mut self, _h: &str, _sql: &str, _params: &[Value]) -> Result<i64, String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_execute_batch(&mut self, _h: &str, _sql: &str) -> Result<i64, String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_query(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_query_one(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_last_insert_rowid(&mut self, _h: &str) -> Result<i64, String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_changes(&mut self, _h: &str) -> Result<i64, String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_in_transaction(&mut self, _h: &str) -> Result<bool, String> {
        Err(LY_DO_SQLITE.to_string())
    }

    fn db_cleanup(&mut self) -> Result<u64, String> {
        // Không có connection nào được mở ⇒ 0, không phải lỗi khi kết thúc phiên.
        Ok(0)
    }

    fn db_path_of(&mut self, _h: &str) -> Result<String, String> {
        Err(LY_DO_SQLITE.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mang_tu_choi_moi_cua() {
        let mut n = TuChoiNet;
        for m in [
            n.connect("127.0.0.1:9", 10).unwrap_err(),
            n.connect_tls("example.com:443", 10).unwrap_err(),
            n.listen("127.0.0.1:0").unwrap_err(),
            n.local_addr("x").unwrap_err(),
            n.accept("x", 10).unwrap_err(),
            n.close("x").unwrap_err(),
        ] {
            assert!(m.contains("net"), "{m}");
            assert!(m.contains("ST-299a"), "{m}");
        }
        assert!(n.read("x", 1, 10).is_err());
        assert!(n.write("x", b"a").is_err());
        assert!(n.check_net("x").unwrap_err().contains("net"));
        assert!(n.http2_get("x", "/", "GET", &[], 10).is_err());
    }

    #[test]
    fn proc_tu_choi_bang_vs0604() {
        let mut p = TuChoiProc;
        let span = Span {
            start: 0,
            end: 0,
            file: 0,
        };
        let e = p.chay("ls", &[], false, "", span).unwrap_err();
        match e {
            Abort::Trap(t) => {
                assert_eq!(t.code.short, "VS0604");
                assert!(t.msg.contains("proc"), "{}", t.msg);
            }
            other => panic!("chờ trap, được {other:?}"),
        }
    }

    #[test]
    fn sqlite_tu_choi_moi_cua() {
        let mut d = TuChoiDb;
        assert!(d
            .db_open(":memory:", true, 0)
            .unwrap_err()
            .contains("sqlite"));
        assert!(d.db_close("x").unwrap_err().contains("sqlite"));
        assert!(d
            .db_execute("x", "select 1", &[])
            .unwrap_err()
            .contains("sqlite"));
        assert!(d
            .db_query("x", "select 1", &[])
            .unwrap_err()
            .contains("sqlite"));
        assert!(d.db_in_transaction("x").unwrap_err().contains("sqlite"));
        assert_eq!(d.db_cleanup().unwrap(), 0);
    }
}
