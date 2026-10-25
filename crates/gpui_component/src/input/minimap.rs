use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    App, Background, Bounds, Context, Entity, Font, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Pixels, Render, SharedString, Size, Styled as _, TextAlign,
    TextRun, Window, canvas, div, fill, point, px,
};
use gpui_base::input::DiagnosticSeverity;

use super::{EditorState, RopeExt as _};
use crate::{ActiveTheme, highlighter::diagnostic_foreground};

/// Total order for "which severity wins" when a single sampled minimap row
/// represents several real source lines with different diagnostics —
/// matches the usual editor convention (an error mark always shows through
/// a warning on the same overview-ruler slot).
fn severity_rank(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 3,
        DiagnosticSeverity::Warning => 2,
        DiagnosticSeverity::Info => 1,
        DiagnosticSeverity::Hint => 0,
    }
}

/// Fixed per-line height inside the minimap, independent of the real
/// editor's font size — the same "always readable at a glance" convention
/// VSCode's minimap uses (it does not grow lines to fill the panel for a
/// short file, it just leaves blank space below).
const MINI_LINE_HEIGHT: Pixels = px(3.0);
/// The glyph size shaped for each mini line. Deliberately smaller than
/// `MINI_LINE_HEIGHT` so glyphs don't clip into the next row.
const MINI_FONT_SIZE: Pixels = px(2.4);
const MINIMAP_WIDTH: Pixels = px(112.0);
/// A source line this long is almost certainly minified/generated content —
/// shaping it at full length every repaint would be wasted work no one can
/// read at 2px anyway. Mirrors `element.rs`'s own `MAX_HIGHLIGHT_LINE_LENGTH`
/// guard for the exact same reason.
const MAX_LINE_BYTES: usize = 2_000;

/// A bird's-eye view of an `EditorState`'s buffer, scaled down to fit a
/// narrow fixed-width strip, with a viewport-indicator rectangle and
/// click/drag-to-scroll — the code-editor equivalent of `gpui-flow`'s
/// `Minimap` for the node canvas. Renders real (tiny) glyphs with real
/// syntax colors via [`EditorState::highlighted_runs`], not an approximated
/// "ink density" bar per line.
///
/// Deliberately does **not** attempt to lay out and shape every line of a
/// huge file every repaint: lines are sampled at a stride so at most one
/// mini-line-height's worth of source lines are shaped per repaint (see
/// `MinimapScale`) — real text throughout, just not every single line for a
/// file taller than the panel. No cross-frame shape cache yet; revisit if
/// this shows up in profiling on a genuinely large file.
pub struct EditorMinimap {
    state: Entity<EditorState>,
    /// This div's own screen bounds, captured each paint and read back by
    /// the mouse handlers — mouse events report *window* coordinates, not
    /// bounds relative to this div. Same fix as `gpui-flow`'s `Minimap`
    /// needed for the node-canvas minimap, applied here up front.
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl EditorMinimap {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl Render for EditorMinimap {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state_for_canvas = self.state.clone();
        let state_for_mouse = self.state.clone();
        let bounds_for_layout = self.bounds.clone();
        let bounds_for_down = self.bounds.clone();
        let bounds_for_move = self.bounds.clone();

        div()
            .id("editor-minimap")
            .w(MINIMAP_WIDTH)
            .h_full()
            .flex_shrink_0()
            .bg(cx.theme().editor_background())
            .border_l_1()
            .border_color(cx.theme().border)
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, _window, _cx| {
                        bounds_for_layout.set(bounds);
                    },
                    move |bounds, _: (), window, cx| {
                        paint_minimap(&bounds, &state_for_canvas, window, cx);
                    },
                )
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, {
                let state = state_for_mouse.clone();
                move |event, _window, cx| {
                    let mm_bounds = bounds_for_down.get();
                    let my = event.position.y.as_f32() - mm_bounds.origin.y.as_f32();
                    scroll_to_minimap_point(&state, my, mm_bounds.size.height, cx);
                }
            })
            .on_mouse_move({
                let state = state_for_mouse.clone();
                move |event, _window, cx| {
                    if event.pressed_button == Some(MouseButton::Left) {
                        let mm_bounds = bounds_for_move.get();
                        let my = event.position.y.as_f32() - mm_bounds.origin.y.as_f32();
                        scroll_to_minimap_point(&state, my, mm_bounds.size.height, cx);
                    }
                }
            })
    }
}

/// How many real source lines a single mini-line-height row represents, and
/// how many mini rows are actually drawn — both derived from how many real
/// lines exist vs. how many mini rows fit in `available_height`. Shared by
/// the paint pass and the click/drag handler so a click lands on the same
/// row it visually appears over.
struct MinimapScale {
    stride: usize,
    rendered_rows: usize,
    total_lines: usize,
}

impl MinimapScale {
    fn compute(total_lines: usize, available_height: Pixels) -> Self {
        let max_rows = ((available_height / MINI_LINE_HEIGHT) as usize).max(1);
        if total_lines <= max_rows {
            return Self {
                stride: 1,
                rendered_rows: total_lines.max(1),
                total_lines,
            };
        }
        let stride = total_lines.div_ceil(max_rows).max(1);
        Self {
            stride,
            rendered_rows: total_lines.div_ceil(stride),
            total_lines,
        }
    }

    /// Real source line -> mini-space y offset (top of that line's row).
    fn real_line_to_mini_y(&self, real_line: f32) -> Pixels {
        (real_line / self.stride as f32) * MINI_LINE_HEIGHT
    }

    /// Mini-space y offset -> the real source line it falls on.
    fn mini_y_to_real_line(&self, mini_y: f32) -> f32 {
        ((mini_y / MINI_LINE_HEIGHT.as_f32()) * self.stride as f32)
            .clamp(0.0, self.total_lines as f32)
    }
}

fn paint_minimap(
    bounds: &Bounds<Pixels>,
    state: &Entity<EditorState>,
    window: &mut Window,
    cx: &mut App,
) {
    // `state.read(cx)` ties its returned reference to `cx`'s lifetime, but
    // painting below needs `cx` mutably (for `shaped.paint`) — so each piece
    // of data is pulled out into an owned local in its own short-lived
    // read, rather than holding one `&EditorState` across the whole
    // function. `Rope::clone()` is cheap (persistent tree, effectively an
    // `Arc` bump), so cloning it up front to iterate over is fine.
    let rope = state.read(cx).text().clone();
    let total_lines = rope.lines_len().max(1);
    let scale = MinimapScale::compute(total_lines, bounds.size.height);

    let bx = bounds.origin.x.as_f32();
    let by = bounds.origin.y.as_f32();
    let base_color = cx.theme().muted_foreground;
    let base_font = window.text_style().font();

    // Highest severity present on each real source line, so a stride'd row
    // (representing several real lines on a large file) shows whichever one
    // an editor would surface first. Collected up front — a fresh, short
    // `.read(cx)` like the rest of this function's per-statement reads —
    // rather than re-walking the whole `DiagnosticSet` per row.
    let diag_by_line: BTreeMap<usize, DiagnosticSeverity> = state
        .read(cx)
        .diagnostics()
        .map(|set| {
            let mut map = BTreeMap::<usize, DiagnosticSeverity>::new();
            for entry in set.iter() {
                let line = rope.offset_to_point(entry.range.start).row;
                map.entry(line)
                    .and_modify(|existing| {
                        if severity_rank(entry.diagnostic.severity) > severity_rank(*existing) {
                            *existing = entry.diagnostic.severity;
                        }
                    })
                    .or_insert(entry.diagnostic.severity);
            }
            map
        })
        .unwrap_or_default();

    // Draw only sampled rows — real text every time, just not every row for
    // a file taller than the panel (see the struct doc comment).
    for row in 0..scale.rendered_rows {
        let real_line = row * scale.stride;
        if real_line >= total_lines {
            break;
        }

        let y = by + scale.real_line_to_mini_y(real_line as f32).as_f32();
        let line_range_end = (real_line + scale.stride).min(total_lines);
        if let Some(&severity) = diag_by_line
            .range(real_line..line_range_end)
            .map(|(_, s)| s)
            .max_by_key(|s| severity_rank(**s))
        {
            // A full-width low-opacity tint reads at a glance even on a
            // short file where the sampled row is only a few pixels tall
            // (see the thread that led here — a thin edge marker was easy
            // to miss entirely). A slightly stronger solid bar on the right
            // edge on top of it keeps a crisp anchor point, same as an
            // overview-ruler mark.
            let tint = diagnostic_foreground(severity, cx).opacity(0.28);
            let row_fill = Bounds::new(
                point(px(bx), px(y)),
                Size {
                    width: bounds.size.width,
                    height: MINI_LINE_HEIGHT,
                },
            );
            window.paint_quad(fill(row_fill, tint));

            let edge_color = diagnostic_foreground(severity, cx);
            let marker = Bounds::new(
                point(px(bx + bounds.size.width.as_f32() - 4.0), px(y)),
                Size {
                    width: px(4.0),
                    height: MINI_LINE_HEIGHT,
                },
            );
            window.paint_quad(fill(marker, edge_color));
        }

        let mut line_text = rope.slice_line(real_line).to_string();
        if line_text.len() > MAX_LINE_BYTES {
            let mut end = MAX_LINE_BYTES.min(line_text.len());
            while end > 0 && !line_text.is_char_boundary(end) {
                end -= 1;
            }
            line_text.truncate(end);
        }
        if line_text.trim().is_empty() {
            continue;
        }

        let byte_start = rope.line_start_offset(real_line);
        let byte_end = byte_start + line_text.len();
        let runs = build_runs(
            state.read(cx),
            byte_start..byte_end,
            &line_text,
            &base_font,
            base_color,
        );

        let shaped = window.text_system().shape_line(
            SharedString::from(line_text),
            MINI_FONT_SIZE,
            &runs,
            None,
        );

        let _ = shaped.paint(
            point(px(bx + 4.0), px(y)),
            MINI_LINE_HEIGHT,
            TextAlign::Left,
            None,
            window,
            cx,
        );
    }

    // Viewport indicator, same visual language as `gpui-flow`'s minimap.
    let Some(real_line_height) = state.read(cx).line_height() else {
        return;
    };
    let Some(viewport_height) = state.read(cx).text_bounds().map(|b| b.size.height) else {
        return;
    };
    let scroll_top_real = -state.read(cx).scroll_offset().y;
    let top_line = (scroll_top_real / real_line_height).max(0.0);
    let visible_lines = viewport_height / real_line_height;

    let vy = by + scale.real_line_to_mini_y(top_line).as_f32();
    let vh = (scale.real_line_to_mini_y(top_line + visible_lines).as_f32()
        - scale.real_line_to_mini_y(top_line).as_f32())
    .max(MINI_LINE_HEIGHT.as_f32());
    let vp_bounds = Bounds::new(
        point(px(bx), px(vy)),
        Size {
            width: bounds.size.width,
            height: px(vh),
        },
    );
    window.paint_quad(fill(vp_bounds, gpui::rgba(0x3b82f622)));
    let border: Background = gpui::rgba(0x3b82f688).into();
    let top = Bounds::new(
        point(px(bx), px(vy)),
        Size {
            width: bounds.size.width,
            height: px(1.0),
        },
    );
    window.paint_quad(fill(top, border.clone()));
    let bottom_y = vy + vh;
    let bottom = Bounds::new(
        point(px(bx), px(bottom_y)),
        Size {
            width: bounds.size.width,
            height: px(1.0),
        },
    );
    window.paint_quad(fill(bottom, border));
}

/// Real syntax-highlighted `TextRun`s for one source line, falling back to a
/// single unstyled run when `EditorState::highlighted_runs` returns `None`
/// (no grammar registered for this language, or not in code-editor mode) —
/// same fallback `element.rs`'s own rendering path takes.
fn build_runs(
    state: &EditorState,
    byte_range: std::ops::Range<usize>,
    line_text: &str,
    font: &Font,
    base_color: Hsla,
) -> Vec<TextRun> {
    let unstyled = || {
        vec![TextRun {
            len: line_text.len(),
            font: font.clone(),
            color: base_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }]
    };

    let Some(styles) = state.highlighted_runs(byte_range.clone()) else {
        return unstyled();
    };

    let mut runs = Vec::with_capacity(styles.len());
    for (range, style) in styles {
        let start = range.start.max(byte_range.start) - byte_range.start;
        let end = range
            .end
            .min(byte_range.end)
            .saturating_sub(byte_range.start);
        if end <= start || start >= line_text.len() {
            continue;
        }
        let end = end.min(line_text.len());
        runs.push(TextRun {
            len: end - start,
            font: font.clone(),
            color: style.color.unwrap_or(base_color),
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    let covered: usize = runs.iter().map(|r| r.len).sum();
    if covered < line_text.len() {
        runs.push(TextRun {
            len: line_text.len() - covered,
            font: font.clone(),
            color: base_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
    if runs.is_empty() {
        return unstyled();
    }

    runs
}

fn scroll_to_minimap_point(
    state: &Entity<EditorState>,
    my: f32,
    minimap_height: Pixels,
    cx: &mut App,
) {
    state.update(cx, |state, cx| {
        let total_lines = state.text().lines_len().max(1);
        let Some(real_line_height) = state.line_height() else {
            return;
        };
        let Some(viewport_height) = state.text_bounds().map(|b| b.size.height) else {
            return;
        };
        let content_height = real_line_height * total_lines as f32;
        let max_scroll = (content_height - viewport_height).max(Pixels::ZERO);

        let scale = MinimapScale::compute(total_lines, minimap_height);
        let target_line = scale.mini_y_to_real_line(my);
        let target_top = (real_line_height * target_line - viewport_height * 0.5)
            .clamp(Pixels::ZERO, max_scroll);

        let x = state.scroll_offset().x;
        state.set_scroll_offset(point(x, -target_top), cx);
    });
}
