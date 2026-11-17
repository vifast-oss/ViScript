<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Góp ý & phát triển ViScript

Cảm ơn bạn muốn đóng góp! Dưới đây là những việc tối thiểu để thay đổi được nhận.

## 1. Cài đặt

```sh
git clone https://github.com/vifast-oss/ViScript.git
cd ViScript
cargo build --release
./vis run vi-du/xin-chao.vis
```

Cần: Rust (stable), `python3` (cho bộ kiểm báo cáo), `git`.

## 2. Trước khi gửi thay đổi

Chạy **cổng bắt buộc** (khoảng 4–5 phút trên máy thường):

```sh
./vis-dev gate --fast
```

Cổng kiểm: định dạng (`cargo fmt`), clippy `-D warnings`, luật cấu trúc R1–R4, ca âm tự kiểm,
`vis explain`, fixture báo cáo, `doc-examples`, build, và **toàn bộ test** (`cargo test --workspace`).

Chỉ gửi khi cổng **PASS**. Không sửa kỳ vọng/ngưỡng của test để lấy xanh.

## 3. Quy ước

- **Commit**: `[ST-##] <tóm tắt tiếng Việt>` (ví dụ `[ST-174] vá #146: symlink_metadata`).
- **Không** `unsafe`, **không** thêm crate ngoài allowlist giấy phép, **không** `#[ignore]`/`#[should_panic]`.
- Mỗi tệp có header SPDX: `//` cho Rust, `#` cho shell/`.vis`/`.toml`, `<!-- -->` cho Markdown.
- Tệp ≤ 1500 dòng (luật R1).
- Thay đổi hành vi ngôn ngữ phải có **ca đỏ-trước** (test viết trước, đỏ, rồi mới sửa).

## 4. Quyết định thiết kế

Thay đổi lớn (cú pháp, kiểu, API chuẩn) cần một mục `VD-##` trong [`agent/DECISIONS.md`](agent/DECISIONS.md)
và task tương ứng trong [`agent/QUEUE.md`](agent/QUEUE.md). Đọc [`AGENTS.md`](AGENTS.md) trước khi sửa mã.

## 5. Báo lỗi

Mở issue kèm: mã lỗi (`VSxxxx`), tệp `.vis` tối thiểu tái hiện, kết quả mong đợi, kết quả thực tế,
và đầu ra `./vis explain <mã>`.
