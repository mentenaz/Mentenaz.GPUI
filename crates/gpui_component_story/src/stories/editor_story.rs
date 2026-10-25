use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, Styled, Subscription, Window, actions, div,
    prelude::FluentBuilder as _, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, Selectable as _, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::DialogButtonProps,
    h_flex,
    input::*,
    switch::Switch,
    tab::{Tab, TabBar},
    v_flex,
};

actions!(editor_story, [Save]);

const CONTEXT: &str = "EditorStory";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-s", Save, Some(CONTEXT))]);
}

/// Emitted when the user saves the active tab (`Ctrl`/`Cmd`-`S`, or the
/// "Save" button) — `EditorStory` can't write to disk itself (framework/app
/// boundary: `gpui-component-story` doesn't depend on `forge_gpui`'s
/// `backend`), so a host application subscribes to this and does the actual
/// write. See `Gallery::editor_story`, which `ForgeShell` uses to get a
/// handle to subscribe to.
pub struct SaveRequested {
    pub path: SharedString,
    pub content: SharedString,
}

impl EventEmitter<SaveRequested> for EditorStory {}

/// Emitted when the user confirms "Disregard changes" on a dirty tab — like
/// `SaveRequested`, `EditorStory` can't read the file itself, so a host
/// application subscribes, re-reads `path` from disk, and calls
/// `force_reload` with the result.
pub struct DiscardRequested {
    pub path: SharedString,
}

impl EventEmitter<DiscardRequested> for EditorStory {}

/// Emitted on every edit to an open tab's content (piggybacks on the same
/// `InputEvent::Change` that already drives dirty-tab tracking below) — like
/// `SaveRequested`, `EditorStory` can't reach a language server itself
/// (framework/app boundary), so a host application subscribes and forwards
/// this to whatever LSP client is attached for `path` (e.g.
/// `textDocument/didChange`).
pub struct ContentChanged {
    pub path: SharedString,
    pub content: SharedString,
}

impl EventEmitter<ContentChanged> for EditorStory {}

/// One open file, backing one tab.
struct OpenTab {
    path: String,
    label: SharedString,
    editor_state: Entity<EditorState>,
    /// Created once alongside `editor_state`, not per-render — it caches
    /// its own screen bounds across frames (see `EditorMinimap`'s doc
    /// comment) between the paint that sets them and the next click that
    /// reads them back, which a fresh `cx.new(..)` every render would lose.
    minimap_state: Entity<EditorMinimap>,
    /// Whether the buffer has edits since it was opened/saved/last synced
    /// with disk — shown as a dot on the tab, and checked by
    /// `reload_if_open` so an external change never silently clobbers
    /// in-progress edits.
    is_dirty: bool,
    _change_subscription: Subscription,
}

/// Empty until `open` is called — this story has no content of its own
/// (unlike `gpui-component`'s original demo, which showed example code and
/// a "Decorations" highlighting showcase by default) since `forge_gpui`'s
/// `ForgeShell` reuses it purely as the real, file-backed editor surface
/// behind the Explorer panel. See `Gallery::open_file`.
pub struct EditorStory {
    tabs: Vec<OpenTab>,
    active_tab: usize,
    readonly: bool,
    /// When true, suppress the tab bar and toolbar row — the host application
    /// manages its own tab bar (e.g. `ForgeShell`'s workspace tab strip).
    bare: bool,
}
impl super::Story for EditorStory {
    fn title() -> &'static str {
        "Editor"
    }

    fn description() -> &'static str {
        "Code editor with theme-aware syntax highlighting and folding."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl EditorStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self {
            tabs: Vec::new(),
            active_tab: 0,
            readonly: false,
            bare: false,
        })
    }

    /// Hide the story's own tab bar and toolbar. Call this when the host
    /// application manages its own tab strip (e.g. `ForgeShell`).
    pub fn set_bare(&mut self, bare: bool, cx: &mut Context<Self>) {
        self.bare = bare;
        cx.notify();
    }

    /// Activates the tab for `path` if it's already open, otherwise opens a
    /// new one — added for a host application (`ForgeShell` in `forge_gpui`)
    /// to show a real file here.
    pub fn open(
        &mut self,
        path: impl Into<SharedString>,
        content: impl Into<SharedString>,
        language: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = path.into().to_string();
        if let Some(ix) = self.tabs.iter().position(|tab| tab.path == path) {
            self.active_tab = ix;
            // Without this, switching tabs any way other than clicking
            // directly into the editor's own text area (a tab-bar click, a
            // keyboard tab switcher, ...) leaves keyboard focus wherever it
            // was — typing goes nowhere until the user clicks into the
            // content a second time.
            self.tabs[ix]
                .editor_state
                .update(cx, |state, cx| state.focus(window, cx));
            cx.notify();
            return;
        }

        let label = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let editor_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .folding(true)
                .default_value(content)
        });
        editor_state.update(cx, |state, cx| state.set_highlighter(language, cx));
        configure_wasm_highlighter(&editor_state, cx);

        // `set_value` (used by `reload_if_open` for a silent external-change
        // refresh) explicitly suppresses `InputEvent::Change`, so this only
        // fires for the user's own edits — exactly what "dirty" should mean.
        let dirty_path = path.clone();
        let change_subscription = cx.subscribe(&editor_state, move |this, state, event, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            if let Some(tab) = this.tabs.iter_mut().find(|tab| tab.path == dirty_path)
                && !tab.is_dirty {
                    tab.is_dirty = true;
                    cx.notify();
                }
            cx.emit(ContentChanged {
                path: dirty_path.clone().into(),
                content: state.read(cx).value(),
            });
        });

        let minimap_state = cx.new(|_| EditorMinimap::new(&editor_state));

        editor_state.update(cx, |state, cx| state.focus(window, cx));
        self.tabs.push(OpenTab {
            path,
            label: label.into(),
            editor_state,
            minimap_state,
            is_dirty: false,
            _change_subscription: change_subscription,
        });
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }

    /// If `path` has an open tab, silently refreshes its content from disk —
    /// added for a host application to pick up an external change to a file
    /// that's already open (see `backend::notify_file_changed`/
    /// `subscribe_file_changed`). No-ops if that tab has unsaved edits, so
    /// this never clobbers in-progress work — the tab's dirty dot already
    /// signals "don't just discard this."
    pub fn reload_if_open(
        &mut self,
        path: impl Into<SharedString>,
        content: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = path.into().to_string();
        let Some(tab) = self.tabs.iter().find(|tab| tab.path == path) else {
            return;
        };
        if tab.is_dirty {
            return;
        }
        tab.editor_state
            .update(cx, |state, cx| state.set_value(content, window, cx));
    }

    fn set_active_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix < self.tabs.len() {
            self.active_tab = ix;
            self.tabs[ix]
                .editor_state
                .update(cx, |state, cx| state.focus(window, cx));
            cx.notify();
        }
    }

    fn close_tab(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.tabs.len() {
            return;
        }
        self.tabs.remove(ix);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        } else if ix < self.active_tab {
            self.active_tab -= 1;
        }
        cx.notify();
    }

    fn on_action_save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        tab.is_dirty = false;
        let content = tab.editor_state.read(cx).value();
        cx.emit(SaveRequested {
            path: tab.path.clone().into(),
            content,
        });
        cx.notify();
    }

    fn on_discard_confirmed(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return;
        };
        cx.emit(DiscardRequested {
            path: tab.path.clone().into(),
        });
    }

    /// Reads `path`'s tab's *current* buffer content — including unsaved
    /// edits, unlike `SaveRequested`'s payload which only arrives on an
    /// explicit save. Added for a host application (`ForgeShell`'s
    /// Designer, in `forge_gpui`) that needs to pull whatever's on screen
    /// back out without requiring the user to save first — e.g. syncing a
    /// visual graph editor's model when its user switches away from this
    /// editor's raw-text view of the same underlying data.
    pub fn content_for(&self, path: impl Into<SharedString>, cx: &App) -> Option<SharedString> {
        let path = path.into().to_string();
        self.tabs
            .iter()
            .find(|tab| tab.path == path)
            .map(|tab| tab.editor_state.read(cx).value())
    }

    /// Whether `path`'s open tab has unsaved edits — `false` if `path`
    /// isn't currently open at all. Added for a host application that
    /// renders its own tab strip in place of this story's (see `set_bare`)
    /// and needs the same dirty-dot indicator `self.tabs`' own suppressed
    /// tab bar would have shown.
    pub fn is_dirty_for(&self, path: impl Into<SharedString>) -> bool {
        let path = path.into().to_string();
        self.tabs
            .iter()
            .find(|tab| tab.path == path)
            .map(|tab| tab.is_dirty)
            .unwrap_or(false)
    }

    /// The `Entity<EditorState>` backing `path`'s open tab, if any — added
    /// for a host application that needs to reach into a specific tab's
    /// editor state directly (e.g. attaching an LSP hover provider, or
    /// pushing `textDocument/publishDiagnostics` results) rather than just
    /// reading/writing its text content like `content_for`/`reload_if_open`.
    ///
    /// Falls back to a case-insensitive match if no tab's path is an exact
    /// match — Windows paths are case-insensitive, but a path recovered
    /// from a `file://` URI a *language server* generated (rather than one
    /// this app built itself) can differ only in case, e.g. a lowercased
    /// drive letter (`c:` vs `C:`), which the WHATWG URL Standard's file-URL
    /// handling normalizes to. Without this, a caller matching server URIs
    /// back to open tabs would silently find nothing on a real mismatch
    /// like that.
    pub fn editor_state_for(&self, path: impl Into<SharedString>) -> Option<Entity<EditorState>> {
        let path = path.into().to_string();
        self.tabs
            .iter()
            .find(|tab| tab.path == path)
            .or_else(|| {
                self.tabs
                    .iter()
                    .find(|tab| tab.path.eq_ignore_ascii_case(&path))
            })
            .map(|tab| tab.editor_state.clone())
    }

    /// Unconditionally replaces `path`'s tab content with `content` (freshly
    /// read from disk) and clears its dirty flag — unlike `reload_if_open`,
    /// which refuses to touch a dirty tab. Used after the user confirms
    /// "Disregard changes" in response to `DiscardRequested`.
    pub fn force_reload(
        &mut self,
        path: impl Into<SharedString>,
        content: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = path.into().to_string();
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == path) else {
            return;
        };
        tab.is_dirty = false;
        tab.editor_state
            .update(cx, |state, cx| state.set_value(content, window, cx));
        cx.notify();
    }

    /// Replaces `path`'s tab content with `content` and marks it dirty —
    /// the opposite of `force_reload`, used right after `open` when a host
    /// application (`ForgeShell`'s crash-recovery draft restore, in
    /// `forge_gpui`) has content that differs from what's on disk and wants
    /// that difference to show as unsaved edits (the tab's dirty dot, a
    /// `Save`/discard prompt on close) rather than silently becoming the new
    /// clean baseline. `set_value` suppresses `InputEvent::Change`, so
    /// `is_dirty` needs setting explicitly here the same way `open`'s own
    /// change-subscription callback does for a real edit.
    pub fn restore_draft(
        &mut self,
        path: impl Into<SharedString>,
        content: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = path.into().to_string();
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == path) else {
            return;
        };
        tab.is_dirty = true;
        tab.editor_state
            .update(cx, |state, cx| state.set_value(content, window, cx));
        cx.notify();
    }
}

/// WASM ships without tree-sitter grammars, so every `EditorState` this
/// story creates swaps in the `syntect` adapter below instead. Native keeps
/// the built-in tree-sitter highlighter, which parses incrementally on a
/// background thread — this is a no-op there.
fn configure_wasm_highlighter(_editor_state: &Entity<EditorState>, _cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        _editor_state.update(_cx, |state, cx| {
            state.set_highlighter_factory(
                std::rc::Rc::new(|language| {
                    syntect_highlighter::SyntectHighlighter::new(language)
                        .map(|highlighter| Box::new(highlighter) as Box<_>)
                }),
                cx,
            );
        });
    }
}

impl Render for EditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(active) =
            (!self.tabs.is_empty()).then(|| self.active_tab.min(self.tabs.len() - 1))
        else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("No file open.")
                .into_any_element();
        };
        let active_editor = self.tabs[active].editor_state.clone();
        let active_minimap = self.tabs[active].minimap_state.clone();
        let active_is_dirty = self.tabs[active].is_dirty;
        let view = cx.entity();

        v_flex()
            .size_full()
            .when(!self.bare, |el| el.gap_3())
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_save))
            .when(!self.bare, |el| {
                el.child(
                    h_flex()
                        .justify_between()
                        .child(
                            TabBar::new("editor-story-tabs")
                                .w_full()
                                .selected_index(active)
                                .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                    this.set_active_tab(*ix, window, cx);
                                }))
                                .border_t_1()
                                .border_color(cx.theme().border)
                                .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                                    Tab::new()
                                        .px_2()
                                        .prefix(Icon::new(IconName::File))
                                        .label(tab.label.clone())
                                        .selected(ix == active)
                                        .suffix(
                                            h_flex()
                                                .gap_1()
                                                .items_center()
                                                .when(tab.is_dirty, |el| {
                                                    el.child(
                                                        div()
                                                            .flex_shrink_0()
                                                            .w(px(6.))
                                                            .h(px(6.))
                                                            .rounded_full()
                                                            .bg(cx.theme().warning),
                                                    )
                                                })
                                                .child(
                                                    Button::new(("editor-tab-close", ix))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Close)
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                this.close_tab(ix, window, cx);
                                                            },
                                                        )),
                                                ),
                                        )
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Button::new("editor-save")
                                        .ghost()
                                        .xsmall()
                                        .label("Save")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.on_action_save(&Save, window, cx);
                                        })),
                                )
                                .when(active_is_dirty, |el| {
                                    let view = view.clone();
                                    el.child(
                                        Button::new("editor-discard")
                                            .ghost()
                                            .xsmall()
                                            .label("Disregard changes")
                                            .on_click(move |_, window, cx| {
                                                let view = view.clone();
                                                window.open_alert_dialog(cx, move |alert, _, _| {
                                                    let view = view.clone();
                                                    alert
                                                    .title("Discard unsaved changes?")
                                                    .description(
                                                        "This tab's edits since it was opened or \
                                                        last saved will be permanently lost — the \
                                                        content on disk will be loaded instead.",
                                                    )
                                                    .button_props(
                                                        DialogButtonProps::default()
                                                            .ok_variant(ButtonVariant::Danger)
                                                            .ok_text("Discard")
                                                            .cancel_text("Cancel")
                                                            .show_cancel(true),
                                                    )
                                                    .on_ok(move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.on_discard_confirmed(cx);
                                                        });
                                                        true
                                                    })
                                                });
                                            }),
                                    )
                                })
                                .child(
                                    Switch::new("editor-read-only")
                                        .label("Read only")
                                        .checked(self.readonly)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            this.readonly = *checked;
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
            })
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .child(
                        Editor::new(&active_editor)
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(cx.theme().mono_font_size)
                            .readonly(self.readonly)
                            .flex_1()
                            .min_w_0()
                            .h_full(),
                    )
                    .child(active_minimap),
            )
            .into_any_element()
    }
}

/// A minimal [`InputHighlighter`] built on `syntect`, for WASM builds, which
/// ship without tree-sitter grammars.
#[cfg(target_family = "wasm")]
mod syntect_highlighter {
    use std::{collections::HashMap, ops::Range, sync::LazyLock};

    use gpui::{Context, HighlightStyle, SharedString, Window};
    use gpui_component::input::*;
    use syntect::{
        parsing::{ParseState, Scope, ScopeStack, SyntaxSet},
        util::LinesWithEndings,
    };

    /// Loading the default syntax definitions deserializes a few megabytes, so
    /// share one set across every highlighter instance.
    static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

    pub(super) struct SyntectHighlighter {
        language: SharedString,
        /// Non-overlapping highlights, ordered by start offset.
        highlights: Vec<(Range<usize>, &'static str)>,
        fold_ranges: Vec<FoldRange>,
        /// Scope ids are cheap to compare but expensive to stringify, so
        /// remember the semantic name each one maps to.
        semantic_names: HashMap<Scope, Option<&'static str>>,
    }

    impl SyntectHighlighter {
        pub(super) fn new(language: &str) -> Option<Self> {
            find_syntax(language)?;

            Some(Self {
                language: language.to_owned().into(),
                highlights: Vec::new(),
                fold_ranges: Vec::new(),
                semantic_names: HashMap::new(),
            })
        }

        fn push_highlight(&mut self, range: Range<usize>, scopes: &ScopeStack) {
            if range.is_empty() {
                return;
            }

            let name = scopes.scopes.iter().rev().find_map(|scope| {
                *self
                    .semantic_names
                    .entry(*scope)
                    .or_insert_with(|| semantic_name(*scope))
            });
            if let Some(name) = name {
                self.highlights.push((range, name));
            }
        }
    }

    impl InputHighlighter for SyntectHighlighter {
        fn language(&self) -> SharedString {
            self.language.clone()
        }

        fn update(
            &mut self,
            _edit: Option<InputEdit>,
            text: &Rope,
            folding: bool,
            _window: &mut Window,
            _cx: &mut Context<EditorState>,
        ) {
            // `syntect` has no incremental mode, so the whole document is
            // reparsed. Read the rope once and reuse it for folding too.
            let text = text.to_string();
            let syntax = find_syntax(self.language.as_ref())
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
            let mut parser = ParseState::new(syntax);
            let mut scopes = ScopeStack::new();
            let mut offset = 0;
            self.highlights.clear();

            for line in LinesWithEndings::from(&text) {
                if let Ok(operations) = parser.parse_line(line, &SYNTAX_SET) {
                    let mut cursor = 0;
                    for (index, operation) in operations {
                        self.push_highlight(offset + cursor..offset + index, &scopes);
                        let _ = scopes.apply(&operation);
                        cursor = index;
                    }
                    self.push_highlight(offset + cursor..offset + line.len(), &scopes);
                }
                offset += line.len();
            }

            self.fold_ranges = if folding {
                brace_fold_ranges(&text)
            } else {
                Vec::new()
            };
        }

        fn styles(
            &self,
            range: &Range<usize>,
            resolver: &dyn HighlightStyleResolver,
        ) -> Vec<(Range<usize>, HighlightStyle)> {
            resolve_styles(&self.highlights, range, resolver)
        }

        fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
            self.fold_ranges.clone()
        }

        fn fold_ranges_for_edit(&self, _: Range<usize>, _: &Rope) -> Vec<FoldRange> {
            self.fold_ranges.clone()
        }
    }

    fn find_syntax(language: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
        SYNTAX_SET
            .find_syntax_by_token(language)
            .or_else(|| SYNTAX_SET.find_syntax_by_extension(language))
    }

    /// Turn the highlights overlapping `range` into gap-free style runs.
    ///
    /// `highlights` is ordered and non-overlapping, so the first candidate is
    /// found by binary search instead of scanning the whole document on every
    /// frame.
    fn resolve_styles(
        highlights: &[(Range<usize>, &'static str)],
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let first = highlights.partition_point(|(highlight, _)| highlight.end <= range.start);
        let mut runs = Vec::new();
        let mut cursor = range.start;

        for (highlight_range, name) in &highlights[first..] {
            if highlight_range.start >= range.end {
                break;
            }

            let start = highlight_range.start.max(range.start);
            let end = highlight_range.end.min(range.end);
            if start >= end || end <= cursor {
                continue;
            }
            if cursor < start {
                runs.push((cursor..start, HighlightStyle::default()));
            }
            runs.push((start..end, resolver.style(name).unwrap_or_default()));
            cursor = end;
        }

        if cursor < range.end {
            runs.push((cursor..range.end, HighlightStyle::default()));
        }
        runs
    }

    fn semantic_name(scope: Scope) -> Option<&'static str> {
        let scope = scope.build_string();
        let name = if scope.starts_with("comment") {
            "comment"
        } else if scope.starts_with("constant.character.escape") {
            "string.escape"
        } else if scope.starts_with("string") {
            "string"
        } else if scope.starts_with("constant.numeric") {
            "number"
        } else if scope.starts_with("constant.language.boolean") {
            "boolean"
        } else if scope.starts_with("keyword.operator") {
            "operator"
        } else if scope.starts_with("keyword") || scope.starts_with("storage") {
            "keyword"
        } else if scope.starts_with("entity.name.function") || scope.starts_with("support.function")
        {
            "function"
        } else if scope.starts_with("entity.name.type")
            || scope.starts_with("entity.name.class")
            || scope.starts_with("support.type")
        {
            "type"
        } else if scope.starts_with("variable") {
            "variable"
        } else if scope.starts_with("constant") {
            "constant"
        } else if scope.starts_with("punctuation") {
            "punctuation"
        } else {
            return None;
        };
        Some(name)
    }

    fn brace_fold_ranges(text: &str) -> Vec<FoldRange> {
        let mut starts = Vec::new();
        let mut ranges = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            let mut chars = line.chars().peekable();
            let mut quoted = false;
            let mut escaped = false;
            while let Some(character) = chars.next() {
                if !quoted && character == '/' && chars.peek() == Some(&'/') {
                    break;
                }
                if character == '"' && !escaped {
                    quoted = !quoted;
                } else if !quoted && character == '{' {
                    starts.push(line_number);
                } else if !quoted && character == '}' {
                    if let Some(start_line) = starts.pop() {
                        if start_line < line_number {
                            ranges.push(FoldRange::new(start_line, line_number));
                        }
                    }
                }
                escaped = quoted && character == '\\' && !escaped;
                if character != '\\' {
                    escaped = false;
                }
            }
        }
        ranges
    }
}
