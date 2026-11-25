// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299b (VD-277)** — `vis-wasm-run`: chạy **một tệp hoặc module đa tệp** ViScript trên
//! `wasm32-wasip1` (nối `vis-loader`; `use ./x` đọc qua preopen WASI).
//!
//! ```text
//! vis-wasm-run [--interp|--vm] [--preopen <guest>[=<host>][@ro]]... [--stdin] <tệp> [-- args…]
//! ```
//!
//! Mọi `std.fs` đi qua preopen; `net`/`proc`/`sqlite` bị từ chối rõ. Cùng binary chạy native
//! (để test) và `wasm32-wasip1`.

use std::io::{Read, Write};
use std::process::ExitCode;

use vis_host_wasm::runner::{chay, chay_tep, Engine, YeuCau};
use vis_host_wasm::{preopen_moi_truong, preopen_tu_argv, Preopen};

const DUNG: &str = "\
vis-wasm-run — chạy một tệp hoặc module đa tệp ViScript (ST-299b)\n\
\n\
Dùng: vis-wasm-run [--interp|--vm] [--preopen <guest>[=<host>][@ro]]... [--stdin] <tệp> [-- args…]\n\
\n\
  --interp            chạy thông dịch (mặc định)\n\
  --vm                biên dịch VBC1 rồi chạy máy ảo\n\
  --preopen SPEC      cấp thư mục cho std.fs; lặp lại được; hậu tố @ro = chỉ đọc\n\
  --stdin             đọc nguồn từ stdin (chế độ script)\n\
  -h, --help          in trợ giúp\n\
  -V, --version       in phiên bản\n\
\n\
Không có --preopen ⇒ dùng VIS_WASM_PREOPENS (guest[:…]) và (trên wasm) mục guest::host\n\
do wasmtime chèn cho --dir. Không có preopen nào ⇒ std.fs bị từ chối rõ.\n";

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut engine = Engine::Interp;
    let mut preopens: Vec<Preopen> = Vec::new();
    let mut co_preopen_cli = false;
    let mut stdin_ra = false;
    let mut tep: Option<String> = None;
    let mut prog_args: Vec<String> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let a = raw[i].as_str();
        // wasmtime chèn preopen dạng `guest::host` vào argv — bỏ qua, không phải đối số chương trình.
        if preopen_tu_argv(a).is_some() {
            i += 1;
            continue;
        }
        match a {
            "--" => {
                prog_args = raw[i + 1..].to_vec();
                break;
            }
            "--interp" => engine = Engine::Interp,
            "--vm" => engine = Engine::Vm,
            "--stdin" => stdin_ra = true,
            "-h" | "--help" => {
                print!("{DUNG}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("vis-wasm-run {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--preopen" => {
                i += 1;
                let Some(v) = raw.get(i) else {
                    eprintln!("vis-wasm-run: `--preopen` cần giá trị");
                    return ExitCode::from(2);
                };
                match Preopen::tu_chuoi(v) {
                    Ok(p) => {
                        preopens.push(p);
                        co_preopen_cli = true;
                    }
                    Err(e) => {
                        eprintln!("vis-wasm-run: {e}");
                        return ExitCode::from(2);
                    }
                }
            }
            x if x.starts_with("--preopen=") => {
                let v = &x["--preopen=".len()..];
                match Preopen::tu_chuoi(v) {
                    Ok(p) => {
                        preopens.push(p);
                        co_preopen_cli = true;
                    }
                    Err(e) => {
                        eprintln!("vis-wasm-run: {e}");
                        return ExitCode::from(2);
                    }
                }
            }
            x if x.starts_with('-') && x != "-" => {
                eprintln!("vis-wasm-run: [USAGE-UNKNOWN] không hỗ trợ cờ `{x}`");
                return ExitCode::from(2);
            }
            _ => {
                if tep.is_some() {
                    eprintln!("vis-wasm-run: chỉ nhận một tệp");
                    return ExitCode::from(2);
                }
                tep = Some(a.to_string());
            }
        }
        i += 1;
    }
    if stdin_ra && tep.is_some() {
        eprintln!("vis-wasm-run: `--stdin` không đi với tệp");
        return ExitCode::from(2);
    }
    let ten = match (&tep, stdin_ra) {
        (Some(t), false) => t.clone(),
        (None, true) => "-".to_string(),
        _ => {
            eprintln!("vis-wasm-run: thiếu tệp nguồn (hoặc --stdin)");
            eprint!("{DUNG}");
            return ExitCode::from(2);
        }
    };
    let nguon = if stdin_ra {
        let mut s = String::new();
        match std::io::stdin().read_to_string(&mut s) {
            Ok(_) => s,
            Err(e) => {
                eprintln!("vis-wasm-run: không đọc được stdin: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        String::new()
    };
    if !co_preopen_cli {
        match preopen_moi_truong() {
            Ok(ds) => preopens = ds,
            Err(e) => {
                eprintln!("vis-wasm-run: preopen môi trường: {e}");
                return ExitCode::from(2);
            }
        }
    }
    // **ST-314a low-peak**: hook chẩn đoán cho harness — chỉ in khi env được bật, không đổi
    // hành vi mặc định. Dùng để chứng minh `--vm` thực sự chọn `Engine::Vm`.
    if std::env::var_os("VIS_WASM_TRACE_ENGINE").is_some() {
        eprintln!("engine={engine:?}");
    }
    let kq = if stdin_ra {
        let mode = vis_syntax::Mode::Script;
        chay(&YeuCau {
            nguon,
            ten: ten.clone(),
            mode,
            args: prog_args,
            engine,
            preopens,
        })
    } else {
        // **ST-299b** — tệp chính đi qua `vis-loader`: `use ./x` đa tệp, chẩn đoán mang tên tệp.
        chay_tep(&ten, engine, &prog_args, &preopens)
    };
    if !kq.stdout.is_empty() {
        let mut w = std::io::stdout().lock();
        if let Err(e) = w.write_all(kq.stdout.as_bytes()).and_then(|()| w.flush()) {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return ExitCode::SUCCESS;
            }
            eprintln!("vis-wasm-run: không ghi được stdout: {e}");
            return ExitCode::from(2);
        }
    }
    if !kq.stderr.is_empty() {
        let _ = std::io::stderr().lock().write_all(kq.stderr.as_bytes());
    }
    ExitCode::from(kq.exit)
}
