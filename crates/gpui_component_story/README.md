# gpui_component_story

A standalone gallery application that shows every `gpui_component` widget
running, one "story" per widget. Vendored from
[longbridge/gpui-component](https://github.com/longbridge/gpui-component)
(package name `gpui-component-story`).

## Why it exists

It is the reference for building panels in this fork. Before writing a panel,
find the widget you need here, see how it behaves, and copy how its story
constructs it. `processes_panel`, for example, follows the `DataTableStory`
template.

It also answers "is this a widget bug or my bug?" quickly: if the widget
works in the gallery, the problem is in how the panel uses it.

It is not part of the editor. Nothing in `crates/zed` depends on it.

## Using it

Run the gallery:

```sh
cargo run -p gpui-component-story -j 8
```

Open it directly on one story by passing its name:

```sh
cargo run -p gpui-component-story -j 8 -- button
```

The window has a sidebar listing the stories, a search box to filter them,
and the selected story on the right.

### Standalone examples

Larger demos live in `examples/`:

```sh
cargo run -p gpui-component-story -j 8 --example dock
```

Available: `brush`, `dock`, `html`, `large-text`, `markdown`,
`stream_markdown`, `tiles`.

The `markdown` example does not compile in this tree at the moment: it
imports `lsp_types::SemanticToken`, which Zed's forked `lsp_types` does not
have (see [`PORTING.md`](../gpui_component/PORTING.md), Part 1).

## Finding the code for a widget

Each story is a file in `src/stories/`, named `<widget>_story.rs`
(`button_story.rs`, `calendar_story.rs`, …). The story is the shortest
working example of that widget, including its state setup and event
handling.

| Path | Contents |
| --- | --- |
| `src/main.rs` | Entry point; parses the optional story name |
| `src/gallery/` | The gallery window: sidebar, header, status bar, story selection |
| `src/stories/` | One file per widget |
| `examples/` | Standalone demos |
| `locales/` | Gallery UI strings |

## Notes

- The gallery uses its own theme handling, not Zed's. Colours will differ
  from how a widget looks inside the editor, where
  `sync_gpui_component_theme` maps Zed's theme onto the widgets.
- It pulls in [`gpui_component_fps`](../gpui_component_fps/README.md) for an
  optional frame-rate overlay.
- Features: `inspector` and `tree-sitter` pass through to `gpui-component`.
  `tree-sitter` is a declared-only feature in this tree and enables no real
  highlighting.

## Development

```sh
cargo check -p gpui-component-story -j 8
```
