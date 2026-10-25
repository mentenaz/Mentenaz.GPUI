# GPUI Component Assets

> **In this fork.** Vendored from
> [longbridge/gpui-component](https://github.com/longbridge/gpui-component).
> It supplies the icons that `gpui_component` widgets render.
>
> - **Adding an icon:** drop an `.svg` into `assets/icons/`. The `IconName`
>   variant is generated from the filename at compile time
>   (`Forge_Cockpit2.svg` becomes `IconName::ForgeCockpit2`). Keep icons
>   small; `Forge_Cockpit2.svg` is a 6.1 MB traced path and is the exception,
>   not the model.
> - **How the app finds them:** GPUI has one process-wide asset source, and
>   Zed's own only knows Zed's `assets/`. `AppAssets` in
>   `crates/zed/src/main.rs` tries Zed's assets first and falls back to this
>   crate. Without it, every `gpui_component` icon fails to load.
> - **Build detail:** `build.rs` publishes the icon directory through Cargo's
>   `links` metadata so `gpui_component` can generate `IconName` without a
>   sibling-path reference.
> - **Tests:** `cargo test -p gpui-component-assets -j 8` checks that every
>   bundled icon loads and is a valid SVG, and that the icons the fork added
>   are present.

The default assets bundle for [GPUI Component](https://github.com/longbridge/gpui-component).

## License

Apache-2.0
