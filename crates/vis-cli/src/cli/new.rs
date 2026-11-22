// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-303 (VD-283)** — `vis new <dir> [--template bin|lib]`: khung dự án chạy được
//! `vis check` / `vis test` / `vis run`. Không đọc mạng, không ghi đè thư mục đã có.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MAIN: &str = r#"# SPDX-License-Identifier: MIT OR Apache-2.0

fn main():
    print("hello from ViScript")
"#;

const LIB: &str = r#"# SPDX-License-Identifier: MIT OR Apache-2.0

pub fn greeting() -> str:
    "hello from ViScript"
"#;

const LIB_MAIN: &str = r#"# SPDX-License-Identifier: MIT OR Apache-2.0

use ./lib

fn main():
    print(greeting())
"#;

const SMOKE: &str = r#"# SPDX-License-Identifier: MIT OR Apache-2.0

print("ok")
"#;

const SMOKE_OUT: &str = "ok\n";

fn usage() -> ExitCode {
    eprintln!("dùng: vis new <dir> [--template bin|lib]");
    ExitCode::from(2)
}

fn ghi(p: &Path, noi_dung: &str) -> Result<(), String> {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(|e| format!("không tạo được `{}`: {e}", d.display()))?;
    }
    fs::write(p, noi_dung).map_err(|e| format!("không ghi được `{}`: {e}", p.display()))
}

fn tao(dir: &Path, template: &str) -> Result<(), String> {
    if dir.exists() {
        return Err(format!("thư mục `{}` đã tồn tại", dir.display()));
    }
    fs::create_dir_all(dir).map_err(|e| format!("không tạo được `{}`: {e}", dir.display()))?;
    let lib = template == "lib";
    ghi(&dir.join("main.vis"), if lib { LIB_MAIN } else { MAIN })?;
    if lib {
        ghi(&dir.join("lib.vis"), LIB)?;
    }
    ghi(&dir.join("test/sem/smoke.vis"), SMOKE)?;
    ghi(&dir.join("test/sem/smoke.out"), SMOKE_OUT)?;
    ghi(
        &dir.join("README.md"),
        &format!(
            "# {}\n\nDự án ViScript mẫu ({}).\n\n```bash\nvis check main.vis\nvis test\nvis run main.vis\n```\n",
            dir.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("vis-project"),
            template
        ),
    )?;
    Ok(())
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    let mut dir: Option<PathBuf> = None;
    let mut template = "bin".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--template" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                template = v.clone();
                i += 2;
            }
            a if a.starts_with('-') => {
                eprintln!("vis new: không hỗ trợ cờ `{a}`");
                return usage();
            }
            a => {
                if dir.is_some() {
                    eprintln!("vis new: chỉ nhận một thư mục");
                    return usage();
                }
                dir = Some(PathBuf::from(a));
                i += 1;
            }
        }
    }
    let Some(dir) = dir else {
        return usage();
    };
    if template != "bin" && template != "lib" {
        eprintln!("vis new: `--template` phải là `bin` hoặc `lib`");
        return usage();
    }
    match tao(&dir, &template) {
        Ok(()) => {
            println!("đã tạo dự án `{}` (template {template})", dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("vis new: {e}");
            ExitCode::from(2)
        }
    }
}
