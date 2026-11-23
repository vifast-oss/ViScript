// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-306** — macro `#[vis::module]` / `#[vis::func]` sinh chữ ký plugin lúc build.
//!
//! ```ignore
//! use vis_plugin_macros as vis;
//! #[vis::module(name = "mau")]
//! mod mau {
//!     #[vis::func]
//!     pub fn cong(a: i64, b: i64) -> i64 { a + b }
//!
//!     #[vis::func]
//!     pub fn doc(ctx: &vis_plugin_abi::PluginCtx, p: String) -> vis_plugin_abi::Result<String> {
//!         ctx.read_text(p)
//!     }
//! }
//! ```
//!
//! Macro sinh, trong module:
//! - `__vis_module() -> vis_plugin_abi::PluginModule` (tên + danh sách `FuncDef`);
//! - wrapper chuyển `PluginValue` ↔ kiểu Rust **có kiểm lúc build** (thiếu `TryFrom`/`From` ⇒
//!   lỗi biên dịch);
//! - tham số đầu `ctx: &PluginCtx` (nếu có) **không** lộ vào chữ ký ViScript — quyền chỉ đến từ
//!   host qua `PluginCtx`.
//!
//! Crate **0 `unsafe`** (`#![forbid(unsafe_code)]`).
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, FnArg, Item, ItemFn, ItemMod, Lit, Meta, Pat, PatType, ReturnType, Type,
};

/// Dấu `#[vis::func]` — macro `module` đọc và bỏ dấu này; dùng riêng ngoài module là no-op.
#[proc_macro_attribute]
pub fn func(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Hàm được đánh dấu: item gốc + (tên, kiểu) tham số + kiểu trả + trả `Result` hay không.
type HamPlugin = (ItemFn, Vec<(String, Type)>, Type, bool);

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut m = parse_macro_input!(item as ItemMod);
    let ten_mod = ten_module(&attr).unwrap_or_else(|| m.ident.to_string());
    let mut funcs: Vec<HamPlugin> = Vec::new();

    if let Some((_, items)) = &mut m.content {
        for it in items.iter_mut() {
            if let Item::Fn(f) = it {
                if bo_dau_func(f) {
                    let (params, ret, la_result) = tach_chu_ky(f);
                    funcs.push((f.clone(), params, ret, la_result));
                }
            }
        }
    }

    let mut wrappers = Vec::new();
    let mut sigs = Vec::new();
    let mut defs = Vec::new();
    for (f, params, ret, la_result) in &funcs {
        let ten = &f.sig.ident;
        let ten_call = format_ident!("__vis_call_{}", ten);
        let ten_sig = format_ident!("__VIS_SIG_{}", ten.to_string().to_uppercase());
        let ten_params = format_ident!("__VIS_PARAMS_{}", ten.to_string().to_uppercase());
        let co_ctx = f.sig.inputs.first().is_some_and(la_ctx);
        let so_tham = params.len();
        let args: Vec<_> = params
            .iter()
            .enumerate()
            .map(|(i, (n, _))| format_ident!("__vis_a{i}_{n}"))
            .collect();
        let lays: Vec<_> = params
            .iter()
            .enumerate()
            .map(|(i, (_, ty))| {
                let a = &args[i];
                quote! {
                    let #a: #ty = ::std::convert::TryInto::try_into(args[#i].clone())?;
                }
            })
            .collect();
        let loi_goi = if *la_result {
            if co_ctx {
                quote! { let kq = #ten(ctx, #( #args ),*)?; }
            } else {
                quote! { let kq = #ten(#( #args ),*)?; }
            }
        } else if co_ctx {
            quote! { let kq = #ten(ctx, #( #args ),*); }
        } else {
            quote! { let kq = #ten(#( #args ),*); }
        };
        let ten_chuoi = ten.to_string();
        let ten_ctx = if co_ctx { quote!(ctx) } else { quote!(_ctx) };
        let goi = quote! {
            fn #ten_call(
                #ten_ctx: &::vis_plugin_abi::PluginCtx,
                args: &[::vis_plugin_abi::PluginValue],
            ) -> ::vis_plugin_abi::Result<::vis_plugin_abi::PluginValue> {
                if args.len() != #so_tham {
                    return Err(::vis_plugin_abi::Error::io(format!(
                        "plugin `{}` cần {} tham số, nhận {}",
                        #ten_chuoi, #so_tham, args.len()
                    )));
                }
                #(#lays)*
                #loi_goi
                Ok(::vis_plugin_abi::PluginValue::from(kq))
            }
        };
        let ps = params.iter().map(|(n, ty)| {
            let t = ten_vs(ty);
            quote! { ::vis_plugin_abi::Param { name: #n, ty: #t } }
        });
        let ten_vs_ret = ten_vs(ret);
        wrappers.push(goi);
        sigs.push(quote! {
            const #ten_params: &[::vis_plugin_abi::Param] = &[#(#ps),*];
            const #ten_sig: ::vis_plugin_abi::Signature = ::vis_plugin_abi::Signature {
                name: #ten_chuoi,
                params: #ten_params,
                ret: #ten_vs_ret,
            };
        });
        defs.push(quote! {
            ::vis_plugin_abi::FuncDef { sig: #ten_sig, call: #ten_call }
        });
    }

    let gen = quote! {
        #(#wrappers)*
        #(#sigs)*
        const __VIS_FUNCS: &[::vis_plugin_abi::FuncDef] = &[#(#defs),*];
        pub fn __vis_module() -> ::vis_plugin_abi::PluginModule {
            ::vis_plugin_abi::PluginModule { name: #ten_mod, funcs: __VIS_FUNCS }
        }
    };
    let attrs = &m.attrs;
    let vis = &m.vis;
    let ten_mod_ident = &m.ident;
    let items = m
        .content
        .as_ref()
        .map(|(_, items)| quote!(#(#items)*))
        .unwrap_or_default();
    quote! {
        #(#attrs)*
        #vis mod #ten_mod_ident {
            #items
            #gen
        }
    }
    .into()
}

fn ten_module(attr: &TokenStream) -> Option<String> {
    if attr.is_empty() {
        return None;
    }
    let meta = syn::parse::<Meta>(attr.clone()).ok()?;
    match meta {
        Meta::NameValue(nv) => match nv.value {
            syn::Expr::Lit(l) => match l.lit {
                Lit::Str(s) => Some(s.value()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn la_ctx(arg: &FnArg) -> bool {
    match arg {
        FnArg::Typed(PatType { pat, .. }) => match &**pat {
            Pat::Ident(i) => i.ident == "ctx",
            _ => false,
        },
        FnArg::Receiver(_) => false,
    }
}

fn bo_dau_func(f: &mut ItemFn) -> bool {
    let truoc = f.attrs.len();
    f.attrs.retain(|a| {
        let p = a.path();
        let cuoi = p.segments.last().map(|s| s.ident.to_string());
        cuoi.as_deref() != Some("func")
    });
    f.attrs.len() != truoc
}

fn tach_chu_ky(f: &ItemFn) -> (Vec<(String, Type)>, Type, bool) {
    let mut params = Vec::new();
    for (i, arg) in f.sig.inputs.iter().enumerate() {
        if i == 0 && la_ctx(arg) {
            continue;
        }
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(id) = &**pat {
                params.push((id.ident.to_string(), (**ty).clone()));
            }
        }
    }
    let (ret, la_result) = match &f.sig.output {
        ReturnType::Default => (syn::parse_quote!(()), false),
        ReturnType::Type(_, ty) => {
            let la = match &**ty {
                Type::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == "Result"),
                _ => false,
            };
            let inner = if la {
                match &**ty {
                    Type::Path(p) => p
                        .path
                        .segments
                        .last()
                        .and_then(|s| match &s.arguments {
                            syn::PathArguments::AngleBracketed(a) => a.args.first(),
                            _ => None,
                        })
                        .and_then(|a| match a {
                            syn::GenericArgument::Type(t) => Some(t.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| syn::parse_quote!(())),
                    _ => syn::parse_quote!(()),
                }
            } else {
                (**ty).clone()
            };
            (inner, la)
        }
    };
    (params, ret, la_result)
}

fn ten_vs(ty: &Type) -> String {
    match ty {
        Type::Path(p) => {
            let s = p
                .path
                .segments
                .last()
                .map(|x| x.ident.to_string())
                .unwrap_or_default();
            match s.as_str() {
                "i64" | "int" => "int".to_string(),
                "f64" | "float" => "float".to_string(),
                "bool" => "bool".to_string(),
                "String" | "str" => "str".to_string(),
                "Vec" => "Bytes".to_string(),
                _ => s,
            }
        }
        _ => quote!(#ty).to_string(),
    }
}
