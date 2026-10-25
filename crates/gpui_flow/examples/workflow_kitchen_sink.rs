// Visual-canvas-only demo of `DOCS/forge-workflow-engine-design.md`'s full
// action vocabulary, laid out as one workflow graph. This does NOT execute
// anything — no schema types, no ActionDef registry, no JSONLogic, no
// runtime. It's purely `gpui-flow` rendering: proof that every shape the
// design doc describes (parallel roots, fan-in, outcome-gated Failed
// branching, a Foreach container, an Until container, and an If container
// with two nested branches) is representable on the canvas built so far,
// using the `parent_id`/`container_size`/`fit_container` nesting support
// added for exactly this purpose.
//
// Action types covered (§2/§4's full leaf vocabulary): http, transform,
// script, delay, notify, sendEmail, log. Container types: Foreach, Until
// (with a `limit`), If (two branches — the one place this demo nests THREE
// levels deep: If -> branch container -> action node, proving
// `absolute_position` isn't a one-level special case).

use gpui::*;
use gpui_flow::*;

const BG: u32 = 0x09090b;
const GRID: u32 = 0x18181b;
const CARD: u32 = 0x0a0a0c;
const CARD_BORDER: u32 = 0x27272a;
const TEXT: u32 = 0xfafafa;
const TEXT_MUTED: u32 = 0xa1a1aa;

// One accent per leaf action type — purely a legend/readability device,
// gpui-flow has no notion of these being "types" beyond the renderer lookup.
const C_HTTP: u32 = 0x3b82f6; // blue
const C_TRANSFORM: u32 = 0x10b981; // emerald
const C_SCRIPT: u32 = 0xf59e0b; // amber
const C_DELAY: u32 = 0x71717a; // slate
const C_NOTIFY: u32 = 0x8b5cf6; // violet
const C_SEND_EMAIL: u32 = 0xf43f5e; // rose
const C_LOG: u32 = 0x06b6d4; // cyan

const C_FOREACH: u32 = 0x8b5cf6; // violet — matches Notify on purpose (both "violet family" in the source palette); container header color, not a leaf-type collision
const C_UNTIL: u32 = 0xf59e0b; // amber
const C_IF: u32 = 0x3b82f6; // blue
const C_FAILED_EDGE: u32 = 0xf43f5e; // rose — outcome-gated ["Failed"] edges

struct KitchenSink {
    flow: Entity<FlowGraph>,
    minimap: Entity<Minimap>,
    controls: Entity<Controls>,
}

impl Render for KitchenSink {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(gpui::rgb(BG))
            .child(self.flow.clone())
            .child(legend())
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

fn legend() -> impl IntoElement {
    let row = |color: u32, label: &'static str| {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded_full()
                    .bg(gpui::rgb(color)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(TEXT_MUTED))
                    .child(label),
            )
    };
    div()
        .absolute()
        .top(px(16.0))
        .right(px(16.0))
        .p_3()
        .rounded_md()
        .bg(gpui::rgba(0x0a0a0cee))
        .border_1()
        .border_color(gpui::rgb(CARD_BORDER))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(gpui::rgb(TEXT))
                .child("Action types"),
        )
        .child(row(C_HTTP, "http"))
        .child(row(C_TRANSFORM, "transform"))
        .child(row(C_SCRIPT, "script"))
        .child(row(C_DELAY, "delay"))
        .child(row(C_NOTIFY, "notify"))
        .child(row(C_SEND_EMAIL, "sendEmail"))
        .child(row(C_LOG, "log"))
        .child(
            div()
                .mt_2()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(gpui::rgb(TEXT))
                .child("Edge color"),
        )
        .child(row(0xb1b1b7, "Succeeded (default)"))
        .child(row(C_FAILED_EDGE, "Failed (outcome-gated)"))
}

fn node_text(node: &FlowNode) -> String {
    if node.label.is_empty() {
        node.id.to_string()
    } else {
        node.label.to_string()
    }
}

/// Shared leaf-action rendering: colored left accent bar + type tag + label,
/// same visual language as the crate's own `basic.rs` example.
fn render_leaf(node: &FlowNode, accent: u32, type_label: &str) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .child(
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

fn render_http(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_leaf(n, C_HTTP, "http")
}
fn render_transform(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_leaf(n, C_TRANSFORM, "transform")
}
fn render_script(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    // Mirrors §4's two source modes — shown as a property, not a separate type.
    let mode = n
        .properties
        .iter()
        .find(|(k, _)| k.as_ref() == "source.kind")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "inline".to_string());
    div()
        .flex()
        .flex_row()
        .child(
            div()
                .w(px(3.0))
                .rounded_l_sm()
                .bg(gpui::rgb(C_SCRIPT))
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
                        .child(format!("script \u{00b7} {mode}")),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(TEXT))
                        .font_weight(FontWeight::MEDIUM)
                        .child(node_text(n)),
                ),
        )
        .into_any_element()
}
fn render_delay(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_leaf(n, C_DELAY, "delay")
}
fn render_notify(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_leaf(n, C_NOTIFY, "notify")
}
fn render_send_email(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_leaf(n, C_SEND_EMAIL, "sendEmail")
}
fn render_log(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_leaf(n, C_LOG, "log")
}

/// A container's own header content — just the label + a type-specific icon
/// and accent color. The children living inside it are ordinary sibling
/// `FlowNode`s in the same `FlowState` (see the module doc), not something
/// this renderer draws.
fn render_container_header(node: &FlowNode, icon: &str, accent: u32) -> AnyElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::rgb(accent))
        .child(format!("{icon} {}", node_text(node)))
        .into_any_element()
}
fn render_foreach(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_container_header(n, "\u{21bb}", C_FOREACH)
}
fn render_until(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_container_header(n, "\u{27f3}", C_UNTIL)
}
fn render_if(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    render_container_header(n, "\u{2442}", C_IF)
}
fn render_branch(n: &FlowNode, _w: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::rgb(TEXT_MUTED))
        .child(node_text(n))
        .into_any_element()
}

/// Handles on `Top`/`Bottom` (not `Left`/`Right`) is the whole difference
/// between a left-to-right and a top-to-bottom layout — `gpui-flow`'s edge
/// routing (`bezier.rs`) already picks its control-point direction from
/// whatever `HandlePosition` a handle declares, for all four sides. Nothing
/// in the crate itself is direction-specific.
fn leaf(id: &str, x: f32, y: f32, label: &str, ty: &str) -> FlowNode {
    FlowNode::new(id, x, y)
        .label(label)
        .node_type(ty)
        .size(172.0, 46.0)
        .handles(vec![
            HandleDef::target(HandlePosition::Top),
            HandleDef::source(HandlePosition::Bottom),
        ])
}

/// Move `node_id` to sit `gap` below `below_id`'s *real* fitted footprint
/// (post `fit_container`), keeping `x`. Used to chain the top-level blocks
/// (Foreach → Until → If → final row) with a fixed gap instead of
/// hand-picked absolute Y guesses that don't track each container's actual
/// computed height.
fn stack_below(state: &mut FlowState, below_id: &str, node_id: &str, x: f32, gap: f32) {
    let y = {
        let below = state.get_node(&below_id.into()).expect("known node");
        let h = state.node_footprint(below).1;
        below.position.y + h + gap
    };
    if let Some(n) = state.get_node_mut(&node_id.into()) {
        n.position = FlowPoint::new(x, y);
    }
}

fn main() {
    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1500.0), px(900.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let mut nodes = vec![
                    // ── Parallel roots (both empty runAfter → run together) ──
                    leaf("fetchOrder", 40.0, 40.0, "Fetch Order", "http"),
                    leaf("fetchCustomer", 320.0, 40.0, "Fetch Customer", "http"),
                    // ── Fan-in: waits on both roots' Succeeded ──
                    {
                        let mut n = leaf("validate", 180.0, 200.0, "Validate Order", "script");
                        n.properties = vec![("source.kind".into(), "inline".into())];
                        n
                    },
                    // ── Outcome-gated error branch: runAfter validate["Failed"] ──
                    // Branches off to the side rather than continuing
                    // downward, so the red edge visibly diverges from the
                    // main top-to-bottom chain instead of looking like a
                    // step in it.
                    leaf("handleError", 460.0, 200.0, "Email Ops Team", "sendEmail"),
                    // ── Foreach container — children stacked vertically ──
                    FlowNode::new("foreach", 60.0, 340.0)
                        .label("Foreach Line Item")
                        .node_type("foreach")
                        // A taller dedicated header than the default 32 —
                        // the strip chrome is sized to this and `fit_container`
                        // reserves the extra room for the children below it.
                        .header_height(40.0)
                        .handles(vec![
                            HandleDef::target(HandlePosition::Top),
                            HandleDef::source(HandlePosition::Bottom),
                        ]),
                    // Stacked children need real room for the connecting
                    // edge to render, not just enough to avoid literal
                    // overlap — each leaf is 46px tall, so a 130px step
                    // (not 80) leaves an actual visible gap instead of the
                    // edge's handle dots nearly touching.
                    leaf("normalize", 20.0, 20.0, "Normalize", "transform").parent("foreach"),
                    leaf("chargeItem", 20.0, 150.0, "Charge Line Item", "http").parent("foreach"),
                    // ── Until container (requires a limit — §3) ──
                    FlowNode::new("until", 60.0, 720.0)
                        .label("Until Confirmed (limit 3)")
                        .node_type("until")
                        .handles(vec![
                            HandleDef::target(HandlePosition::Top),
                            HandleDef::source(HandlePosition::Bottom),
                        ]),
                    leaf("pollStatus", 20.0, 20.0, "Poll Status", "http").parent("until"),
                    leaf("backoff", 20.0, 150.0, "Backoff", "delay").parent("until"),
                    leaf("checkDone", 20.0, 280.0, "Check Confirmed", "transform").parent("until"),
                    // ── If container: two branches side-by-side, each its own nested container ──
                    FlowNode::new("ifVip", 60.0, 1140.0)
                        .label("If VIP Customer")
                        .node_type("if")
                        .handles(vec![
                            HandleDef::target(HandlePosition::Top),
                            HandleDef::source(HandlePosition::Bottom),
                        ]),
                    FlowNode::new("branchTrue", 20.0, 20.0)
                        .label("True")
                        .node_type("branch")
                        .parent("ifVip")
                        .handles(vec![HandleDef::source(HandlePosition::Bottom)]),
                    // Three actions in sequence, same 130px vertical step
                    // used inside Foreach/Until.
                    leaf(
                        "applyDiscount",
                        20.0,
                        20.0,
                        "Apply VIP Discount",
                        "transform",
                    )
                    .parent("branchTrue"),
                    leaf("notifyVip", 20.0, 150.0, "Notify VIP", "notify").parent("branchTrue"),
                    leaf("logVip", 20.0, 280.0, "Log VIP Applied", "log").parent("branchTrue"),
                    FlowNode::new("branchFalse", 320.0, 20.0)
                        .label("False")
                        .node_type("branch")
                        .parent("ifVip")
                        .handles(vec![HandleDef::source(HandlePosition::Bottom)]),
                    leaf(
                        "standardPricing",
                        20.0,
                        20.0,
                        "Standard Pricing",
                        "transform",
                    )
                    .parent("branchFalse"),
                    leaf("applyTax", 20.0, 150.0, "Apply Tax", "transform").parent("branchFalse"),
                    leaf("logStandard", 20.0, 280.0, "Log Standard Applied", "log")
                        .parent("branchFalse"),
                    // ── Final parallel fan-out ──
                    leaf("notifyDone", 60.0, 1680.0, "Notify Customer", "notify"),
                    leaf("logFinal", 340.0, 1680.0, "Log Completion", "log"),
                ];

                let edges = vec![
                    FlowEdge::new("e1", "fetchOrder", "validate"),
                    FlowEdge::new("e2", "fetchCustomer", "validate"),
                    FlowEdge::new("e3", "validate", "handleError")
                        .color(C_FAILED_EDGE)
                        .label("Failed"),
                    FlowEdge::new("e4", "validate", "foreach"),
                    FlowEdge::new("e5", "normalize", "chargeItem"),
                    FlowEdge::new("e6", "foreach", "until"),
                    FlowEdge::new("e7", "pollStatus", "backoff"),
                    FlowEdge::new("e8", "backoff", "checkDone"),
                    FlowEdge::new("e9", "until", "ifVip"),
                    FlowEdge::new("e10", "branchTrue", "applyDiscount"),
                    FlowEdge::new("e10b", "applyDiscount", "notifyVip"),
                    FlowEdge::new("e10c", "notifyVip", "logVip"),
                    FlowEdge::new("e11", "branchFalse", "standardPricing"),
                    FlowEdge::new("e11b", "standardPricing", "applyTax"),
                    FlowEdge::new("e11c", "applyTax", "logStandard"),
                    FlowEdge::new("e12", "ifVip", "notifyDone"),
                    FlowEdge::new("e13", "ifVip", "logFinal"),
                ];

                // Auto-size every container from its children — innermost
                // first, so an outer container's own footprint (via
                // `node_footprint`) already sees its nested containers'
                // *fitted* size, not their un-fitted default.
                let mut state = FlowState::new(std::mem::take(&mut nodes), edges);
                let padding = 24.0;
                // True/False get extra breathing room around their single
                // child — a bigger padding than the other containers,
                // purely for visual weight (there's nothing else inside
                // them to fit around).
                let branch_padding = 80.0;
                // This top-to-bottom layout is much taller than wide, so
                // the zoom `fit_view` needs to fit everything is well below
                // the default `min_zoom` (0.8) — without raising it first,
                // fit_view's own clamp would win and push the first/last
                // rows off-screen instead of actually fitting the graph.
                state.min_zoom = 0.35;

                // `fit_container` sizes a box in flow-space but its leaf
                // children are fixed-screen-pixel content (`leaf_px / zoom`
                // flow-units — see the `fit_container` doc), so a box must
                // be fitted *at the zoom the view will settle on*, or the
                // leaves outgrow it the moment `fit_view` zooms out.
                // Several passes converge the fit and the zoom: `fit_view`
                // establishes the zoom from the graph's span,
                // `fit_container` sizes the boxes for that zoom, and each
                // later pass tightens both against the now-real fitted
                // heights until they're equal.
                for _ in 0..3 {
                    // Chain each block's Y off the previous one's *real*
                    // footprint (post `fit_container`), so the top-level
                    // blocks (Foreach → Until → If → final row) sit a fixed
                    // gap apart with no slack in between.
                    const BLOCK_GAP: f32 = 60.0;
                    state.fit_container(&"foreach".into(), padding);
                    state.fit_container(&"until".into(), padding);
                    state.fit_container(&"branchTrue".into(), branch_padding);
                    state.fit_container(&"branchFalse".into(), branch_padding);
                    state.fit_container(&"ifVip".into(), padding);

                    stack_below(&mut state, "foreach", "until", 60.0, BLOCK_GAP);
                    stack_below(&mut state, "until", "ifVip", 60.0, BLOCK_GAP);
                    let final_y = {
                        let if_vip = state.get_node(&"ifVip".into()).expect("known node");
                        let h = state.node_footprint(if_vip).1;
                        if_vip.position.y + h + BLOCK_GAP
                    };
                    if let Some(n) = state.get_node_mut(&"notifyDone".into()) {
                        n.position.y = final_y;
                    }
                    if let Some(n) = state.get_node_mut(&"logFinal".into()) {
                        n.position.y = final_y;
                    }

                    state.fit_view(60.0, 1500.0, 900.0);
                }

                let state = cx.new(|_| state);

                let flow = cx.new(|cx| {
                    FlowGraph::new(state.clone(), cx)
                        .bg_color(BG)
                        .grid_color(GRID)
                        .bg_pattern(BackgroundPattern::Cross)
                        .node_bg_color(CARD)
                        .node_border_color(CARD_BORDER)
                        .node_renderer("http", render_http)
                        .node_renderer("transform", render_transform)
                        .node_renderer("script", render_script)
                        .node_renderer("delay", render_delay)
                        .node_renderer("notify", render_notify)
                        .node_renderer("sendEmail", render_send_email)
                        .node_renderer("log", render_log)
                        .node_renderer("foreach", render_foreach)
                        .node_renderer("until", render_until)
                        .node_renderer("if", render_if)
                        .node_renderer("branch", render_branch)
                        // Dedicated per-container header slot: the content is
                        // the same label as the `node_renderer` fallback, but
                        // it now renders inside its own chrome'd strip (tint +
                        // divider along the container's top) instead of a bare
                        // text row — sized to each container's `header_height`.
                        .container_header("foreach", render_foreach)
                        .container_header("until", render_until)
                        .container_header("if", render_if)
                        .container_header("branch", render_branch)
                        // §If a branch, or the body of a loop, ever tried to
                        // wire an edge straight to something outside its own
                        // container — the gap flagged in conversation — this
                        // is where a consumer would reject it:
                        .validate_connection(|conn, state| {
                            let src_parent = state
                                .get_node(&conn.source)
                                .and_then(|n| n.parent_id.clone());
                            let tgt_parent = state
                                .get_node(&conn.target)
                                .and_then(|n| n.parent_id.clone());
                            src_parent == tgt_parent
                        })
                });

                let minimap =
                    cx.new(|_| Minimap::new(state.clone()).container_bounds(1500.0, 900.0));
                let controls = cx.new(|_| Controls::new(state).container_size(1500.0, 900.0));

                cx.new(|_| KitchenSink {
                    flow,
                    minimap,
                    controls,
                })
            },
        )
        .expect("Failed to open window");
    });
}
