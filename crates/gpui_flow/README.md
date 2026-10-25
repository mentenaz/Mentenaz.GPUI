# gpui-flow

> **In this fork.** Vendored from
> [pacifio/gpui-flow](https://github.com/pacifio/gpui-flow) (MIT). It is the
> node-graph canvas behind three features:
>
> - the workflow Designer ([`designer_panel`](../designer_panel/README.md)),
> - the database schema graph ([`database_panel`](../database_panel/README.md)),
> - the Actions job graph in Helm ([`helm_panel`](../helm_panel/README.md)).
>
> It depends only on `gpui` and has no notion of a theme: colours are raw
> `u32` values, so callers convert from the active theme themselves. Depend
> on it with `gpui_flow.workspace = true`. Run an example with
> `cargo run -p gpui_flow --example basic -j 8`.
>
> **How zoom works here.** The graph is laid out once in flow units inside a
> single world container that is zoomed with `.scale(zoom)`. In this fork's
> gpui, `scale` multiplies the element's own insets as well as its size, so
> the world's `left`/`top` are given as `viewport / zoom`: once scaled, the
> world starts exactly at the viewport. Culling, fit view, the minimap,
> zoom-to-cursor and `flow_point_at` all assume that position;
> `tests/zoom_geometry.rs` covers it, with a pan, at several zooms.
>
> **The canvas size is measured, not assumed.** `FlowGraph` reports its
> on-screen size to `FlowState` every time it is laid out
> (`FlowState::canvas_size()`). The minimap's viewport rectangle and the
> zoom/fit buttons use that size. The sizes given to
> `Minimap::container_bounds` and `Controls::container_size` are only
> fallbacks for the first frame. To fit a flow that may not have been drawn
> yet, call `FlowState::request_fit(padding)`: it fits right away when the
> size is known, otherwise as soon as it is.
>
> **Less detail when zoomed out.** Zoomed out, text is only a few pixels
> tall and smears, so `DetailLevel::for_zoom` (or `FlowState::detail_level()`)
> tells renderers how much to draw: `Full` from zoom 0.6 up, `Compact` (the
> title only, at least `DetailLevel::MIN_TITLE_SIZE` pixels tall) down to
> 0.35, and `Minimal` (a bar in place of the title) below that. Renderers are
> only given the node, so a renderer that follows it captures the
> `Entity<FlowState>` and reads the zoom; the Designer's do. The graph itself
> hides edge labels below `Full`, and fades the background grid out instead
> of letting its dots shrink below a pixel.
>
> The rest of this file is the upstream README.

A node-based flow graph editor for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui), inspired by [React Flow / xyflow](https://reactflow.dev).

Build interactive node graphs, mind maps, workflow editors, and data pipelines in Rust with the same GPU-accelerated framework that powers the [Zed editor](https://zed.dev).

<p align="center">
  <img src="./media/mindmap.png" width="49%" />
  <img src="./media/virtual.png" width="49%" />
</p>

## Features

- **Node rendering** with custom renderers per type — plug in any GPUI element
- **Edge types** — Bezier, Straight, SmoothStep with per-edge color and stroke width
- **Edge labels** at midpoints
- **Pan & zoom** — scroll, drag, pinch-to-zoom with zoom-toward-cursor
- **Node dragging** — single and multi-select drag with snap-to-grid
- **Selection** — click, Cmd+click multi-select, Shift+drag box selection, Cmd+A select all
- **Connect handles** — drag from source to target with snap-to-handle and visual feedback
- **Connection validation** — no self-connections, no duplicates, per-handle `is_connectable`
- **Edge arrowheads** — filled triangles at target endpoints
- **Handle dots** — visual circles at connection points with hover highlighting
- **Delete** — Backspace/Delete removes selected nodes and cascades edge removal
- **Undo/Redo** — Cmd+Z / Cmd+Shift+Z with 100-entry history
- **Focus management** — child elements (inputs, buttons) properly capture keyboard focus
- **Minimap** — bird's-eye view with viewport indicator and click-to-pan
- **Controls panel** — zoom +/-, fit-view buttons
- **Background patterns** — Dots, Lines, Cross grid
- **Viewport culling** — only renders visible nodes and edges for large graphs
- **Customizable theming** — background color, grid color, node chrome colors, pattern style
- **Graph utilities** — `get_incomers()`, `get_outgoers()`, `get_connected_edges()`, `fit_view()`, `zoom_in()`, `zoom_out()`, `set_center()`
- **Compatible with [gpui-component](https://github.com/longbridge/gpui-component)** — drop Input, Button, or any component inside nodes

## Quick Start

```rust
use gpui::*;
use gpui_flow::*;

// Create nodes
let nodes = vec![
    FlowNode::new("input", 50.0, 100.0)
        .label("Start")
        .size(120.0, 50.0)
        .handles(vec![HandleDef::source(HandlePosition::Right)]),
    FlowNode::new("output", 350.0, 100.0)
        .label("End")
        .size(120.0, 50.0)
        .handles(vec![HandleDef::target(HandlePosition::Left)]),
];

// Create edges
let edges = vec![
    FlowEdge::new("e1", "input", "output")
        .color(0x3b82f6)
        .stroke_width(2.0),
];

// Create the flow state and graph
let state = cx.new(|_| FlowState::new(nodes, edges));
let flow = cx.new(|cx| {
    FlowGraph::new(state.clone(), cx)
        .bg_color(0x09090b)
        .grid_color(0x18181b)
        .bg_pattern(BackgroundPattern::Cross)
        .node_renderer("custom", |node, _window, _cx| {
            div()
                .text_sm()
                .text_color(gpui::rgb(0xfafafa))
                .child(node.label.to_string())
                .into_any_element()
        })
});
```

## Examples

### Basic — Pipeline Editor

A dark-themed workflow editor with typed nodes (triggers, processes, outputs, sinks) and colored edges.

```sh
cargo run --example basic
```

### Mind Map

A mind map with branching categories, editable labels (Enter to edit), and colored connections.

```sh
cargo run --example mindmap
```

### Stress Test

1000 nodes with viewport culling, incremental edge additions, and performance monitoring.

```sh
cargo run --example stress
```

## API

### FlowNode

```rust
FlowNode::new("id", x, y)
    .label("Display Name")
    .node_type("custom")           // routes to registered renderer
    .size(120.0, 50.0)             // initial size hint for edge positioning
    .handles(vec![
        HandleDef::source(HandlePosition::Right),
        HandleDef::target(HandlePosition::Left).connectable(false),
    ])
    .z_index(1)
```

### FlowEdge

```rust
FlowEdge::new("edge-id", "source-node", "target-node")
    .edge_type(EdgeType::Bezier { curvature: 0.25 })
    .color(0x3b82f6)
    .stroke_width(2.0)
    .label("connection label")
```

### FlowGraph

```rust
FlowGraph::new(state, cx)
    .bg_color(0x09090b)                        // canvas background
    .grid_color(0x18181b)                      // grid pattern color
    .bg_pattern(BackgroundPattern::Cross)      // Dots, Lines, or Cross
    .node_bg_color(0x0a0a0c)                   // node card background
    .node_border_color(0x27272a)               // node card border
    .no_node_chrome()                          // disable default card styling
    .node_renderer("type", render_fn)          // custom renderer per node type
    .default_renderer(render_fn)               // fallback renderer
    .on_connect(|connection, state| { ... })   // connection callback
    .validate_connection(|conn, state| true)   // custom validation
```

### FlowState

```rust
let state = FlowState::new(nodes, edges);

// Viewport
state.detail_level();                      // Full, Compact or Minimal at this zoom (fork)
state.request_fit(40.0);                   // fit into the canvas as drawn (fork)
state.canvas_size();                       // Some((w, h)) once laid out (fork)
state.fit_view(40.0, width, height);
state.zoom_in(width, height);
state.zoom_out(width, height);
state.set_center(flow_x, flow_y, width, height);

// Selection
state.select_all();

// Graph traversal
state.get_incomers(&node_id);
state.get_outgoers(&node_id);
state.get_connected_edges(&node_id);

// History
state.push_undo();
state.undo();
state.redo();
```

### Components

```rust
// Minimap (in this fork the size is a fallback until the canvas is laid out)
Minimap::new(state).container_bounds(width, height)

// Zoom controls (same: a fallback until the canvas is laid out)
Controls::new(state).container_size(width, height)
```

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| Click | Select node |
| Cmd + Click | Multi-select |
| Shift + Drag | Box selection |
| Cmd + A | Select all |
| Delete / Backspace | Remove selected |
| Cmd + Z | Undo |
| Cmd + Shift + Z | Redo |
| Scroll | Pan |
| Cmd + Scroll | Zoom |
| Pinch | Zoom (macOS) |

## License

MIT
