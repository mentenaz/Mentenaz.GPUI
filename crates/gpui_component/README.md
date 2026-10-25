# gpui_component

The UI widget library every custom panel in this fork is built on. Vendored
from [longbridge/gpui-component](https://github.com/longbridge/gpui-component)
(package name `gpui-component`, version 0.5.2).

## Why it exists

The panels ported from the Forge repo (`E:\Forge_GPUI`) were written against
this library, not against Zed's own `ui` crate. Vendoring it lets them come
across with their real widgets — settings pages, resizable splits, data
tables, charts, markdown — instead of being rewritten.

That is also the rule for this tree: **a ported panel must be composed from
these widgets.** A port that renders flat, hand-rolled `div` lists that merely
look similar is a failed port, even if it compiles and works. See
[`CLAUDE.md`](../../CLAUDE.md) at the repo root.

## Using it in a panel

Add the dependency:

```toml
[dependencies]
gpui-component.workspace = true
```

Then build the UI from its widgets:

```rust
use gpui_component::{
    ActiveTheme as _,
    button::Button,
    input::{Input, InputState},
    resizable::{h_resizable, resizable_panel},
    setting::{SettingGroup, SettingItem, SettingPage, Settings},
};
```

Look at an existing panel for a working pattern before starting:

| You need | Copy from |
| --- | --- |
| Sidebar pages with grouped rows (`setting::`) and a resizable details pane | `npm_manager_panel` |
| A sortable data table with a context menu | `processes_panel` |
| Charts and gauges | `cockpit_panel` |
| A searchable, keyboard-navigable list | `designer_panel/src/catalog.rs` |
| A tree, tabs, description lists and three resizable panes | `database_panel` |

To see every widget running, use the gallery in
[`gpui_component_story`](../gpui_component_story/README.md).

### What is in it

Layout and containers (`dock`, `resizable`, `sidebar`, `tab`, `sheet`,
`dialog`, `popover`, `group_box`, `accordion`, `collapsible`), inputs
(`input`, `select`, `combobox`, `checkbox`, `radio`, `switch`, `slider`,
`color_picker`, `form`), data display (`table`, `list`, `tree`,
`description_list`, `chart`, `plot`, `progress`, `badge`, `tag`, `avatar`),
text (`text` for markdown and HTML, `label`, `link`, `kbd`, `highlighter`),
and application chrome (`setting`, `menu`, `command`, `notification`,
`status_bar`, `title_bar`, `tooltip`, `theme`).

## What the host app must do

These are already done once in `crates/zed/src/main.rs`. A new panel does not
repeat them — it only adds its own `<crate>::init(cx)` call next to the
others.

| Requirement | Where | If missing |
| --- | --- | --- |
| `gpui_component::init(cx)` at startup | `main.rs` | Every `cx.theme()` call panics during render. GPUI swallows that, so the panel just appears to do nothing |
| Asset fallback (`AppAssets`) | `main.rs` | Icons fail to load |
| `sync_gpui_component_theme(cx)` at startup and on every settings change | `main.rs` | Panels render in the library's light default regardless of Zed's theme |
| `gpui_component::rebind_default_keys(cx)` after each keymap reload | `zed.rs` | Input keys such as Backspace stop working once Zed reloads its keymap |

### Two theme systems

Zed and this library each have their own theme global, and both expose an
`ActiveTheme` trait with a `theme()` method. They are unrelated.
`sync_gpui_component_theme` copies Zed's colours into this library's theme.

- If a widget you use shows the wrong colours, extend that function. Do not
  special-case the panel.
- Do not import both `ActiveTheme` traits into one scope; the method names
  collide. Use a fully qualified call for one of them.

## Differences from upstream

This is not an untouched copy:

- `src/inspector.rs` was reshaped for this repo's
  `register_inspector_element`, which takes a factory.
- `rebind_default_keys` was added for the keymap-reload problem above.
- The `tree-sitter` feature is declared but not wired to a grammar. Syntax
  highlighting in the vendored editor is therefore a no-op; `database_panel`
  works around it with its own SQL highlighter.

Read [`PORTING.md`](PORTING.md) before re-syncing from upstream or porting
another panel. It records each incompatibility and its fix, and ends with a
checklist.

## Documents in this directory

| File | Contents |
| --- | --- |
| [`PORTING.md`](PORTING.md) | How the library and the Cockpit panel were ported; the checklist for the next port |
| [`NPM_MANAGER_PANEL_PLAN.md`](NPM_MANAGER_PANEL_PLAN.md) | Original npm manager port plan |
| [`NPM_MANAGER_PANEL_UI_RESTORE_PLAN.md`](NPM_MANAGER_PANEL_UI_RESTORE_PLAN.md) | The approved remedy after the first npm manager port failed |
| [`DATABASE_PANEL_SPEC.md`](DATABASE_PANEL_SPEC.md) | Database panel architecture and phases |

## Related crates

| Crate | Role |
| --- | --- |
| [`gpui_base`](../gpui_base/README.md) | Unstyled behaviour layer this crate builds on |
| [`gpui_component_assets`](../gpui_component_assets/README.md) | Bundled icons |
| [`gpui_component_macros`](../gpui_component_macros/README.md) | Proc macros (`icon_named!`, `IntoPlot`) |
| [`gpui_component_story`](../gpui_component_story/README.md) | Widget gallery |
| [`gpui_component_fps`](../gpui_component_fps/README.md) | FPS overlay used by the gallery |

## Development

```sh
cargo check -p gpui-component -j 8
```

Note the hyphenated package name; the directory uses underscores.
