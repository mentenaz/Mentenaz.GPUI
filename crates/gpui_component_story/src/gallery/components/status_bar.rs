use gpui::{AnyElement, Context, SharedString, prelude::*};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    separator::Separator,
    status_bar::StatusBar,
};

use crate::Gallery;

impl Gallery {
    /// The bottom bar: zero or more `leading` items (each its own item in
    /// the left-aligned group, plus a trailing separator if any were given),
    /// component count, active story name, theme name/version, and the
    /// GitHub link button. `leading` and the component-count/story group
    /// both live in `StatusBar`'s `left` region (not `.child(...)`/center)
    /// so they render as one contiguous left-aligned block instead of
    /// `leading` sitting pinned to the far edge with the rest centered in
    /// the remaining space.
    pub(crate) fn render_status_bar(
        &self,
        leading: Vec<AnyElement>,
        trailing: Vec<AnyElement>,
        total_components: usize,
        current_story: SharedString,
        show_component_count: bool,
        cx: &mut Context<Self>,
    ) -> StatusBar {
        let has_leading = !leading.is_empty();
        let has_trailing = !trailing.is_empty();
        let bar = leading
            .into_iter()
            .fold(StatusBar::new(), |bar, item| bar.left(item));

        let bar = bar
            .when(has_leading, |this| this.left(Separator::vertical()))
            .when(show_component_count, |this| {
                this.left(Icon::new(IconName::GalleryVerticalEnd).xsmall())
                    .left(format!("{total_components} components"))
                    .left(Separator::vertical())
            })
            .when(!current_story.is_empty(), |this| {
                this.left(current_story.clone())
            })
            .when(has_trailing, |this| this.left(Separator::vertical()));
        let bar = trailing.into_iter().fold(bar, |bar, item| bar.left(item));

        bar.right(cx.theme().theme_name().clone())
            .right(format!("v{}", env!("CARGO_PKG_VERSION")))
            .right(
                Button::new("assistant")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Github)
                    .tooltip("GPUI Component GitHub repository")
                    .on_click(|_, _, cx| {
                        cx.open_url("https://github.com/longbridge/gpui-component")
                    }),
            )
    }

    /// [`Gallery::render_status_bar`] with its `total_components`/
    /// `current_story` arguments pre-filled from live state and no leading
    /// items, exposed `pub` (not `pub(crate)`) so a host application
    /// embedding Gallery via [`Gallery::view_without_status_bar`] can render
    /// this bar itself, elsewhere in its own layout.
    pub fn status_bar_element(&self, cx: &mut Context<Self>) -> StatusBar {
        let (total_components, current_story) = self.active_story_info(cx);
        self.render_status_bar(
            Vec::new(),
            Vec::new(),
            total_components,
            current_story,
            true,
            cx,
        )
    }

    /// [`Gallery::status_bar_element`], with `leading` items inserted at the
    /// front of the status bar's left-aligned group (before Gallery's own
    /// icon/count/story content), followed by a separator — e.g. a host
    /// application's own panel-toggle buttons, so they read as part of the
    /// same left-aligned block rather than disconnected far-left items. Each
    /// item in `leading` renders as its own entry in that group (own
    /// `gap_2` spacing from the bar itself) — pass one per button, in the
    /// order they should appear, growing the `vec![...]` as more are added.
    pub fn status_bar_element_with_leading(
        &self,
        leading: Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) -> StatusBar {
        let (total_components, current_story) = self.active_story_info(cx);
        self.render_status_bar(
            leading,
            Vec::new(),
            total_components,
            current_story,
            true,
            cx,
        )
    }

    /// [`Gallery::status_bar_element_with_leading`], plus `trailing` items
    /// appended after Gallery's own story name (preceded by a separator) —
    /// e.g. a host application's own background-task status (a spinner while
    /// busy, a checkmark when done), so it reads as part of the same
    /// left-aligned block instead of competing with the always-present
    /// theme-name/version/GitHub group on the right.
    ///
    /// `title_override`, if given, replaces Gallery's own active-story name
    /// in this slot — e.g. a host application like Forge that isn't showing
    /// Gallery stories at all wants its own active file name there instead.
    /// `None` renders nothing in this slot (not Gallery's own current-story
    /// name — a host with no active-file concept, or Forge with no tab
    /// open, has nothing meaningful to show here). The component-count
    /// group (icon + `"N components"`) is always hidden for this variant —
    /// it's Forge's own dedicated entry point, and Forge isn't a component
    /// showcase, so that count has no meaning here.
    pub fn status_bar_element_with_leading_and_trailing(
        &self,
        leading: Vec<AnyElement>,
        trailing: Vec<AnyElement>,
        title_override: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> StatusBar {
        let (total_components, _current_story) = self.active_story_info(cx);
        let title = title_override.unwrap_or_default();
        self.render_status_bar(leading, trailing, total_components, title, false, cx)
    }
}
