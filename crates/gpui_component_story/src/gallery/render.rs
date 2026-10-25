use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};
use gpui_component::{
    resizable::{h_resizable, resizable_panel},
    v_flex,
};

use crate::Gallery;

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stories = self.filtered_stories(cx);

        let active_group = self.active_group_index.and_then(|index| stories.get(index));
        let active_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()));
        let (story_name, description) =
            if let Some(story) = active_story.as_ref().map(|story| story.read(cx)) {
                (story.name.clone(), story.description.clone())
            } else {
                ("".into(), "".into())
            };

        let current_story = story_name.clone();
        let total_components: usize = self.stories.iter().map(|(_, items)| items.len()).sum();

        if self.embedded {
            return div()
                .id("embedded-story")
                .size_full()
                .when_some(active_story, |this, story| this.child(story.clone()))
                .into_any_element();
        }

        let content_column = v_flex()
            .flex_1()
            .h_full()
            .overflow_x_hidden()
            .child(self.render_header(story_name, description, cx))
            .child(
                div()
                    .id("story")
                    .flex_1()
                    .overflow_y_scroll()
                    .when_some(active_story, |this, active_story| {
                        this.child(active_story.clone())
                    }),
            )
            .into_any_element();

        // `hide_sidebar` hosts (e.g. `ForgeShell`) render the sidebar
        // themselves elsewhere via `sidebar_element`, on the same `Gallery`
        // entity — rendering it again here would just duplicate it.
        let body = if self.hide_sidebar {
            content_column
        } else {
            h_resizable("gallery-container")
                .child(
                    resizable_panel()
                        .size(px(255.))
                        .size_range(px(200.)..px(320.))
                        .child(self.render_sidebar(stories.clone(), cx)),
                )
                .child(content_column)
                .into_any_element()
        };

        v_flex()
            .size_full()
            .child(div().flex_1().min_h_0().child(body))
            .when(!self.hide_status_bar, |this| {
                this.child(self.render_status_bar(
                    Vec::new(),
                    Vec::new(),
                    total_components,
                    current_story,
                    true,
                    cx,
                ))
            })
            .into_any_element()
    }
}
