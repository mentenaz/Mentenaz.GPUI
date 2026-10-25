//! Integration harness for "clicking a node nested inside a container
//! selects the container instead" (regression test).
//!
//! NOTE: do NOT `use gpui::*` (or glob-import gpui into the same module as a
//! `#[gpui::test]`) — the glob pulls gpui's re-exported `test` macro into
//! scope and rustc's `#[test]` expansion recurses into it. Qualify paths or
//! import specific items only.

use gpui::AppContext;
use gpui::ParentElement;
use gpui::Styled;
use gpui_flow::FlowGraph;
use gpui_flow::FlowNode;
use gpui_flow::FlowState;

struct Harness {
    graph: gpui::Entity<FlowGraph>,
}

impl gpui::Render for Harness {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
            .size_full()
            .pt(gpui::px(40.0))
            .child(self.graph.clone())
    }
}

fn build_nodes() -> Vec<FlowNode> {
    let mut container = FlowNode::new("c", 160.0, 120.0)
        .node_type("If")
        .label("If checkout")
        .container_size(460.0, 320.0);
    container.context_menu = true;

    let leaf_a = FlowNode::new("a", 40.0, 40.0)
        .node_type("transform")
        .label("inner A")
        .parent("c")
        .size(172.0, 80.0);

    vec![container, leaf_a]
}

fn click_and_read_selected(
    cx: &mut gpui::TestAppContext,
    state: &gpui::Entity<FlowState>,
    point: gpui::Point<gpui::Pixels>,
) -> Vec<String> {
    let (_, visual_cx) = cx.add_window_view(|_window, cx| {
        let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
        Harness { graph }
    });
    visual_cx.simulate_click(point, gpui::Modifiers::default());
    visual_cx.run_until_parked();

    cx.read(|app| {
        state
            .read(app)
            .nodes
            .iter()
            .filter(|n| n.selected)
            .map(|n| n.id.to_string())
            .collect()
    })
}

#[gpui::test]
fn click_nested_node_selects_nested_node(cx: &mut gpui::TestAppContext) {
    let state = cx.new(|_| FlowState::new(build_nodes(), vec![]));

    let click_point = {
        let (_, visual_cx) = cx.add_window_view(|_window, cx| {
            let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
            Harness { graph }
        });
        let a_bounds = visual_cx.debug_bounds("a").expect("child 'a' bound");
        let c_bounds = visual_cx.debug_bounds("c").expect("container 'c' bound");
        assert!(
            c_bounds.contains(&a_bounds.center()),
            "child 'a' should be inside container 'c'"
        );
        a_bounds.center()
    };

    let selected = click_and_read_selected(cx, &state, click_point);
    assert_eq!(
        selected,
        vec!["a".to_string()],
        "clicking a nested node should select it, not its container"
    );
}

#[gpui::test]
fn click_nested_node_selects_nested_node_after_pan_and_zoom(cx: &mut gpui::TestAppContext) {
    let state = cx.new(|_| {
        let mut state = FlowState::new(build_nodes(), vec![]);
        state.viewport.x = 180.0;
        state.viewport.y = 90.0;
        state.viewport.zoom = 1.5;
        state
    });
    let (_, visual_cx) = cx.add_window_view(|_window, cx| {
        let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
        Harness { graph }
    });
    visual_cx.run_until_parked();

    let a_bounds = visual_cx.debug_bounds("a").expect("child 'a' bound");
    let c_bounds = visual_cx.debug_bounds("c").expect("container 'c' bound");
    assert!(
        c_bounds.contains(&a_bounds.center()),
        "child 'a' should be inside container 'c'"
    );
    visual_cx.simulate_click(a_bounds.center(), gpui::Modifiers::default());
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
        "pan and zoom must not make the enclosing container steal a nested-node click"
    );
}

#[gpui::test]
fn click_container_empty_interior_selects_container(cx: &mut gpui::TestAppContext) {
    let state = cx.new(|_| FlowState::new(build_nodes(), vec![]));

    let click_point = {
        let (_, visual_cx) = cx.add_window_view(|_window, cx| {
            let graph = cx.new(|cx| FlowGraph::new(state.clone(), cx));
            Harness { graph }
        });
        let c_bounds = visual_cx.debug_bounds("c").expect("container 'c' bound");
        // Below the header strip (32px * zoom) and left of child "a".
        gpui::Point::new(
            c_bounds.origin.x + gpui::px(20.0),
            c_bounds.origin.y + gpui::px(60.0),
        )
    };

    let selected = click_and_read_selected(cx, &state, click_point);
    assert_eq!(
        selected,
        vec!["c".to_string()],
        "clicking empty container interior should still select the container"
    );
}
