// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318e (VD-297)** — phân tích cờ CLI và điều phối `vis test` (`run`), tách nguyên văn
//! khỏi `vistest.rs` (R1 mềm, 0 đổi hành vi/byte TAP/JSON).

use std::path::PathBuf;
use std::process::ExitCode;

use super::*;

fn usage() -> ExitCode {
    eprintln!("dùng: vis test [--vm] [--strict-caps] [--timeout <ms>] [--json] [--json-out <path>] [--filter <substr>] [--fail-fast] [--list] [--repeat <N>] [--shuffle [--seed <u64>]] [--shard <I>/<N>] [--jobs <N>] [--slowest <N>] [thư mục|tệp]");
    eprintln!("  --filter <substr>: chỉ chạy ca có tên `test fn` hoặc đường dẫn hiển thị chứa <substr> (phân biệt hoa/thường); chuỗi rỗng hoặc không khớp ⇒ `1..0`, exit 0
  --list: chỉ in tên các ca đã chọn (mỗi dòng), không chạy; --list --json in {{\"tests\":[…]}}.");
    eprintln!("  --timeout <ms>: giới hạn mỗi ca (test fn **và** fixture .out/.err) ≤ ms (≥ 50); quá hạn ⇒ kill + `not ok` với `quá hạn <ms> ms`; mặc định **không** giới hạn");
    eprintln!("  --fail-fast: dừng sau ca `not ok` đầu tiên; kế hoạch TAP = số ca **đã chạy**");
    eprintln!("  --repeat <N>: chạy mỗi ca đã chọn N lần (số nguyên ≥ 1, mặc định 1); N > 1 ⇒ mỗi lần lặp là một ca TAP/JSON tên `<tên> [k/N]`; --list bỏ qua --repeat (liệt kê một lần)");
    eprintln!("  --shuffle [--seed <u64>]: xáo thứ tự ca đã chọn trước khi chạy/liệt kê; cùng seed ⇒ cùng thứ tự, mặc định seed 0; --seed chỉ dùng cùng --shuffle (thiếu/sai ⇒ exit 2)");
    eprintln!("  --shard <I>/<N>: chỉ chạy phần I/N của ca đã chọn (1 ≤ I ≤ N); chia theo chỉ số sau --filter/--shuffle, trước --list/--repeat; N phần rời nhau, hợp = đủ tập");
    eprintln!("  --jobs <N>: chạy N đơn vị (ca × lần lặp) song song (số nguyên ≥ 1, mặc định 1); kết quả/TAP/JSON in theo đúng thứ tự cũ; --fail-fast ngừng lập lịch sau lần đỏ đầu (đơn vị đang chạy vẫn xong ⇒ plan = số đã chạy); --list bỏ qua --jobs");
    eprintln!("  --slowest <N>: sau TAP/JSON (stdout không đổi) in tối đa N đơn vị đã chạy lâu nhất ra stderr, giảm dần, hoà giữ thứ tự gốc: `# slowest: <ms> ms - <tên>` (dưới 1 ms dùng `<us> us`); N nguyên ≥ 1; không đi với --list");
    ExitCode::from(2)
}

pub fn run(args: &[String]) -> ExitCode {
    let mut vm = false;
    let mut strict = false;
    // **ST-101**: `--json` — đầu ra máy đọc; không cờ ⇒ TAP y hệt.
    let mut json_ra = false;
    // **VD-270**: `--json-out <path>` — ghi cùng object JSON ra tệp, stdout vẫn TAP nếu không `--json`.
    let mut json_out: Option<String> = None;
    // **ST-105**: `--timeout <ms>` (≥50) chỉ áp cho **`test fn`**; mặc định **không** giới hạn.
    let mut timeout_han: Option<u64> = None;
    // **VD-267**: `--filter <substr>` — lọc **trước khi chạy**; `None` = chạy tất cả.
    let mut loc: Option<String> = None;
    // **VD-267**: `--fail-fast` — dừng sau ca `not ok` đầu tiên.
    let mut fail_fast = false;
    // **ST-281**: `--list` — chỉ in tên ca đã chọn, không chạy.
    let mut list = false;
    // **VD-270**: `--repeat <N>` — chạy mỗi ca đã chọn N lần (mặc định 1).
    let mut repeat: usize = 1;
    // **VD-270**: `--shuffle [--seed <u64>]` — xáo thứ tự ca đã chọn; seed mặc định 0.
    let mut shuffle = false;
    let mut seed: Option<u64> = None;
    // **VD-270**: `--shard <I>/<N>` — chỉ chạy phần `I` của `N` phần rời nhau (1-based).
    let mut shard: Option<(usize, usize)> = None;
    // **VD-270**: `--jobs <N>` — số worker song song; mặc định 1 (tuần tự, byte cũ).
    let mut jobs: Option<usize> = None;
    // **VD-270**: `--slowest <N>` — in N đơn vị chậm nhất ra stderr sau TAP/JSON.
    let mut slowest: Option<usize> = None;
    let mut duong: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--vm" => vm = true,
            "--strict-caps" => strict = true,
            "--json" => json_ra = true,
            "--json-out" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--json-out` cần đường dẫn tệp");
                    return ExitCode::from(2);
                };
                if v.is_empty() {
                    eprintln!("vis test: `--json-out` cần đường dẫn tệp (rỗng)");
                    return ExitCode::from(2);
                }
                if json_out.is_some() {
                    eprintln!("vis test: `--json-out` lặp lại");
                    return ExitCode::from(2);
                }
                json_out = Some(v.clone());
                i += 1;
            }
            s if s.starts_with("--json-out=") => {
                let v = &s["--json-out=".len()..];
                if v.is_empty() {
                    eprintln!("vis test: `--json-out` cần đường dẫn tệp (rỗng)");
                    return ExitCode::from(2);
                }
                if json_out.is_some() {
                    eprintln!("vis test: `--json-out` lặp lại");
                    return ExitCode::from(2);
                }
                json_out = Some(v.to_string());
            }
            "--fail-fast" => fail_fast = true,
            "--list" => list = true,
            // **VD-270**: `--shuffle` không tham số; `--seed` là u64 (thiếu/sai ⇒ exit 2).
            "--shuffle" => shuffle = true,
            "--seed" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--seed` cần số nguyên u64 (0..=2^64-1)");
                    return ExitCode::from(2);
                };
                let Ok(n) = v.parse::<u64>() else {
                    eprintln!("vis test: `--seed` cần số nguyên u64 (0..=2^64-1), được `{v}`");
                    return ExitCode::from(2);
                };
                seed = Some(n);
                i += 1;
            }
            s if s.starts_with("--seed=") => {
                let v = &s["--seed=".len()..];
                let Ok(n) = v.parse::<u64>() else {
                    eprintln!("vis test: `--seed` cần số nguyên u64 (0..=2^64-1), được `{v}`");
                    return ExitCode::from(2);
                };
                seed = Some(n);
            }
            // **VD-270**: N sai (0, âm, không nguyên, thiếu) ⇒ exit 2; usage nêu `--repeat`.
            "--repeat" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--repeat` cần số nguyên ≥ 1");
                    return ExitCode::from(2);
                };
                let Ok(n) = v.parse::<usize>() else {
                    eprintln!("vis test: `--repeat` cần số nguyên ≥ 1, được `{v}`");
                    return ExitCode::from(2);
                };
                if n < 1 {
                    eprintln!("vis test: `--repeat` phải ≥ 1, được {n}");
                    return ExitCode::from(2);
                }
                repeat = n;
                i += 1;
            }
            // **VD-270**: `--shard <I>/<N>`; sai cú pháp/miền/lặp ⇒ exit 2, usage nêu `--shard`.
            "--shard" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--shard` cần `<I>/<N>` (1 ≤ I ≤ N)");
                    return ExitCode::from(2);
                };
                let Some((i_shard, n_shard)) = parse_shard(v) else {
                    eprintln!("vis test: `--shard` cần `<I>/<N>` (1 ≤ I ≤ N), được `{v}`");
                    return ExitCode::from(2);
                };
                if shard.is_some() {
                    eprintln!("vis test: `--shard` lặp lại");
                    return ExitCode::from(2);
                }
                shard = Some((i_shard, n_shard));
                i += 1;
            }
            s if s.starts_with("--shard=") => {
                let v = &s["--shard=".len()..];
                let Some((i_shard, n_shard)) = parse_shard(v) else {
                    eprintln!("vis test: `--shard` cần `<I>/<N>` (1 ≤ I ≤ N), được `{v}`");
                    return ExitCode::from(2);
                };
                if shard.is_some() {
                    eprintln!("vis test: `--shard` lặp lại");
                    return ExitCode::from(2);
                }
                shard = Some((i_shard, n_shard));
            }
            // **VD-270**: `--jobs <N>` — N sai (0, âm, không nguyên, thiếu) hoặc lặp cờ ⇒ exit 2,
            // thông điệp/usage nêu `--jobs`.
            "--jobs" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--jobs` cần số nguyên ≥ 1");
                    return ExitCode::from(2);
                };
                if jobs.is_some() {
                    eprintln!("vis test: `--jobs` lặp lại");
                    return ExitCode::from(2);
                }
                let Ok(n) = v.parse::<usize>() else {
                    eprintln!("vis test: `--jobs` cần số nguyên ≥ 1, được `{v}`");
                    return ExitCode::from(2);
                };
                if n < 1 {
                    eprintln!("vis test: `--jobs` phải ≥ 1, được {n}");
                    return ExitCode::from(2);
                }
                jobs = Some(n);
                i += 1;
            }
            s if s.starts_with("--jobs=") => {
                if jobs.is_some() {
                    eprintln!("vis test: `--jobs` lặp lại");
                    return ExitCode::from(2);
                }
                let v = &s["--jobs=".len()..];
                let Ok(n) = v.parse::<usize>() else {
                    eprintln!("vis test: `--jobs` cần số nguyên ≥ 1, được `{v}`");
                    return ExitCode::from(2);
                };
                if n < 1 {
                    eprintln!("vis test: `--jobs` phải ≥ 1, được {n}");
                    return ExitCode::from(2);
                }
                jobs = Some(n);
            }
            // **VD-270**: `--slowest <N>` — N sai (0, âm, không nguyên, thiếu) hoặc lặp cờ ⇒
            // exit 2, thông điệp/usage nêu `--slowest`.
            "--slowest" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--slowest` cần số nguyên ≥ 1");
                    return ExitCode::from(2);
                };
                if slowest.is_some() {
                    eprintln!("vis test: `--slowest` lặp lại");
                    return ExitCode::from(2);
                }
                let Ok(n) = v.parse::<usize>() else {
                    eprintln!("vis test: `--slowest` cần số nguyên ≥ 1, được `{v}`");
                    return ExitCode::from(2);
                };
                if n < 1 {
                    eprintln!("vis test: `--slowest` phải ≥ 1, được {n}");
                    return ExitCode::from(2);
                }
                slowest = Some(n);
                i += 1;
            }
            s if s.starts_with("--slowest=") => {
                if slowest.is_some() {
                    eprintln!("vis test: `--slowest` lặp lại");
                    return ExitCode::from(2);
                }
                let v = &s["--slowest=".len()..];
                let Ok(n) = v.parse::<usize>() else {
                    eprintln!("vis test: `--slowest` cần số nguyên ≥ 1, được `{v}`");
                    return ExitCode::from(2);
                };
                if n < 1 {
                    eprintln!("vis test: `--slowest` phải ≥ 1, được {n}");
                    return ExitCode::from(2);
                }
                slowest = Some(n);
            }
            "--filter" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--filter` cần chuỗi con");
                    return ExitCode::from(2);
                };
                if v.starts_with('-') {
                    eprintln!(
                        "vis test: `--filter` cần chuỗi con (gặp `{v}`; dùng --filter=<substr> nếu chuỗi bắt đầu bằng `-`)"
                    );
                    return ExitCode::from(2);
                }
                loc = Some(v.clone());
                i += 1;
            }
            s if s.starts_with("--filter=") => {
                loc = Some(s["--filter=".len()..].to_string());
            }
            "--timeout" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--timeout` cần số nguyên ≥ 50 (ms)");
                    return ExitCode::from(2);
                };
                let Ok(ms) = v.parse::<u64>() else {
                    eprintln!("vis test: `--timeout` cần số nguyên ≥ 50 (ms), được `{v}`");
                    return ExitCode::from(2);
                };
                if ms < 50 {
                    eprintln!("vis test: `--timeout` phải ≥ 50 ms, được {ms}");
                    return ExitCode::from(2);
                }
                timeout_han = Some(ms);
                i += 1;
            }
            "-h" | "--help" => return usage(),
            s if s.starts_with('-') => {
                eprintln!("vis test: không hỗ trợ cờ `{s}`");
                return usage();
            }
            s => duong = Some(PathBuf::from(s)),
        }
        i += 1;
    }
    // **VD-270**: `--json-out` không đi với `--list` (hai đầu ra máy khác khuôn).
    if json_out.is_some() && list {
        eprintln!("vis test: `--json-out` không đi với `--list`");
        return ExitCode::from(2);
    }
    // **VD-270**: `--slowest` cần ca **đã chạy**; `--list` không chạy ⇒ không có thời gian.
    if slowest.is_some() && list {
        eprintln!("vis test: `--slowest` không đi với `--list` (không có ca nào chạy)");
        return ExitCode::from(2);
    }
    // **VD-270**: `--seed` chỉ có nghĩa khi xáo; thiếu `--shuffle` ⇒ lỗi dùng (exit 2).
    if seed.is_some() && !shuffle {
        eprintln!("vis test: `--seed` chỉ dùng cùng `--shuffle`");
        return ExitCode::from(2);
    }
    let goc = match duong {
        Some(p) => {
            if !p.exists() {
                eprintln!("vis test: không có đường dẫn `{}`", p.display());
                return ExitCode::from(2);
            }
            p
        }
        None => {
            match tim_mac_dinh() {
                Some(p) => p,
                None => {
                    eprintln!("vis test: không tìm thấy `test/sem` (chạy ở gốc repo hoặc truyền đường dẫn)");
                    return ExitCode::from(2);
                }
            }
        }
    };
    let mut files: Vec<PathBuf> = Vec::new();
    if goc.is_dir() {
        gom(&goc, &mut files);
    } else {
        files.push(goc.clone());
    }
    // **ST-79 (VD-108)**: tệp có `test fn` ⇒ **mỗi test một ca TAP** (chạy bằng bản sao đổi
    // `test fn <tên>` → `fn main`); tệp thường giữ nguyên hành vi fixture `.out`/`.err`.
    // **VD-267**: áp `--filter` **tại đây**, trước mọi lần chạy ⇒ ca bị loại không có tác dụng phụ.
    let loc = loc.as_deref();
    let chon = |ten: &str| match loc {
        None => true,
        // Quy ước VD-267: chuỗi rỗng ⇒ chọn 0 ca (tài liệu ở `usage`).
        Some(f) => !f.is_empty() && ten.contains(f),
    };
    let mut ca: Vec<(PathBuf, Option<(String, String)>)> = Vec::new();
    for p in files {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let tests = ds_test(&text);
        if tests.is_empty() {
            let rel = p.strip_prefix(&goc).unwrap_or(&p);
            // Tên hiển thị của **một tệp** (gốc = chính tệp) là rỗng; khi lọc dùng đường dẫn đã
            // truyền để `--filter <tên-tệp>` vẫn khớp. Tên in ra TAP giữ nguyên hành vi cũ.
            let ten_loc = if rel.as_os_str().is_empty() {
                p.to_string_lossy().to_string()
            } else {
                rel.to_string_lossy().to_string()
            };
            if chon(&ten_loc) {
                ca.push((p, None));
            }
        } else {
            for (ten, test_span, name_span) in tests {
                if !chon(&ten) {
                    continue;
                }
                let src = thanh_main(&text, test_span, name_span);
                ca.push((p.clone(), Some((ten, src))));
            }
        }
    }
    // **VD-270**: xáo **ca đã chọn** (sau `--filter`), trước `--list` và trước vòng lặp
    // ca-trước-lặp-sau; không cờ ⇒ thứ tự cũ nguyên byte.
    if shuffle {
        xao_tron(&mut ca, seed.unwrap_or(0));
    }
    // **VD-270**: `--shard I/N` — sau `--filter`/`--shuffle`, trước `--list`/`--repeat`; các
    // phần rời nhau và hợp đúng tập đã chọn.
    if let Some((i_shard, n_shard)) = shard {
        ca = ca
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| idx % n_shard == i_shard - 1)
            .map(|(_, c)| c)
            .collect();
    }
    if list {
        let mut tens: Vec<String> = Vec::new();
        for (p, bai) in &ca {
            let ten = match bai {
                Some((ten, _)) => ten.clone(),
                None => {
                    let rel = p.strip_prefix(&goc).unwrap_or(p);
                    if rel.as_os_str().is_empty() {
                        p.to_string_lossy().to_string()
                    } else {
                        rel.to_string_lossy().to_string()
                    }
                }
            };
            tens.push(ten);
        }
        if json_ra {
            let ds: Vec<String> = tens
                .iter()
                .map(|t| format!("\"{}\"", esc_json(t)))
                .collect();
            println!("{{\"tests\":[{}]}}", ds.join(","));
        } else {
            for ten in &tens {
                println!("{ten}");
            }
        }
        return ExitCode::SUCCESS;
    }
    let mut than = String::new();
    let mut hong = 0usize;
    let mut cases: Vec<CaJson> = Vec::new();
    // **VD-270**: mọi đơn vị (ca × lần lặp) chạy tuần tự khi `--jobs 1` (giữ nguyên byte), song
    // song khi `--jobs > 1`; kết quả vẫn nằm đúng ô theo **thứ tự gốc** nên phần in bên dưới
    // không đổi thứ tự.
    let cfg = CauHinhChay {
        goc: &goc,
        vm,
        strict,
        timeout_han,
        repeat,
        jobs: jobs.unwrap_or(1),
        fail_fast,
    };
    let ket_qua = if cfg.jobs <= 1 {
        chay_tuan_tu(&ca, &cfg)
    } else {
        chay_song_song(&ca, &cfg)
    };
    // **VD-270** (`--slowest`): chỉ giữ thời gian của **đơn vị đã chạy** (đúng thứ tự gốc);
    // `--fail-fast` để đuôi `None` nên không bao giờ lọt vào báo cáo.
    let mut thoi_gian_ds: Vec<(String, std::time::Duration)> = Vec::new();
    for (ten, kq, thoi_gian) in ket_qua.into_iter().flatten() {
        let so = cases.len() + 1;
        if kq.ok {
            than.push_str(&format!("ok {so} - {ten}\n"));
        } else {
            hong += 1;
            than.push_str(&format!("not ok {so} - {ten}\n# {}\n", kq.chi_tiet));
        }
        if slowest.is_some() {
            thoi_gian_ds.push((ten.clone(), thoi_gian));
        }
        cases.push(CaJson {
            ten,
            ok: kq.ok,
            msg: kq.msg,
        });
    }
    let mut tap = String::new();
    tap.push_str("TAP version 13\n");
    // **VD-267/VD-270**: kế hoạch đếm **lần lặp đã chạy** (với `--fail-fast` dừng giữa chừng,
    // TAP vẫn hợp lệ); không có `--fail-fast` thì số đã chạy = số ca đã chọn × N.
    tap.push_str(&format!("1..{}\n", cases.len()));
    tap.push_str(&than);
    let body = if json_ra || json_out.is_some() {
        Some(json_body(&cases, hong))
    } else {
        None
    };
    if let Some(path) = json_out.as_deref() {
        if let Err(e) = std::fs::write(path, body.as_deref().unwrap_or_default()) {
            eprintln!("vis test: không ghi được `{path}`: {e}");
            return ExitCode::from(2);
        }
    }
    if json_ra {
        print!("{}", body.as_deref().unwrap_or_default());
    } else {
        print!("{tap}");
    }
    // **VD-270** (`--slowest`): sau TAP/JSON, **chỉ stderr**, tối đa N dòng, giảm dần; hoà giữ
    // thứ tự gốc nhờ `sort_by_key` ổn định. Không cờ ⇒ không in gì thêm.
    if let Some(n) = slowest {
        for (ten, tg) in sap_xep_slowest(thoi_gian_ds, n) {
            eprintln!("# slowest: {} - {ten}", dinh_dang_thoi_gian(tg));
        }
    }
    if hong > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
