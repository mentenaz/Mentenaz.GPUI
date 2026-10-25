use gpui::{App, Window};

use crate::Gallery;

impl Gallery {
    pub(crate) fn set_active_story(&mut self, name: &str, window: &mut Window, cx: &mut App) {
        let name = name.trim().to_string();
        let exact_index = self
            .stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .filter(|story| {
                story
                    .read(cx)
                    .name
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            })
            .position(|story| story.read(cx).name.eq_ignore_ascii_case(&name));
        self.search_input.update(cx, |this, cx| {
            this.set_value(&name, window, cx);
        });
        self.active_group_index = Some(0);
        self.active_index = Some(exact_index.unwrap_or(0));
    }
}
