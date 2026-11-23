// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-306/ST-310c (VD-285)** — đăng ký **plugin tĩnh** + cài I/O `cap[Dir]` cho host.
//!
//! Plugin tĩnh (macro `#[vis::module]`) chạy **trong tiến trình host**, nên `PluginCtx` phải
//! được host cài I/O thật. Mọi thao tác tệp đi qua **một** nguồn primitive:
//! `vis_host_native::quyen` (`mo_tep_doc`/`mo_tep_ghi`) — **không** có bản sao `openat2` ở crate
//! này (ST-310c). Trên Unix, `RESOLVE_BENEATH|NO_SYMLINKS` giữ nguyên bảo đảm F2 của ST-309;
//! Windows giữ đường `canonicalize` như trước (mức bảo đảm đã ghi ở `agent/HOST-BAO-DAM.md`).

use std::path::Path;

use vis_plugin_abi::{Cap, Error, PluginCtx, PluginIo, PluginModule, PluginValue, Result};

/// I/O host cho [`PluginCtx`] trên Unix: đọc/ghi **từ fd gốc** của cap.
#[cfg(unix)]
fn doc_host(goc: &Path, rel: &Path, hien_thi: &Path) -> Result<String> {
    vis_host_native::quyen::mo_tep_doc(goc, rel).map_err(|e| loi_mo(hien_thi, e, false))
}

#[cfg(unix)]
fn ghi_host(goc: &Path, rel: &Path, data: &[u8], hien_thi: &Path) -> Result<()> {
    vis_host_native::quyen::mo_tep_ghi(goc, rel, data).map_err(|e| loi_mo(hien_thi, e, true))
}

/// Symlink/thoát gốc là **vi phạm quyền** (`VS0604`); còn lại là lỗi I/O (`VS0501`).
/// Dùng errno **theo nền** qua helper của `vis-host-native` (Darwin `ELOOP = 62`).
#[cfg(unix)]
fn loi_mo(hien_thi: &Path, e: std::io::Error, ghi: bool) -> Error {
    if vis_host_native::quyen::la_loi_symlink(&e) {
        return Error::cap_denied(
            if ghi { "Dir (write)" } else { "Dir" },
            &hien_thi.to_string_lossy(),
        );
    }
    Error::io(format!(
        "không {} được `{}`: {e}",
        if ghi { "ghi" } else { "đọc" },
        hien_thi.display()
    ))
}

/// Windows/không Unix: giữ đường `canonicalize` cũ (đã ghi giới hạn ở `HOST-BAO-DAM.md`).
#[cfg(not(unix))]
fn doc_host(goc: &Path, rel: &Path, hien_thi: &Path) -> Result<String> {
    let dich = goc.join(rel);
    std::fs::read_to_string(&dich)
        .map_err(|e| Error::io(format!("không đọc được `{}`: {e}", hien_thi.display())))
}

#[cfg(not(unix))]
fn ghi_host(goc: &Path, rel: &Path, data: &[u8], hien_thi: &Path) -> Result<()> {
    let dich = goc.join(rel);
    std::fs::write(&dich, data)
        .map_err(|e| Error::io(format!("không ghi được `{}`: {e}", hien_thi.display())))
}

/// `PluginCtx` đã cài I/O của host (dùng cho plugin tĩnh và test).
pub fn ctx_voi_host(caps: Vec<Cap>) -> PluginCtx {
    PluginCtx::new(caps).voi_io(PluginIo {
        doc: doc_host,
        ghi: ghi_host,
    })
}

/// Sổ đăng ký plugin tĩnh: tên module → danh sách hàm do macro sinh.
#[derive(Default)]
pub struct SoDangKy {
    modules: Vec<PluginModule>,
}

impl SoDangKy {
    pub fn moi() -> Self {
        Self::default()
    }

    /// Đăng ký một module; trùng tên là lỗi cấu hình (`VS0501`), không âm thầm ghi đè.
    pub fn dang_ky(&mut self, m: PluginModule) -> Result<()> {
        if self.modules.iter().any(|x| x.name == m.name) {
            return Err(Error::new(
                "VS0501",
                format!("đăng ký plugin tĩnh trùng tên `{}`", m.name),
            ));
        }
        self.modules.push(m);
        Ok(())
    }

    pub fn modules(&self) -> &[PluginModule] {
        &self.modules
    }

    pub fn tim_module(&self, ten: &str) -> Option<&PluginModule> {
        self.modules.iter().find(|m| m.name == ten)
    }

    /// Gọi `module.hàm` với quyền do host trao.
    pub fn goi(
        &self,
        module: &str,
        ham: &str,
        ctx: &PluginCtx,
        args: &[PluginValue],
    ) -> Result<PluginValue> {
        let m = self
            .tim_module(module)
            .ok_or_else(|| Error::new("VS0501", format!("không có plugin tĩnh `{module}`")))?;
        let f = m
            .tim(ham)
            .ok_or_else(|| Error::new("VS0501", format!("plugin `{module}` không có `{ham}`")))?;
        f.call(ctx, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vis_plugin_abi::{FuncDef, Param, Signature};

    fn cong(_: &PluginCtx, args: &[PluginValue]) -> Result<PluginValue> {
        let a = i64::try_from(args[0].clone())?;
        let b = i64::try_from(args[1].clone())?;
        Ok(PluginValue::Int(a + b))
    }

    static PARAMS: &[Param] = &[
        Param {
            name: "a",
            ty: "int",
        },
        Param {
            name: "b",
            ty: "int",
        },
    ];
    static FUNCS: &[FuncDef] = &[FuncDef {
        sig: Signature {
            name: "cong",
            params: PARAMS,
            ret: "int",
        },
        call: cong,
    }];
    static MOD: PluginModule = PluginModule {
        name: "mau",
        funcs: FUNCS,
    };

    #[test]
    fn dang_ky_va_goi_plugin_tinh() {
        let mut so = SoDangKy::moi();
        so.dang_ky(MOD).expect("đăng ký");
        assert!(so.dang_ky(MOD).is_err(), "trùng tên phải lỗi");
        let ctx = ctx_voi_host(Vec::new());
        let kq = so
            .goi(
                "mau",
                "cong",
                &ctx,
                &[PluginValue::Int(2), PluginValue::Int(3)],
            )
            .expect("gọi");
        assert_eq!(kq, PluginValue::Int(5));
    }
}
