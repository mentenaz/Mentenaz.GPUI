use gpui::{App, Context, Entity, SharedString, Window};

use crate::{EditorStory, Gallery};

impl Gallery {
    /// The "Editor" story's view, if the Gallery has one and it's actually
    /// an `EditorStory` (always true in practice — `new_with_mode` always
    /// registers `StoryContainer::panel::<EditorStory>`). Added for a host
    /// application (`ForgeShell` in `forge_gpui`) to reach it directly —
    /// `open_file` uses this to write to it, and `ForgeShell` also
    /// subscribes to its `SaveRequested` events using the returned handle.
    pub fn editor_story(&self, cx: &App) -> Option<Entity<EditorStory>> {
        self.stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .find(|story| story.read(cx).name == "Editor")
            .and_then(|container| container.read(cx).story_view())
            .and_then(|view| view.downcast::<EditorStory>().ok())
    }

    /// Navigates to the "Editor" story and loads `content` into it,
    /// replacing whatever it was last showing. Added for a host application
    /// (`ForgeShell` in `forge_gpui`) to open real files from its own file
    /// tree — the Gallery itself has no notion of a filesystem, it just
    /// hosts the story that can display arbitrary text.
    ///
    /// Returns `false` if the "Editor" story isn't registered (see
    /// `editor_story`).
    pub fn open_file(
        &mut self,
        path: impl Into<SharedString>,
        content: impl Into<SharedString>,
        language: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.select_story("Editor", window, cx) {
            return false;
        }
        let Some(editor) = self.editor_story(cx) else {
            return false;
        };
        editor.update(cx, |editor, cx| {
            editor.open(path, content, language, window, cx)
        });
        true
    }

    /// Replaces an already-open tab's content and marks it dirty — added
    /// for a host application (`ForgeShell`'s crash-recovery draft restore,
    /// in `forge_gpui`) to layer recovered-but-unsaved content onto a tab
    /// right after `open_file` opened it from disk. See
    /// `EditorStory::restore_draft`. No-op if the "Editor" story isn't
    /// registered or `path` isn't already open.
    pub fn restore_draft(
        &mut self,
        path: impl Into<SharedString>,
        content: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(editor) = self.editor_story(cx) else {
            return false;
        };
        editor.update(cx, |editor, cx| {
            editor.restore_draft(path, content, window, cx)
        });
        true
    }
}
