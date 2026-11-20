// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267** — bảng chữ ký builtin trần, tách khỏi `builtin.rs` để giữ **R1** ≤ 1.500 dòng.
//!
//! `SIGS` là một nguồn cho `builtin_name` (interp) và `frame::is_builtin_name`; nội dung **không
//! đổi** so với bản cũ, chỉ thêm `panic` (`panic(str) -> never`, VD-267).

/// Chữ ký builtin (số đối số tối thiểu/tối đa).
#[derive(Debug, Clone, Copy)]
pub struct BuiltinSig {
    pub name: &'static str,
    pub min: usize,
    pub max: usize,
}

/// Bảng chữ ký: **8 tên dựng sẵn** cũ + **3 lệnh chế độ lệnh** (ST-18) + `assert` + `panic`.
// **ST-79**: `assert` (dùng cho `test fn`); **VD-267**: `panic` — 23 mục.
pub const SIGS: [BuiltinSig; 23] = [
    BuiltinSig {
        name: "print",
        // **R6d (VD-178)**: `print(a, b, …)` — cách nhau 1 dấu cách; `print()` ⇒ `\n`.
        min: 0,
        max: 255,
    },
    BuiltinSig {
        name: "assert",
        // **R20a (VD-178)**: `assert cond, "thông điệp"`.
        min: 1,
        max: 2,
    },
    BuiltinSig {
        name: "panic",
        // **VD-267**: `panic(str)` (hoặc `panic(Error)` ở tầng động) — phân kỳ.
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "len",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "str",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "echo",
        min: 1,
        max: 4,
    },
    BuiltinSig {
        name: "lines",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "take",
        min: 2,
        max: 2,
    },
    BuiltinSig {
        name: "ls",
        min: 0,
        max: 3,
    },
    BuiltinSig {
        name: "open",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "render",
        min: 1,
        max: 2,
    },
    BuiltinSig {
        name: "get",
        min: 2,
        max: 2,
    },
    BuiltinSig {
        name: "select",
        min: 2,
        max: 2,
    },
    BuiltinSig {
        name: "sort_by",
        min: 2,
        max: 3,
    },
    BuiltinSig {
        name: "where",
        min: 2,
        max: 2,
    },
    BuiltinSig {
        name: "each",
        min: 2,
        max: 2,
    },
    BuiltinSig {
        name: "first",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "count",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "Ok",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "Err",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "Some",
        min: 1,
        max: 1,
    },
    BuiltinSig {
        name: "None",
        min: 0,
        max: 0,
    },
    BuiltinSig {
        name: "ref",
        min: 1,
        max: 1,
    },
];

/// Tra chữ ký theo tên.
pub fn sig(name: &str) -> Option<BuiltinSig> {
    SIGS.iter().copied().find(|s| s.name == name)
}

/// `true` nếu `name` là builtin.
pub fn builtin_name(name: &str) -> Option<&'static str> {
    sig(name).map(|s| s.name)
}
