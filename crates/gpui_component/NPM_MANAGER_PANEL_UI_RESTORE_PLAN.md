# Rebuild `npm_manager_panel` to match the source's Settings-page UI

## Context

`NPM_MANAGER_PANEL_PLAN.md` (§3) specifies that the npm manager panel should be
built on `gpui_component::setting::{Settings, SettingPage, SettingGroup,
SettingItem}` — a Settings-page-style UI with sidebar page navigation, a
resizable list/details split, live npm-registry search, a package-details
pane with README rendering, and bulk update actions — mirroring the source at
`E:\Forge_GPUI\src\forge_shell\panels\npm_manager_panel.rs` (2,387 lines).

What actually shipped (`crates/npm_manager_panel/src/npm_manager_panel.rs`,
926 lines) is a flat, hand-rolled three-tab list view with no `setting::`
usage, no resizable split, no registry search, no package-details/README
pane, and no bulk actions. The plan's own "Where it landed" section didn't
disclose this — it claimed "§3 shipped" when only the simplest third of it
did. The user has confirmed (after review) that this is a real gap and asked
for the missing UI to be built properly, using `gpui_component`'s actual
layout system, with real npm-registry search included (not stubbed).

This plan restores fidelity to the source's structure while keeping the two
things the current implementation already got right and that the source
didn't have available: the direct Script Runner wiring (§4.2, via
`workspace.panel::<ScriptRunnerPanel>(cx)` + `cx.observe`) and the dropped
cross-ecosystem security aggregator (§4.3 — no Dashboard exists in this Zed
host, so it stays dropped, as already decided).

## Confirmed API facts (from research, not guesses)

- `gpui_component::setting::{Settings, SettingPage, SettingGroup, SettingItem}`
  (`crates/gpui_component/src/setting/{settings,page,group,item}.rs`) are all
  `RenderOnce`/`IntoElement` — no `Entity` needed on the caller's struct.
  `Settings` manages its own sidebar-selection state internally via
  `window.use_keyed_state`.
  - `Settings::new("npm-package-manager").pages(vec![...])` — drop straight
    into `.child(...)`.
  - `SettingPage::new(title).icon(IconName::X).description(..).default_open(bool)
    .resettable(false).sidebar_badge(|_,_| ...).groups(vec![...])`.
  - `SettingGroup::new().title(..).items(vec![...])`.
  - `SettingItem::render(move |_options: &RenderOptions, window: &mut Window,
    cx: &mut App| { ... })` — **note the closure gets `&mut App`, not
    `Context<Self>`**. Any panel-state read/write inside must go through a
    captured `Entity<NpmManagerPanel>` clone (`view.read(cx)` /
    `view.update(cx, |panel, cx| {...})`), exactly like the source's
    `view: Entity<Self>` capture pattern. Pages are rebuilt fresh on every
    `Render::render` call (via `cx.entity()` for the capture), same as the
    source does.
- `h_resizable`/`resizable_panel` live in `crates/gpui_base/src/resizable/`
  (re-exported through `gpui_component`). `resizable_panel()` takes no id
  (order-addressed). No `ResizableState` entity is required unless we want
  persistence/observation — we don't, so build it fully declaratively per
  render, same as `Settings::render` itself does internally
  (`crates/gpui_component/src/setting/settings.rs:393-409` is the existing
  in-repo usage example).
- `gpui_component::text::markdown(source) -> TextView` — pure per-render
  call (`#[track_caller]`-keyed), drop into `.child(...)`. Use
  `.flex_1().scrollable(true).selectable(true)` inside a fixed-height
  container, matching the source's README view.
- HTTP: `cx.http_client() -> Arc<dyn HttpClient>` is available directly on
  `App`/`AsyncApp` (`crates/gpui/src/app.rs:1738`) — no need to thread a
  `client::Client`/`Workspace` through. Real working precedent:
  `crates/auto_update/src/auto_update.rs:677-738` and
  `crates/auto_update_ui/src/auto_update_ui.rs:111-136` — grab the client
  before `cx.spawn`, then inside: `http_client.get(url, Default::default(),
  true).await?`, `response.body_mut().read_to_end(&mut body).await?`,
  `serde_json::from_slice(&body)`. Same `WeakEntity::upgrade()` +
  `entity.update(cx, |this, cx| { ...; cx.notify(); })` pattern already used
  in `npm_manager_panel.rs`'s existing `reload()` (lines 186-266) — reuse
  that shape for the new fetches, just swapping `background_spawn(subprocess)`
  for an awaited HTTP call inside the same `cx.spawn`.
- All needed icon assets exist: `settings-2`, `search`, `inbox`, `arrow-up`,
  `triangle-alert`, `redo-2`, `close` are present under
  `crates/gpui_component_assets`, so `IconName::Settings2/Search/Inbox/
  ArrowUp/TriangleAlert/Redo2/Close` are all valid.
- `InputEvent::PressEnter { secondary, shift }` exists
  (`crates/gpui_base/src/input/base/state.rs:114-116`, re-exported through
  `gpui_component`) — subscribe to the search `InputState` with
  `cx.subscribe` to trigger search on Enter, same as the source.
- Workspace deps already available and unused so far by this crate: `url`,
  `urlencoding`, `futures` (root `Cargo.toml`).

## Scope

### 1. `npm_backend` additions (pure logic, no gpui — mirrors `python_backend`'s
   already-established-but-unused `parse_pypi_json`/`parse_pypi_vulnerabilities`
   split: parsing lives in the backend crate, the HTTP fetch stays in the panel)

New types in `crates/npm_backend/src/npm_backend.rs`:
- `NpmSearchResult { name, version, description, downloads: Option<u64>,
  engines_node: Option<String>, compat: Option<bool> }`
- `NpmVersionEntry { version: String, peer_deps: Vec<(String, String)> }`
- `NpmPackageDetails { name, description: Option<String>, version, license:
  Option<String>, homepage: Option<String>, weekly_downloads: Option<u64>,
  versions: Vec<NpmVersionEntry>, readme: Option<String> }`

New pure functions:
- `parse_npm_search(json: &serde_json::Value) -> (Vec<NpmSearchResult>, usize)`
  — parses `registry.npmjs.org/-/v1/search`'s `objects[]`/`total`.
- `parse_npm_packument(json: &serde_json::Value) -> Result<NpmPackageDetails, String>`
  — parses the **full packument** (`GET registry.npmjs.org/{name}`), which
  contains `dist-tags.latest`, every version's `peerDependencies`,
  `description`, `license`, `homepage`, and `readme` in one response. This
  intentionally collapses the source's three separate registry calls
  (`/latest` + abbreviated packument + downloads) into one packument fetch —
  the `/latest` request is redundant with the full packument, which already
  contains the latest version's metadata; only the downloads-API call stays
  separate below. Flag this as a deliberate, disclosed simplification, not a
  silent one.
- `parse_npm_downloads(json: &serde_json::Value) -> Option<u64>` — parses
  `api.npmjs.org/downloads/point/last-week/{name}`'s `downloads` field.
- `node_engine_compatible(node_version: &str, range: &str) -> Option<bool>` —
  thin wrapper over the crate's existing `semver::VersionReq`, used for the
  Search page's compat badge. (The source hand-rolled its own range parser
  for this one check; reusing `semver::VersionReq` — already a dependency,
  already used by every other range check in this crate — is more robust and
  consistent. Disclosed deviation, not a silent one.)
- `peer_conflicts_with_installed(peer_deps: &[(String, String)], installed:
  &[NpmInstalledPkg]) -> Vec<String>` — for each peer dep, if a package of
  that name is installed and its version doesn't satisfy the peer's range
  (via `semver::VersionReq`, reusing the same pattern as the existing
  `peer_conflicts_for`), record it as a conflict; peers not installed are
  ignored (fails open) — matches the source's `version_has_peer_conflict`
  semantics exactly.

`crates/npm_backend/Cargo.toml`: no new dependencies needed (all parsing is
plain `serde_json`/`semver`, already present).

### 2. `npm_manager_panel` — struct + fields

Add to `NpmManagerPanel`:
- `search_input: Entity<InputState>` (created in `new`, with a `cx.subscribe`
  for `InputEvent::PressEnter` → reset `search_page = 0` and call
  `search_npm`)
- `search_results: Vec<NpmSearchResult>`, `search_loading: bool`,
  `search_total: usize`, `search_page: usize`, `search_error: Option<String>`
- `install_as_dev: bool` (General page's dev/prod toggle; read by the
  install/update command-building code)
- `selected: Option<String>` (already exists — reuse; gates the resizable
  details split, same as source)
- `details: Option<npm_backend::NpmPackageDetails>`, `details_loading: bool`
- `show_readme: bool`
- `load_error: Option<String>` — set when the initial `reload()`'s
  `list_installed` call fails (e.g. no npm project), short-circuiting the
  whole render to an error view, matching the source's top-level
  `load_error` early-return. (The current `reload()` silently swallows CLI
  failures via `unwrap_or_default()` — keep that default-on-failure behavior
  for `outdated`/`audit`, but surface a real error when `installed` itself
  fails, since that's the "is this even an npm project" signal.)

`Cargo.toml`: add `http_client.workspace = true`, `futures.workspace = true`,
`urlencoding.workspace = true`.

### 3. `npm_manager_panel` — rendering restructure

Split the growing file into modules under `crates/npm_manager_panel/src/`
(the crate stays a single `lib` target; `npm_manager_panel.rs` keeps the
struct/state/async methods and adds `mod pages; mod search; mod details;`):

- **`pages.rs`** — five `fn build_general_page/build_search_page/
  build_installed_page/build_outdated_page/build_vuln_page(view:
  Entity<NpmManagerPanel>, cx: &App) -> SettingPage` functions, each
  snapshotting the data it needs from `view.read(cx)` before building
  `SettingItem::render` closures (mirrors the source's per-page builder
  functions and its `view: Entity<Self>` capture pattern):
  - **General**: package-manager picker (npm/yarn/pnpm/bun buttons, styled
    primary/outline by current `self.package_manager`, click sets it —
    override-only, doesn't re-detect), dev/prod install toggle, a
    Refresh button (disabled while any run/reload is in flight, with a
    `Spinner::xsmall()` next to it while loading).
  - **Search**: input + Search button row, then a results list: loading
    spinner / error banner / empty hint / result rows (name → click sets
    `selected` + calls `fetch_details`; version; description; compat badge
    via `node_engine_compatible`; an "Install" button that installs that
    exact version directly). Add a "Load more" button when
    `search_results.len() < search_total`, incrementing `search_page` and
    re-calling `search_npm` (the source has the mechanism but no visible
    trigger for it — this closes that gap rather than reproducing it).
  - **Installed**: one row per package (name click → `fetch_details`;
    version; dev-badge; Remove button).
  - **Outdated** (titled "Updates", icon `ArrowUp`): a bulk-actions
    `SettingItem` first ("Update All Safe" / "Update All Patch" — **fixed**
    to actually pass `"patch"` for the second button instead of the
    source's bug of passing `"all"` to both; call this out explicitly as an
    intentional correction), then one row per outdated package (current →
    latest, kind pill, Update button).
  - **Vulnerabilities**: one row per finding (name, severity pill, title,
    conditional "Fix → v" button when a fix version is known).
  - All three list pages get a `sidebar_badge` pill showing the item count
    (or a spinner while loading), matching the source's `count_badge`.

- **`search.rs`** — `fn search_npm(&mut self, cx)` and the packument/downloads
  fetch (`fn fetch_details(&mut self, name: String, cx)`), both using the
  established `cx.http_client()` + `cx.spawn` + `WeakEntity::upgrade()` +
  `entity.update(cx, ...)` shape already in this file's `reload()`. Reuse
  `npm_backend::parse_npm_search` / `parse_npm_packument` /
  `parse_npm_downloads` / `node_engine_compatible`.

- **`details.rs`** — the right-hand details pane content builder (mirrors
  `details_panel_content_static`): description/version/license/downloads,
  homepage line, README button (shown when `details.readme.is_some()`),
  divider + up to 12 compatible versions (filtered via
  `peer_conflicts_with_installed` against currently-installed packages),
  each with an Install/Update button. Plus the README-view branch (back
  button, header, `gpui_component::text::markdown(readme)`).

**`Render::render`** becomes:
1. If `load_error.is_some()` → return the danger-colored full-panel error
   view (short-circuit, matching source).
2. Otherwise build the 5 pages via `pages::build_*` (passing `cx.entity()`),
   assemble `Settings::new("npm-package-manager").pages(pages)`.
3. If `self.selected.is_some()`: wrap in
   `h_resizable("npm-details-split")` with two `resizable_panel()`s — panel 1
   (no fixed size, `flex_1().min_w_0().h_full()`) holds the whole `Settings`
   widget; panel 2 (`.size(px(400.)).size_range(px(280.)..px(900.)).flex_none()`)
   holds the details header (name + Close button, which clears
   `selected`/`details`/`details_loading`/`show_readme`) and body
   (loading → README → details → empty, in that priority, per source).
4. Otherwise render just the `Settings` widget full-size.

Drop the current flat tab-strip (`render_header`/`render_tabs`/
`render_status`/`render_installed`/`render_outdated`/`render_vulnerabilities`/
`render_loading`/`render_body`) — fully superseded by the above.

### 4. Command dispatch — keep, adapt call sites

Keep `kick_run`/`background_run`/the Script Runner wiring as-is (§4.2 is
already correctly implemented and is a documented, intentional improvement
over the source's broadcast-channel plumbing — not part of this gap). Just
update the call sites that build install/update args to respect
`install_as_dev` (append `--save-dev` when installing a new package if the
toggle is on) and to support the version-specific install calls the details
pane and search results need (`kick_run` already takes arbitrary `args`, so
`install_pkg(name, version)` → `vec!["install", format!("{name}@{version}")]`
composes directly, no changes needed to `kick_run` itself).

## Files touched

- `crates/npm_backend/src/npm_backend.rs` — new types + pure functions (§1)
- `crates/npm_manager_panel/Cargo.toml` — 3 new deps
- `crates/npm_manager_panel/src/npm_manager_panel.rs` — struct fields, new
  `mod` declarations, rewritten `Render::render`, `install_pkg`/`update_pkg`
  tweaks for dev-flag + explicit version
- `crates/npm_manager_panel/src/pages.rs` (new)
- `crates/npm_manager_panel/src/search.rs` (new)
- `crates/npm_manager_panel/src/details.rs` (new)
- `crates/gpui_component/NPM_MANAGER_PANEL_PLAN.md` — update "Where it
  landed" to accurately reflect what's now built and call out the three
  disclosed deviations from the source (collapsed packument fetch,
  `semver`-based engine check instead of a hand-rolled parser, fixed
  "Update All Patch" bug)

## Verification

1. `cargo check -p npm_backend -p npm_manager_panel` — must be clean.
2. `cargo build -p zed` (full binary — new deps/modules must link).
3. Launch the built `zed.exe` against a real npm project (one with a
   `package.json` and some installed deps, e.g. the same
   `npm_test_proj` scratch setup used in the prior session), open the Node
   panel, click "npm mgr", and manually verify:
   - Sidebar shows General/Search/Installed/Updates/Vulnerabilities pages
     with icons and item-count badges.
   - Search page: type a real package name, press Enter, see results with
     compat badges; "Load more" fetches another page; clicking a result
     name opens the resizable details pane on the right.
   - Details pane: shows description/license/downloads, a version list
     filtered for peer conflicts, an Install/Update button per version, a
     README button that swaps to the markdown-rendered README and back.
   - Installed/Updates/Vulnerabilities pages render real data from the test
     project, and Update/Remove/Fix buttons still stream through the Script
     Runner dock exactly as before.
   - Resizing the details panel by dragging its edge works and respects the
     280–900px range.
4. No regression: re-run the existing manual check from the prior session
   (open via command palette action, open via quick-action button, existing
   tab gets reused/activated rather than duplicated).
