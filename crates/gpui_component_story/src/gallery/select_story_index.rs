use gpui::{Context, Window};

use crate::Gallery;

impl Gallery {
    pub(crate) fn select_story_index(
        &mut self,
        index: gpui_component::IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if index.section != 0 {
            return false;
        }
        let Some(name) = self
            .stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .nth(index.row)
            .map(|story| story.read(cx).name.clone())
        else {
            return false;
        };
        self.select_story(&name, window, cx)
    }
}
