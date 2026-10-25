// Smoke test for nested/container nodes (`FlowNode::parent`/`container_size`,
// `FlowState::absolute_position`/`fit_container`) — the capability the
// `forge-workflow-engine-design.md` decision record's §3 (Loops) needs: a
// `Foreach`/`Until` action rendered as one container node whose own
// independent sub-graph of child actions is laid out visually inside it.
//
// Two top-level nodes ("Fetch Orders" -> the Foreach container) plus a third
// top-level node the container's output feeds into ("Send Summary"), and
// inside the container: two child nodes connected by their own edge. Proves,
// in one glance: container sizing/chrome, child positions resolving through
// the parent offset, edges between two children, and an edge from a
// top-level node into/out of the container itself.

use gpui::*;
use gpui_flow::*;

const BG: u32 = 0x09090b;
const GRID: u32 = 0x18181b;
const CARD: u32 = 0x0a0a0c;
const CARD_BORDER: u32 = 0x27272a;
const TEXT: u32 = 0xfafafa;
const ACCENT_BLUE: u32 = 0x3b82f6;
const ACCENT_VIOLET: u32 = 0x8b5cf6;

struct NestedExample {
    flow: Entity<FlowGraph>,
    minimap: Entity<Minimap>,
    controls: Entity<Controls>,
}

impl Render for NestedExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(gpui::rgb(BG))
            .child(self.flow.clone())
            .child(
                div()
                    .absolute()
                    .bottom(px(16.0))
                    .left(px(16.0))
                    .child(self.controls.clone()),
            )
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

fn render_task(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .text_sm()
        .text_color(gpui::rgb(TEXT))
        .child(node_text(node))
        .into_any_element()
}

/// The container's own content — just a header label; the sub-graph inside
/// it is a set of ordinary sibling `FlowNode`s in the same `FlowState`, not
/// content this renderer draws itself (see the module doc).
fn render_foreach(node: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::rgb(ACCENT_VIOLET))
        .child(format!("\u{21bb} {} (per item)", node_text(node)))
        .into_any_element()
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
                    FlowNode::new("fetch", 60.0, 140.0)
                        .label("Fetch Orders")
                        .node_type("task")
                        .size(140.0, 44.0)
                        .handles(vec![HandleDef::source(HandlePosition::Right)]),
                    // The container node — footprint fixed via
                    // `.container_size(w, h)` here for the demo; a real
                    // consumer would instead call
                    // `FlowState::fit_container` after adding/moving its
                    // children so the box always exactly encloses them.
                    FlowNode::new("loop", 340.0, 60.0)
                        .label("Foreach Order")
                        .node_type("foreach")
                        .container_size(420.0, 220.0)
                        .handles(vec![
                            HandleDef::target(HandlePosition::Left),
                            HandleDef::source(HandlePosition::Right),
                        ]),
                    // Children: position is relative to the container's own
                    // interior origin (below its header strip), not
                    // top-level flow-space — see `FlowNode::parent`'s doc
                    // comment.
                    FlowNode::new("normalize", 30.0, 20.0)
                        .label("Normalize")
                        .node_type("task")
                        .parent("loop")
                        .size(130.0, 44.0)
                        .handles(vec![
                            HandleDef::target(HandlePosition::Left),
                            HandleDef::source(HandlePosition::Right),
                        ]),
                    FlowNode::new("charge", 230.0, 20.0)
                        .label("Charge Card")
                        .node_type("task")
                        .parent("loop")
                        .size(130.0, 44.0)
                        .handles(vec![HandleDef::target(HandlePosition::Left)]),
                    FlowNode::new("summary", 850.0, 140.0)
                        .label("Send Summary")
                        .node_type("task")
                        .size(140.0, 44.0)
                        .handles(vec![HandleDef::target(HandlePosition::Left)]),
                ];

                let edges = vec![
                    FlowEdge::new("e1", "fetch", "loop")
                        .color(ACCENT_BLUE)
                        .stroke_width(2.0),
                    // Edge between two children of the same container.
                    FlowEdge::new("e2", "normalize", "charge")
                        .color(ACCENT_VIOLET)
                        .stroke_width(2.0),
                    FlowEdge::new("e3", "loop", "summary")
                        .color(ACCENT_BLUE)
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
                        .node_renderer("task", render_task)
                        .node_renderer("foreach", render_foreach)
                });

                let minimap =
                    cx.new(|_| Minimap::new(state.clone()).container_bounds(1100.0, 750.0));
                let controls = cx.new(|_| Controls::new(state).container_size(1100.0, 750.0));

                cx.new(|_| NestedExample {
                    flow,
                    minimap,
                    controls,
                })
            },
        )
        .expect("Failed to open window");
    });
}
