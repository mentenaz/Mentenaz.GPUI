use gpui::Context;

use crate::{Gallery, StoryContainer};

impl Gallery {
    /// `self.stories`, filtered by the sidebar search box — shared by
    /// `render` (drawing its own sidebar) and `sidebar_element` (a host
    /// application rendering the sidebar elsewhere), so the two never drift
    /// out of sync on how filtering works.
    pub(crate) fn filtered_stories(
        &self,
        cx: &Context<Self>,
    ) -> Vec<(&'static str, Vec<gpui::Entity<StoryContainer>>)> {
        let query = self.search_input.read(cx).value().trim().to_lowercase();

        self.stories
            .iter()
            .filter_map(|(name, items)| {
                let filtered_items: Vec<_> = items
                    .iter()
                    .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
                    .cloned()
                    .collect();

                if !filtered_items.is_empty() {
                    Some((*name, filtered_items))
                } else {
                    None
                }
            })
            .collect()
    }
}
