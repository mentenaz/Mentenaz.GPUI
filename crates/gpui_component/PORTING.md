# Porting `gpui-component` panels into this tree

This document records how `gpui-component` (the vendored UI-kit crates under
`crates/gpui_base`, `crates/gpui_component`, `crates/gpui_component_assets`,
`crates/gpui_component_macros`) and the Cockpit panel were ported into this
Zed checkout, and the gotchas that cost the most time. Read it before porting
the next panel from the same source.

## Source

Everything here was ported from a separate, unrelated checkout:

```
E:\Forge_GPUI
```

That repo is its own Zed-derived app ("Forge") with its own vendored copy of
`gpui` and `gpui-component` under `src/crates/zed/` and
`src/crates/gpui_components/`. Its application code (the actual panels,
`forge_shell`, `backend::AppState`, etc.) lives under `src/forge_shell/`.

The Cockpit panel specifically came from:

```
E:\Forge_GPUI\src\forge_shell\panels\cockpit_panel.rs
E:\Forge_GPUI\src\forge_shell\panels\cockpit_panel\dashboard.rs   (NOT ported — see below)
E:\Forge_GPUI\src\forge_shell\panels\cockpit_panel\header.rs
E:\Forge_GPUI\src\forge_shell\panels\cockpit_panel\system_metrics.rs
```

Note there were **multiple copies** of a "cockpit" panel in that tree
(`forge-gpui-main/src/panels/cockpit`, `Forge_Old/panels/cockpit`,
`src/forge_shell/panels/cockpit_panel`) — confirm which one is current before
porting anything else from that repo; they are not identical.

## Part 1 — Vendoring `gpui-component` itself

The four crates were copied in wholesale (not written from scratch) and
wired into this workspace's `Cargo.toml`:

- Added to `[workspace] members` and `[workspace.dependencies]`:
  `gpui_base`, `gpui_component`, `gpui_component_assets`,
  `gpui_component_macros` (package names `gpui-base`, `gpui-component`,
  `gpui-component-assets`, `gpui-component-macros`).
- Added their extra dependencies to `[workspace.dependencies]`: `instant`,
  `ropey`, `rust-i18n`, `serde_repr`, `sum-tree` (`= zed-sum-tree`),
  `syntect`. `notify` was already patched to the Zed fork via
  `[patch.crates-io]`; a comment was added noting `notify.workspace = true`
  in the vendored crates resolves against that fork.

### The core incompatibility: `lsp_types`

The vendored code was written against upstream crates.io `lsp_types`, but
this workspace pins **Zed's own fork**
(`git = "https://github.com/zed-industries/lsp-types", rev = "f1783e6..."`),
which has a materially different API. Only one `lsp_types` can resolve
workspace-wide, so the vendored code had to be adapted, not the dependency:

- No LSP 3.18 inline-completion types at all (`InlineCompletionItem`,
  `InlineCompletionContext`, `InlineCompletionResponse`,
  `InlineCompletionTriggerKind`). **Fix:** defined minimal local equivalents
  in `crates/gpui_base/src/input/editor/lsp/completions.rs` mirroring the
  spec's wire shape, re-exported through the `lsp` module.
- `Diagnostic.message` is `DiagnosticMessage` (String or MarkupContent), not
  a plain `String`. **Fix:** `.as_str().into()` at the conversion site
  (`crates/gpui_base/src/input/editor/diagnostics.rs`).
- `Uri::scheme()` returns `&str` directly, not `Option<&str>`. **Fix:**
  compare directly instead of `.map(...)` in
  `crates/gpui_base/src/input/editor/lsp/definitions.rs`.
- `SemanticTokens::data` is a flat `Vec<u32>` (5 values per token: deltaLine,
  deltaStart, length, tokenType, tokenModifiers), not
  `Vec<SemanticToken>` with named fields. **Fix:** rewrote the decoder in
  `crates/gpui_base/src/input/editor/lsp/semantic_tokens.rs` to walk
  `chunks_exact(5)` (dropping a malformed trailing partial group instead of
  indexing out of bounds); updated its unit tests to build flat `u32`
  arrays instead of `SemanticToken` literals.

### A second incompatibility: `gpui::register_inspector_element`

`crates/gpui_component/src/inspector.rs` called
`register_inspector_element` with a single 4-arg closure
`(id, state, window, cx) -> R`. This repo's `gpui::App::register_inspector_element`
instead takes a two-level **factory**: `Fn(&mut Window, &mut App) -> F` where
`F: FnMut(id, &T, &mut Window, &mut App) -> R` — it caches the factory's
result itself, so the vendored code's manual `OnceCell` was both wrong-shaped
and redundant. Fixed by restructuring the call into that factory shape and
dropping the `OnceCell`. Also fixed one `Diagnostic { message, .. }` literal
in the same file to wrap the string in `lsp_types::DiagnosticMessage::String(..)`.

**Lesson for the next port:** don't assume a vendored crate that used to
build against upstream `gpui`/`lsp_types` builds unmodified here. Check
`cargo check -p <new-crate>` output line by line — every mismatch traces back
to this workspace's forked `gpui`/`lsp_types`, not a mistake in the vendored
code itself.

## Part 2 — Porting the Cockpit panel

New crate: `crates/cockpit_panel` (package `cockpit_panel`), added to
`[workspace] members` / `[workspace.dependencies]` in the root `Cargo.toml`.
Files: `src/cockpit_panel.rs` (crate root, per this repo's `[lib] path =
"src/<name>.rs"` convention), `src/header.rs`, `src/system_metrics.rs`.

Deliberate deviations from the source:

- **`dashboard.rs` (863 lines) was not ported.** It's a separate,
  workspace-level dashboard tab that embeds `NodePanel`/`PythonPanel`/
  `DotNet`/`GitPanel` and Forge's security-scan/`AppState` machinery — none
  of which exists here. Only the sidebar panel (`cockpit_panel.rs` +
  `header.rs` + `system_metrics.rs`) was ported.
- **`state: Arc<crate::backend::AppState>` was dropped entirely.** The
  source used it for exactly one thing — `crate::backend::open_dashboard`,
  which just sends a broadcast signal telling Forge's own tab-based
  workspace shell to open a dashboard tab. That's shell-specific plumbing
  this panel has no equivalent for.
- **The "Dashboard" header button is kept but `.disabled(true)`**, with a
  comment in `header.rs` explaining it's a wiring point for whoever later
  embeds this panel in a shell that has somewhere for "Dashboard" to open.
- **`Forge_Cockpit2.svg` was copied into
  `crates/gpui_component_assets/assets/icons/`** so `IconName::ForgeCockpit2`
  resolves (icon names are generated at compile time by scanning that
  directory — the `icon_named!` macro turns `Forge_Cockpit2.svg` into the
  `ForgeCockpit2` variant automatically). **Caveat:** that file is a
  **6.1 MB** single traced vector path, not a normal hand-drawn icon (the
  rest of the set runs 1–2 KB). It works, but it's disproportionately heavy
  for an icon; worth swapping for something smaller if this becomes visible
  in profiling.

## Part 3 — Wiring into Zed's real dock/activity-bar system

This was requested as a follow-up once the panel existed as a plain
`gpui::Render` entity. It is a materially bigger integration than the panel
port itself:

- `CockpitPanel` implements `workspace::dock::Panel` (`Focusable` +
  `EventEmitter<PanelEvent>` + `Render` + the trait's own methods): docked
  left by default, movable to the right through the `cockpit_panel.dock`
  setting (`position` reads it, `set_position` writes it; a `button`
  setting next to it hides the status bar button), `panel_key()` /
  `persistent_name()` for the generic dock-persistence machinery (panel
  size/open-state persistence needs no other code — it keys off
  `panel_key()` alone), `activation_priority() = 20` (the ported panels
  take 20–29, clear of Zed's own 0–7, because a movable panel can meet any
  other panel in a dock and two equal priorities there panic debug builds).
- A `cockpit_panel::ToggleFocus` action (`actions!(cockpit_panel,
  [ToggleFocus])`) is registered on every `Workspace` via `cx.observe_new`
  in `cockpit_panel::init(cx)` — this is what the dock's per-panel icon
  button actually dispatches on click (`toggle_action()` returns
  `Box::new(ToggleFocus)`).
- `CockpitPanel::load(workspace: WeakEntity<Workspace>, cx: AsyncWindowContext)
  -> Result<Entity<Self>>` follows the same convention as
  `ProjectPanel::load`, and is added to the `futures::join!` inside
  `initialize_panels` in `crates/zed/src/zed.rs`, alongside the other
  panels' `load` calls.
- **Gotcha that cost the most time:** `crates/zed/src/zed.rs` contains
  **two** near-identical blocks that both call `project_panel::init(cx)`,
  `outline_panel::init(cx)`, etc. — the real one is in
  `crates/zed/src/main.rs` (`fn main()`'s actual startup path), and a
  lookalike one inside `zed.rs`'s `mod tests { fn init_test_with_state(...) }`,
  used only by `#[gpui::test]` unit tests. **Adding a new panel's `init` call
  only to the test-module copy compiles fine and does nothing for the real
  app.** Both places call the same sequence of `*_panel::init(cx)` calls, so
  it's easy to grep-and-edit the wrong one — grep for the function name
  (`fn init_test_with_state` vs `fn main`) around the match, not just the
  line, before editing.

## Part 4 — Two more required initializations, and why the panel "did nothing" first

After the dock wiring compiled, the panel's icon appeared in the status bar
(Zed's left-dock panel toggles render in the **status bar**, not a separate
VSCode-style vertical icon strip) but clicking it produced no visible
effect. Two independent root causes, found in order:

1. **`gpui_component::init(cx)` was never called.** `gpui-component`'s own
   doc comment says outright: *"You must initialize the components at your
   application's entry point."* It sets up a required `Theme` global; every
   `cx.theme()` call in `CockpitPanel`'s render path panics without it, and
   gpui swallows a per-frame render panic silently — from the outside this
   looks exactly like "nothing happens," not a crash.
2. **Icons failed to load** (`ERROR ... loading asset at path "icons/..."`
   in the log) even after (1) was fixed. GPUI's SVG element resolves every
   icon path through **one process-wide `AssetSource`**, set once via
   `.with_assets(...)` at app startup. Zed's own `assets::Assets` only
   embeds Zed's own `assets/` directory — it has no idea
   `crates/gpui_component_assets/assets/icons/*.svg` exists. **Fix:** a
   small `AppAssets` wrapper in `crates/zed/src/main.rs` implementing
   `gpui::AssetSource`, trying `assets::Assets` first and falling back to
   `gpui_component_assets::Assets` for anything Zed's own source doesn't
   have; registered via `.with_assets(AppAssets)` instead of bare `Assets`.

Both fixes, plus the `cockpit_panel::init(cx)` call, live in
`crates/zed/src/main.rs`'s real startup path (the same one from the Part 3
gotcha above) — search for `AppAssets` and `gpui_component::init` there.

## Part 5 — Theme bridging

Even with the above fixed, the panel rendered in `gpui-component`'s own
**hardcoded light default** regardless of Zed's active (often dark) theme.
Root cause: Zed and `gpui-component` are two entirely independent theming
systems that happen to coexist in one process — different global types,
coincidentally-same-named `ActiveTheme` trait / `cx.theme()` method, no
connection between them. `gpui_component::init(cx)` calls
`Theme::change(ThemeMode::Light, None, cx)` unconditionally.

**Fix:** `sync_gpui_component_theme(cx)` in `crates/zed/src/main.rs`:

- Reads Zed's active theme (`theme::ActiveTheme::theme(cx)` — note the
  fully-qualified path form, since importing `theme::ActiveTheme` and
  `gpui_component::ActiveTheme` into the same scope would collide on the
  `theme()` method name).
- Calls `gpui_component::Theme::change(mode, None, cx)` first, to pick the
  closer of gpui-component's own light/dark presets as a base (this
  determines every field this function does *not* explicitly override:
  table row shading, list striping, sliders, switches, etc. — the niche,
  gpui-component-only concepts that have no Zed equivalent to borrow from).
- Then overwrites the ~30 fields that widgets actually read for a
  Zed-shaped look — `background`/`foreground`/`border`, `muted`*, `primary`
  (from `text_accent`), the four status colors (`danger`/`warning`/
  `success`/`info`, from Zed's `StatusColors`), sidebar/popover/input/
  scrollbar/title-bar/status-bar/tab chrome, and `chart_1..5` (from Zed's
  `theme.accents()` palette — the same colors Zed uses for git-blame/cursor
  coloring).
- Is called once at startup (right after `gpui_component::init(cx)`) and
  again on every `cx.observe_global::<SettingsStore>(...)` firing, since
  theme switches route through the settings store — so switching Zed's
  theme live re-syncs the Cockpit panel's colors too, not just at launch.

**If porting a panel that uses `gpui-component` widgets this function
doesn't yet map a color for** (e.g. `table`, `list_*`, `switch`, `slider`,
`skeleton`, `group_box`, `chart_bullish`/`chart_bearish` beyond what's set),
extend `sync_gpui_component_theme` rather than special-casing the panel —
this keeps every `gpui-component`-based panel visually consistent from one
place. Full field lists to map from/to:

- Target: `crates/gpui_component/src/theme/theme_color.rs` (`ThemeColor`).
- Source: `crates/theme/src/styles/colors.rs` (`ThemeColors`),
  `crates/theme/src/styles/status.rs` (`StatusColors`),
  `crates/theme/src/styles/accents.rs` (`AccentColors`, via
  `theme.accents()`).

## Checklist for the next port

1. Confirm which copy of the panel in `E:\Forge_GPUI` is current (see
   "Source" above — there are several).
2. Port the panel's own files as a new crate under `crates/`, following this
   repo's `[lib] path = "src/<name>.rs"` convention (`.rules`) — don't reuse
   `cockpit_panel`'s crate for an unrelated panel.
3. Strip anything that depended on Forge's `backend::AppState` /
   `forge_shell` tab system; replace with either nothing, a disabled
   placeholder (see the Dashboard button), or a generic callback the host
   app wires up — don't try to rebuild Forge's shell plumbing here.
4. `cargo check -p <new-crate>` in isolation first. Expect `lsp_types` /
   `gpui` API-shape errors if the panel touches LSP or inspector code — see
   Part 1's fixes for the pattern to follow, not necessarily the same
   fields.
5. If the panel needs a `workspace::dock::Panel` (an activity-bar/status-bar
   toggle), follow Part 3 — and edit `crates/zed/src/main.rs`'s real
   `fn main()` init path, **not** the lookalike block in
   `zed.rs`'s `mod tests`.
6. Any new icon: add the `.svg` to `crates/gpui_component_assets/assets/icons/`
   (keep it small — see the `Forge_Cockpit2.svg` caveat above) — the
   `IconName` variant is generated automatically from the filename.
7. `gpui_component::init(cx)`, the `AppAssets` asset-source fallback, and
   `sync_gpui_component_theme` only need to exist **once** for the whole
   app (they're already wired in `main.rs`) — a new panel doesn't repeat
   them, it just needs its own `<crate>::init(cx)` call added next to the
   existing panels' in the same real init path.
