// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — binary gộp cho bộ kiểm thử thông dịch cốt lõi của `vis-interp`.

#[path = "interp_it/interp_v2.rs"]
mod interp_v2;
#[path = "interp_it/run.rs"]
mod run;
#[path = "interp_it/show.rs"]
mod show;
#[path = "interp_it/vd211_enum_eq.rs"]
mod vd211_enum_eq;
#[path = "interp_it/vd263_ctrl.rs"]
mod vd263_ctrl;
#[path = "interp_it/vd270_map_eq.rs"]
mod vd270_map_eq;
#[path = "interp_it/vd270_map_eq_diff.rs"]
mod vd270_map_eq_diff;

// ST-313c: frame/suspend tách từ binary `frame_it` (R8).
#[path = "frame_it/lat0_suspend.rs"]
mod lat0_suspend;
#[path = "frame_it/lat1_frame.rs"]
mod lat1_frame;
