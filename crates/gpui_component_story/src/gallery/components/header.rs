use gpui::{Context, SharedString, prelude::*};
use gpui_component::{ActiveTheme as _, StyledExt as _, h_flex, v_flex};

use crate::Gallery;

impl Gallery {
    /// The active story's title + description, shown above the story content.
    pub(crate) fn render_header(
        &self,
        story_name: SharedString,
        description: SharedString,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .id("header")
            .p_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_start()
            .child(
                v_flex()
                    .gap_1()
                    .child(gpui::div().text_2xl().font_semibold().child(story_name))
                    .child(
                        gpui::div()
                            .text_color(cx.theme().muted_foreground)
                            .child(description),
                    ),
            )
    }
}
