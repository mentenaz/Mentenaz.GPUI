use gpui::*;

use crate::store::FlowState;

/// A controls panel with zoom +/-, fit-view, and lock buttons.
///
/// Position this absolutely in a corner of your flow graph container.
pub struct Controls {
    state: Entity<FlowState>,
    container_size: (f32, f32),
    background: Hsla,
    border: Hsla,
    foreground: Hsla,
}

impl Controls {
    pub fn new(state: Entity<FlowState>) -> Self {
        Self {
            state,
            container_size: (900.0, 600.0),
            background: gpui::rgba(0x18181b_ee).into(),
            border: gpui::rgb(0x27272a).into(),
            foreground: gpui::rgb(0xa1a1aa).into(),
        }
    }

    /// Set the canvas size to assume (for zoom centering and fit-view) until
    /// the graph has been laid out. After that the canvas's real size is
    /// used (`FlowState::canvas_size`).
    pub fn container_size(mut self, width: f32, height: f32) -> Self {
        self.container_size = (width, height);
        self
    }

    pub fn theme_colors(&mut self, background: Hsla, border: Hsla, foreground: Hsla) {
        self.background = background;
        self.border = border;
        self.foreground = foreground;
    }
}

impl Render for Controls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state_zoom_in = self.state.clone();
        let state_zoom_out = self.state.clone();
        let state_fit = self.state.clone();
        // The size to assume until the graph has reported its real one.
        let fallback = self.container_size;
        let entity_id = cx.entity_id();
        let background = self.background;
        let border = self.border;
        let foreground = self.foreground;

        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_1()
            .bg(background)
            .rounded_md()
            .border_1()
            .border_color(border)
            .child(control_button("+", foreground, {
                move |_, _, cx| {
                    state_zoom_in.update(cx, |state, _| {
                        let (cw, ch) = state.canvas_size().unwrap_or(fallback);
                        state.zoom_in(cw, ch);
                    });
                    cx.notify(entity_id);
                }
            }))
            .child(control_button("\u{2212}", foreground, {
                // Unicode minus sign
                move |_, _, cx| {
                    state_zoom_out.update(cx, |state, _| {
                        let (cw, ch) = state.canvas_size().unwrap_or(fallback);
                        state.zoom_out(cw, ch);
                    });
                    cx.notify(entity_id);
                }
            }))
            .child(div().h(px(1.0)).bg(border).mx_1())
            .child(control_button("\u{2922}", foreground, {
                // Fit view icon (↢)
                move |_, _, cx| {
                    state_fit.update(cx, |state, _| {
                        let (cw, ch) = state.canvas_size().unwrap_or(fallback);
                        state.fit_view(40.0, cw, ch);
                    });
                    cx.notify(entity_id);
                }
            }))
    }
}

fn control_button(
    label: &'static str,
    foreground: Hsla,
    on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(label))
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .cursor(CursorStyle::PointingHand)
        .text_sm()
        .text_color(foreground)
        .on_mouse_down(MouseButton::Left, on_click)
        .child(label)
}
