//! Stress test: 1000 nodes, incremental random edge additions.
//! Press Space to play/pause. Clean monochromatic dark UI.

use gpui::*;
use gpui_flow::*;

const NODE_COUNT: usize = 1000;
const EDGES_PER_TICK: usize = 5;
const MAX_EDGES: usize = 200;
const TICK_MS: u64 = 50;

const COLS: usize = 40;
const SPACING_X: f32 = 260.0;
const SPACING_Y: f32 = 120.0;

// Vercel/shadcn palette
const BG: u32 = 0x09090b;
const GRID: u32 = 0x18181b;
const CARD: u32 = 0x0a0a0c;
const CARD_BORDER: u32 = 0x27272a;
const _TEXT: u32 = 0xfafafa;
const TEXT_MUTED: u32 = 0x71717a;

const ACCENT_BLUE: u32 = 0x3b82f6;
const ACCENT_EMERALD: u32 = 0x10b981;
const ACCENT_VIOLET: u32 = 0x8b5cf6;
const ACCENT_AMBER: u32 = 0xf59e0b;
const ACCENT_ROSE: u32 = 0xf43f5e;

struct StressApp {
    flow: Entity<FlowGraph>,
    state: Entity<FlowState>,
    minimap: Entity<Minimap>,
    controls: Entity<Controls>,
    focus_handle: FocusHandle,
    playing: bool,
    edges_added: usize,
    rng_state: u64,
}

impl StressApp {
    fn cheap_random(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    fn add_random_edges(&mut self, cx: &mut Context<Self>) {
        if !self.playing || self.edges_added >= MAX_EDGES {
            return;
        }

        let count = EDGES_PER_TICK.min(MAX_EDGES - self.edges_added);
        let colors = [
            ACCENT_BLUE,
            ACCENT_EMERALD,
            ACCENT_VIOLET,
            ACCENT_AMBER,
            ACCENT_ROSE,
        ];

        let mut randoms = Vec::with_capacity(count * 3);
        for _ in 0..(count * 3) {
            randoms.push(self.cheap_random());
        }
        let edges_added_base = self.edges_added;

        self.state.update(cx, |state, _| {
            let node_count = state.nodes.len();
            if node_count < 2 {
                return;
            }
            for i in 0..count {
                let src_idx = (randoms[i * 3] as usize) % node_count;
                let tgt_idx = (randoms[i * 3 + 1] as usize) % node_count;
                if src_idx == tgt_idx {
                    continue;
                }
                let src_id = state.nodes[src_idx].id.clone();
                let tgt_id = state.nodes[tgt_idx].id.clone();
                let edge_id: SharedString = format!("se-{}", edges_added_base + i).into();
                let color = colors[(randoms[i * 3 + 2] as usize) % colors.len()];

                let edge = FlowEdge::new(edge_id, src_id, tgt_id)
                    .color(color)
                    .stroke_width(1.5);
                state.edges.push(edge);
            }
        });
        self.edges_added += count;
        cx.notify();
    }
}

impl Render for StressApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let node_count = state.nodes.len();
        let edge_count = state.edges.len();
        let _ = state;

        let status = if self.playing { "Playing" } else { "Paused" };

        div()
            .id("stress-root")
            .track_focus(&self.focus_handle)
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
            // Subtle status bar top-right
            .child(
                div()
                    .absolute()
                    .top(px(16.0))
                    .right(px(16.0))
                    .px_3()
                    .py_1p5()
                    .bg(gpui::rgba(0x18181b_ee))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(CARD_BORDER))
                    .text_xs()
                    .text_color(gpui::rgb(TEXT_MUTED))
                    .child(format!(
                        "{} nodes  {}  edges  {}  +{}/{}",
                        node_count, "\u{00b7}", edge_count, self.edges_added, MAX_EDGES,
                    )),
            )
            // Status pill top-left
            .child(
                div()
                    .absolute()
                    .top(px(16.0))
                    .left(px(16.0))
                    .px_3()
                    .py_1p5()
                    .bg(gpui::rgba(0x18181b_ee))
                    .rounded_md()
                    .border_1()
                    .border_color(gpui::rgb(CARD_BORDER))
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded_full()
                            .bg(if self.playing {
                                gpui::rgb(ACCENT_EMERALD)
                            } else {
                                gpui::rgb(TEXT_MUTED)
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(TEXT_MUTED))
                            .child(format!("{}  \u{00b7}  Space to toggle", status)),
                    ),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let key: &str = event.keystroke.key.as_ref();
                if key == "space" {
                    this.playing = !this.playing;
                    if this.playing {
                        schedule_tick(cx);
                    }
                    cx.notify();
                }
            }))
    }
}

fn schedule_tick(cx: &mut Context<StressApp>) {
    cx.spawn(async move |this, cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(TICK_MS))
            .await;
        this.update(cx, |this, cx| {
            this.add_random_edges(cx);
            if this.playing && this.edges_added < MAX_EDGES {
                schedule_tick(cx);
            }
        })
        .ok();
    })
    .detach();
}

fn render_stress_node(node: &FlowNode, _window: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .text_xs()
        .text_color(gpui::rgb(TEXT_MUTED))
        .child(if node.label.is_empty() {
            node.id.to_string()
        } else {
            node.label.to_string()
        })
        .into_any_element()
}

fn main() {
    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1400.0), px(900.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let mut nodes = Vec::with_capacity(NODE_COUNT);
                for i in 0..NODE_COUNT {
                    let col = i % COLS;
                    let row = i / COLS;
                    let x = col as f32 * SPACING_X;
                    let y = row as f32 * SPACING_Y;
                    let id: SharedString = format!("n{}", i).into();
                    nodes.push(
                        FlowNode::new(id, x, y)
                            .label(format!("#{}", i))
                            .node_type("stress")
                            .size(45.0, 24.0)
                            .handles(vec![
                                HandleDef::target(HandlePosition::Left),
                                HandleDef::source(HandlePosition::Right),
                            ]),
                    );
                }

                // Subtle grid-neighbor edges
                let mut edges = Vec::new();
                for i in 0..NODE_COUNT {
                    let col = i % COLS;
                    if col + 1 < COLS && i + 1 < NODE_COUNT {
                        let eid: SharedString = format!("ge{}", i).into();
                        edges.push(
                            FlowEdge::new(eid, format!("n{}", i), format!("n{}", i + 1))
                                .color(CARD_BORDER)
                                .stroke_width(1.0),
                        );
                    }
                }

                let state = cx.new(|_| FlowState::new(nodes, edges));

                let flow = cx.new(|cx| {
                    FlowGraph::new(state.clone(), cx)
                        .bg_color(BG)
                        .grid_color(GRID)
                        .bg_pattern(BackgroundPattern::Dots)
                        .node_bg_color(CARD)
                        .node_border_color(CARD_BORDER)
                        .node_renderer("stress", render_stress_node)
                });

                let minimap =
                    cx.new(|_| Minimap::new(state.clone()).container_bounds(1400.0, 900.0));
                let controls =
                    cx.new(|_| Controls::new(state.clone()).container_size(1400.0, 900.0));

                cx.new(|cx| StressApp {
                    flow,
                    state,
                    minimap,
                    controls,
                    focus_handle: cx.focus_handle(),
                    playing: false,
                    edges_added: 0,
                    rng_state: 0xdeadbeef12345678,
                })
            },
        )
        .expect("Failed to open window");
    });
}
