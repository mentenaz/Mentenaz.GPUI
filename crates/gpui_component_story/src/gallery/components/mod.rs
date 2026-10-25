//! Gallery's UI-building pieces, one per file — kept separate from
//! `gallery/`'s state/logic methods (`set_active_story`, `select_story`,
//! etc.). Each is an `impl Gallery` method returning `impl IntoElement`,
//! called from `gallery/render.rs`.

mod header;
mod sidebar;
mod status_bar;
