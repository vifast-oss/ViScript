<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-174) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-174 · Nguyên tắc mô hình ViScript (bổ sung R14)
Ghi vào thiết kế như nguyên tắc nền. ST-172b soạn đặc tả + ca đỏ-trước cho mục 4-6.

1. Không có class, không có kế thừa. Trừu tượng hoá chỉ qua type + trait + impl; dùng lại code bằng method mặc định trong trait hoặc record chứa record.
2. Không có lifetime, con trỏ, tham chiếu, unsafe, macro người dùng (trước 1.0). Sinh code chỉ qua derives có sẵn.
3. Không có tham chiếu dùng chung có thể sửa (kiểu Rc<RefCell>) ở 1.0. Trạng thái dùng chung trong một luồng: map + ID; giữa các luồng: Chan. Vì truyền theo giá trị nên không có vòng tham chiếu, đếm tham chiếu là đủ.
4. Trait dùng được làm kiểu: list[Shape] chứa mọi kiểu cài Shape; ViScript tự đóng gói, không có cú pháp Box/dyn. Đặc tả: trait nào được dùng làm kiểu (luật an toàn đối tượng), gọi method qua bảng ảo, ca đỏ-trước với 2 kiểu cài cùng trait trong 1 list.
5. Sửa trường lồng sâu trên var: "u.addr.city = x" và "xs[i].name = x" hợp lệ, tự sao chép đúng đường dẫn. Probe hiện trạng.
6. Chuỗi method trên list/map/range là lười (iterator); tạo collection khi gọi .to_list()/.to_map()/.to_set() hoặc khi dùng trong for. Đặc tả kiểu trả về của filter/map/take/enumerate/zip. Chốt trước R20.
Mục 4-6 đứng cùng R14 ở đầu pha 2; mục 1-3 chỉ sửa tài liệu.
