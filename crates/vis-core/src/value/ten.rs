// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267/ST-317a** — `Name` nội bộ hoá tên enum/biến thể + bảng tên thread-local, tag
//! `Option`/`Result` và singleton `None`/map rỗng. Tách khỏi `mod.rs` ở `ST-317b` (thuần move,
//! 0 đổi ngữ nghĩa; `size_of::<Value>() == 32` giữ nguyên).

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use super::{EnumArgs, Value, VisMap};

/// **VD-267** — FNV-1a cho bảng tên. Tên enum/biến thể rất ngắn; SipHash mặc định đắt gấp
/// nhiều lần trong vòng lặp nóng. Không cần kháng HashDoS: số tên bị chặn bởi kích thước mã
/// nguồn, không phải dữ liệu người dùng.
pub(crate) struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = h;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type NameSet = HashSet<Rc<str>, BuildHasherDefault<Fnv>>;

thread_local! {
    /// **VD-267** — bảng tên enum/biến thể **thread-local** (mỗi tên đúng **một** `Rc<str>`).
    ///
    /// Tên enum/biến thể đến từ mã nguồn nên số tên hữu hạn; bảng chỉ giữ một bản mỗi tên.
    /// `Value` vốn đã chứa `Rc` (không `Send`) nên thread-local là đủ, không cần đồng bộ.
    static ENUM_NAMES: RefCell<NameSet> = RefCell::new(HashSet::default());
}

/// **VD-267** — tên enum/biến thể **chia sẻ**.
///
/// Trước đây `Value::Enum(String, String, _)` cấp phát hai `String` cho **mỗi** giá trị enum:
/// vòng lặp dựng `Some(i)`/`E.A(i)` trả hai `malloc` + hai `free` mỗi vòng (chưa kể `Vec` đối
/// số). `Name` giữ `Rc<str>` lấy từ bảng tên thread-local: lần đầu cấp phát, các lần sau chỉ
/// tăng refcount (không `malloc`). So sánh vẫn **theo nội dung** như `String` cũ, nên `==`,
/// pattern và `show` không đổi ngữ nghĩa; tên nội bộ hoá còn cho phép `eq` đi tắt bằng
/// `Rc::ptr_eq`.
#[derive(Clone)]
pub struct Name(Rc<str>);

/// **VD-267** — cache trực tiếp (direct-mapped) cho [`Name::new`]: chỗ dựng tên từ **cùng một
/// chuỗi** (literal, HIR, hằng) chỉ cần so con trỏ + độ dài + nội dung, không tra bảng băm.
/// 64 ô; trượt chỉ tốn thêm một phép kiểm — nội dung **luôn** được xác minh nên địa chỉ bộ đệm
/// bị tái dùng không thể trả sai tên.
const NAME_CACHE_LEN: usize = 64;

#[derive(Clone)]
struct CachedName {
    ptr: usize,
    len: usize,
    name: Name,
}

thread_local! {
    static NAME_CACHE: RefCell<Vec<Option<CachedName>>> =
        RefCell::new(vec![None; NAME_CACHE_LEN]);
}

impl Name {
    /// Tên đã có trong bảng ⇒ chia sẻ bản cũ; chưa có ⇒ cấp phát **một lần**.
    pub fn new(s: &str) -> Self {
        let ptr = s.as_ptr() as usize;
        let len = s.len();
        let idx = (ptr.rotate_left(3) ^ len.wrapping_mul(0x9e37_79b9)) & (NAME_CACHE_LEN - 1);

        // Đường nhanh: cùng con trỏ + độ dài + **nội dung** (bộ đệm có thể bị tái dùng địa chỉ).
        let hit = NAME_CACHE.with(|c| {
            let c = c.borrow();
            c.get(idx).and_then(|slot| {
                slot.as_ref()
                    .filter(|e| e.ptr == ptr && e.len == len && e.name.as_str() == s)
                    .map(|e| e.name.clone())
            })
        });
        if let Some(n) = hit {
            return n;
        }

        let name = ENUM_NAMES.with(|names| {
            let mut names = names.borrow_mut();
            if let Some(n) = names.get(s) {
                return Name(Rc::clone(n));
            }
            let n: Rc<str> = Rc::from(s);
            names.insert(Rc::clone(&n));
            Name(n)
        });
        NAME_CACHE.with(|c| {
            if let Some(slot) = c.borrow_mut().get_mut(idx) {
                *slot = Some(CachedName {
                    ptr,
                    len,
                    name: name.clone(),
                });
            }
        });
        name
    }

    /// Tên dạng `&str` (tương thích chỗ cũ gọi `String::as_str`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `Debug` giữ **đúng** dạng cũ của `String` (`"Some"`) — không lộ wrapper `Name`.
impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

/// Mọi literal `.into()` cũ (`"Option".into()`) tự nội bộ hoá, không cần sửa từng chỗ lạnh.
impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<&String> for Name {
    fn from(s: &String) -> Self {
        Self::new(s)
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        // Tên nội bộ hoá ⇒ cùng nội dung luôn cùng `Rc`; `ptr_eq` là đường nhanh.
        Rc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl Eq for Name {}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().hash(state);
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl PartialEq<&String> for Name {
    fn eq(&self, other: &&String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        self == &*other.0
    }
}

impl PartialEq<Name> for String {
    fn eq(&self, other: &Name) -> bool {
        self.as_str() == &*other.0
    }
}

thread_local! {
    /// Tên `Option`/`Result` dựng sẵn, giữ sẵn để `some`/`none`/`ok`/`err` không phải tra bảng.
    pub(super) static TAGS: Tags = Tags {
        option: Name::new("Option"),
        some: Name::new("Some"),
        none: Name::new("None"),
        result: Name::new("Result"),
        ok: Name::new("Ok"),
        err: Name::new("Err"),
    };
}

pub(super) struct Tags {
    pub(super) option: Name,
    pub(super) some: Name,
    pub(super) none: Name,
    pub(super) result: Name,
    pub(super) ok: Name,
    pub(super) err: Name,
}

thread_local! {
    /// **ST-317a** — singleton `None` theo thread: `Value::none()` chỉ tăng refcount.
    pub(super) static NONE_VALUE: Value = TAGS.with(|t| {
        Value::enum_tu(t.option.clone(), t.none.clone(), EnumArgs::Zero)
    });

    /// **ST-317a** (đóng F6 `ST-315b`) — singleton map rỗng: `{}` không cấp phát.
    ///
    /// `Value` chứa `Rc` nên không `Send`; mỗi thread có bản riêng. Mọi đường ghi dùng
    /// `Rc::make_mut`; singleton luôn giữ một refcount nên `strong_count` không bao giờ bằng 1
    /// ⇒ không đường nào sửa được bản dùng chung này tại chỗ.
    pub(super) static EMPTY_MAP: Rc<VisMap> = Rc::new(VisMap::new());
}

#[cfg(test)]
mod tests_name {
    use super::{Name, Value};
    use std::rc::Rc;

    /// **VD-267**: cùng nội dung ⇒ **cùng** `Rc<str>` (chỉ tăng refcount), `Debug`/`Display`
    /// vẫn như `String` cũ để đầu ra không đổi.
    #[test]
    fn name_chia_se_theo_noi_dung() {
        let a = Name::new("Option");
        let b = Name::from(String::from("Opt") + "ion");
        let c = Name::new("Option");
        assert!(Rc::ptr_eq(&a.0, &b.0), "phải dùng chung một Rc");
        assert!(
            Rc::ptr_eq(&a.0, &c.0),
            "đường nhanh theo con trỏ cũng chia sẻ"
        );
        assert_eq!(a, b);
        assert_eq!(a.as_str(), "Option");
        assert_eq!(format!("{a}"), "Option");
        assert_eq!(format!("{a:?}"), "\"Option\"");
        assert_eq!(a, "Option");
        assert_ne!(a, "Result");
        // Hai giá trị enum dựng độc lập vẫn bằng nhau + show y hệt trước đây.
        let v1 = Value::enum_val(Name::new("E"), Name::new("A"), vec![Value::Int(1)]);
        let v2 = Value::enum_val(Name::new("E"), Name::new("A"), vec![Value::Int(1)]);
        assert!(v1.eq_value(&v2));
        assert_eq!(v1.show(), "A(1)");
        assert_eq!(
            format!("{:?}", Value::some(Value::Int(1))),
            "Enum(VisEnum { enum_name: \"Option\", variant: \"Some\", args: One(Int(1)) })"
        );
    }
}
