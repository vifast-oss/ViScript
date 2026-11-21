// SPDX-License-Identifier: MIT OR Apache-2.0

// ===========================================================================
// 8. RSS
// ===========================================================================

fn status_kb(key: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

fn print_rss(label: &str) {
    let rss = status_kb("VmRSS:");
    let hwm = status_kb("VmHWM:");
    println!(
        "RSS\t{label}\t{:.1}\t{:.1}",
        rss as f64 / 1024.0,
        hwm as f64 / 1024.0
    );
}

fn rss_child(scenario: &str) {
    match scenario {
        "baseline" => print_rss("baseline"),
        "threads4" => {
            let b = Arc::new(Barrier::new(5));
            let mut hs = Vec::new();
            for _ in 0..4 {
                let b = Arc::clone(&b);
                hs.push(std::thread::spawn(move || {
                    b.wait();
                    b.wait();
                }));
            }
            b.wait();
            print_rss("threads4-idle");
            b.wait();
            for h in hs {
                h.join().unwrap();
            }
        }
        "threads4-256m-stack" => {
            let b = Arc::new(Barrier::new(5));
            let mut hs = Vec::new();
            for _ in 0..4 {
                let b = Arc::clone(&b);
                hs.push(
                    std::thread::Builder::new()
                        .stack_size(256 * 1024 * 1024)
                        .spawn(move || {
                            b.wait();
                            b.wait();
                        })
                        .unwrap(),
                );
            }
            b.wait();
            print_rss("threads4-256m-stack-idle");
            b.wait();
            for h in hs {
                h.join().unwrap();
            }
        }
        "copy-50x10k" => {
            let a: Vec<Value> = (0..50).map(|_| list_ints(10_000)).collect();
            print_rss("copy-50x10k-built");
            let b = a.clone();
            print_rss("copy-50x10k-cloned");
            drop(b);
            print_rss("copy-50x10k-dropped-clone");
            black_box(&a);
        }
        "per-thread-copy-100x10k" => {
            let b = Arc::new(Barrier::new(5));
            let mut hs = Vec::new();
            for _ in 0..4 {
                let b = Arc::clone(&b);
                hs.push(std::thread::spawn(move || {
                    let v: Vec<Value> = (0..25).map(|_| list_ints(10_000)).collect();
                    black_box(&v);
                    b.wait();
                    b.wait();
                    drop(v);
                }));
            }
            b.wait();
            print_rss("per-thread-copy-100x10k-live");
            b.wait();
            for h in hs {
                h.join().unwrap();
            }
        }
        "arc-100x10k" => {
            let a: Vec<ArcValue> = (0..100).map(|_| arc_list_ints(10_000)).collect();
            print_rss("arc-100x10k-built");
            let heaps: Vec<Vec<ArcValue>> = (0..4).map(|_| a.clone()).collect();
            print_rss("arc-100x10k-4heap-clones");
            black_box(&heaps);
            black_box(&a);
        }
        "queue-copy-64x10k" => {
            let q: Vec<Wire> = (0..64).map(|_| wire_list_ints(10_000)).collect();
            print_rss("queue-copy-64x10k");
            black_box(&q);
        }
        "queue-arc-64x10k" => {
            let one = arc_list_ints(10_000);
            let q: Vec<ArcValue> = (0..64).map(|_| one.clone()).collect();
            print_rss("queue-arc-64x10k");
            black_box(&q);
        }
        "calib-1m" => {
            let v = list_ints(1_000_000);
            print_rss("calib-1m-values");
            black_box(&v);
        }
        other => eprintln!("RSS scenario không biết: {other}"),
    }
}

fn run_rss_children() {
    let exe = std::env::current_exe().expect("current_exe");
    let scenarios = [
        "baseline",
        "threads4",
        "threads4-256m-stack",
        "copy-50x10k",
        "per-thread-copy-100x10k",
        "arc-100x10k",
        "queue-copy-64x10k",
        "queue-arc-64x10k",
        "calib-1m",
    ];
    for s in scenarios {
        let out = std::process::Command::new(&exe)
            .arg("--rss-child")
            .arg(s)
            .output()
            .expect("chạy tiến trình con RSS");
        print!("{}", String::from_utf8_lossy(&out.stdout));
        if !out.status.success() {
            eprintln!(
                "RSS child {s} lỗi: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}

// ===========================================================================
// 9. Main
// ===========================================================================

fn cpu_model() -> String {
    let s = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            if let Some(v) = rest.split(':').nth(1) {
                return v.trim().to_string();
            }
        }
    }
    "unknown".to_string()
}

fn cpu_scale_only() {
    warm_cpu(800);
    let tasks = 512usize;
    let (iters, us) = calibrate_cpu(200_000.0);
    println!(
        "CPUSCALE\tcpu={}\ttasks={tasks}\titers={iters}\tper_task_us={us:.1}",
        cpu_model()
    );
    println!("CPUSCALE\tworkers\tms_med\tms_min\tms_max\tspeedup");
    let workers_list = [1usize, 2, 4, 8, 12];
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); workers_list.len()];
    // Chạy xen kẽ 3 vòng để giảm thiên lệch thứ tự (xung CPU tăng dần).
    for _ in 0..3 {
        for (idx, w) in workers_list.iter().enumerate() {
            let start = Instant::now();
            let mut hs = Vec::new();
            for k in 0..*w {
                let n = tasks / *w + usize::from(k < tasks % *w);
                hs.push(std::thread::spawn(move || {
                    let mut acc = 0u64;
                    for _ in 0..n {
                        acc ^= cpu_work(iters);
                    }
                    black_box(acc);
                }));
            }
            for h in hs {
                h.join().unwrap();
            }
            samples[idx].push(start.elapsed().as_secs_f64());
        }
    }
    let mut med = Vec::new();
    for s in &mut samples {
        s.sort_by(|a, b| a.total_cmp(b));
        med.push(s[1]);
    }
    for (idx, w) in workers_list.iter().enumerate() {
        let s = &samples[idx];
        println!(
            "CPUSCALE\t{w}\t{:.1}\t{:.1}\t{:.1}\t{:.2}",
            med[idx] * 1000.0,
            s[0] * 1000.0,
            s[2] * 1000.0,
            med[0] / med[idx]
        );
    }
}

/// Đo riêng `run_pool_copy` với payload rỗng — tách chi phí pool khỏi chi phí copy.
fn pool_cpu_only() {
    warm_cpu(800);
    let (iters, us) = calibrate_cpu(200_000.0);
    println!(
        "POOLSCALE\tcpu={}\ttasks=512\titers={iters}\tper_task_us={us:.1}",
        cpu_model()
    );
    println!("POOLSCALE\tworkers\tms_med\tms_min\tms_max\tspeedup");
    let workers_list = [1usize, 2, 4];
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); workers_list.len()];
    for _ in 0..3 {
        for (idx, w) in workers_list.iter().enumerate() {
            let (secs, got) = run_pool_copy(*w, 512, iters, &Wire::Unit);
            assert_eq!(got, 512);
            samples[idx].push(secs);
        }
    }
    let mut med = Vec::new();
    for s in &mut samples {
        s.sort_by(|a, b| a.total_cmp(b));
        med.push(s[1]);
    }
    for (idx, w) in workers_list.iter().enumerate() {
        let s = &samples[idx];
        println!(
            "POOLSCALE\t{w}\t{:.1}\t{:.1}\t{:.1}\t{:.2}",
            med[idx] * 1000.0,
            s[0] * 1000.0,
            s[2] * 1000.0,
            med[0] / med[idx]
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--cpu-scale" {
        cpu_scale_only();
        return;
    }
    if args.len() >= 2 && args[1] == "--pool-cpu-scale" {
        pool_cpu_only();
        return;
    }
    if args.len() >= 3 && args[1] == "--rss-child" {
        rss_child(&args[2]);
        return;
    }
    warm_cpu(800);

    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    println!(
        "ENV\tnproc={nproc}\tload={}\tarch={}\tos={}\tcpu={}",
        load.trim(),
        std::env::consts::ARCH,
        std::env::consts::OS,
        cpu_model()
    );
    println!(
        "SIZE\tValue={}\tWire={}\tArcValue={}",
        std::mem::size_of::<Value>(),
        std::mem::size_of::<Wire>(),
        std::mem::size_of::<ArcValue>()
    );
    assert_send_sync::<Wire>();
    assert_send_sync::<ArcValue>();

    // ------------------------------------------------------------------
    // 9.1 Clone / copy / move một luồng
    // ------------------------------------------------------------------
    println!("CLONE\tlabel\tns_med\tns_min\tMiB/s\tallocs\tKiB");

    let list1k = list_ints(1000);
    let list10k = list_ints(10_000);
    report_clone("value.list1k", &list1k, value_stats(&list1k), 2000);
    report_clone("value.list10k", &list10k, value_stats(&list10k), 200);

    let map1k = map_value(1000);
    let map10k = map_value(10_000);
    report_clone("value.map1k", &map1k, value_stats(&map1k), 1000);
    report_clone("value.map10k", &map10k, value_stats(&map10k), 100);

    let rec1k = record_value(16, 1000);
    let rec10k = record_value(16, 10_000);
    report_clone("value.record16+list1k", &rec1k, value_stats(&rec1k), 1000);
    report_clone(
        "value.record16+list10k",
        &rec10k,
        value_stats(&rec10k),
        100,
    );

    let enum1k = enum_value(1000);
    let enum10k = enum_value(10_000);
    report_clone("value.enum(list1k)", &enum1k, value_stats(&enum1k), 1000);
    report_clone(
        "value.enum(list10k)",
        &enum10k,
        value_stats(&enum10k),
        100,
    );

    let enumvals10k = enum_values(10_000);
    report_clone(
        "value.10k-enum-values",
        &enumvals10k,
        value_stats(&enumvals10k),
        100,
    );

    let str1k = Value::Str("x".repeat(1024));
    let str1m = Value::Str("x".repeat(1024 * 1024));
    report_clone("value.str1KiB", &str1k, value_stats(&str1k), 50_000);
    report_clone("value.str1MiB", &str1m, value_stats(&str1m), 200);

    let bytes1m = Value::Bytes(vec![7u8; 1024 * 1024]);
    report_clone("value.bytes1MiB", &bytes1m, value_stats(&bytes1m), 200);

    let closure = Value::Fn(Rc::new(ClosureStub {
        name: "f".into(),
        env: Rc::new(RefCell::new(HashMap::new())),
    }));
    report_clone("value.closure(Rc)", &closure, value_stats(&closure), 2_000_000);

    // Move trong cùng luồng: O(1), không copy. Clone setup nằm NGOÀI vùng đo.
    let mut move_samples = Vec::new();
    for _ in 0..5 {
        let mut v = list10k.clone();
        let t = Instant::now();
        for _ in 0..10_000_000 {
            let x = std::mem::take(&mut v);
            v = x;
            black_box(&v);
        }
        move_samples.push(t.elapsed().as_nanos() as f64 / 10_000_000.0);
    }
    move_samples.sort_by(|a, b| a.total_cmp(b));
    report_line(
        "move.list10k(in-thread)",
        move_samples[2],
        move_samples[0],
        (0, 0),
    );

    // Clone + push (ngữ nghĩa hiện tại: đọc biến đã deep-copy, push O(1)).
    let (push_med, push_min) = bench(2000, 5, || {
        let mut v = list1k.clone();
        if let Value::List(xs) = &mut v {
            xs.push(Value::Int(-1));
        }
        black_box(v);
    });
    report_line("value.list1k.clone+push", push_med, push_min, (1, 0));
    let (push10_med, push10_min) = bench(200, 5, || {
        let mut v = list10k.clone();
        if let Value::List(xs) = &mut v {
            xs.push(Value::Int(-1));
        }
        black_box(v);
    });
    report_line("value.list10k.clone+push", push10_med, push10_min, (1, 0));

    // Wire: copy tại biên và decode ở đầu nhận.
    let wire1k = wire_list_ints(1000);
    let wire10k = wire_list_ints(10_000);
    let (med, min) = bench(2000, 5, || {
        black_box(wire1k.clone());
    });
    report_line("wire.list1k.clone", med, min, wire_stats(&wire1k));
    let (med, min) = bench(200, 5, || {
        black_box(wire10k.clone());
    });
    report_line("wire.list10k.clone", med, min, wire_stats(&wire10k));
    let (med, min) = bench(2000, 5, || {
        black_box(to_wire(&list1k));
    });
    report_line("to_wire.list1k", med, min, wire_stats(&wire1k));
    let (med, min) = bench(200, 5, || {
        black_box(to_wire(&list10k));
    });
    report_line("to_wire.list10k", med, min, wire_stats(&wire10k));
    let (med, min) = bench(2000, 5, || {
        black_box(wire_to_value(&wire1k));
    });
    report_line("wire_to_value.list1k", med, min, wire_stats(&wire1k));
    let (med, min) = bench(200, 5, || {
        black_box(wire_to_value(&wire10k));
    });
    report_line("wire_to_value.list10k", med, min, wire_stats(&wire10k));

    let wire_str1m = Wire::Str("x".repeat(1024 * 1024).into());

    // Move-convert: đối chiếu với copy-convert ở trên. Container được move,
    // chỉ lá (String/Box<str>, tên enum) phải chuyển; đây là chi phí **tối thiểu**
    // của biên nếu ta tiêu thụ giá trị nguồn thay vì mượn+rồi copy.
    report_copy_value(
        "to_wire.batch.list1k",
        1000,
        || list_ints(1000),
        to_wire,
        wire_stats(&wire1k),
    );
    report_move_value(
        "into_wire.move.list1k",
        1000,
        || list_ints(1000),
        into_wire,
        wire_stats(&wire1k),
    );
    report_copy_value(
        "to_wire.batch.list10k",
        200,
        || list_ints(10_000),
        to_wire,
        wire_stats(&wire10k),
    );
    report_move_value(
        "into_wire.move.list10k",
        200,
        || list_ints(10_000),
        into_wire,
        wire_stats(&wire10k),
    );
    report_copy_wire(
        "wire_to_value.batch.list1k",
        1000,
        || wire_list_ints(1000),
        wire_to_value,
        wire_stats(&wire1k),
    );
    report_move_wire(
        "wire_into_value.move.list1k",
        1000,
        || wire_list_ints(1000),
        wire_into_value,
        wire_stats(&wire1k),
    );
    report_copy_wire(
        "wire_to_value.batch.list10k",
        200,
        || wire_list_ints(10_000),
        wire_to_value,
        wire_stats(&wire10k),
    );
    report_move_wire(
        "wire_into_value.move.list10k",
        200,
        || wire_list_ints(10_000),
        wire_into_value,
        wire_stats(&wire10k),
    );
    report_copy_value(
        "to_wire.batch.str1MiB",
        50,
        || Value::Str("x".repeat(1024 * 1024)),
        to_wire,
        wire_stats(&wire_str1m),
    );
    report_move_value(
        "into_wire.move.str1MiB",
        50,
        || Value::Str("x".repeat(1024 * 1024)),
        into_wire,
        wire_stats(&wire_str1m),
    );
    report_copy_wire(
        "wire_to_value.batch.str1MiB",
        50,
        || Wire::Str("x".repeat(1024 * 1024).into()),
        wire_to_value,
        wire_stats(&wire_str1m),
    );
    report_move_wire(
        "wire_into_value.move.str1MiB",
        50,
        || Wire::Str("x".repeat(1024 * 1024).into()),
        wire_into_value,
        wire_stats(&wire_str1m),
    );

    // Arc/COW: clone O(1); COW write vẫn O(n).
    let arc1k = arc_list_ints(1000);
    let arc10k = arc_list_ints(10_000);
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc1k.clone());
    });
    report_line("arc.list1k.clone", med, min, (0, 0));
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc10k.clone());
    });
    report_line("arc.list10k.clone", med, min, (0, 0));
    let arc_map = arc_map_value(1000);
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc_map.clone());
    });
    report_line("arc.map1k.clone", med, min, (0, 0));
    let arc_rec = arc_record_value(16, 1000);
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc_rec.clone());
    });
    report_line("arc.record16+list1k.clone", med, min, (0, 0));
    let arc_enum = arc_enum_value(1000);
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc_enum.clone());
    });
    report_line("arc.enum-list1k.clone", med, min, (0, 0));

    // Arc cho lá str/bytes: clone O(1) — đối chiếu trực tiếp với String/Vec clone.
    let arc_str1k = ArcValue::Str(Arc::from("x".repeat(1024)));
    let arc_str1m = ArcValue::Str(Arc::from("x".repeat(1024 * 1024)));
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc_str1k.clone());
    });
    report_line("arc.str1KiB.clone", med, min, (0, 0));
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc_str1m.clone());
    });
    report_line("arc.str1MiB.clone", med, min, (0, 0));
    let arc_bytes1m = ArcValue::Bytes(Arc::from(vec![7u8; 1024 * 1024]));
    let (med, min) = bench(2_000_000, 5, || {
        black_box(arc_bytes1m.clone());
    });
    report_line("arc.bytes1MiB.clone", med, min, (0, 0));

    // Wire cho lá lớn: encode/decode vẫn copy toàn bộ.
    let (med, min) = bench(200, 5, || {
        black_box(wire_str1m.clone());
    });
    report_line("wire.str1MiB.clone", med, min, wire_stats(&wire_str1m));
    let (med, min) = bench(200, 5, || {
        black_box(to_wire(&str1m));
    });
    report_line("to_wire.str1MiB", med, min, wire_stats(&wire_str1m));
    let (med, min) = bench(200, 5, || {
        black_box(wire_to_value(&wire_str1m));
    });
    report_line("wire_to_value.str1MiB", med, min, wire_stats(&wire_str1m));
    let (med, min) = bench(2000, 5, || {
        let mut v = arc1k.clone();
        if let ArcValue::List(xs) = &mut v {
            Arc::make_mut(xs).push(ArcValue::Int(-1));
        }
        black_box(v);
    });
    report_line(
        "arc.list1k.cow-push",
        med,
        min,
        (1, (1001 * std::mem::size_of::<ArcValue>()) as u64),
    );
    let (med, min) = bench(200, 5, || {
        let mut v = arc10k.clone();
        if let ArcValue::List(xs) = &mut v {
            Arc::make_mut(xs).push(ArcValue::Int(-1));
        }
        black_box(v);
    });
    report_line(
        "arc.list10k.cow-push",
        med,
        min,
        (1, (10_001 * std::mem::size_of::<ArcValue>()) as u64),
    );

    // ------------------------------------------------------------------
    // 9.2 Spawn + channel 2–4 luồng
    // ------------------------------------------------------------------
    let (cpu_iters, cpu_us) = calibrate_cpu(200_000.0);
    println!("CPU\titers={cpu_iters}\tper_task_us={cpu_us:.1}");

    println!("POOL\tmode\tpayload\tworkers\ttasks\tms\tus_per_task\tspeedup\tgot");

    // 9.2a Dispatch rỗng: chi phí spawn + channel.
    let unit_wire = Wire::Unit;
    let unit_tasks = 20_000;
    let mut base_dispatch = 0.0;
    for w in [1usize, 2, 4] {
        let (secs, got) = run_pool_copy(w, unit_tasks, 0, &unit_wire);
        if w == 1 {
            base_dispatch = secs;
        }
        println!(
            "POOL\tcopy\tunit\t{w}\t{unit_tasks}\t{:.1}\t{:.2}\t{:.2}\t{got}",
            secs * 1000.0,
            secs * 1e6 / unit_tasks as f64,
            base_dispatch / secs
        );
    }

    // 9.2b CPU-bound, payload rỗng: trần song song của máy.
    let cpu_tasks = 512;
    let mut base_cpu = 0.0;
    for w in [1usize, 2, 4] {
        let (secs, got) = run_pool_copy(w, cpu_tasks, cpu_iters, &unit_wire);
        if w == 1 {
            base_cpu = secs;
        }
        println!(
            "POOL\tcopy\tcpu-unit\t{w}\t{cpu_tasks}\t{:.1}\t{:.2}\t{:.2}\t{got}",
            secs * 1000.0,
            secs * 1e6 / cpu_tasks as f64,
            base_cpu / secs
        );
    }

    // 9.2c CPU + payload; so copy vs Arc vs local.
    let payloads: [(&str, Wire, ArcValue, usize); 2] = [
        ("list1k", wire_list_ints(1000), arc_list_ints(1000), 512),
        ("list10k", wire_list_ints(10_000), arc_list_ints(10_000), 256),
    ];
    for (pname, wpay, apay, tasks) in &payloads {
        let tasks = *tasks;
        let local_copy = run_local_copy(tasks, cpu_iters, wpay);
        println!(
            "POOL\tcopy-local\t{pname}\t0\t{tasks}\t{:.1}\t{:.2}\t-\t{tasks}",
            local_copy * 1000.0,
            local_copy * 1e6 / tasks as f64
        );
        let local_arc = run_local_arc(tasks, cpu_iters, apay);
        println!(
            "POOL\tarc-local\t{pname}\t0\t{tasks}\t{:.1}\t{:.2}\t-\t{tasks}",
            local_arc * 1000.0,
            local_arc * 1e6 / tasks as f64
        );

        let mut base_copy = 0.0;
        let mut base_arc = 0.0;
        for w in [1usize, 2, 4] {
            let (secs, got) = run_pool_copy(w, tasks, cpu_iters, wpay);
            if w == 1 {
                base_copy = secs;
            }
            println!(
                "POOL\tcopy\t{pname}\t{w}\t{tasks}\t{:.1}\t{:.2}\t{:.2}\t{got}",
                secs * 1000.0,
                secs * 1e6 / tasks as f64,
                base_copy / secs
            );
            let (secs, got) = run_pool_arc(w, tasks, cpu_iters, apay);
            if w == 1 {
                base_arc = secs;
            }
            println!(
                "POOL\tarc\t{pname}\t{w}\t{tasks}\t{:.1}\t{:.2}\t{:.2}\t{got}",
                secs * 1000.0,
                secs * 1e6 / tasks as f64,
                base_arc / secs
            );
        }
    }

    // 9.2d Channel: copy (2 lần: encode + decode), move (không copy), Arc (O(1)).
    println!("CHANNEL\tmode\tpayload\tproducers\tmsgs\tms\tus_per_msg\tMiB/s\tgot");
    let ch_cases: [(&str, Wire, usize, usize); 4] = [
        ("unit", Wire::Unit, 20_000, 0),
        ("list1k", wire_list_ints(1000), 5_000, 1000),
        ("list10k", wire_list_ints(10_000), 500, 10_000),
        (
            "str1MiB",
            Wire::Str("x".repeat(1024 * 1024).into()),
            200,
            1024 * 1024,
        ),
    ];
    for (pname, template, per, len) in &ch_cases {
        let pname = *pname;
        let per = *per;
        for producers in [2usize, 4] {
            let msgs = producers * per;
            let (secs, got) = run_channel_copy(producers, per, template, true);
            let bytes = (got * len * std::mem::size_of::<Wire>()) as f64;
            println!(
                "CHANNEL\tcopy+decode\t{pname}\t{producers}\t{msgs}\t{:.1}\t{:.2}\t{:.1}\t{got}",
                secs * 1000.0,
                secs * 1e6 / got as f64,
                bytes / secs / 1024.0 / 1024.0
            );
            let arc_template = match pname {
                "unit" => ArcValue::Unit,
                "list1k" => arc_list_ints(1000),
                "str1MiB" => ArcValue::Str(Arc::from("x".repeat(1024 * 1024))),
                _ => arc_list_ints(10_000),
            };
            let (secs, got) = run_channel_arc(producers, per, &arc_template);
            let bytes = (got * len * std::mem::size_of::<ArcValue>()) as f64;
            println!(
                "CHANNEL\tarc\t{pname}\t{producers}\t{msgs}\t{:.1}\t{:.2}\t{:.1}\t{got}",
                secs * 1000.0,
                secs * 1e6 / got as f64,
                bytes / secs / 1024.0 / 1024.0
            );
        }
        // Move: prebuild, đo riêng phần channel (không copy payload).
        if pname != "unit" {
            let n = match pname {
                "list1k" => 1000,
                "str1MiB" => 50,
                _ => 100,
            };
            let messages: Vec<Wire> = (0..n).map(|_| template.clone()).collect();
            let (secs, got) = run_channel_move(messages);
            let bytes = (got * len * std::mem::size_of::<Wire>()) as f64;
            println!(
                "CHANNEL\tmove\t{pname}\t1\t{got}\t{:.1}\t{:.2}\t{:.1}\t{got}",
                secs * 1000.0,
                secs * 1e6 / got as f64,
                bytes / secs / 1024.0 / 1024.0
            );
        }
    }

    // ------------------------------------------------------------------
    // 9.3 RSS (tiến trình con sạch)
    // ------------------------------------------------------------------
    println!("RSS\tlabel\tRSS_MB\tHWM_MB");
    run_rss_children();
}
