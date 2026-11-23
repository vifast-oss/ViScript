// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-284/ST-302 (#8)** — runtime cho `derives Json`: chuyển `Value` ↔ cây `Json` theo
//! **descriptor** do lower sinh (một chuỗi mô tả kiểu, không cần `TypeTable` lúc chạy).
//!
//! Descriptor: `i` int, `f` float, `s` str, `b` bool, `y` bytes, `h` path, `c` char,
//! `u` unit, `z` size, `d` duration, `p` percent, `j` Json, `O…` Option, `L…` list,
//! `M… …` map, `R… …` Result, `Tên{trường:desc;…}` record, `E Tên{biến_thể:desc;…}` enum
//! (unit dùng `_`). Chỉ dùng nội bộ cho method sinh; người dùng không thấy descriptor.

use crate::value::Value;

fn json_null() -> Value {
    Value::Enum("Json".into(), "Null".into(), Vec::new())
}

fn json_num_int(n: i64) -> Value {
    Value::Enum("Json".into(), "Num".into(), vec![Value::Int(n)])
}

fn json_num_float(f: f64) -> Value {
    Value::Enum("Json".into(), "Num".into(), vec![Value::Float(f)])
}

fn json_str(s: String) -> Value {
    Value::Enum("Json".into(), "Str".into(), vec![Value::Str(s)])
}

fn json_bool(b: bool) -> Value {
    Value::Enum("Json".into(), "Bool".into(), vec![Value::Bool(b)])
}

fn json_arr(xs: Vec<Value>) -> Value {
    Value::Enum("Json".into(), "Arr".into(), vec![Value::List(xs)])
}

fn json_obj(kv: Vec<(Value, Value)>) -> Value {
    Value::Enum("Json".into(), "Obj".into(), vec![Value::Map(kv)])
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s: s.as_bytes(),
            i: 0,
        }
    }

    fn tag(&mut self) -> Result<u8, String> {
        let b = *self.s.get(self.i).ok_or("descriptor Json hết")?;
        self.i += 1;
        Ok(b)
    }

    fn name(&mut self, stop: &[u8]) -> Result<String, String> {
        let st = self.i;
        while let Some(b) = self.s.get(self.i) {
            if stop.contains(b) {
                break;
            }
            self.i += 1;
        }
        std::str::from_utf8(&self.s[st..self.i])
            .map(str::to_string)
            .map_err(|_| "descriptor Json không UTF-8".to_string())
    }

    fn eat(&mut self, b: u8) -> Result<(), String> {
        if self.tag()? != b {
            return Err("descriptor Json sai dấu".to_string());
        }
        Ok(())
    }
}

/// `Value` → `Json` theo descriptor.
pub(crate) fn to_json(v: &Value, desc: &str) -> Result<Value, String> {
    let mut p = P::new(desc);
    to_json_p(v, &mut p)
}

fn to_json_p(v: &Value, p: &mut P<'_>) -> Result<Value, String> {
    let t = p.tag()?;
    Ok(match t {
        b'i' => match v {
            Value::Int(n) => json_num_int(*n),
            _ => return Err(format!("to_json: cần int, được {}", v.show())),
        },
        b'f' => match v {
            Value::Float(f) => json_num_float(*f),
            Value::Int(n) => json_num_float(*n as f64),
            _ => return Err(format!("to_json: cần float, được {}", v.show())),
        },
        b's' => match v {
            Value::Str(s) => json_str(s.clone()),
            _ => return Err(format!("to_json: cần str, được {}", v.show())),
        },
        b'b' => match v {
            Value::Bool(b) => json_bool(*b),
            _ => return Err(format!("to_json: cần bool, được {}", v.show())),
        },
        b'y' => match v {
            Value::Bytes(b) => json_arr(b.iter().map(|x| json_num_int(*x as i64)).collect()),
            _ => return Err(format!("to_json: cần bytes, được {}", v.show())),
        },
        b'h' => match v {
            Value::Path(s) => json_str(s.clone()),
            _ => return Err(format!("to_json: cần path, được {}", v.show())),
        },
        b'c' => match v {
            Value::Char(c) => json_str(c.to_string()),
            _ => return Err(format!("to_json: cần char, được {}", v.show())),
        },
        b'u' => json_null(),
        b'z' | b'd' => match v {
            Value::Size(n) | Value::Duration(n) => json_num_int(*n),
            _ => return Err(format!("to_json: cần size/duration, được {}", v.show())),
        },
        b'p' => match v {
            Value::Percent(f) => json_num_float(*f),
            _ => return Err(format!("to_json: cần percent, được {}", v.show())),
        },
        b'j' => v.clone(),
        b'O' => match v {
            Value::Enum(ea, va, ag) if ea == "Option" && va == "None" && ag.is_empty() => {
                skip_desc(p)?;
                json_null()
            }
            Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => {
                to_json_p(&ag[0], p)?
            }
            _ => return Err(format!("to_json: cần Option, được {}", v.show())),
        },
        b'L' => {
            let Value::List(xs) = v else {
                return Err(format!("to_json: cần list, được {}", v.show()));
            };
            let st = p.i;
            skip_desc(p)?;
            let sub = std::str::from_utf8(&p.s[st..p.i]).map_err(|_| "descriptor lỗi")?;
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(to_json(x, sub)?);
            }
            json_arr(out)
        }
        b'M' => {
            let Value::Map(kv) = v else {
                return Err(format!("to_json: cần map, được {}", v.show()));
            };
            let st = p.i;
            skip_desc(p)?;
            let ks = std::str::from_utf8(&p.s[st..p.i]).map_err(|_| "descriptor lỗi")?;
            let st = p.i;
            skip_desc(p)?;
            let vs = std::str::from_utf8(&p.s[st..p.i]).map_err(|_| "descriptor lỗi")?;
            let mut out = Vec::with_capacity(kv.len());
            for (k, val) in kv {
                let kj = to_json(k, ks)?;
                let vj = to_json(val, vs)?;
                out.push((kj, vj));
            }
            json_obj(out)
        }
        b'R' => {
            let Value::Enum(ea, va, ag) = v else {
                return Err(format!("to_json: cần Result, được {}", v.show()));
            };
            if ea != "Result" || ag.len() != 1 {
                return Err(format!("to_json: cần Result, được {}", v.show()));
            }
            if va == "Ok" {
                json_obj(vec![(json_str("Ok".into()), to_json_p(&ag[0], p)?)])
            } else {
                json_obj(vec![(json_str("Err".into()), to_json_p(&ag[0], p)?)])
            }
        }
        b'T' => {
            let name = p.name(b"{")?;
            p.eat(b'{')?;
            let Value::Record(_rn, fields) = v else {
                return Err(format!("to_json: cần record {name}, được {}", v.show()));
            };
            let mut out = Vec::new();
            while *p.s.get(p.i).unwrap_or(&b'}') != b'}' {
                let fn_ = p.name(b":")?;
                p.eat(b':')?;
                let Some((_, fv)) = fields.iter().find(|(n, _)| n == &fn_) else {
                    return Err(format!("to_json: record {name} thiếu trường `{fn_}`"));
                };
                out.push((json_str(fn_), to_json_p(fv, p)?));
                if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
                    p.i += 1;
                }
            }
            p.eat(b'}')?;
            json_obj(out)
        }
        b'E' => {
            let name = p.name(b"{")?;
            p.eat(b'{')?;
            let Value::Enum(en, va, ag) = v else {
                return Err(format!("to_json: cần enum {name}, được {}", v.show()));
            };
            if en != &name {
                return Err(format!("to_json: cần enum {name}, được {en}"));
            }
            let mut found = None;
            while *p.s.get(p.i).unwrap_or(&b'}') != b'}' {
                let vn = p.name(b":")?;
                p.eat(b':')?;
                let mut d = Vec::new();
                if *p.s.get(p.i).unwrap_or(&b';') == b'_' {
                    p.i += 1;
                } else {
                    // đọc descriptor con bằng cách chạy thử trên giá trị rỗng? Đơn giản: lưu
                    // vị trí rồi parse độ dài bằng đệ quy `bo_desc`.
                    let st = p.i;
                    skip_desc(p)?;
                    while *p.s.get(p.i).unwrap_or(&b';') == b'|' {
                        p.i += 1;
                        skip_desc(p)?;
                    }
                    d = p.s[st..p.i].to_vec();
                }
                if va == &vn {
                    found = Some(d);
                }
                if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
                    p.i += 1;
                }
            }
            p.eat(b'}')?;
            let Some(d) = found else {
                return Err(format!("to_json: enum {name} không có biến thể `{va}`"));
            };
            if d.is_empty() || ag.is_empty() {
                json_str(va.to_string())
            } else if ag.len() == 1 {
                let sub = std::str::from_utf8(&d).map_err(|_| "descriptor lỗi")?;
                json_obj(vec![(json_str(va.to_string()), to_json(&ag[0], sub)?)])
            } else {
                let sub = std::str::from_utf8(&d).map_err(|_| "descriptor lỗi")?;
                let parts: Vec<&str> = sub.split('|').collect();
                let mut xs = Vec::new();
                for (i, a) in ag.iter().enumerate() {
                    let sd = parts.get(i).copied().unwrap_or("_");
                    xs.push(to_json(a, sd)?);
                }
                json_obj(vec![(json_str(va.to_string()), json_arr(xs))])
            }
        }
        _ => {
            return Err(format!(
                "to_json: descriptor không hỗ trợ tag {}",
                t as char
            ))
        }
    })
}

/// Bỏ qua một descriptor con (dùng khi quét biến thể enum).
fn skip_desc(p: &mut P<'_>) -> Result<(), String> {
    let t = p.tag()?;
    match t {
        b'i' | b'f' | b's' | b'b' | b'y' | b'h' | b'c' | b'u' | b'z' | b'd' | b'p' | b'j' => Ok(()),
        b'O' | b'L' => skip_desc(p),
        b'M' | b'R' => {
            skip_desc(p)?;
            skip_desc(p)
        }
        b'T' | b'E' => {
            p.name(b"{")?;
            p.eat(b'{')?;
            while *p.s.get(p.i).unwrap_or(&b'}') != b'}' {
                p.name(b":")?;
                p.eat(b':')?;
                if *p.s.get(p.i).unwrap_or(&b';') == b'_' {
                    p.i += 1;
                } else {
                    skip_desc(p)?;
                }
                if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
                    p.i += 1;
                }
            }
            p.eat(b'}')
        }
        _ => Err("skip descriptor lạ".to_string()),
    }
}

/// `Json` → `Value` theo descriptor; lỗi trả chuỗi để method bọc `Err(Error)`.
pub(crate) fn from_json(j: &Value, desc: &str) -> Result<Value, String> {
    let mut p = P::new(desc);
    from_json_p(j, &mut p)
}

fn loi(kind: &str, got: &Value) -> String {
    format!("from_json: cần {kind}, được {}", got.show())
}

fn from_json_p(j: &Value, p: &mut P<'_>) -> Result<Value, String> {
    let t = p.tag()?;
    Ok(match t {
        b'i' | b'z' | b'd' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Num" && a.len() == 1 => match &a[0] {
                Value::Int(n) => Value::Int(*n),
                Value::Float(f) if f.fract() == 0.0 => Value::Int(*f as i64),
                _ => return Err(loi("int", j)),
            },
            _ => return Err(loi("int", j)),
        },
        b'f' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Num" && a.len() == 1 => match &a[0] {
                Value::Int(n) => Value::Float(*n as f64),
                Value::Float(f) => Value::Float(*f),
                _ => return Err(loi("float", j)),
            },
            _ => return Err(loi("float", j)),
        },
        b's' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Str" && a.len() == 1 => a[0].clone(),
            _ => return Err(loi("str", j)),
        },
        b'b' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Bool" && a.len() == 1 => a[0].clone(),
            _ => return Err(loi("bool", j)),
        },
        b'y' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Arr" && a.len() == 1 => match &a[0] {
                Value::List(xs) => {
                    let mut out = Vec::with_capacity(xs.len());
                    for x in xs {
                        let Value::Enum(_, _, ax) = x else {
                            return Err(loi("bytes", j));
                        };
                        let Value::Int(n) = ax.first().cloned().unwrap_or(Value::Unit) else {
                            return Err(loi("bytes", j));
                        };
                        out.push(n as u8);
                    }
                    Value::Bytes(out)
                }
                _ => return Err(loi("bytes", j)),
            },
            _ => return Err(loi("bytes", j)),
        },
        b'h' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Str" && a.len() == 1 => match &a[0] {
                Value::Str(s) => Value::Path(s.clone()),
                _ => return Err(loi("path", j)),
            },
            _ => return Err(loi("path", j)),
        },
        b'c' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Str" && a.len() == 1 => match &a[0] {
                Value::Str(s) if s.chars().count() == 1 => Value::Char(s.chars().next().unwrap()),
                _ => return Err(loi("char", j)),
            },
            _ => return Err(loi("char", j)),
        },
        b'u' => Value::Unit,
        b'p' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Num" && a.len() == 1 => match &a[0] {
                Value::Int(n) => Value::Percent(*n as f64),
                Value::Float(f) => Value::Percent(*f),
                _ => return Err(loi("percent", j)),
            },
            _ => return Err(loi("percent", j)),
        },
        b'j' => j.clone(),
        b'O' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Null" && a.is_empty() => {
                skip_desc(p)?;
                Value::none()
            }
            _ => Value::some(from_json_p(j, p)?),
        },
        b'L' => {
            let st = p.i;
            skip_desc(p)?;
            let sub = std::str::from_utf8(&p.s[st..p.i]).map_err(|_| "descriptor lỗi")?;
            match j {
                Value::Enum(e, v, a) if e == "Json" && v == "Arr" && a.len() == 1 => match &a[0] {
                    Value::List(xs) => {
                        let mut out = Vec::with_capacity(xs.len());
                        for x in xs {
                            out.push(from_json(x, sub)?);
                        }
                        Value::List(out)
                    }
                    _ => return Err(loi("list", j)),
                },
                _ => return Err(loi("list", j)),
            }
        }
        b'M' => {
            let st = p.i;
            skip_desc(p)?;
            let ks = std::str::from_utf8(&p.s[st..p.i]).map_err(|_| "descriptor lỗi")?;
            let st = p.i;
            skip_desc(p)?;
            let vs = std::str::from_utf8(&p.s[st..p.i]).map_err(|_| "descriptor lỗi")?;
            match j {
                Value::Enum(e, v, a) if e == "Json" && v == "Obj" && a.len() == 1 => match &a[0] {
                    Value::Map(kv) => {
                        let mut out = Vec::with_capacity(kv.len());
                        for (k, val) in kv {
                            let k = from_json(k, ks)?;
                            let val = from_json(val, vs)?;
                            out.push((k, val));
                        }
                        Value::Map(out)
                    }
                    _ => return Err(loi("map", j)),
                },
                _ => return Err(loi("map", j)),
            }
        }
        b'R' => match j {
            Value::Enum(e, v, a) if e == "Json" && v == "Obj" && a.len() == 1 => match &a[0] {
                Value::Map(kv) if kv.len() == 1 => {
                    let (k, val) = &kv[0];
                    let Value::Enum(_, kvn, _) = k else {
                        return Err(loi("Result", j));
                    };
                    if kvn == "Ok" {
                        Value::ok(from_json_p(val, p)?)
                    } else {
                        Value::err(from_json_p(val, p)?)
                    }
                }
                _ => return Err(loi("Result", j)),
            },
            _ => return Err(loi("Result", j)),
        },
        b'T' => {
            let name = p.name(b"{")?;
            p.eat(b'{')?;
            let Value::Enum(e, v, a) = j else {
                return Err(loi(&name, j));
            };
            if e != "Json" || v != "Obj" || a.len() != 1 {
                return Err(loi(&name, j));
            }
            let Value::Map(kv) = &a[0] else {
                return Err(loi(&name, j));
            };
            let mut out = Vec::new();
            while *p.s.get(p.i).unwrap_or(&b'}') != b'}' {
                let fn_ = p.name(b":")?;
                p.eat(b':')?;
                let Some((_, jv)) = kv.iter().find(|(k, _)| {
                    matches!(k, Value::Enum(e, v, a)
                        if e == "Json" && v == "Str" && a.len() == 1 && matches!(&a[0], Value::Str(s) if s == &fn_))
                }) else {
                    // trường Option thiếu ⇒ None (hợp đồng default); trường khác ⇒ lỗi.
                    if *p.s.get(p.i).unwrap_or(&b';') == b'O' {
                        p.i += 1;
                        skip_desc(p)?;
                        out.push((fn_, Value::none()));
                        if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
                            p.i += 1;
                        }
                        continue;
                    }
                    return Err(format!("from_json: record {name} thiếu trường `{fn_}`"));
                };
                out.push((fn_, from_json_p(jv, p)?));
                if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
                    p.i += 1;
                }
            }
            p.eat(b'}')?;
            Value::Record(String::new(), out)
        }
        b'E' => {
            let name = p.name(b"{")?;
            p.eat(b'{')?;
            let mut found: Option<(String, Vec<u8>)> = None;
            while *p.s.get(p.i).unwrap_or(&b'}') != b'}' {
                let vn = p.name(b":")?;
                p.eat(b':')?;
                let mut d = Vec::new();
                if *p.s.get(p.i).unwrap_or(&b';') == b'_' {
                    p.i += 1;
                } else {
                    let st = p.i;
                    skip_desc(p)?;
                    while *p.s.get(p.i).unwrap_or(&b';') == b'|' {
                        p.i += 1;
                        skip_desc(p)?;
                    }
                    d = p.s[st..p.i].to_vec();
                }
                if found.is_none() {
                    found = Some((vn, d));
                }
                if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
                    p.i += 1;
                }
            }
            p.eat(b'}')?;
            // `Json.Str("Variant")` cho unit; `Json.Obj({"Variant": payload})` cho có payload.
            match j {
                Value::Enum(e, v, a) if e == "Json" && v == "Str" && a.len() == 1 => {
                    let Value::Str(vn) = &a[0] else {
                        return Err(loi(&name, j));
                    };
                    Value::Enum(name.into(), vn.as_str().into(), Vec::new())
                }
                Value::Enum(e, v, a) if e == "Json" && v == "Obj" && a.len() == 1 => {
                    let Value::Map(kv) = &a[0] else {
                        return Err(loi(&name, j));
                    };
                    if kv.len() != 1 {
                        return Err(loi(&name, j));
                    }
                    let (k, val) = &kv[0];
                    let Value::Enum(_, kvn, ka) = k else {
                        return Err(loi(&name, j));
                    };
                    if kvn != "Str" || ka.len() != 1 {
                        return Err(loi(&name, j));
                    }
                    let Value::Str(vn) = &ka[0] else {
                        return Err(loi(&name, j));
                    };
                    // Tìm descriptor biến thể trong enum descriptor.
                    let desc = std::str::from_utf8(p.s).map_err(|_| "descriptor lỗi")?;
                    let sub = variant_desc(desc, vn)?;
                    let val = if matches!(val, Value::Enum(e, v, _) if e == "Json" && v == "Arr") {
                        match val {
                            Value::Enum(_, _, xs) => match &xs[0] {
                                Value::List(items) => {
                                    let parts: Vec<&str> = sub.split('|').collect();
                                    let mut out = Vec::new();
                                    for (i, it) in items.iter().enumerate() {
                                        out.push(from_json(
                                            it,
                                            parts.get(i).copied().unwrap_or("_"),
                                        )?);
                                    }
                                    Value::Enum(name.into(), vn.as_str().into(), out)
                                }
                                _ => return Err(loi(&name, j)),
                            },
                            _ => return Err(loi(&name, j)),
                        }
                    } else {
                        Value::Enum(name.into(), vn.as_str().into(), vec![from_json(val, &sub)?])
                    };
                    val
                }
                _ => return Err(loi(&name, j)),
            }
        }
        _ => return Err("from_json: descriptor không hỗ trợ".to_string()),
    })
}

/// Tìm descriptor của biến thể trong descriptor enum `E...`.
fn variant_desc(desc: &str, vn: &str) -> Result<String, String> {
    let i = desc.find('{').ok_or("descriptor enum thiếu {")?;
    let body = &desc[i + 1..];
    let mut p = P::new(body);
    while *p.s.get(p.i).unwrap_or(&b'}') != b'}' {
        let n = p.name(b":")?;
        p.eat(b':')?;
        let st = p.i;
        if *p.s.get(p.i).unwrap_or(&b';') == b'_' {
            p.i += 1;
        } else {
            skip_desc(&mut p)?;
            while *p.s.get(p.i).unwrap_or(&b';') == b'|' {
                p.i += 1;
                skip_desc(&mut p)?;
            }
        }
        if n == vn {
            let d = &body[st..p.i];
            return Ok(d.trim_start_matches('_').to_string());
        }
        if *p.s.get(p.i).unwrap_or(&b'}') == b';' {
            p.i += 1;
        }
    }
    Err(format!("from_json: enum không có biến thể `{vn}`"))
}
