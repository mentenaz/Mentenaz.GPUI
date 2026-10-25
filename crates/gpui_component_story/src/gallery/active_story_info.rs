use gpui::{App, SharedString};

use crate::Gallery;

impl Gallery {
    /// The total component count and the active story's name, computed from
    /// `self.stories` (unfiltered by the sidebar search query) — the same
    /// list `active_group_index`/`active_index` are set against everywhere
    /// else (`select_story`, `select_story_index`, `set_active_story`).
    pub(crate) fn active_story_info(&self, cx: &App) -> (usize, SharedString) {
        let total_components: usize = self.stories.iter().map(|(_, items)| items.len()).sum();
        let active_group = self
            .active_group_index
            .and_then(|index| self.stories.get(index));
        let current_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()))
            .map(|story| story.read(cx).name.clone())
            .unwrap_or_default();

        (total_components, current_story)
    }

    /// The active story's name alone — for a host application embedding
    /// `Gallery` (e.g. via [`Self::view_without_status_bar_and_sidebar`])
    /// that wants to show it somewhere outside Gallery's own status bar,
    /// without needing the total-component-count half of
    /// [`Self::active_story_info`].
    pub fn current_story_name(&self, cx: &App) -> SharedString {
        self.active_story_info(cx).1
    }
}
