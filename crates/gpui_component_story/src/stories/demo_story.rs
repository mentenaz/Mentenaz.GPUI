use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled as _, Window, div,
};

use gpui_component::{
    ActiveTheme, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::Story;

pub struct DemoStory {
    focus_handle: FocusHandle,
    count: usize,
}

impl DemoStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            count: 0,
        }
    }
}

impl Story for DemoStory {
    fn title() -> &'static str {
        "Demo"
    }

    fn description() -> &'static str {
        "A small interactive example panel."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for DemoStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DemoStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                v_flex()
                    .items_center()
                    .gap_3()
                    .p_6()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary)
                    .child(div().text_3xl().font_bold().child(self.count.to_string()))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("demo-increment")
                                    .primary()
                                    .label("Increment")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.count += 1;
                                        cx.notify();
                                    })),
                            )
                            .child(Button::new("demo-reset").outline().label("Reset").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.count = 0;
                                    cx.notify();
                                }),
                            )),
                    ),
            )
    }
}
