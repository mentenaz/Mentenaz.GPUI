use gpui::*;
use gpui_flow::*;

// Vercel/shadcn palette
const BG: u32 = 0x09090b;
const GRID: u32 = 0x18181b;
const CARD: u32 = 0x0a0a0c;
const CARD_BORDER: u32 = 0x27272a;
const TEXT: u32 = 0xfafafa;
const TEXT_MUTED: u32 = 0xa1a1aa;
const ACCENT_BLUE: u32 = 0x3b82f6;
const ACCENT_EMERALD: u32 = 0x10b981;
const ACCENT_VIOLET: u32 = 0x8b5cf6;
const ACCENT_AMBER: u32 = 0xf59e0b;
const ACCENT_ROSE: u32 = 0xf43f5e;

struct FlowExample {
    flow: Entity<FlowGraph>,
    minimap: Entity<Minimap>,
    controls: Entity<Controls>,
}

impl Render for FlowExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(gpui::rgb(BG))
            .child(self.flow.clone())
            // Controls bottom-left
            .child(
                div()
                    .absolute()
                    .bottom(px(16.0))
                    .left(px(16.0))
                    .child(self.controls.clone()),
            )
            // Minimap bottom-right
            .child(
                div()
                    .absolute()
                    .bottom(px(16.0))
                    .right(px(16.0))
                    .child(self.minimap.clone()),
            )
    }
}

fn node_text(node: &FlowNode) -> String {
    if node.label.is_empty() {
        node.id.to_string()
    } else {
        node.label.to_string()
    }
}

/// Render node with a colored left accent bar.
fn render_node_with_accent(node: &FlowNode, accent: u32, type_label: &str) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .child(
            // Accent bar
            div()
                .w(px(3.0))
                .rounded_l_sm()
                .bg(gpui::rgb(accent))
                .my(px(-8.0))
                .ml(px(-16.0))
                .mr(px(12.0)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(gpui::rgb(TEXT_MUTED))
                        .font_weight(FontWeight::MEDIUM)
                        .child(type_label.to_string()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(TEXT))
                        .font_weight(FontWeight::MEDIUM)
                        .child(node_text(node)),
                ),
        )
        .into_any_element()
}

fn render_input(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_node_with_accent(node, ACCENT_EMERALD, "Input")
}

fn render_process(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_node_with_accent(node, ACCENT_BLUE, "Process")
}

fn render_output(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_node_with_accent(node, ACCENT_VIOLET, "Output")
}

fn render_trigger(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_node_with_accent(node, ACCENT_AMBER, "Trigger")
}

fn render_sink(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_node_with_accent(node, ACCENT_ROSE, "Sink")
}

fn main() {
    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(750.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let nodes = vec![
                    // Left column — triggers
                    FlowNode::new("webhook", 60.0, 80.0)
                        .label("Webhook")
                        .node_type("trigger")
                        .size(130.0, 52.0)
                        .handles(vec![HandleDef::source(HandlePosition::Right)]),
                    FlowNode::new("schedule", 60.0, 200.0)
                        .label("Cron Schedule")
                        .node_type("trigger")
                        .size(145.0, 52.0)
                        .handles(vec![HandleDef::source(HandlePosition::Right)]),
                    // Middle — processing
                    FlowNode::new("auth", 340.0, 80.0)
                        .label("Authenticate")
                        .node_type("process")
                        .size(140.0, 52.0)
                        .handles(vec![
                            HandleDef::target(HandlePosition::Left),
                            HandleDef::source(HandlePosition::Right),
                            HandleDef::source(HandlePosition::Bottom),
                        ]),
                    FlowNode::new("transform", 340.0, 200.0)
                        .label("Transform")
                        .node_type("process")
                        .size(125.0, 52.0)
                        .handles(vec![
                            HandleDef::target(HandlePosition::Left),
                            HandleDef::source(HandlePosition::Right),
                        ]),
                    FlowNode::new("validate", 340.0, 340.0)
                        .label("Validate")
                        .node_type("process")
                        .size(120.0, 52.0)
                        .handles(vec![
                            HandleDef::target(HandlePosition::Left),
                            HandleDef::source(HandlePosition::Right),
                        ]),
                    // Right column — data
                    FlowNode::new("db", 640.0, 80.0)
                        .label("Database")
                        .node_type("output")
                        .size(120.0, 52.0)
                        .handles(vec![HandleDef::target(HandlePosition::Left)]),
                    FlowNode::new("cache", 640.0, 200.0)
                        .label("Redis Cache")
                        .node_type("output")
                        .size(130.0, 52.0)
                        .handles(vec![HandleDef::target(HandlePosition::Left)]),
                    FlowNode::new("queue", 640.0, 340.0)
                        .label("Message Queue")
                        .node_type("sink")
                        .size(148.0, 52.0)
                        .handles(vec![HandleDef::target(HandlePosition::Left)]),
                    // Bottom — input
                    FlowNode::new("config", 60.0, 340.0)
                        .label("Environment")
                        .node_type("input")
                        .size(133.0, 52.0)
                        .handles(vec![HandleDef::source(HandlePosition::Right)]),
                ];

                let edges = vec![
                    FlowEdge::new("e1", "webhook", "auth")
                        .color(ACCENT_AMBER)
                        .stroke_width(2.0),
                    FlowEdge::new("e2", "schedule", "transform")
                        .color(ACCENT_AMBER)
                        .stroke_width(2.0),
                    FlowEdge::new("e3", "auth", "db")
                        .color(ACCENT_BLUE)
                        .stroke_width(2.0),
                    FlowEdge::new("e4", "auth", "transform")
                        .color(ACCENT_BLUE)
                        .stroke_width(1.5),
                    FlowEdge::new("e5", "transform", "cache")
                        .color(ACCENT_BLUE)
                        .stroke_width(2.0),
                    FlowEdge::new("e6", "config", "validate")
                        .color(ACCENT_EMERALD)
                        .stroke_width(2.0),
                    FlowEdge::new("e7", "validate", "queue")
                        .color(ACCENT_VIOLET)
                        .stroke_width(2.0),
                ];

                let state = cx.new(|_| FlowState::new(nodes, edges));

                let flow = cx.new(|cx| {
                    FlowGraph::new(state.clone(), cx)
                        .bg_color(BG)
                        .grid_color(GRID)
                        .bg_pattern(BackgroundPattern::Cross)
                        .node_bg_color(CARD)
                        .node_border_color(CARD_BORDER)
                        .node_renderer("input", render_input)
                        .node_renderer("process", render_process)
                        .node_renderer("output", render_output)
                        .node_renderer("trigger", render_trigger)
                        .node_renderer("sink", render_sink)
                });

                let minimap =
                    cx.new(|_| Minimap::new(state.clone()).container_bounds(1100.0, 750.0));
                let controls = cx.new(|_| Controls::new(state).container_size(1100.0, 750.0));

                cx.new(|_| FlowExample {
                    flow,
                    minimap,
                    controls,
                })
            },
        )
        .expect("Failed to open window");
    });
}
