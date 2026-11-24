// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-306/ST-310c** — plugin tĩnh mẫu (macro sinh ABI) chạy parity interp↔VM + test âm
//! `cap[Dir]`.
//!
//! Plugin **không** có quyền ngầm: `doc`/`ghi` chỉ chạm tệp qua `PluginCtx` do host truyền; không
//! có `cap` ⇒ `VS0604`. Parity được kiểm ở **biên ABI**: cùng một phép tính chạy bằng interp và
//! VM, chuyển cả hai sang `PluginValue`, rồi so với kết quả plugin — ba đường phải bằng nhau.
//!
//! **ST-310c**: plugin tĩnh lấy `PluginCtx` từ `vis_plugin_host::static_reg::ctx_voi_host` — I/O
//! `cap[Dir]` đi qua `vis_host_native::quyen`, guest ABI không còn `cap_fs.rs`.

use std::path::PathBuf;

use vis_plugin_abi::{Cap, Error, PluginCtx, PluginValue, VIS_PLUGIN_ABI_V1};
use vis_plugin_host::convert::tu_value;
use vis_plugin_host::static_reg::ctx_voi_host;
use vis_plugin_macros as vis;

#[vis::module(name = "mau")]
mod mau {
    #[vis::func]
    pub fn cong(a: i64, b: i64) -> i64 {
        a + b
    }

    #[vis::func]
    pub fn doc(ctx: &vis_plugin_abi::PluginCtx, p: String) -> vis_plugin_abi::Result<String> {
        ctx.read_text(p)
    }

    #[vis::func]
    pub fn ghi(
        ctx: &vis_plugin_abi::PluginCtx,
        p: String,
        noi_dung: String,
    ) -> vis_plugin_abi::Result<()> {
        ctx.write_text(p, &noi_dung)
    }
}

fn chay_ca_hai(src: &str) -> (vis_core::RunOutput, vis_core::RunOutput) {
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["fs".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let i = vis_interp::run(src, &h.hir, &h.symbols, &h.types, &[]);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    let v = vis_vm::run(&m, src, &[]);
    (i, v)
}

fn thu_muc_tam(ten: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-plugin-{ten}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

#[test]
fn abi_v1_va_chu_ky_macro() {
    assert_eq!(VIS_PLUGIN_ABI_V1, 1);
    let m = mau::__vis_module();
    assert_eq!(m.name, "mau");
    let cong = m.tim("cong").expect("có `cong`");
    assert_eq!(cong.sig.name, "cong");
    assert_eq!(
        cong.sig
            .params
            .iter()
            .map(|p| (p.name, p.ty))
            .collect::<Vec<_>>(),
        vec![("a", "int"), ("b", "int")]
    );
    assert_eq!(cong.sig.ret, "int");
    // `ctx` không lộ vào chữ ký ViScript.
    let doc = m.tim("doc").expect("có `doc`");
    assert_eq!(doc.sig.params.len(), 1);
    assert_eq!(doc.sig.params[0].name, "p");
    assert_eq!(doc.sig.ret, "str");
}

#[test]
fn plugin_parity_interp_vm_qua_bien_abi() {
    let src = "fn cong(a: int, b: int) -> int:\n    a + b\n\nfn main() -> int:\n    cong(2, 3)\n";
    let (i, v) = chay_ca_hai(src);
    assert!(i.is_clean(), "interp: {:?}", i.diagnostics);
    assert!(v.is_clean(), "vm: {:?}", v.diagnostics);
    let iv = tu_value(i.value.clone().expect("interp value")).expect("interp ABI");
    let vv = tu_value(v.value.clone().expect("vm value")).expect("vm ABI");
    assert_eq!(iv, vv, "interp và VM phải cùng giá trị qua ABI");

    let ctx = PluginCtx::default();
    let kq = mau::__vis_module()
        .tim("cong")
        .expect("cong")
        .call(&ctx, &[PluginValue::Int(2), PluginValue::Int(3)])
        .expect("plugin chạy");
    assert_eq!(kq, iv);
    assert_eq!(kq, PluginValue::Int(5));
}

#[test]
fn cap_am_khong_co_quyen_thi_khong_doc_fs() {
    let d = thu_muc_tam("cap-am");
    let f = d.join("a.txt");
    std::fs::write(&f, "xin chào").expect("fixture");

    let khong_cap = PluginCtx::default();
    let e = mau::__vis_module()
        .tim("doc")
        .expect("doc")
        .call(&khong_cap, &[PluginValue::Str(f.to_string_lossy().into())])
        .expect_err("thiếu cap[Dir] phải bị từ chối");
    assert_eq!(e.code, "VS0604", "{e}");
    assert!(e.message.contains("cap[Dir]"), "{e}");

    let co_cap = ctx_voi_host(vec![Cap::dir(&d, false)]);
    let kq = mau::__vis_module()
        .tim("doc")
        .expect("doc")
        .call(&co_cap, &[PluginValue::Str(f.to_string_lossy().into())])
        .expect("có cap[Dir] phải đọc được");
    assert_eq!(kq, PluginValue::Str("xin chào".to_string()));

    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn cap_ghi_can_co_write_va_chan_duong_dan_thoat_root() {
    let d = thu_muc_tam("cap-ghi");
    let f = d.join("b.txt");

    let chi_doc = ctx_voi_host(vec![Cap::dir(&d, false)]);
    let e = mau::__vis_module()
        .tim("ghi")
        .expect("ghi")
        .call(
            &chi_doc,
            &[
                PluginValue::Str(f.to_string_lossy().into()),
                PluginValue::Str("x".into()),
            ],
        )
        .expect_err("cap chỉ đọc không được ghi");
    assert_eq!(e.code, "VS0604", "{e}");

    let ghi = ctx_voi_host(vec![Cap::dir(&d, true)]);
    mau::__vis_module()
        .tim("ghi")
        .expect("ghi")
        .call(
            &ghi,
            &[
                PluginValue::Str(f.to_string_lossy().into()),
                PluginValue::Str("nội dung".into()),
            ],
        )
        .expect("có cap ghi phải ghi được");
    assert_eq!(std::fs::read_to_string(&f).expect("đọc lại"), "nội dung");

    let ngoai = std::env::temp_dir().join(format!("vis-plugin-ngoai-{}.txt", std::process::id()));
    let e = mau::__vis_module()
        .tim("ghi")
        .expect("ghi")
        .call(
            &ghi,
            &[
                PluginValue::Str(ngoai.to_string_lossy().into()),
                PluginValue::Str("x".into()),
            ],
        )
        .expect_err("ngoài root phải bị chặn");
    assert_eq!(e.code, "VS0604", "{e}");

    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_file(&ngoai);
}

#[test]
fn so_tham_sai_tra_loi_khong_panic() {
    let ctx = PluginCtx::default();
    let e = mau::__vis_module()
        .tim("cong")
        .expect("cong")
        .call(&ctx, &[PluginValue::Int(1)])
        .expect_err("thiếu tham số phải lỗi");
    assert_eq!(e.code, "VS0501", "{e}");
    assert!(e.message.contains("2 tham số"), "{e}");
}

#[test]
fn chuyen_doi_khong_nuot_kieu_la() {
    let e =
        tu_value(vis_core::value::Value::Builtin("in")).expect_err("builtin không qua biên plugin");
    assert_eq!(e.code, "VS0501");
    let _ = Error::new("VS0501", "x");
}

/// **ST-309 (kiểm toán bảo mật) — đỏ trước, tất định.** `write_text` cũ canonicalize **thư mục
/// cha** rồi `std::fs::write(canon_parent/ten)`; nếu **tên cuối** là symlink trỏ ra ngoài `cap`,
/// lệnh ghi đi theo link và sửa tệp ngoài (không cần đua). Ca này khẳng định: từ chối + tệp ngoài
/// giữ nguyên. **ST-310c**: bản vá nay đi qua `vis_host_native::quyen::mo_tep_ghi` (một nguồn).
#[cfg(unix)]
#[test]
fn cap_ghi_khong_di_theo_symlink_o_ten_cuoi() {
    use std::os::unix::fs::symlink;
    let d = thu_muc_tam("cap-link-trong");
    let n = thu_muc_tam("cap-link-ngoai");
    let ngoai = n.join("that.txt");
    std::fs::write(&ngoai, "NGOAI").expect("tệp ngoài");
    let link = d.join("link.txt");
    symlink(&ngoai, &link).expect("symlink ra ngoài");

    let ghi = ctx_voi_host(vec![Cap::dir(&d, true)]);
    let e = ghi
        .write_text(&link, "GHI-DE")
        .expect_err("ghi qua symlink ra ngoài cap phải bị từ chối");
    assert_eq!(e.code, "VS0604", "phải là lỗi quyền, không phải I/O: {e}");
    assert_eq!(
        std::fs::read_to_string(&ngoai).expect("đọc tệp ngoài"),
        "NGOAI",
        "tệp ngoài cap không được bị ghi đè"
    );
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&n);
}
