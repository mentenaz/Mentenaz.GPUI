use gpui::{App, Entity, Window, prelude::*};

use crate::Gallery;

impl Gallery {
    pub fn view(init_story: Option<&str>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(init_story, window, cx))
    }

    pub fn embedded_view(story: &str, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new_with_mode(Some(story), true, false, false, window, cx))
    }

    /// Like [`Gallery::view`], but without Gallery's own bottom status bar —
    /// for a host application that wants to render that status bar itself
    /// (via [`Gallery::status_bar_element`]) full-width outside Gallery's own
    /// content column. See `ForgeShell` in `forge_gpui`.
    pub fn view_without_status_bar(
        init_story: Option<&str>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new_with_mode(init_story, false, true, false, window, cx))
    }

    /// Like [`Gallery::view_without_status_bar`], but also without Gallery's
    /// own sidebar — for a host application that wants to render the story
    /// list itself elsewhere (via [`Gallery::sidebar_element`]), keeping only
    /// the header/content column here. Since it's the same `Gallery` entity,
    /// selecting a story from the host-rendered sidebar still updates this
    /// content column — no extra state syncing needed. See `ForgeShell` in
    /// `forge_gpui`.
    pub fn view_without_status_bar_and_sidebar(
        init_story: Option<&str>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new_with_mode(init_story, false, true, true, window, cx))
    }
}
