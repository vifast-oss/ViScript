<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ViScript

**ViScript là một ngôn ngữ lập trình dễ học, viết cho người Việt.** Câu lệnh đọc gần như tiếng Việt, thông báo lỗi cũng bằng tiếng Việt, và những thứ dễ gây lỗi trong các ngôn ngữ khác (con trỏ, `unsafe`, kế thừa lớp) thì ViScript **không có**.

> Mục tiêu: học sinh cấp 3 đọc 10 phút là viết được chương trình đầu tiên, và người mới vẫn được nhắc lỗi rõ ràng thay vì những dòng khó hiểu.

## Chạy thử trong 2 phút

```sh
git clone https://github.com/vifast-oss/ViScript.git
cd ViScript
cargo build --release
./vis run vi-du/xin-chao.vis
```

Kết quả:

```
Xin chào, ViScript!
```

## Viết chương trình đầu tiên

Tạo tệp `xin-chao.vis`:

```vis
print("Xin chào, ViScript!")
```

Chạy:

```sh
./vis run xin-chao.vis
```

Không cần `;`, không cần ngoặc nhọn — **thụt lề chính là khối lệnh**, giống cách bạn viết bài.

## Vài ví dụ nhỏ

**Biến và nội suy chuỗi** (đặt giá trị vào chuỗi bằng `{...}`):

```vis
let ten = "bạn"
print("Xin chào, {ten}!")
```

**Hàm** — khai báo kiểu rõ ràng, thân hàm thụt lề:

```vis
fn chao(ten: str) -> str:
    "Chào {ten}, hẹn gặp lại!"

print(chao("Minh"))
```

**Đơn vị đo là một phần của kiểu** — không còn cảnh “2024 byte hay 2024 kilobyte?”:

```vis
let dung_luong = 1kb + 1kib
print(dung_luong)      # 2024b
```

**Danh sách và map** — viết như bạn nghĩ:

```vis
let ds = [1, 2, 3, 4]
print(ds.map(x => x * 2))       # [2, 4, 6, 8]
print(ds.filter(x => x > 2))    # [3, 4]
```

**Từ điển (map)**:

```vis
let tuoi = {"Minh": 17, "Lan": 16}
print(tuoi["Minh"])             # 17
```

## Khi viết sai thì sao?

ViScript báo lỗi bằng tiếng Việt, kèm **mã lỗi** để tra cứu:

```
bai.vis:2:5: error VS0412: toán tử `+`: `str` và `str` không cùng kiểu/đơn vị
```

Gõ `./vis explain VS0412` để xem giải thích và cách sửa.

## Các lệnh hay dùng

| Lệnh | Việc |
|---|---|
| `./vis run <tệp.vis>` | chạy chương trình |
| `./vis check <tệp…>` | kiểm tra lỗi mà không chạy |
| `./vis fmt --write <tệp…>` | tự căn lại thụt lề, khoảng trắng |
| `./vis explain <mã>` | giải thích một mã lỗi (vi/en/ja) |
| `./vis lsp --stdio` | bật gợi ý cho trình soạn thảo (VS Code…) |
| `./vis pkg add <gói>` | thêm gói (đang phát triển) |

## Vì sao gọi là “tối ưu công thái học”?

“Công thái học” ở đây nghĩa là **thiết kế cho người viết dễ chịu và ít lỗi**, không phải chạy nhanh hơn một chút. Một số điểm đã có, một số đang làm:

| Điểm | Trạng thái |
|---|---|
| Thụt lề thay ngoặc nhọn, câu lệnh ngắn | **đã có** |
| Thông báo lỗi tiếng Việt + mã tra cứu (`vis explain`) | **đã có** |
| Nội suy chuỗi `{expr}` | **đã có** |
| Đơn vị đo trong kiểu (`1kb + 1kib = 2024b`) | **đã có** |
| `if let` / `let … else` cho giá trị có thể thiếu | **đã có** |
| Đường dẫn viết thô `p"C:\thu\muc"` (không phải escape) | **đã có** |
| Tự căn định dạng (`vis fmt`), gợi ý trong editor (LSP) | **đã có** |
| Không con trỏ, không `unsafe`, không kế thừa lớp | **đã có** |
| `str + str` cho phép cộng chuỗi | **đang làm** (pha 1) |
| `let` che biến cùng phạm vi (không báo lỗi) | **đang làm** (pha 1) |
| `%` luôn trả số không âm (`-7 % 2 == 1`) | **đang làm** (pha 1) |
| Chia hai đơn vị cùng loại ra số, chia số ra đơn vị | **đang làm** (pha 1) |
| Iterator lười `.lazy()` cho danh sách lớn | **dự kiến** (pha 2) |
| Gán lồng sâu `u.addr.city = "A"` | **dự kiến** (pha 2) |

## Cấu trúc repo

```
crates/    8 crate Rust: từ vựng, kiểu, thông dịch, máy ảo, CLI…
bin/       công cụ: vis, vis-dev, vis-fmt, vis-remote…
test/      bộ kiểm: ví dụ chạy được, fixture, báo cáo
docs/      đặc tả ngôn ngữ (SPEC-lex.md) và thiết kế
agent/     sổ quyết định, QUEUE, nhật ký, báo cáo từng đợt
```

## Góp ý / phát triển

- Mọi thay đổi phải qua cổng: `./vis-dev gate --fast` (định dạng, clippy, luật cấu trúc, toàn bộ test).
- Quy ước commit: `[ST-##] <tóm tắt>`; quyết định thiết kế ghi trong `agent/DECISIONS.md`.
- Đọc `AGENTS.md` trước khi sửa mã.

## Giấy phép

MIT OR Apache-2.0 — dùng, sửa, chia sẻ tự do.
