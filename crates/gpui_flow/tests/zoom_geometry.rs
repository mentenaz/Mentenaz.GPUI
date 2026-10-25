//! Regression tests for the layout-level `.scale()` world container.
//!
//! The graph is laid out once in **flow units** and zoomed by a single
//! `.scale(zoom)` container. These tests pin the three properties that make
//! that design correct:
//!
//! 1. **Node geometry scales.** A container's box is explicitly sized in flow
//!    units, so what lands on screen must be exactly `flow_size * zoom`. A
//!    leaf is sized by its own content, so its screen size must track the
//!    zoom ratio too. This is the invariant that regressed when leaf
//!    footprints were reported in *screen* pixels (`screen_footprint`), which
//!    pinned every card to one on-screen size regardless of zoom.
//! 2. **Hitboxes scale with their visuals.** A point that lies outside a node
//!    at 1x but inside it at 2x must hit-test to that node, proving the
//!    interactive area grew in lockstep with the painted area.
//! 3. **Screen-space chrome does not scale.** The minimap is a sibling of the
//!    world container, not a descendant, so zooming must leave its window
//!    bounds bit-identical.
//!
//! A fourth property is *resizing*: the world is sized `viewport_size / zoom`
//! flow units so its scaled size matches the viewport, and node geometry —
//! which lives in flow space — is independent of how big the window happens
//! to be. The background pattern is a separate viewport-sized layer so pan
//! cannot expose an unpainted strip. The tests at the end pin these properties.
//!
//! A fifth is *placement*: `.scale()` multiplies the world's own insets too,
//! so the world is positioned at `viewport / zoom` and lands at the viewport
//! once scaled. With the viewport at (0, 0) a wrong placement is invisible,
//! so `panned_world_starts_at_the_viewport` pans before it checks.
//!
//! ## Why the fixtures are pre-seeded
//!
//! Container boxes are re-fitted to their children by a measure pass that runs
//! during *paint*, and the resulting `cx.notify` does not by itself request
//! another frame — so in a bare test harness the first frame keeps rendering
//! the bootstrap `container_size` even though state has already been refitted.
//! (In the app this is a one-frame glitch that heals on the next redraw, and
//! it predates the scale work.) Rather than depend on that scheduling detail,
//! every fixture here seeds the converged leaf measurement and the fitted
//! container size into state *before* the first render, so the very first
//! frame is already self-consistent and no settling is required.
//!
//! NOTE: do NOT `use gpui::*` (or glob-import gpui into the same module as a
//! `#[gpui::test]`) — the glob pulls gpui's re-exported `test` macro into
//! scope and rustc's `#[test]` expansion recurses into it. Qualify paths or
//! import specific items only.

use gpui::AppContext;
use gpui::ParentElement;
use gpui::Styled;
use gpui::prelude::FluentBuilder;
use gpui_flow::FlowGraph;
use gpui_flow::FlowNode;
use gpui_flow::FlowState;
use gpui_flow::Minimap;

/// Padding above the graph, so the world container's origin is not at (0, 0)
/// and a bug that scaled position differently from size would be visible.
const HARNESS_PAD: f32 = 40.0;

/// Bootstrap container size, deliberately larger than the content needs so
/// the fit that `calibrate` measures is a visible shrink.
const BOOTSTRAP_CONTAINER: (f32, f32) = (460.0, 320.0);

/// The leaf's chrome is a 1-unit border on each side, in flow units. It
/// scales with everything else, so a leaf's screen size is
/// `(measured + 2 * LEAF_BORDER) * zoom`.
const LEAF_BORDER: f32 = 1.0;

/// A leaf's size comes from laying out its own text inside the world, whose
/// flow-space width is `viewport_px / zoom`. That width changes with zoom, so
/// the text can lay out a fraction of a pixel differently. Container boxes are
/// explicitly sized and checked exactly; leaves get a relative tolerance.
const LEAF_RELATIVE_TOLERANCE: f32 = 0.02;

struct Harness {
    graph: gpui::Entity<FlowGraph>,
    minimap: Option<gpui::Entity<Minimap>>,
}

impl gpui::Render for Harness {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let graph = self.graph.clone();
        let minimap = self.minimap.clone();
        gpui::div()
            .size_full()
            .pt(gpui::px(HARNESS_PAD))
            // The world container lives inside `graph`; the minimap is a
            // sibling, mirroring how `database_panel`/`designer_panel` stack
            // them.
            .child(graph)
            .when(minimap.is_some(), |el| el.child(minimap.unwrap()))
    }
}

/// A container with one nested leaf, so both the explicitly-sized container box
/// and the content-sized leaf are covered.
fn build_nodes(leaf_measured: (f32, f32), container: (f32, f32)) -> Vec<FlowNode> {
    let container_node = FlowNode::new("c", 160.0, 120.0)
        .node_type("If")
        .label("If checkout")
        .container_size(container.0, container.1);

    let leaf = FlowNode::new("a", 40.0, 40.0)
        .node_type("transform")
        .label("inner A")
        .parent("c")
        .size(leaf_measured.0, leaf_measured.1);

    vec![container_node, leaf]
}

/// The converged layout of the fixture, in flow units.
#[derive(Clone, Copy)]
struct Layout {
    /// The leaf's own content size, excluding its border.
    leaf: (f32, f32),
    /// The container's fitted box.
    container: (f32, f32),
}

/// Run one render purely to learn what the measure pass converges to. Only the
/// resulting *state* is used; the frame it produced is discarded.
fn calibrate(cx: &mut gpui::TestAppContext) -> Layout {
    let state = cx.new(|_| FlowState::new(build_nodes((172.0, 80.0), BOOTSTRAP_CONTAINER), vec![]));
    state.update(cx, |state, _| {
        state.viewport.zoom = 1.0;
        state.viewport.x = 0.0;
        state.viewport.y = 0.0;
    });

    {
        let (_, visual_cx) = cx.add_window_view(|_window, cx| {
            let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
            Harness {
                graph,
                minimap: None,
            }
        });
        // The measure pass runs during paint and writes straight into state,
        // so its result is available after a single frame.
        visual_cx.run_until_parked();
    }

    cx.read(|app| {
        let state = state.read(app);
        let leaf = state.get_node(&"a".into()).unwrap();
        Layout {
            leaf: (
                leaf.measured_width.expect("leaf measured").as_f32(),
                leaf.measured_height.expect("leaf measured").as_f32(),
            ),
            container: state
                .get_node(&"c".into())
                .unwrap()
                .container_size
                .expect("container fitted"),
        }
    })
}

struct Rendered {
    leaf: gpui::Bounds<gpui::Pixels>,
    container: gpui::Bounds<gpui::Pixels>,
    minimap: Option<gpui::Bounds<gpui::Pixels>>,
}

/// Render the fixture at `zoom` with a pre-seeded, already-consistent layout,
/// so the first frame is the frame we measure.
fn render_at_zoom(
    cx: &mut gpui::TestAppContext,
    layout: Layout,
    zoom: f32,
    with_minimap: bool,
) -> (gpui::Entity<FlowState>, Rendered) {
    let state = cx.new(|_| FlowState::new(build_nodes(layout.leaf, layout.container), vec![]));
    state.update(cx, |state, _| {
        state.viewport.zoom = zoom;
        state.viewport.x = 0.0;
        state.viewport.y = 0.0;
    });

    let (_, visual_cx) = cx.add_window_view(|_window, cx| {
        let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
        let minimap = with_minimap.then(|| cx.new(|_| Minimap::new(state.clone())));
        Harness { graph, minimap }
    });
    visual_cx.run_until_parked();

    // `VisualTestContext` holds a mutable borrow of `cx`, so gather the window
    // bounds before anything else reaches into the app.
    let rendered = Rendered {
        leaf: visual_cx.debug_bounds("a").expect("leaf 'a' bounds"),
        container: visual_cx.debug_bounds("c").expect("container 'c' bounds"),
        minimap: with_minimap.then(|| {
            visual_cx
                .debug_bounds("flow-minimap")
                .expect("minimap bounds")
        }),
    };
    (state, rendered)
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < 0.5,
        "{what}: expected {expected}, got {actual}"
    );
}

fn assert_close_relative(actual: f32, expected: f32, what: &str) {
    let tolerance = expected.abs().max(1.0) * LEAF_RELATIVE_TOLERANCE;
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: expected {expected} +/- {tolerance}, got {actual}"
    );
}

#[gpui::test]
fn calibrate_shrinks_container_to_fit(cx: &mut gpui::TestAppContext) {
    // Guards the premise of every other test here: the fixture must actually
    // exercise a refit, otherwise "scale is exact" would be vacuous because
    // nothing would ever change the box.
    let layout = calibrate(cx);
    assert!(
        layout.container.0 < BOOTSTRAP_CONTAINER.0 && layout.container.1 < BOOTSTRAP_CONTAINER.1,
        "calibration should shrink the container from {BOOTSTRAP_CONTAINER:?}, got {:?}",
        layout.container
    );
    assert!(
        layout.leaf.0 > 1.0 && layout.leaf.1 > 1.0,
        "leaf should have a real measured size, got {:?}",
        layout.leaf
    );
}

#[gpui::test]
fn container_box_is_flow_size_times_zoom(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    for zoom in [1.0_f32, 0.5, 2.0] {
        let (_, r) = render_at_zoom(cx, layout, zoom, false);

        // The box is a purely flow-space quantity; the screen is that number
        // times the zoom. This is the whole point of the refactor: one
        // coordinate space, and zoom applied in exactly one place.
        assert_close(
            r.container.size.width.as_f32(),
            layout.container.0 * zoom,
            &format!("container width @zoom {zoom}"),
        );
        assert_close(
            r.container.size.height.as_f32(),
            layout.container.1 * zoom,
            &format!("container height @zoom {zoom}"),
        );

        // Position is `pan + flow_position * zoom`, with the pan pinned to 0.
        assert_close(
            r.container.origin.x.as_f32(),
            160.0 * zoom,
            &format!("container origin x @zoom {zoom}"),
        );
        // Only the part inside the world scales: the flow position is in flow
        // units, while `HARNESS_PAD` belongs to the harness above the world
        // container and therefore stays at its unscaled screen size.
        assert_close(
            r.container.origin.y.as_f32(),
            HARNESS_PAD + 120.0 * zoom,
            &format!("container origin y @zoom {zoom}"),
        );
    }
}

#[gpui::test]
fn leaf_box_scales_with_zoom(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    for zoom in [0.5_f32, 2.0] {
        let (_, r) = render_at_zoom(cx, layout, zoom, false);

        // Border included, since it is flow-space chrome that must scale with
        // the content rather than staying a hairline.
        assert_close_relative(
            r.leaf.size.width.as_f32(),
            (layout.leaf.0 + 2.0 * LEAF_BORDER) * zoom,
            &format!("leaf width @zoom {zoom}"),
        );
        assert_close_relative(
            r.leaf.size.height.as_f32(),
            (layout.leaf.1 + 2.0 * LEAF_BORDER) * zoom,
            &format!("leaf height @zoom {zoom}"),
        );
    }
}

#[gpui::test]
fn leaf_stays_nested_at_every_zoom(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    let (_, at_1) = render_at_zoom(cx, layout, 1.0, false);
    let base = (
        at_1.leaf.origin.x.as_f32() - at_1.container.origin.x.as_f32(),
        at_1.leaf.origin.y.as_f32() - at_1.container.origin.y.as_f32(),
    );

    for zoom in [0.5_f32, 2.0] {
        let (_, r) = render_at_zoom(cx, layout, zoom, false);
        let offset = (
            r.leaf.origin.x.as_f32() - r.container.origin.x.as_f32(),
            r.leaf.origin.y.as_f32() - r.container.origin.y.as_f32(),
        );
        assert_close(
            offset.0,
            base.0 * zoom,
            &format!("leaf x offset from container @zoom {zoom}"),
        );
        assert_close(
            offset.1,
            base.1 * zoom,
            &format!("leaf y offset from container @zoom {zoom}"),
        );
    }
}

#[gpui::test]
fn minimap_chrome_does_not_scale(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    let (_, at_1) = render_at_zoom(cx, layout, 1.0, true);
    let (_, at_2) = render_at_zoom(cx, layout, 2.0, true);
    let (at_1, at_2) = (at_1.minimap.unwrap(), at_2.minimap.unwrap());

    // The minimap is a sibling of the world container, so zooming the graph
    // must not touch it. Zoom-dependent minimap chrome was an explicit
    // requirement.
    assert_eq!(
        at_1.size, at_2.size,
        "minimap size must not change with zoom"
    );
    assert_eq!(
        at_1.origin, at_2.origin,
        "minimap position must not change with zoom"
    );
}

#[gpui::test]
fn zoomed_node_hitbox_matches_visual_size(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    let (_, at_1) = render_at_zoom(cx, layout, 1.0, false);
    let (_, at_2) = render_at_zoom(cx, layout, 2.0, false);

    // Three quarters of the way across the *2x* leaf, halfway down: inside the
    // 2x box, and well outside the 1x one, so the assertion below is decisive.
    let target = gpui::Point::new(
        gpui::px(at_2.leaf.origin.x.as_f32() + at_2.leaf.size.width.as_f32() * 0.75),
        gpui::px(at_2.leaf.origin.y.as_f32() + at_2.leaf.size.height.as_f32() * 0.5),
    );
    assert!(
        at_2.leaf.contains(&target),
        "2x leaf {:?} should contain {target:?}",
        at_2.leaf
    );
    assert!(
        !at_1.leaf.contains(&target),
        "test is not decisive: the 1x leaf {:?} already contains {target:?}",
        at_1.leaf
    );

    let (state, _) = render_at_zoom(cx, layout, 2.0, false);
    let (_, visual_cx) = cx.add_window_view(|_window, cx| {
        let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
        Harness {
            graph,
            minimap: None,
        }
    });
    visual_cx.run_until_parked();
    visual_cx.simulate_click(target, gpui::Modifiers::default());
    visual_cx.run_until_parked();

    let selected = cx.read(|app| {
        state
            .read(app)
            .nodes
            .iter()
            .filter(|n| n.selected)
            .map(|n| n.id.to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        selected,
        vec!["a".to_string()],
        "a point inside the 2x leaf should hit-test to the leaf, proving the \
         hitbox scaled with the painted box"
    );
}

// ---------------------------------------------------------------------------
// Resizing
// ---------------------------------------------------------------------------

/// The window sizes the resize test walks through: start, grow (wider *and*
/// shorter), then shrink back below the starting width. Every step is large
/// enough that the fixture stays inside the culled flow rect at all three
/// zooms, so the only thing under test is the viewport change itself.
const SIZES: [(f32, f32); 3] = [(1000.0, 800.0), (2000.0, 1400.0), (1200.0, 1000.0)];

/// A node parked far to the right, so it falls outside the culled flow rect of
/// a small window and inside that of a wide one.
const FAR_FLOW_X: f32 = 4000.0;

fn gpui_size(w: f32, h: f32) -> gpui::Size<gpui::Pixels> {
    gpui::size(gpui::px(w), gpui::px(h))
}

fn build_spread_nodes() -> Vec<FlowNode> {
    vec![
        FlowNode::new("near", 40.0, 40.0)
            .node_type("transform")
            .label("near"),
        FlowNode::new("far", FAR_FLOW_X, 40.0)
            .node_type("transform")
            .label("far"),
    ]
}

/// Open a window of an explicit size, so a resize test can start from a known
/// viewport and then change it.
fn open_windowed(
    cx: &mut gpui::TestAppContext,
    nodes: Vec<FlowNode>,
    zoom: f32,
    window_size: gpui::Size<gpui::Pixels>,
) -> (gpui::Entity<FlowState>, gpui::WindowHandle<Harness>) {
    let state = cx.new(|_| FlowState::new(nodes, vec![]));
    state.update(cx, |state, _| {
        state.viewport.zoom = zoom;
        state.viewport.x = 0.0;
        state.viewport.y = 0.0;
    });
    let window = cx.open_window(window_size, |_window, cx| Harness {
        graph: cx.new(|cx| FlowGraph::new(state.clone(), cx)),
        minimap: None,
    });
    cx.run_until_parked();
    (state, window)
}

/// A `VisualTestContext` for a window that wasn't opened via `add_window_view`,
/// which is what the resize path needs (that helper always maximizes).
fn view_of(
    cx: &gpui::TestAppContext,
    window: &gpui::WindowHandle<Harness>,
) -> gpui::VisualTestContext {
    gpui::VisualTestContext::from_window((*window).into(), cx)
}

fn bounds_of(
    view: &mut gpui::VisualTestContext,
    selector: &'static str,
) -> gpui::Bounds<gpui::Pixels> {
    view.debug_bounds(selector)
        .unwrap_or_else(|| panic!("no bounds recorded for {selector:?}"))
}

/// Draw one more frame. The measure pass runs during *paint* and the `cx.notify`
/// it fires does not by itself request one (`Window::refresh` is a no-op
/// mid-paint), so a fixture whose container has just been re-fitted needs an
/// explicit frame before its rendered box reflects the new fit. One is enough:
/// the leaf re-fits its container, and the container's own re-measure then has
/// no ancestors left to re-fit.
///
/// Only ever used to settle a freshly opened fixture, never after a resize —
/// the resize assertions must read the frame the resize itself produced.
fn settle(cx: &mut gpui::TestAppContext, window: &gpui::WindowHandle<Harness>) {
    let mut view = view_of(cx, window);
    view.update(|window, _| window.refresh());
    drop(view);
    cx.run_until_parked();
}

/// Resize the window and let the new frame be drawn.
fn resize_to(
    cx: &mut gpui::TestAppContext,
    window: &gpui::WindowHandle<Harness>,
    size: gpui::Size<gpui::Pixels>,
) {
    let view = view_of(cx, window);
    view.simulate_resize(size);
    drop(view);
    cx.run_until_parked();
}

fn assert_bounds_close(
    actual: gpui::Bounds<gpui::Pixels>,
    expected: gpui::Bounds<gpui::Pixels>,
    what: &str,
) {
    assert_close(
        actual.origin.x.as_f32(),
        expected.origin.x.as_f32(),
        &format!("{what} origin x"),
    );
    assert_close(
        actual.origin.y.as_f32(),
        expected.origin.y.as_f32(),
        &format!("{what} origin y"),
    );
    assert_close(
        actual.size.width.as_f32(),
        expected.size.width.as_f32(),
        &format!("{what} width"),
    );
    assert_close(
        actual.size.height.as_f32(),
        expected.size.height.as_f32(),
        &format!("{what} height"),
    );
}

#[gpui::test]
fn world_covers_the_viewport_at_every_zoom_and_size(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    for zoom in [0.5_f32, 1.0, 2.0] {
        let (state, window) = open_windowed(
            cx,
            build_nodes(layout.leaf, layout.container),
            zoom,
            gpui_size(SIZES[0].0, SIZES[0].1),
        );
        state.update(cx, |state, _| {
            state.viewport.x = 240.0;
            state.viewport.y = -180.0;
        });

        for (w, h) in SIZES {
            resize_to(cx, &window, gpui_size(w, h));
            let mut view = view_of(cx, &window);
            let world = bounds_of(&mut view, "flow-world");
            let graph = bounds_of(&mut view, "flow-graph");
            let background = bounds_of(&mut view, "flow-background");

            // The world is `viewport_size / zoom` flow units, so the scale
            // multiplies it back to exactly the viewport size, and that
            // geometry has to be re-derived on every resize and zoom.
            assert_close(
                world.size.width.as_f32(),
                w,
                &format!("world width @zoom {zoom}, window {w}x{h}"),
            );
            assert_close(
                world.size.height.as_f32(),
                h,
                &format!("world height @zoom {zoom}, window {w}x{h}"),
            );
            assert_bounds_close(
                background,
                graph,
                &format!("background coverage @zoom {zoom}, window {w}x{h}"),
            );
        }
    }
}

/// A panned viewport puts the world, and so every node, exactly where the
/// viewport says: a node at flow `(x, y)` is drawn at
/// `graph origin + viewport + (x, y) * zoom`. Culling, fit-view, the minimap and
/// `flow_point_at` all assume that. The world used to be placed at
/// `viewport * zoom` (gpui's `scale` multiplies an element's own insets), which
/// only agreed at viewport (0, 0), the case every other test here uses.
#[gpui::test]
fn panned_world_starts_at_the_viewport(cx: &mut gpui::TestAppContext) {
    for zoom in [0.25_f32, 0.5, 2.0] {
        let (state, window) =
            open_windowed(cx, build_spread_nodes(), zoom, gpui_size(900.0, 700.0));
        state.update(cx, |state, _| {
            state.viewport.x = 240.0;
            state.viewport.y = -180.0;
        });
        settle(cx, &window);

        let mut view = view_of(cx, &window);
        let graph = bounds_of(&mut view, "flow-graph");
        let world = bounds_of(&mut view, "flow-world");
        assert_close(
            world.origin.x.as_f32(),
            graph.origin.x.as_f32() + 240.0,
            &format!("world origin x @zoom {zoom}"),
        );
        assert_close(
            world.origin.y.as_f32(),
            graph.origin.y.as_f32() - 180.0,
            &format!("world origin y @zoom {zoom}"),
        );

        // `near` sits at flow (40, 40).
        let near = bounds_of(&mut view, "near");
        assert_close(
            near.origin.x.as_f32(),
            graph.origin.x.as_f32() + 240.0 + 40.0 * zoom,
            &format!("node x @zoom {zoom}"),
        );
        assert_close(
            near.origin.y.as_f32(),
            graph.origin.y.as_f32() - 180.0 + 40.0 * zoom,
            &format!("node y @zoom {zoom}"),
        );
    }
}

#[gpui::test]
fn resize_does_not_move_or_resize_nodes(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    for zoom in [0.5_f32, 1.0, 2.0] {
        let (_state, window) = open_windowed(
            cx,
            build_nodes(layout.leaf, layout.container),
            zoom,
            gpui_size(SIZES[0].0, SIZES[0].1),
        );
        // The calibrated fit was measured in a *different* window (and at 1x),
        // so let this window's own measure pass converge before snapshotting.
        // Resizes below are *not* settled: they must land on their own.
        settle(cx, &window);

        let baseline = {
            let mut view = view_of(cx, &window);
            let baseline = (bounds_of(&mut view, "a"), bounds_of(&mut view, "c"));
            drop(view);
            baseline
        };

        for (w, h) in SIZES.iter().skip(1) {
            resize_to(cx, &window, gpui_size(*w, *h));
            let mut view = view_of(cx, &window);
            let leaf = bounds_of(&mut view, "a");
            let container = bounds_of(&mut view, "c");
            drop(view);

            // Flow-space geometry has no window-size term: a node's screen box
            // is `flow_box * zoom` and the pan is untouched, so growing the
            // window must not shift a single pixel. If a resize ever feeds the
            // window size back into the layout, this is what catches it.
            assert_bounds_close(leaf, baseline.0, &format!("leaf @zoom {zoom}"));
            assert_bounds_close(container, baseline.1, &format!("container @zoom {zoom}"));
            assert!(
                container.contains(&leaf.center()),
                "leaf {leaf:?} is no longer inside container {container:?}"
            );
        }
    }
}

#[gpui::test]
fn resize_does_not_rescale_measured_flow_sizes(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);

    for zoom in [0.5_f32, 1.0, 2.0] {
        let (state, window) = open_windowed(
            cx,
            build_nodes(layout.leaf, layout.container),
            zoom,
            gpui_size(SIZES[0].0, SIZES[0].1),
        );

        let read_geometry = |cx: &gpui::TestAppContext| {
            cx.read(|app| {
                let state = state.read(app);
                let leaf = state.get_node(&"a".into()).unwrap();
                let container = state.get_node(&"c".into()).unwrap();
                (
                    leaf.measured_width.expect("leaf measured"),
                    leaf.measured_height.expect("leaf measured"),
                    container.container_size.expect("container fitted"),
                )
            })
        };

        let before = read_geometry(cx);

        for (w, h) in SIZES.iter().skip(1) {
            resize_to(cx, &window, gpui_size(*w, *h));
            let after = read_geometry(cx);

            // Measurements are divided by `Window::element_scale()` before
            // being stored, so they are in flow units and must be invariant
            // under both zoom and resize. A resize that fed screen pixels back
            // in would show up here as a refit — and as a visible jump, since
            // the container is sized from this number.
            assert_eq!(
                after, before,
                "flow-space measurements moved on resize to {w}x{h} @zoom {zoom}"
            );
        }
    }
}

#[gpui::test]
fn resize_recomputes_culling(cx: &mut gpui::TestAppContext) {
    let (_state, window) = open_windowed(
        cx,
        build_spread_nodes(),
        1.0,
        gpui_size(SIZES[0].0, SIZES[0].1),
    );

    // Culling is recomputed from `window.viewport_size()` on every frame, so
    // widening the window must bring the far node in and narrowing it must
    // cull the node again. A graph that cached the cull rect across a resize
    // would leave a hole in the graph, or paint a node that isn't there.
    // At 1x with no pan, a node is painted while `flow_x <= win_w + 200`
    // (the cull margin is 200 screen pixels), so 800 hides it and 4400 shows it.
    resize_to(cx, &window, gpui_size(800.0, 600.0));
    let mut view = view_of(cx, &window);
    assert!(
        view.debug_bounds("near").is_some(),
        "the near node should be painted"
    );
    assert!(
        view.debug_bounds("far").is_none(),
        "at 800 wide, a node {FAR_FLOW_X} flow units to the right is culled"
    );
    drop(view);

    resize_to(cx, &window, gpui_size(FAR_FLOW_X + 400.0, 600.0));
    let mut view = view_of(cx, &window);
    let far = view
        .debug_bounds("far")
        .expect("the far node should be painted once the window covers it");
    assert_close(
        far.origin.x.as_f32(),
        FAR_FLOW_X,
        "the far node keeps its flow x on screen",
    );
    drop(view);

    resize_to(cx, &window, gpui_size(800.0, 600.0));
    let mut view = view_of(cx, &window);
    assert!(
        view.debug_bounds("far").is_none(),
        "narrowing the window again must cull the far node again"
    );
}

#[gpui::test]
fn hit_testing_survives_a_resize(cx: &mut gpui::TestAppContext) {
    let layout = calibrate(cx);
    let (state, window) = open_windowed(
        cx,
        build_nodes(layout.leaf, layout.container),
        2.0,
        gpui_size(SIZES[0].0, SIZES[0].1),
    );

    for (w, h) in SIZES.iter().skip(1) {
        resize_to(cx, &window, gpui_size(*w, *h));

        let target = {
            let mut view = view_of(cx, &window);
            bounds_of(&mut view, "a").center()
        };

        // `window_to_flow` reads the canvas origin captured during layout, so
        // this is really a check that the resize refreshed it: a stale origin
        // would map this point into the wrong place in flow space and select
        // nothing (or the wrong node).
        let mut view = view_of(cx, &window);
        view.simulate_click(target, gpui::Modifiers::default());
        drop(view);
        cx.run_until_parked();

        let selected = cx.read(|app| {
            state
                .read(app)
                .nodes
                .iter()
                .filter(|n| n.selected)
                .map(|n| n.id.to_string())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            selected,
            vec!["a".to_string()],
            "a click on the leaf should still land after resizing to {w}x{h}"
        );
    }
}
