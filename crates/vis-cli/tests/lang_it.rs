// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Ngôn ngữ: HIR/lower/tyck/parser/CLI trait/generic.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

#[path = "lang_it/ban_ghi_an_danh_ten.rs"]
mod ban_ghi_an_danh_ten;
#[path = "lang_it/generic_cli.rs"]
mod generic_cli;
#[path = "lang_it/ma_truong.rs"]
mod ma_truong;
#[path = "lang_it/ngoai_vm.rs"]
mod ngoai_vm;
#[path = "lang_it/p0_newtype.rs"]
mod p0_newtype;
#[path = "lang_it/prelude.rs"]
mod prelude;
#[path = "lang_it/ref_identity.rs"]
mod ref_identity;
#[path = "lang_it/sap_xep.rs"]
mod sap_xep;
#[path = "lang_it/self_amp_hop_long.rs"]
mod self_amp_hop_long;
#[path = "lang_it/sem.rs"]
mod sem;
#[path = "lang_it/stage_dau.rs"]
mod stage_dau;
#[path = "lang_it/trait_cli.rs"]
mod trait_cli;
#[path = "lang_it/vd258_checker.rs"]
mod vd258_checker;
#[path = "lang_it/vd260_parity3.rs"]
mod vd260_parity3;
#[path = "lang_it/vd260_rest_pattern.rs"]
mod vd260_rest_pattern;
#[path = "lang_it/vd260_semantics3.rs"]
mod vd260_semantics3;
#[path = "lang_it/vd262_globals.rs"]
mod vd262_globals;
#[path = "lang_it/vd262_method.rs"]
mod vd262_method;
#[path = "lang_it/vd263_ctrl.rs"]
mod vd263_ctrl;
#[path = "lang_it/vd263_generic.rs"]
mod vd263_generic;
#[path = "lang_it/vd263_letelse.rs"]
mod vd263_letelse;
#[path = "lang_it/vd263_withunit.rs"]
mod vd263_withunit;
#[path = "lang_it/vd264_infer.rs"]
mod vd264_infer;
#[path = "lang_it/vd264_range.rs"]
mod vd264_range;
#[path = "lang_it/vd265_genericres.rs"]
mod vd265_genericres;
#[path = "lang_it/vd265_methodret.rs"]
mod vd265_methodret;
#[path = "lang_it/vd265_stdlib.rs"]
mod vd265_stdlib;
#[path = "lang_it/vd266_intlit.rs"]
mod vd266_intlit;
#[path = "lang_it/vd266_pexpr.rs"]
mod vd266_pexpr;
#[path = "lang_it/vd267_enumalloc.rs"]
mod vd267_enumalloc;
#[path = "lang_it/vd267_fmt_size.rs"]
mod vd267_fmt_size;
#[path = "lang_it/vd267_fmtstdin.rs"]
mod vd267_fmtstdin;
#[path = "lang_it/vd267_panic.rs"]
mod vd267_panic;
#[path = "lang_it/vd268_nestassign.rs"]
mod vd268_nestassign;
#[path = "lang_it/vd269_fmtmode.rs"]
mod vd269_fmtmode;
#[path = "lang_it/vd269_optmethod.rs"]
mod vd269_optmethod;
#[path = "lang_it/vd269_strpad.rs"]
mod vd269_strpad;
#[path = "lang_it/vd270_eq_record.rs"]
mod vd270_eq_record;
#[path = "lang_it/vd270_impl_generic.rs"]
mod vd270_impl_generic;
#[path = "lang_it/vd270_impl_module.rs"]
mod vd270_impl_module;
#[path = "lang_it/vd270_matches.rs"]
mod vd270_matches;
#[path = "lang_it/vd270_r5c_sugar.rs"]
mod vd270_r5c_sugar;
#[path = "lang_it/vd270_sugar_arity.rs"]
mod vd270_sugar_arity;
#[path = "lang_it/vd_204.rs"]
mod vd_204;
#[path = "lang_it/vd_205.rs"]
mod vd_205;
#[path = "lang_it/vd_208.rs"]
mod vd_208;
#[path = "lang_it/vd_212.rs"]
mod vd_212;
#[path = "lang_it/vd_215.rs"]
mod vd_215;
#[path = "lang_it/vd_218.rs"]
mod vd_218;
#[path = "lang_it/vd_223.rs"]
mod vd_223;
#[path = "lang_it/vd_226.rs"]
mod vd_226;
#[path = "lang_it/vd_227.rs"]
mod vd_227;
#[path = "lang_it/vd_228.rs"]
mod vd_228;
