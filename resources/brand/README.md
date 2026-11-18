<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ViScript — bộ nhận diện

Icon vector cho hai sản phẩm cùng dòng: **ViScript** (ngôn ngữ, lệnh `vis`, tệp `.vis`) và
**ViShell** (shell, lệnh `vish`, tệp `.vish`).

Logo, wordmark và các tệp trong thư mục này **không** thuộc phạm vi giấy phép MIT/Apache-2.0 —
xem [`../../TRADEMARKS.md`](../../TRADEMARKS.md).

## Ý nghĩa hình

Cả hai icon dùng chung nền đỏ + ngôi sao vàng của thương hiệu mẹ ViFast, để đứng cạnh nhau trên
`vifast.org` và trang tổ chức GitHub vẫn thành một họ.

| Icon | Hình | Nghĩa |
|---|---|---|
| **ViScript** | Hai ngoặc vuông trắng `[ ]` | khối mã / cú pháp — ngôn ngữ lập trình |
| | Ngôi sao vàng ở giữa | tâm điểm, cùng vị trí với icon ViFast |
| **ViShell** | Dấu nhắc `>` bên trái | dấu nhắc lệnh của shell |
| | Ngôi sao vàng **cùng vị trí** với icon ViScript | hai sản phẩm là anh em, không phải hai thương hiệu rời |
| | Con trỏ `_` nhấp nháy bên phải | con trỏ chờ gõ lệnh (chỉ có trong SVG) |

Không dùng chữ cái trong icon — dấu hiệu nhận biết nằm ở hình (ngoặc / dấu nhắc + sao).

## Tệp

| Tệp | Dùng cho |
|---|---|
| `viscript-icon.svg` | nguồn vector — favicon, app icon, avatar, README |
| `vishell-icon.svg` | nguồn vector (có hiệu ứng con trỏ nhấp nháy) |
| `viscript-icon-{16,32,180,192,512}.png` | favicon PNG, apple-touch-icon, PWA, avatar GitHub |
| `vishell-icon-{16,32,180,192,512}.png` | như trên |

PNG sinh từ SVG bằng headless Chrome ở 1024 px rồi thu nhỏ Lanczos, nền trong suốt (alpha 0 ở
bốn góc bo). Muốn sinh lại:

```sh
# cần Google Chrome + ImageMagick
for n in viscript-icon vishell-icon; do
  for s in 16 32 180 192 512; do
    magick "$n-1024.png" -filter Lanczos -resize "${s}x${s}" -strip "$n-$s.png"
  done
done
```

## Bảng màu

Dùng đúng bảng màu của thương hiệu mẹ, không thêm màu mới.

| Vai trò | Mã |
|---|---|
| Đỏ chính | `#E8352B` → `#B3150F` (gradient chéo) |
| Vàng sao | `#FFE04A` → `#FFB800` (gradient dọc) |
| Viền sao | `#FFE680` |
| Nét trắng | `#FFFFFF` |
| Bo góc nền | `rx="112"` trên khung 512 |

## Dùng ở đâu

| Nơi | Tệp |
|---|---|
| `README.md` gốc repo này | `viscript-icon.svg` |
| `vifast.org` (`ViFastSite/brand/`) | `*-icon-192.png` + `*-icon.svg` |
| Trang tổ chức GitHub (`ViFastOrg/profile/assets/`) | `*-icon-512.png` |
| Avatar repo GitHub | `*-icon-512.png` |
