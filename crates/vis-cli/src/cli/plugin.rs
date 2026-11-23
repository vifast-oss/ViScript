// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307 (VD-278/VD-281/VD-283)** — `vis plugin`: nạp plugin động WASI/WASM có kiểm soát quyền.
//!
//! - `vis plugin check <plugin.wasm>` — kiểm ABI + manifest, in hàm/cap/runtime; **không** chạy hàm.
//! - `vis plugin run <plugin.wasm> <hàm> [giá trị…]` — gọi một hàm qua biên ABI.
//! - `--cap-dir <path>[=ro|rw]` (lặp được) — trao `cap[Dir]`; guest path `/cap0`, `/cap1`, …
//! - `--strict-caps` / `--sandbox` — **tắt cứng** plugin động (`VS0604`) trước khi đọc tệp.
//!
//! Giá trị tham số: `int:5`, `str:abc`, `bool:true`, `float:1.5`, `bytes:<hex>`, `unit`; không có
//! tiền tố ⇒ `str`. Exit: 0 sạch, 2 lỗi dùng/quyền/ABI (cùng quy ước `vis run`).

use std::path::PathBuf;
use std::process::ExitCode;

use vis_plugin_abi::{Cap, ChinhSach, Error, PluginValue, PluginWasi};

fn usage() -> ExitCode {
    eprintln!(
        "dùng: vis plugin check <plugin.wasm> [--cap-dir <path>[=ro|rw]]… [--strict-caps] [--sandbox]\n  \
         vis plugin run <plugin.wasm> <hàm> [giá trị…] [--cap-dir <path>[=ro|rw]]… [--strict-caps] [--sandbox]\n  \
         --cap-dir: trao `cap[Dir]` cho plugin (mặc định `ro`; guest path `/cap0`, `/cap1`, …)\n  \
         --strict-caps/--sandbox: tắt cứng plugin động (`VS0604`), không đọc tệp plugin\n  \
         giá trị: int:5 | str:abc | bool:true | float:1.5 | bytes:<hex> | unit (không tiền tố ⇒ str)"
    );
    ExitCode::from(2)
}

struct YeuCau {
    lenh: String,
    wasm: PathBuf,
    ham: Option<String>,
    args: Vec<PluginValue>,
    caps: Vec<Cap>,
    chinh_sach: ChinhSach,
}

fn cap_tu_spec(spec: &str) -> Result<Cap, String> {
    let (path, write) = match spec.rsplit_once('=') {
        Some((p, "rw")) => (p, true),
        Some((p, "ro")) => (p, false),
        Some((_, q)) => return Err(format!("quyền cap lạ `{q}` (cần ro|rw)")),
        None => (spec, false),
    };
    if path.is_empty() {
        return Err("`--cap-dir` cần đường dẫn".into());
    }
    Ok(Cap::dir(path, write))
}

fn gia_tri(s: &str) -> Result<PluginValue, String> {
    if s == "unit" {
        return Ok(PluginValue::Unit);
    }
    if let Some(x) = s.strip_prefix("int:") {
        return x
            .parse::<i64>()
            .map(PluginValue::Int)
            .map_err(|_| format!("int sai: `{x}`"));
    }
    if let Some(x) = s.strip_prefix("float:") {
        return x
            .parse::<f64>()
            .map(PluginValue::Float)
            .map_err(|_| format!("float sai: `{x}`"));
    }
    if let Some(x) = s.strip_prefix("bool:") {
        return match x {
            "true" => Ok(PluginValue::Bool(true)),
            "false" => Ok(PluginValue::Bool(false)),
            _ => Err(format!("bool sai: `{x}` (cần true|false)")),
        };
    }
    if let Some(x) = s.strip_prefix("bytes:") {
        return giai_hex(x)
            .map(PluginValue::Bytes)
            .ok_or_else(|| format!("bytes hex sai: `{x}`"));
    }
    if let Some(x) = s.strip_prefix("str:") {
        return Ok(PluginValue::Str(x.to_string()));
    }
    Ok(PluginValue::Str(s.to_string()))
}

fn giai_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let b = s.as_bytes();
    let mut ra = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        ra.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(ra)
}

fn doc_yeu_cau(args: &[String]) -> Result<YeuCau, String> {
    let lenh = match args.first().map(String::as_str) {
        Some("check") => "check".to_string(),
        Some("run") => "run".to_string(),
        Some(khac) => return Err(format!("lệnh lạ `{khac}` (cần check|run)")),
        None => return Err("thiếu lệnh con (check|run)".into()),
    };
    let mut strict = false;
    let mut sandbox = false;
    let mut caps = Vec::new();
    let mut vi_tri = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--strict-caps" => strict = true,
            "--sandbox" => sandbox = true,
            "--cap-dir" => {
                let spec = args
                    .get(i + 1)
                    .ok_or_else(|| "`--cap-dir` cần giá trị".to_string())?;
                caps.push(cap_tu_spec(spec)?);
                i += 1;
            }
            a if a.starts_with('-') && a != "-" => {
                return Err(format!("cờ lạ `{a}`"));
            }
            a => vi_tri.push(a.to_string()),
        }
        i += 1;
    }
    let wasm = vi_tri
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| "thiếu đường dẫn plugin.wasm".to_string())?;
    let (ham, args_vi_tri) = if lenh == "run" {
        let ham = vi_tri
            .get(1)
            .cloned()
            .ok_or_else(|| "`vis plugin run` cần tên hàm".to_string())?;
        (Some(ham), &vi_tri[2..])
    } else {
        if vi_tri.len() > 1 {
            return Err(format!("`vis plugin check` thừa đối số `{}`", vi_tri[1]));
        }
        (None, &vi_tri[0..0])
    };
    let mut ds = Vec::with_capacity(args_vi_tri.len());
    for s in args_vi_tri {
        ds.push(gia_tri(s)?);
    }
    Ok(YeuCau {
        lenh,
        wasm,
        ham,
        args: ds,
        caps,
        chinh_sach: ChinhSach::tu_co(strict, sandbox),
    })
}

fn in_loi(e: &Error) -> ExitCode {
    eprintln!("vis plugin: {}: {}", e.code, e.message);
    ExitCode::from(2)
}

fn run(args: &[String]) -> ExitCode {
    let yc = match doc_yeu_cau(args) {
        Ok(y) => y,
        Err(e) => {
            eprintln!("vis plugin: {e}");
            return usage();
        }
    };
    let p = match PluginWasi::nap(&yc.wasm, yc.caps, yc.chinh_sach) {
        Ok(p) => p,
        Err(e) => return in_loi(&e),
    };
    if yc.lenh == "check" {
        println!("plugin: {}", p.manifest().module);
        println!("abi: {}", p.manifest().abi);
        println!("runtime: {}", p.runtime().ten());
        for h in &p.manifest().funcs {
            println!("hàm: {}({}) -> {}", h.name, h.params.join(", "), h.ret);
        }
        for (kind, write) in &p.manifest().caps {
            println!(
                "cap yêu cầu: {kind} ({})",
                if *write { "ghi" } else { "đọc" }
            );
        }
        for (i, c) in p.caps().iter().enumerate() {
            println!(
                "cap trao: {} <- {} ({})",
                PluginWasi::guest_cap(i),
                c.root.display(),
                if c.write { "rw" } else { "ro" }
            );
        }
        return ExitCode::SUCCESS;
    }
    let ham = yc.ham.expect("run có hàm");
    match p.goi(&ham, &yc.args) {
        Ok(v) => {
            let v = vis_interp::value::Value::try_from(v).unwrap_or(vis_interp::value::Value::Unit);
            println!("{}", vis_interp::value::show(&v));
            ExitCode::SUCCESS
        }
        Err(e) => in_loi(&e),
    }
}

/// Điểm vào `vis plugin …` (main.rs gọi).
pub fn run_plugin(args: &[String]) -> ExitCode {
    run(args)
}
