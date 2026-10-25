# gpui_component_macros

Procedural macros used by `gpui_component`. Vendored from
[longbridge/gpui-component](https://github.com/longbridge/gpui-component)
(package name `gpui-component-macros`).

## Why it exists

Proc macros must live in their own crate, so the two that `gpui_component`
needs are here. You will rarely depend on this crate directly; it matters
mainly because it is the reason adding an icon needs no code change.

## What it provides

### `icon_named!`

Generates an icon enum by scanning a directory of `.svg` files at compile
time, with one variant per file.

```rust
// Path relative to the calling crate's CARGO_MANIFEST_DIR
icon_named!(IconName, "icons");

// Path taken from an environment variable at expansion time
icon_named!(IconName, "$GPUI_COMPONENT_DEFAULT_ICONS_DIR");

// With extra derives
icon_named!(IconName, "icons", [Debug, Copy, PartialEq, Eq]);
```

This is how `gpui_component::IconName` is produced. To add an icon to the
fork, drop the `.svg` into
[`gpui_component_assets/assets/icons/`](../gpui_component_assets/README.md);
the filename becomes the variant (`Forge_Cockpit2.svg` gives
`IconName::ForgeCockpit2`).

### `#[derive(IntoPlot)]`

Derive for chart types in `gpui_component`'s `plot` module, so a plot struct
can be used as a GPUI element. See `src/derive_into_plot.rs`.

## Development

```sh
cargo check -p gpui-component-macros -j 8
```
