// SPDX-License-Identifier: MIT OR Apache-2.0
//! Các sổ mã `01xx`..`07xx`, [`BOOKS`], [`BLOCKS_RESERVED`] + [`all_codes`]. Tách nguyên văn
//! từ `lib.rs` (ST-318d).

use super::codes::{Code, Severity};

/// Sổ mã lỗi **vòng 1**: đúng 10 mã `VS0101..VS0110` của `docs/SPEC-lex.md` §7.
///
/// **ĐỀ XUẤT quy tắc cấp** (chưa phải chuẩn — chờ Agent 0 duyệt qua YC-14; SPEC-lex là nơi
/// *định nghĩa* chuẩn, crate này chỉ *thực thi*):
/// - mã 4 chữ số thập phân, khối theo tầng: `01xx` từ vựng (lexer), `02xx` cú pháp/parser,
///   `03xx` tên/phân giải, `04xx` **kiểu** (`VS0412` = "kiểu không khớp" đã có trong thiết kế),
///   `05xx` runtime; cấp tăng dần, **không tái dùng** số đã cấp;
/// - mọi mã phải có mặt trong SPEC-lex/errata trước khi vào sổ; `message_vi` ở đây là **placeholder
///   vòng 1** — bảng vi/en/ja thuộc ST-03;
/// - đổi nghĩa/đánh số lại phải qua Agent 0 (VD-08), không tự sửa.
pub const CODES: [Code; 11] = [
    Code {
        id: 101,
        short: "VS0101",
        message_vi: "chuỗi chưa đóng",
        message_en: "unterminated string",
        message_ja: "文字列が閉じられていません",
        severity: Severity::Error,
    },
    Code {
        id: 102,
        short: "VS0102",
        message_vi: "escape không hợp lệ",
        message_en: "invalid escape",
        message_ja: "無効なエスケープです",
        severity: Severity::Error,
    },
    Code {
        id: 103,
        short: "VS0103",
        message_vi: "ký tự lạ ngoài bảng token",
        message_en: "unexpected character outside the token table",
        message_ja: "トークン表にない不正な文字です",
        severity: Severity::Error,
    },
    Code {
        id: 104,
        short: "VS0104",
        message_vi: "literal số sai/tràn i64",
        message_en: "invalid numeric literal / i64 overflow",
        message_ja: "数値リテラルが不正、または i64 を超過しています",
        severity: Severity::Error,
    },
    Code {
        id: 105,
        short: "VS0105",
        message_vi: "char literal không đúng 1 scalar",
        message_en: "char literal must be exactly one scalar",
        message_ja: "char リテラルはちょうど 1 スカラーである必要があります",
        severity: Severity::Error,
    },
    Code {
        id: 106,
        short: "VS0106",
        message_vi: "byte string có ký tự > 0x7F",
        message_en: "byte string contains a character > 0x7F",
        message_ja: "バイト文字列に 0x7F を超える文字があります",
        severity: Severity::Error,
    },
    Code {
        id: 107,
        short: "VS0107",
        message_vi: "tab (cảnh báo-lỗi; tab = 4)",
        message_en: "tab (warning-error; tab = 4)",
        message_ja: "タブ（警告エラー、タブ = 4）",
        severity: Severity::Warning,
    },
    Code {
        id: 108,
        short: "VS0108",
        message_vi: "nội suy `{` không đóng",
        message_en: "unclosed `{` interpolation",
        message_ja: "補間 `{` が閉じられていません",
        severity: Severity::Error,
    },
    Code {
        id: 109,
        short: "VS0109",
        message_vi: "thụt lề không khớp đơn vị/DEDENT sai",
        message_en: "inconsistent indentation / invalid DEDENT",
        message_ja: "インデントの不一致／DEDENT が不正です",
        severity: Severity::Error,
    },
    Code {
        id: 110,
        short: "VS0110",
        message_vi: "`^` sai ngữ cảnh (lexer chế độ lệnh biết đầu stage)",
        message_en: "`^` out of context (command-mode lexer knows stage start)",
        message_ja: "`^` の文脈が不正です（コマンドモードの字句解析器が stage 先頭を判定）",
        severity: Severity::Error,
    },
    Code {
        id: 111,
        short: "VS0111",
        message_vi:
            "tên biến thể nên viết **hoa** chữ đầu (`Some`/`None`/`Ok`/`Err`) — **ngừng phát từ VD-212** (sai hoa thường trong pattern nay báo `VS0203`)",
        message_en: "variant name should be UpperCamel (`Some`/`None`/`Ok`/`Err`) — **stops emitting from VD-212** (wrong case in patterns now report `VS0203`)",
        message_ja: "列挙型の名前は先頭を大文字にしてください（`Some`/`None`/`Ok`/`Err`）",
        severity: Severity::Warning,
    },
];

/// Sổ **mã cú pháp** `02xx` (VD-35) — tách khỏi [`CODES`] để test "CODES khớp SPEC-lex §7" vẫn
/// là hàng rào thật (không nới kỳ vọng cũ). Quy tắc cấp: `VS0201` lỗi cú pháp chung; `VS0202`
/// `%` dùng như phần trăm (thông điệp phải gợi ý `50pct`, L3 của `docs/DUYET-AGENT0-vong-va.md`).
/// VD-215 (2026-11-18): +**VS0207** (`trộn loại khoá trong {…}`, lỗi) và +**VS0208**
/// (`` `{[k]: …}` khoá là danh sách``, **cảnh báo**) — đăng ký theo VD-215 (không dùng lại `VS0205`).
/// VD-223 (2026-11-19, proxy A0): +**VS0211** (tên trùng module std đã import, **cảnh báo**).
/// **VD-224 (2026-11-19, proxy A0)**: cấm `--b` (VS0209) **huỷ** — `--b` hợp lệ ở mọi hàm (docs §5.1)
/// ⇒ **53** mã.
pub const SYNTAX_CODES: [Code; 9] = [
    Code {
        id: 201,
        short: "VS0201",
        message_vi: "lỗi cú pháp: token không mong đợi/thiếu vế",
        message_en: "syntax error: unexpected token / missing operand",
        message_ja: "構文エラー: 予期しないトークン／項が不足しています",
        severity: Severity::Error,
    },
    Code {
        id: 202,
        short: "VS0202",
        message_vi: "`%` dùng như phần trăm — viết `50pct`",
        message_en: "`%` used as percent — write `50pct`",
        message_ja: "`%` がパーセントとして使われています — `50pct` と書いてください",
        severity: Severity::Error,
    },
    // **R9 (VD-178)**: tách nghĩa khỏi `VS0201` — mỗi mã một nghĩa.
    Code {
        id: 203,
        short: "VS0203",
        message_vi: "từ khoá đã bỏ — dùng cú pháp mới (xem gợi ý)",
        message_en: "removed keyword — use the new syntax (see hint)",
        message_ja: "廃止されたキーワードです — 新しい構文を使ってください（ヒント参照）",
        severity: Severity::Error,
    },
    Code {
        id: 204,
        short: "VS0204",
        message_vi: "`const` cần ghi kiểu khi vế phải không phải literal",
        message_en: "`const` needs a type annotation when the right-hand side is not a literal",
        message_ja: "右辺がリテラルでない場合、`const` には型注釈が必要です",
        severity: Severity::Error,
    },
    Code {
        id: 205,
        short: "VS0205",
        message_vi: "dựng bản ghi bằng `{…}` — dùng `Tên(trường: …)` hoặc khoá chuỗi cho map",
        message_en:
            "record construction with `{…}` — use `Name(field: …)` or string keys for a map",
        message_ja:
            "`{…}` によるレコード構築 — `Name(field: …)` か、マップには文字列キーを使ってください",
        severity: Severity::Error,
    },
    Code {
        id: 206,
        short: "VS0206",
        message_vi: "khối `else` của `let … else` phải thoát (return/break/continue/panic)",
        message_en: "the `else` block of `let … else` must diverge (return/break/continue/panic)",
        message_ja:
            "`let … else` の `else` ブロックは脱出する必要があります（return/break/continue/panic）",
        severity: Severity::Error,
    },
    // **VD-215 (2026-11-18)**: `{…}` trộn loại khoá — bất kể thứ tự.
    Code {
        id: 207,
        short: "VS0207",
        message_vi: "trộn loại khoá trong `{…}` — bản ghi và map không trộn được",
        message_en: "mixed key kinds in `{…}` — records and maps cannot be mixed",
        message_ja: "`{…}` のキーの種類が混在しています — レコードとマップは混ぜられません",
        severity: Severity::Error,
    },
    // **VD-215 (2026-11-18)**: `{[k]: …}` — danh sách làm khoá; gần quen JavaScript.
    // `{([k]): …}` (lồng trong ngoặc) là cách tắt cảnh báo này.
    Code {
        id: 208,
        short: "VS0208",
        message_vi: "`{[k]: …}` khoá là danh sách — dùng `{(k): v}`; viết `{([k]): …}` nếu muốn giữ danh sách làm khoá",
        message_en: "`{[k]: …}` uses a list as key — use `{(k): v}`; write `{([k]): …}` to keep a list key",
        message_ja:
            "`{[k]: …}` はリストをキーにします — `{(k): v}` を使ってください。リストのキーを保持するなら `{([k]): …}` と書いてください",
        severity: Severity::Warning,
    },
    // **VD-223 (2026-11-19, uỷ quyền A0-proxy)**: tên trần trùng module std đã import —
    // `x.m(...)` hiểu là thành viên của giá trị (không còn route im lặng tới `std.<module>`).
    Code {
        id: 211,
        short: "VS0211",
        message_vi: "tên `X` là module std đã import — `X.xxx(...)` được hiểu là thành viên của giá trị",
        message_en: "`X` is a std module you imported — `X.xxx(...)` is read as a member of the value",
        message_ja: "`X` は import 済みの std モジュールです — `X.xxx(...)` は値のメンバー呼び出しとして解釈されます",
        severity: Severity::Warning,
    },
];

/// Sổ **mã tên** `03xx` (VD-42, ST-11) — sổ **thứ ba**, tách khỏi [`CODES`]/[`SYNTAX_CODES`] vì
/// cùng lý do VD-35: sổ từ vựng vẫn đúng bằng bảng SPEC-lex §7.
///
/// Cấp tối thiểu theo VD-42: `VS0301` tên không tồn tại (ST-11 dùng cho **tên giá trị**; tên kiểu
/// để ST-12), `VS0302` khai báo trùng trong **cùng** phạm vi.
pub const NAME_CODES: [Code; 4] = [
    Code {
        id: 301,
        short: "VS0301",
        message_vi: "tên không tồn tại",
        message_en: "name does not exist",
        message_ja: "名前が見つかりません",
        severity: Severity::Error,
    },
    Code {
        id: 302,
        short: "VS0302",
        message_vi: "khai báo trùng trong cùng phạm vi",
        message_en: "duplicate declaration in the same scope",
        message_ja: "同じスコープで重複した宣言です",
        severity: Severity::Error,
    },
    // **R9 (VD-178)**: tách "kiểu không tồn tại" khỏi `VS0301` (tên giá trị).
    Code {
        id: 303,
        short: "VS0303",
        message_vi: "kiểu không tồn tại",
        message_en: "type does not exist",
        message_ja: "型が存在しません",
        severity: Severity::Error,
    },
    // **ST-315d (VD-291 §3.4)**: chu trình phụ thuộc `const` bắt **tĩnh** ở `vis-types`.
    Code {
        id: 304,
        short: "VS0304",
        message_vi: "chu trình phụ thuộc hằng số (`const`)",
        message_en: "constant (`const`) dependency cycle",
        message_ja: "定数（`const`）の依存関係が循環しています",
        severity: Severity::Error,
    },
];

/// Sổ **mã kiểu** `04xx` (VD-42, ST-12) — sổ thứ tư. `VS0412` giữ đúng nghĩa "kiểu không khớp"
/// của thiết kế §9; `VS0401` sai số/thứ tự đối số; `VS0402` `match` không xét hết; `VS0403`
/// trường/kiểu không tồn tại.
pub const TYPE_CODES: [Code; 9] = [
    Code {
        id: 401,
        short: "VS0401",
        message_vi: "sai số lượng hoặc thứ tự đối số",
        message_en: "wrong number or order of arguments",
        message_ja: "引数の数または順序が正しくありません",
        severity: Severity::Error,
    },
    Code {
        id: 402,
        short: "VS0402",
        message_vi: "`match` không xét hết trường hợp",
        message_en: "`match` is not exhaustive",
        message_ja: "`match` が網羅的ではありません",
        severity: Severity::Error,
    },
    Code {
        id: 403,
        short: "VS0403",
        // **SỬA KÈM LÝ DO (ST-29)**: mở rộng nghĩa sang **phương thức** (trait/impl) — Agent 0 chốt
        // giữ mã, không cấp `VS0404`; mỗi ca có thông điệp riêng nêu cụ thể.
        message_vi: "trường, kiểu hoặc phương thức không tồn tại",
        message_en: "field, type or method does not exist",
        message_ja: "フィールド・型・メソッドが存在しません",
        severity: Severity::Error,
    },
    Code {
        // **ST-50 (Agent 0)**: trường không có **là lỗi KIỂU** ⇒ về sổ `04xx`, **đổi tên**
        // `VS0605` → `VS0405` (không thêm mã); câu gốc **trung tính** cho cả tyck + hạ.
        id: 405,
        short: "VS0405",
        message_vi: "không có trường này trên kiểu bản ghi (trường không tồn tại)",
        message_en: "no such field on this record type (field does not exist)",
        message_ja: "このレコード型にそのフィールドはありません（フィールドが存在しません）",
        severity: Severity::Error,
    },
    Code {
        id: 412,
        short: "VS0412",
        message_vi: "kiểu không khớp",
        message_en: "type mismatch",
        message_ja: "型が一致しません",
        severity: Severity::Error,
    },
    // **R9 (VD-178)**: tách 4 nghĩa khỏi `VS0412` — mỗi mã một nghĩa.
    Code {
        id: 413,
        short: "VS0413",
        message_vi: "gán vào ràng buộc bất biến (`let`) — dùng `var` nếu cần đổi",
        message_en: "assignment to an immutable binding (`let`) — use `var` to mutate",
        message_ja: "不変束縛 (`let`) への代入です — 変更するには `var` を使ってください",
        severity: Severity::Error,
    },
    Code {
        id: 414,
        short: "VS0414",
        message_vi: "toán tử không hỗ trợ cho kiểu này",
        message_en: "operator not supported for this type",
        message_ja: "この型では演算子がサポートされていません",
        severity: Severity::Error,
    },
    Code {
        id: 415,
        short: "VS0415",
        message_vi: "`async` sai chỗ — chỉ dùng cho hàm cấp tệp (`async fn`)",
        message_en: "`async` in the wrong place — only for top-level functions (`async fn`)",
        message_ja: "`async` の位置が不正です — トップレベル関数 (`async fn`) のみ",
        severity: Severity::Error,
    },
    Code {
        id: 416,
        short: "VS0416",
        message_vi: "range chỉ nhận `int`",
        message_en: "range only accepts `int`",
        message_ja: "range は `int` のみ受け付けます",
        severity: Severity::Error,
    },
];

/// Mã **`VS0405`** — "không có trường này trên kiểu bản ghi" (ST-50: trường là lỗi **kiểu**).
/// Dùng hàm thay chỉ số mảng để **đổi thứ tự/khối** không làm lệch mã.
pub fn ma_truong() -> &'static Code {
    TYPE_CODES
        .iter()
        .find(|c| c.id == 405)
        .expect("VS0405 phải có trong TYPE_CODES")
}

/// Sổ **mã runtime** `05xx` (VD-42, ST-13) — sổ thứ năm: `VS0501` lỗi runtime chung (tràn `i64`,
/// phép chưa hỗ trợ ở VS0), `VS0502` chia/lấy dư cho 0, `VS0503` chỉ số ngoài phạm vi.
/// Khối `06xx` — **chế độ lệnh** (ST-18, VD-52). Bốn mã, đúng phạm vi VS0:
/// lệnh không có trong bảng chữ ký, sai đối số, pipeline không nối được, tính năng chưa hỗ trợ.
pub const COMMAND_CODES: [Code; 6] = [
    Code {
        id: 601,
        short: "VS0601",
        message_vi: "không có lệnh này: tên không có trong bảng chữ ký (cmd/builtin/extern)",
        message_en: "no such command: name not found in the signature table (cmd/builtin/extern)",
        message_ja: "そのコマンドはありません: シグネチャ表（cmd/builtin/extern）にありません",
        severity: Severity::Error,
    },
    Code {
        id: 602,
        short: "VS0602",
        message_vi: "sai đối số lệnh: thiếu, thừa hoặc sai kiểu",
        message_en: "bad command argument: missing, extra or wrong type",
        message_ja: "コマンド引数が不正です: 不足・過剰・型違い",
        severity: Severity::Error,
    },
    Code {
        id: 603,
        short: "VS0603",
        message_vi: "pipeline không nối được: kiểu ra của stage trước ≠ kiểu vào của stage sau",
        message_en: "pipeline not connectable: previous stage output type ≠ next stage input type",
        message_ja: "パイプが接続できません: 前段の出力型と次段の入力型が異なります",
        severity: Severity::Error,
    },
    Code {
        id: 604,
        short: "VS0604",
        message_vi: "tính năng chưa hỗ trợ (ví dụ chạy nền `&`)",
        message_en: "feature not supported (e.g. background `&`)",
        message_ja: "未対応の機能です（例: バックグラウンド `&`）",
        severity: Severity::Error,
    },
    // **R9 (VD-178)**: tách 2 nghĩa khỏi `VS0604` — mỗi mã một nghĩa.
    Code {
        id: 605,
        short: "VS0605",
        message_vi: "chạy nền `&` chưa hỗ trợ",
        message_en: "background `&` not supported",
        message_ja: "バックグラウンド `&` は未対応です",
        severity: Severity::Error,
    },
    Code {
        id: 606,
        short: "VS0606",
        message_vi: "chương trình ngoài (`^…`) chưa hỗ trợ",
        message_en: "external program (`^…`) not supported",
        message_ja: "外部プログラム (`^…`) は未対応です",
        severity: Severity::Error,
    },
];

pub const RUNTIME_CODES: [Code; 10] = [
    Code {
        id: 501,
        short: "VS0501",
        message_vi: "lỗi runtime: phép toán hoặc tính năng chưa hỗ trợ",
        message_en: "runtime error: operation or feature not supported",
        message_ja: "実行時エラー: 未対応の演算または機能です",
        severity: Severity::Error,
    },
    Code {
        id: 502,
        short: "VS0502",
        message_vi: "chia hoặc lấy dư cho 0",
        message_en: "division or remainder by zero",
        message_ja: "0 による除算または剰余",
        severity: Severity::Error,
    },
    Code {
        id: 503,
        short: "VS0503",
        message_vi: "chỉ số ngoài phạm vi",
        message_en: "index out of range",
        message_ja: "インデックスが範囲外です",
        severity: Severity::Error,
    },
    // **ST-61 (VD-96)**: kênh — **gửi sau `close`**. Thứ tự **tăng dần**
    // theo `id` (bất biến `so_runtime_05xx_khoa_theo_vd42`) ⇒ `VS0506` dịch index 3 → 5.
    Code {
        id: 504,
        short: "VS0504",
        message_vi: "gửi vào kênh đã `close` (không nhận thêm phần tử)",
        message_en: "send on a closed channel (no more elements accepted)",
        message_ja: "閉じたチャネルへの送信（これ以上受け付けません）",
        severity: Severity::Error,
    },
    // **ST-61 (VD-96)**: **deadlock** — kênh chặn (đầy/rỗng) mà **không còn** tác vụ nào chạy
    // được ⇒ lỗi có chẩn đoán, **không** treo chương trình.
    Code {
        id: 505,
        short: "VS0505",
        message_vi: "deadlock: kênh bị chặn và không còn tác vụ nào chạy được (khoá)",
        message_en: "deadlock: the channel is blocked and no task is runnable",
        message_ja: "デッドロック: チャネルがブロックされ、実行可能なタスクがありません",
        severity: Severity::Error,
    },
    // **ST-60 (VD-90/VD-95)**: lỗi trong **thân tác vụ** (`async fn`) — bọc **mọi** lỗi, message
    // kèm **mã gốc**, `span` = lúc **tạo** task; một quy tắc cho cả `await` lẫn join cuối.
    Code {
        id: 506,
        short: "VS0506",
        message_vi: "tác vụ (`async fn`) lỗi: lỗi phát sinh trong thân tác vụ (kèm mã gốc)",
        message_en: "task (`async fn`) failed: an error occurred inside the task body (original code included)",
        message_ja: "タスク (`async fn`) でエラー: タスク本体でエラーが発生しました（元のコード付き）",
        severity: Severity::Error,
    },
    // **R9 (VD-178)**: tách 2 nghĩa khỏi `VS0501` — mỗi mã một nghĩa.
    Code {
        id: 507,
        short: "VS0507",
        message_vi: "tràn `i64` lúc chạy",
        message_en: "`i64` overflow at runtime",
        message_ja: "実行時の `i64` オーバーフロー",
        severity: Severity::Error,
    },
    Code {
        id: 508,
        short: "VS0508",
        message_vi: "giá trị không gửi được sang tác vụ khác (không `Sendable`)",
        message_en: "value cannot be sent to another task (not `Sendable`)",
        message_ja: "他のタスクに送信できない値です（`Sendable` ではありません）",
        severity: Severity::Error,
    },
    // **ST-290 (VD-273)** — hai mã cấp đích danh, cùng lượt để W1 (`ST-292`) dùng `VS0509`
    // ngay sau merge. `VS0509` tách khỏi `VS0508` (không Sendable) và khỏi mọi lỗi task;
    // `VS0510` tách hẳn ngân sách ngữ nghĩa khỏi `VS0501` (trap nội bộ / fuel VM là hàng rào
    // an toàn riêng, không phải ngân sách).
    Code {
        id: 509,
        short: "VS0509",
        message_vi: "ghi biến cấp tệp chéo worker ở chế độ song song (W > 1)",
        message_en: "cross-worker write to a file-level variable in parallel mode (W > 1)",
        message_ja: "並列モード (W > 1) でのワーカー間のファイルレベル変数への書き込み",
        severity: Severity::Error,
    },
    Code {
        id: 510,
        short: "VS0510",
        message_vi: "chương trình vượt ngân sách thực thi ngữ nghĩa",
        message_en: "program exceeded the semantic execution budget",
        message_ja: "プログラムが意味実行予算を超過しました",
        severity: Severity::Error,
    },
];

/// **Sổ thứ bảy** — `07xx` module (ST-25, VD-57): `use` không thấy tệp, vòng phụ thuộc, `use` trùng.
///
/// `07xx` được cấp ở **ST-25** cho khung module; `std.*` vẫn **hoãn** (ST-26) và `pkg.*` trước ST-71
/// hoãn — cả hai báo `VS0701` với thông điệp riêng (VD-57).
///
/// **ST-128 (VD-125)**: cấp **`VS0704`** trong **cùng khối `07xx`** — "không giải được ràng buộc gói";
/// thông điệp **kèm chuỗi ràng buộc** gây xung đột. Chọn số **kế tiếp của khối** (`704`) theo VD-125;
/// **không** dùng `next_block()` (nó trả `801` vì suy từ khối đã dành trước — xem JOURNAL ST-128 §7).
///
/// **ST-288 (VD-273)**: cấp **`VS0708`** — lỗi xác minh **chữ ký số / khoá tin cậy** registry gói
/// (`VS3-5b/5c`); tách hẳn khỏi `VS0707` (hash nội dung lệch).
pub const MODULE_CODES: [Code; 8] = [
    Code {
        id: 701,
        short: "VS0701",
        message_vi: "không thấy tệp module",
        message_en: "module file not found",
        message_ja: "モジュールファイルが見つかりません",
        severity: Severity::Error,
    },
    Code {
        id: 702,
        short: "VS0702",
        message_vi: "vòng phụ thuộc giữa các tệp module",
        message_en: "dependency cycle between module files",
        message_ja: "モジュールファイル間の循環依存",
        severity: Severity::Error,
    },
    Code {
        id: 703,
        short: "VS0703",
        message_vi: "`use` trùng hoặc mơ hồ (hai đường dẫn cùng một module)",
        message_en: "duplicate or ambiguous `use` (two paths to the same module)",
        message_ja: "重複または曖昧な `use` (同じモジュールへの2つの経路)",
        severity: Severity::Error,
    },
    Code {
        id: 704,
        short: "VS0704",
        message_vi: "không giải được ràng buộc gói: không phiên bản nào thoả đồng thời các ràng buộc (kèm chuỗi ràng buộc)",
        message_en: "package dependency resolution failed: no version satisfies all constraints (constraint chain included)",
        message_ja: "パッケージの依存解決に失敗しました: すべての制約を同時に満たすバージョンがありません（制約の連鎖付き）",
        severity: Severity::Error,
    },
    // **R9 (VD-178)**: tách 3 nghĩa khỏi `VS0701` — mỗi mã một nghĩa.
    Code {
        id: 705,
        short: "VS0705",
        message_vi: "`std` chưa hỗ trợ",
        message_en: "`std` not supported",
        message_ja: "`std` は未対応です",
        severity: Severity::Error,
    },
    Code {
        id: 706,
        short: "VS0706",
        message_vi: "`pkg` chưa hỗ trợ",
        message_en: "`pkg` not supported",
        message_ja: "`pkg` は未対応です",
        severity: Severity::Error,
    },
    Code {
        id: 707,
        short: "VS0707",
        message_vi: "hash gói không khớp `vis.lock`/sổ",
        message_en: "package hash does not match `vis.lock`/sumdb",
        message_ja: "パッケージのハッシュが `vis.lock`/sumdb と一致しません",
        severity: Severity::Error,
    },
    // **ST-288 (VD-273)**: cấp `VS0708` cho lỗi xác minh chữ ký/khoá tin cậy của registry gói
    // (`VS3-5b/5c`). Tách hẳn khỏi `VS0707` (hash nội dung) — chữ ký hỏng là lỗi **niềm tin**.
    Code {
        id: 708,
        short: "VS0708",
        message_vi: "lỗi xác minh chữ ký số hoặc khoá tin cậy của registry gói",
        message_en: "package registry signature or trusted key verification failed",
        message_ja: "パッケージレジストリの署名または信頼キーの検証に失敗しました",
        severity: Severity::Error,
    },
];

/// **Năm sổ** mã lỗi đã cấp ở ST-13: từ vựng `01xx`, cú pháp `02xx`, tên `03xx`, kiểu `04xx`,
/// runtime `05xx` (VD-27/VD-35/VD-42). Nay **mọi** khối dành trước đều đã có mã; `06xx` là khối
/// kế tiếp chưa cấp nên [`next_block`] vẫn trả **601**.
pub const BOOKS: [&[Code]; 7] = [
    &CODES,
    &SYNTAX_CODES,
    &NAME_CODES,
    &TYPE_CODES,
    &RUNTIME_CODES,
    &COMMAND_CODES,
    &MODULE_CODES,
];

/// Khối mã **đã dành trước** theo sơ đồ VD-27 (`01xx`..`06xx`); VD-42 cấp `04xx` ở ST-12, `05xx` ở
/// ST-13, ST-18 cấp `06xx`. Dùng cho [`next_block`] để khối kế không rơi vào khối đã dành.
pub const BLOCKS_RESERVED: [u16; 7] = [1, 2, 3, 4, 5, 6, 7];

/// Duyệt **cả bốn sổ** đã cấp, theo đúng thứ tự khối.
pub fn all_codes() -> impl Iterator<Item = &'static Code> {
    BOOKS.iter().flat_map(|b| b.iter())
}
