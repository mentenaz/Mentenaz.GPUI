use gpui::{ClickEvent, Context, prelude::*};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, ThemeStyled as _,
    input::Input,
    sidebar::{Sidebar, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex,
};

use crate::{Gallery, StoryContainer};

impl Gallery {
    /// The left navigation column: brand header, search box, and the
    /// (possibly search-filtered) story list.
    pub(crate) fn render_sidebar(
        &self,
        stories: Vec<(&'static str, Vec<gpui::Entity<StoryContainer>>)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Sidebar::new("gallery-sidebar")
            .w(gpui::relative(1.))
            .border_0()
            .collapsed(self.collapsed)
            .header(
                v_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        SidebarHeader::new()
                            .w_full()
                            .child(
                                gpui::div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(cx.theme().radius_lg)
                                    .bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground)
                                    .size_8()
                                    .flex_shrink_0()
                                    .when(!self.collapsed, |this| {
                                        this.child(Icon::new(IconName::GalleryVerticalEnd))
                                    })
                                    .when(self.collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                            .child(Icon::new(IconName::GalleryVerticalEnd))
                                    }),
                            )
                            .when(!self.collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .gap_0()
                                        .text_sm()
                                        .flex_1()
                                        .line_height(gpui::relative(1.25))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child("GPUI Component")
                                        .child(
                                            gpui::div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("Component showcase")
                                                .text_xs(),
                                        ),
                                )
                            }),
                    )
                    .child(
                        gpui::div()
                            .bg(cx.theme().sidebar_accent)
                            .rounded_full_style(cx)
                            .px_1()
                            .flex_1()
                            .mx_1()
                            .child(
                                Input::new(&self.search_input)
                                    .appearance(false)
                                    .cleanable(true),
                            ),
                    ),
            )
            .children(
                stories
                    .into_iter()
                    .enumerate()
                    .map(|(group_ix, (_, sub_stories))| {
                        SidebarMenu::new().children(sub_stories.iter().enumerate().map(
                            |(ix, story)| {
                                SidebarMenuItem::new(story.read(cx).name.clone())
                                    .active(
                                        self.active_group_index == Some(group_ix)
                                            && self.active_index == Some(ix),
                                    )
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.active_group_index = Some(group_ix);
                                        this.active_index = Some(ix);
                                        cx.notify();
                                    }))
                            },
                        ))
                    }),
            )
    }

    /// [`Gallery::render_sidebar`], pre-filled from live search state,
    /// exposed `pub` (not `pub(crate)`) so a host application embedding
    /// Gallery via [`Gallery::view_without_status_bar_and_sidebar`] can
    /// render the sidebar itself, elsewhere in its own layout — e.g.
    /// `ForgeShell`'s left column.
    pub fn sidebar_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let stories = self.filtered_stories(cx);
        self.render_sidebar(stories, cx)
    }
}
