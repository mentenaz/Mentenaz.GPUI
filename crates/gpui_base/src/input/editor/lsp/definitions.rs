use anyhow::Result;
use gpui::{
    App, Context, HighlightStyle, Hitbox, MouseDownEvent, Task, UnderlineStyle, Window, px,
};
use ropey::Rope;
use std::{ops::Range, rc::Rc};

use crate::input::{EditorMode, GoToDefinition, InputBaseState, RopeExt};

/// Definition provider
///
/// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
pub trait DefinitionProvider {
    /// textDocument/definition
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
    fn definitions(
        &self,
        _text: &Rope,
        _offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>>;
}

#[derive(Clone, Default)]
pub(crate) struct HoverDefinition {
    /// The range of the symbol that triggered the hover.
    symbol_range: Range<usize>,
    /// The word range a `textDocument/definition` request is currently
    /// in flight for, if any. Real mouse input never holds perfectly still
    /// while Ctrl is down — tiny sub-pixel jitter fires a fresh
    /// `MouseMoveEvent` on nearly every frame, and each one used to replace
    /// `_hover_task` outright, cancelling the previous request before its
    /// round trip could ever complete. As long as the new offset falls
    /// within this same pending word, `handle_hover_definition` now skips
    /// re-requesting so the in-flight lookup gets a chance to resolve.
    pending_range: Option<Range<usize>>,
    pub(crate) locations: Rc<Vec<lsp_types::LocationLink>>,
    last_location: Option<(Range<usize>, Rc<Vec<lsp_types::LocationLink>>)>,
}

impl HoverDefinition {
    pub(crate) fn update(
        &mut self,
        symbol_range: Range<usize>,
        locations: Vec<lsp_types::LocationLink>,
    ) {
        self.clear();
        self.symbol_range = symbol_range;
        self.locations = Rc::new(locations);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        if !self.locations.is_empty() {
            self.last_location = Some((self.symbol_range.clone(), self.locations.clone()));
        }

        self.symbol_range = 0..0;
        self.locations = Rc::new(vec![]);
        self.pending_range = None;
    }

    pub(crate) fn is_same(&self, offset: usize) -> bool {
        self.symbol_range.contains(&offset)
    }

    /// Whether `offset` falls within a word a definition lookup is already
    /// in flight for — see `pending_range`'s doc comment.
    pub(crate) fn is_pending(&self, offset: usize) -> bool {
        self.pending_range
            .as_ref()
            .is_some_and(|range| range.contains(&offset) || range.end == offset)
    }

    pub(crate) fn set_pending(&mut self, range: Range<usize>) {
        self.pending_range = Some(range);
    }
}

impl InputBaseState<EditorMode> {
    pub(crate) fn handle_hover_definition(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self.extras.lsp.definition_provider.clone() else {
            return;
        };

        if self.extras.hover_definition.is_same(offset) {
            return;
        }

        let mut symbol_range = self.text.word_range(offset).unwrap_or(offset..offset);

        // Mouse input jitters even while "holding still" over a word — every
        // sub-pixel move fires another `MouseMoveEvent`, and each one used to
        // stomp `_hover_task`, cancelling the previous request before its LSP
        // round trip could ever finish. As long as we're still inside the
        // word a lookup is already running for, don't restart it.
        if self.extras.hover_definition.is_pending(offset) {
            return;
        }
        self.extras
            .hover_definition
            .set_pending(symbol_range.clone());

        let task = provider.definitions(&self.text, offset, window, cx);
        let editor = cx.entity();
        self.extras.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let locations = task.await?;

            editor.update(cx, |editor, cx| {
                if locations.is_empty() {
                    editor.extras.hover_definition.clear();
                } else {
                    if let Some(location) = locations.first()
                        && let Some(range) = location.origin_selection_range {
                            let start = editor.text.position_to_offset(&range.start);
                            let end = editor.text.position_to_offset(&range.end);
                            symbol_range = start..end;
                        }

                    editor
                        .extras
                        .hover_definition
                        .update(symbol_range.clone(), locations.clone());
                }
                cx.notify();
            });

            Ok(())
        });
    }

    pub(crate) fn on_action_go_to_definition(
        &mut self,
        _: &GoToDefinition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.cursor();
        if let Some((symbol_range, locations)) = self.extras.hover_definition.last_location.clone()
        {
            if !(symbol_range.start..=symbol_range.end).contains(&offset) {
                return;
            }

            if let Some(location) = locations.first().cloned() {
                self.go_to_definition(&location, window, cx);
            }
        }
    }

    /// Return true if handled.
    pub(crate) fn handle_click_hover_definition(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<InputBaseState<EditorMode>>,
    ) -> bool {
        if !event.modifiers.secondary() {
            return false;
        }

        if self.extras.hover_definition.is_empty() {
            return false;
        };
        if !self.extras.hover_definition.is_same(offset) {
            return false;
        }

        let Some(location) = self.extras.hover_definition.locations.first().cloned() else {
            return false;
        };

        self.go_to_definition(&location, window, cx);

        true
    }

    pub(crate) fn go_to_definition(
        &mut self,
        location: &lsp_types::LocationLink,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scheme = location.target_uri.scheme();
        let external = scheme == "https" || scheme == "http";

        // Give the host a chance to show the document first (window/showDocument),
        // e.g. to open virtual/external documents (stdlib docs) in an app window.
        if let Some(handler) = self.extras.lsp.show_document.clone() {
            let params = lsp_types::ShowDocumentParams {
                uri: location.target_uri.clone(),
                external: Some(external),
                take_focus: Some(true),
                selection: Some(location.target_selection_range),
            };
            if handler(&params, window, cx) {
                return;
            }
        }

        if external {
            cx.open_url(&location.target_uri.to_string());
        } else {
            // Move to the location.
            let target_range = location.target_selection_range;
            let start = self.text.position_to_offset(&target_range.start);
            let end = self.text.position_to_offset(&target_range.end);

            self.move_to(start, None, cx);
            self.select_to(end, cx);
        }
    }
}

/// These read the editor's own state; they sit here rather than on the element
/// because the element has nothing to do with the answer.
impl InputBaseState<EditorMode> {
    /// The range highlighted while Cmd-hovering a symbol, with its style.
    pub(crate) fn hover_definition_style(&self) -> Option<(Range<usize>, HighlightStyle)> {
        let editor = self;
        if editor.extras.hover_definition.is_empty() {
            return None;
        };

        let mut highlight_style = editor.editor_style.highlight_styles.style("link_text")?;

        highlight_style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            ..UnderlineStyle::default()
        });

        Some((
            editor.extras.hover_definition.symbol_range.clone(),
            highlight_style,
        ))
    }

    /// The hitbox that makes a Cmd-hovered symbol clickable.
    pub(crate) fn hover_definition_hitbox(&self, window: &mut Window) -> Option<Hitbox> {
        let editor = self;
        if editor.extras.hover_definition.is_empty() {
            return None;
        };

        let Some(bounds) = editor.range_to_bounds(&editor.extras.hover_definition.symbol_range)
        else {
            return None;
        };

        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }
}
