use gpui::{Context, Window};

use crate::Gallery;

use super::find_story_index::find_story_index;

impl Gallery {
    pub(crate) fn select_story(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let names = self
            .stories
            .iter()
            .map(|(_, stories)| {
                stories
                    .iter()
                    .map(|story| story.read(cx).name.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let Some((group_ix, story_ix)) = find_story_index(
            names
                .iter()
                .map(|group| group.iter().map(|name| name.as_ref())),
            name,
        ) else {
            return false;
        };

        self.search_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.active_group_index = Some(group_ix);
        self.active_index = Some(story_ix);
        cx.notify();
        true
    }
}
