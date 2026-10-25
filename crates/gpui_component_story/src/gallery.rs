//! `Gallery` — the component showcase's top-level view.
//!
//! Split by concern, one thing per file:
//! - This file: the `Gallery` struct itself and its trivial `new`.
//! - `gallery/` (functions — state/behavior): `new_with_mode`,
//!   `set_active_story`, `select_story`, `select_story_index`,
//!   `command_entries`, `component_command`, `find_story_index`,
//!   `active_story_info`, `view`.
//! - `gallery/components/` (UI-building): `render_sidebar`, `render_header`,
//!   `render_status_bar`, `status_bar_element`, each an `impl Gallery` method
//!   returning `impl IntoElement`. `status_bar_element` is `pub` (not
//!   `pub(crate)`) so a host application can pull Gallery's real status bar
//!   out and render it separately — see `ForgeShell` in `forge_gpui`, which
//!   renders it full-width below its own left panel instead of letting it
//!   sit only under Gallery's own content column.
//! - `gallery/render.rs`: the `impl Render for Gallery` that orchestrates
//!   the pieces above — the only place that assembles the full layout.

use gpui::{Entity, Subscription};
use gpui_component::input::InputState;

use crate::StoryContainer;

mod active_story_info;
mod command_entries;
mod component_command;
mod components;
mod filtered_stories;
mod find_story_index;
mod new_with_mode;
mod open_file;
mod render;
mod select_story;
mod select_story_index;
mod set_active_story;
mod view;

#[cfg(test)]
use component_command::component_command;
#[cfg(test)]
use find_story_index::find_story_index;

pub struct Gallery {
    stories: Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    collapsed: bool,
    embedded: bool,
    hide_status_bar: bool,
    hide_sidebar: bool,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl Gallery {
    pub fn new(
        init_story: Option<&str>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        Self::new_with_mode(init_story, false, false, false, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::{component_command, find_story_index};
    use gpui_component::command::CommandEntry;

    #[test]
    fn component_command_uses_story_name() {
        let entry = component_command("Command");

        assert!(matches!(entry, CommandEntry::Item(_)));
    }

    #[test]
    fn story_index_matches_names_without_filtering() {
        let groups = [vec!["Welcome", "Button"], vec!["Command", "Dialog"]];

        assert_eq!(
            find_story_index(groups.iter().map(|group| group.iter().copied()), "command"),
            Some((1, 0))
        );
        assert_eq!(
            find_story_index(groups.iter().map(|group| group.iter().copied()), "missing"),
            None
        );
    }
}
