# Plan: porting `npm_manager_panel`

**Complete as of this session** — all four build-order steps shipped. Status &
deviations from this plan are recorded at the bottom ("Where it landed"). The
design below is kept as the reference for the port; read it if you're
re-touching any of the crates it produced.

## Source

```
E:\Forge_GPUI\src\forge_shell\panels\npm_manager_panel.rs   (2,387 lines)
```

Doc comment at the top of that file: *"npm package manager — virtual
workspace tab opened from the Node panel."* That's the one thing that makes
this port structurally different from every panel ported so far
(`cockpit_panel`, `script_runner_panel`, `processes_panel`, `python_panel`,
and — already done by a separate session, see below — `node_panel`): **this
is not a dock panel.** It's opened as an ordinary editor-space tab, the same
category of thing as a settings page or the keymap editor, not a sidebar
widget. Section 3 below covers exactly what that changes.

## Why this one is bigger than the others (recap)

Checked directly against the source file:

- **2,387 lines**, ~3.2x `python_panel` (744 lines).
- **No separable backend module exists in the source** — unlike
  `backend/python.rs`, all of npm's subprocess/JSON/HTTP logic is mixed into
  the panel file itself. It's cleanly separable by inspection (see §5), but
  someone has to actually do that separation — it isn't a free copy-paste
  step this time.
- **Three `gpui-component` widget systems no port has exercised yet**:
  `setting::{Settings, SettingPage, SettingGroup, SettingItem}` (confirmed
  present at `crates/gpui_component/src/setting/`), `resizable::{h_resizable,
  resizable_panel}` (confirmed present — re-exported in
  `crates/gpui_component/src/lib.rs:60-66` as a "backwards-compatible" alias
  module, so it's there even though `find -iname "*resizable*"` doesn't turn
  up a same-named file), and `text::markdown` (confirmed at
  `crates/gpui_component/src/text/format/markdown.rs`, for rendering package
  READMEs). None of this is missing from our vendored copy — it's just new
  ground for a port to cover.
- **Live HTTP calls** to the npm registry (search, packument fetch, downloads
  API) — every other port so far only ever shells out to a local subprocess.
- **A real scope decision**: it reads and writes `AppState`'s cross-ecosystem
  security-findings aggregator (`Ecosystem`, `SecurityFinding`,
  `ProjectFindings`, `SecuritySummary`, `update_security_findings`,
  `subscribe_security_scan_requests` — all defined in
  `src/crates/backend/system/mod.rs:100-165`). This is the same subsystem
  that feeds Forge's Cockpit *Dashboard*, which we already chose not to port.
  See §4.3 for the recommendation.

## Related: `node_panel` already exists

While this plan was being written, `crates/node_backend` and
`crates/node_panel` appeared in this workspace (added to
`Cargo.toml`/`crates/zed/Cargo.toml` by a separate session) — **not** done by
this session, but built to the exact same conventions established in
`PORTING.md`: `node_backend` is the pure-logic crate, `node_panel`'s own doc
comment drops `open_npm_manager`/`open_task_chain_wizard` as quick actions
with "no near-term port planned" — word-for-word the same call this session
made for `open_python_manager` in `python_panel`. Two implications for this
plan:

1. Once `npm_manager_panel` exists, the natural trigger for opening it is a
   quick-action button re-added to `node_panel` (it's currently just
   dropped — see `crates/node_panel/src/node_panel.rs:19-24`). That's a
   small follow-up edit to `node_panel`, not part of this crate.
2. `node_backend` may already contain Node/npm-adjacent detection logic
   (runtime/version queries) worth checking for overlap before writing
   `npm_backend`'s own — read `crates/node_backend/src/node_backend.rs`
   first so the two crates don't duplicate a `run_captured`-style helper or
   PATH-enrichment logic (`python_backend::path_env` already exists too;
   check whether `node_backend` rolled its own copy or something shareable
   is worth factoring out at this point).

## 1. Crate layout

Two new crates, matching the `python_backend` / `python_panel` split:

- **`crates/npm_backend`** — pure logic, no GPUI, no `AppState`. Mirrors
  `python_backend`'s shape (`src/npm_backend.rs` + a `path_env`-equivalent
  only if CREATE_NO_WINDOW/PATH-enrichment isn't already reachable from
  `gpui_util`/`node_backend`).
- **`crates/npm_manager_panel`** — the UI, implementing `workspace::Item`
  (not `workspace::dock::Panel`).

## 2. What moves into `npm_backend` (pure logic, from the source file)

All of this is already free-standing (no `self`, no `AppState`) in the
source — confirmed by reading it, not assumed:

- **Data types** (source lines 78-152): `NpmInstalledPkg`, `NpmOutdatedPkg`,
  `UpdateKind` (+ `impl UpdateKind`), `NpmVulnPkg`, `NpmSearchResult`,
  `NpmVersionEntry`, `NpmPackageDetails`.
- **Subprocess + parsing** (lines 153-442): `run_npm_cli` (mirrors
  `python_backend::run_captured` — same `gpui_util::new_std_command` +
  PATH-enrichment treatment applies here), `detect_package_manager` (npm vs
  pnpm vs yarn, by lockfile presence), `parse_installed`, `parse_outdated`,
  `parse_audit` (all JSON→struct), `classify_update`,
  `version_has_peer_conflict`.
- **Engine-compat checker** (bottom of file): `check_engine_compat(version:
  &semver::Version, range: &str) -> bool` — needs `semver` (already a
  workspace dependency, `Cargo.toml:863`, already used by `npm_manager_panel`
  itself in the source for the same purpose).
- **PyPI-style registry parsing**: the source fetches search results,
  packument (`/latest` + abbreviated packument), and downloads-API JSON over
  HTTP, then parses the response bodies. Following the `python_backend`
  precedent (`parse_pypi_json`/`parse_pypi_vulnerabilities` are pure parsing;
  the HTTP fetch is the *caller's* job) — the parsing functions belong in
  `npm_backend`, the actual `async` HTTP fetch stays in the panel (or a small
  `npm_manager_panel`-local async helper), since that needs an HTTP client
  instance that's a UI/app-layer concern.

Everything in this section is mechanical, low-risk work — copy, adapt the
`Command`-spawning bits the same way `python_backend::run_captured` did, and
`cargo check` will catch anything actually broken.

## 3. What moves into `npm_manager_panel` (UI, from the source file)

The bulk of the file (source lines 443-2325, `NpmManagerPanel` struct +
`impl` blocks): installed/outdated/audit package lists, the search box with
lazy "load more" pagination, the package-details sidebar (README rendered via
`text::markdown`, version dropdown filtered by peer-dependency conflicts),
bulk "Update All Safe/Patch" and per-vulnerability "Fix → v" buttons, and the
`Settings`-page chrome (`SettingPage`/`SettingGroup`/`SettingItem` sidebar
navigation) that wraps all of the above.

## 4. `AppState` decisions

### 4.1 Workspace root — same as every other port

`AppState::workspace_root` → this panel needs the actual project root, same
resolution `script_runner_panel::workspace_root_directory` already does
(first visible worktree, falling back to cwd). Since this panel is
constructed with access to a `Workspace` anyway (see §6), reuse that same
helper — or factor it out of `script_runner_panel` into somewhere shared if
a third panel ends up needing it too (`processes_panel`/`python_panel` never
did, since they weren't dock-wired against a project root when built; if a
fourth thing needs it, that's the signal to factor it out, not before).

### 4.2 Script Runner — the first panel where real wiring makes sense

Every prior port left the "run this command" hook as a disabled button
because nothing held both entities live. `npm_manager_panel` is opened *from*
`node_panel`, and both would need to reach a live `Entity<ScriptRunnerPanel>`
to wire install/remove/update through it for real. Two ways to get there:

- Whatever constructs `NpmManagerPanel` (the `node_panel` quick-action,
  ultimately) already has a `&mut Workspace` in scope (see §6's opening
  pattern) — `workspace.panel::<ScriptRunnerPanel>(cx)` returns the live dock
  panel entity directly, no broadcast channel needed. Simpler than the
  source's `run_in_script_runner`/`state.run_script_tx` plumbing.
- For "did the run finish" (source: `subscribe_script_done`), prefer GPUI's
  own reactivity over a broadcast channel: `cx.observe(&script_runner_entity,
  |this, script_runner, cx| { ... })` fires whenever `ScriptRunnerPanel`
  calls `cx.notify()` — which it already does on every output line and on
  completion. `npm_manager_panel` can just check `script_runner.read(cx)`'s
  running state there instead of needing a new completion-specific signal
  added to `script_runner_panel`.

This is real, non-trivial glue code (new to this port), not a copy-paste —
flagging it up front rather than discovering it mid-port.

### 4.3 Security-findings aggregator — needs an explicit decision

`Ecosystem`/`SecurityFinding`/`ProjectFindings`/`SecuritySummary` and the
`update_security_findings`/`subscribe_security_scan_requests` functions
(`src/crates/backend/system/mod.rs:100-165` in the source) are a shared,
cross-ecosystem vulnerability aggregator — every ecosystem manager
(npm/NuGet/Python) feeds it, and only the Dashboard (not ported, see
`PORTING.md`'s Part 2) reads it. Porting it properly means either:

- **(Recommended) Drop it.** `npm audit`'s per-package vulnerability list
  still renders locally in this panel (that's `NpmVulnPkg`/`parse_audit`,
  already scoped into `npm_backend` in §2) — only the "publish these findings
  somewhere else for a dashboard to aggregate" half goes away. Consistent
  with already not having a Dashboard to feed.
- **Port a minimal version anyway**, if a cross-ecosystem summary view is
  actually wanted later (e.g. a future, smaller "Security" panel, not a full
  Dashboard port). This is new scope beyond "port this one file" and should
  be its own decision when/if that panel is wanted — not bundled into this
  one silently.

This plan assumes **drop it** unless told otherwise before work starts.

## 5. Dependencies to add

- `npm_manager_panel`: `gpui`, `gpui-component`, `npm_backend`, `semver`,
  `serde`/`serde_json` (already pervasive), `http_client` (Zed's own HTTP
  abstraction — see §6.1, not raw `reqwest`), `workspace`, `ui` (for the
  `Item` impl's icon type, same as every dock-panel port needed it for
  `Panel::icon`).
- `npm_backend`: `gpui_util` (for `new_std_command`, matching
  `python_backend`), `semver`, `serde`/`serde_json`, `log`. Windows PATH
  enrichment: check `node_backend` first (see "Related" section above)
  before duplicating `python_backend::path_env`.

## 6. Mounting: this is a `workspace::Item`, not a `Panel`

This is the part every prior port in this repo hasn't needed, so it's worth
spelling out in full. Modeled directly on `crates/keymap_editor`, the closest
existing precedent for "a whole custom-UI feature opened as a tab, not
editing a real file."

### 6.1 The trait

`workspace::item::Item` (`crates/workspace/src/item.rs:170`) — bound
`Focusable + EventEmitter<Self::Event> + Render + Sized`. Only two things are
actually required (everything else on the trait has a default):

```rust
impl Item for NpmManagerPanel {
    type Event = ();

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "npm".into()
    }
}
```

(`KeymapEditor`'s own impl, `crates/keymap_editor/src/keymap_editor.rs:1953`,
is exactly this shape — two lines.) Worth also overriding `tab_icon` (returns
`Option<ui::Icon>`) for a nicer tab, using the same `ui::IconName` approach
every dock panel already uses for its toggle icon.

Also needed, same as every dock panel already has: `Focusable` (a
`FocusHandle` field + `.track_focus(...)` on the render root) and
`EventEmitter<()>` (an empty impl is enough — nothing needs to observe this
panel's events; the source's `NpmCommandRequested` event is emitted but never
subscribed to anywhere in the source either, per its own doc comment).

### 6.2 Opening it (the part that replaces `Panel::toggle_action`)

No action/keybinding/dock-button plumbing — instead, a plain function that:
finds an existing tab of this type in the active pane and focuses it, or
constructs and inserts a new one. Copied pattern, not guessed — this is
`crates/keymap_editor/src/keymap_editor.rs:92-130` almost verbatim:

```rust
fn open_npm_manager(
    project_root: String,
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let existing = workspace
        .active_pane()
        .read(cx)
        .items()
        .find_map(|item| item.downcast::<NpmManagerPanel>());

    if let Some(existing) = existing {
        workspace.activate_item(&existing, true, true, window, cx);
    } else {
        let panel = cx.new(|cx| NpmManagerPanel::new(project_root, workspace, window, cx));
        workspace.add_item_to_active_pane(Box::new(panel), None, true, window, cx);
    }
}
```

(`Workspace::add_item_to_active_pane`,
`crates/workspace/src/workspace.rs:4939`; `Workspace::activate_item`,
`crates/workspace/src/workspace.rs:5559`.) The `project_root: String`
parameter matters here specifically — unlike every dock panel (one instance
per workspace, whatever cwd it defaults to), the source scopes each
`npm_manager_panel` tab to *one* Node sub-project
(`open_npm_manager(state, project_root)` in the source), so re-opening it for
a different sub-project in a monorepo should arguably open a second tab
rather than reusing the first. The `existing` lookup above will need to
compare `project_root`, not just "any `NpmManagerPanel` at all," once that's
wired up — flagging it here so it isn't lost between planning and building.

### 6.3 Where the open call lives

No `init(cx)` registering a dock-toggle action this time. Instead: a public
`pub fn open(project_root: String, workspace: &mut Workspace, window: &mut
Window, cx: &mut Context<Workspace>)` exported from `npm_manager_panel`,
called from wherever `node_panel`'s dropped "npm mgr" quick-action button
gets re-added (see "Related" section) — that's the one piece of glue that
touches a crate outside this one, and it's a small, contained edit once both
sides exist.

## 7. Suggested build order

1. `npm_backend`: port §2's pure logic (types, subprocess/parsing,
   engine-compat), check `node_backend` for overlap first. Verify with
   `cargo check -p npm_backend` before moving on, same as every crate so far.
2. `npm_manager_panel`, UI-only, standalone: `Item` impl + the
   package-list/search/details/version-dropdown UI, driven by a project root
   passed in at construction (no cross-crate wiring yet). Confirms the three
   new widget systems (`setting`, `resizable`, `text::markdown`) actually
   render before adding any cross-panel complexity on top.
3. Script Runner wiring (§4.2) — `workspace.panel::<ScriptRunnerPanel>(cx)` +
   `cx.observe(...)`, once step 2's UI has real Install/Remove/Update buttons
   to attach it to.
4. `node_panel`'s dropped quick-action button, re-added to call
   `npm_manager_panel::open(...)`.

Steps 1-2 are the bulk of the actual porting effort (§0's size estimate);
3-4 are comparatively small once 1-2 exist.

## Where it landed

All of §1/§2/§3/§6 shipped: `crates/npm_backend` (pure CLI/parse/classify
logic, incl. `detect_package_manager`, `parse_installed`/`parse_outdated`/
`parse_audit`, `classify_update`, `engine_compat`) and `crates/npm_manager_panel`
(a `workspace::Item` tab whose UI is built on `gpui_component::setting`
— five `SettingPage`s — General, Search, Installed, Updates, Vulnerabilities —
rendered by a `Settings` widget, with a resizable `h_resizable`/`resizable_panel`
details split on the right while a package is selected). The `setting::` layout
was restored per `NPM_MANAGER_PANEL_UI_RESTORE_PLAN.md` after the first attempt
shipped a flat three-tab list; the panel is split across
`npm_manager_panel.rs` (struct/state/async methods + `Render`),
`pages.rs` (the five page builders), `search.rs` (`search_npm` / `fetch_details`,
both through `cx.http_client()`), and `details.rs` (the details/README pane,
with `gpui_component::text::markdown` rendering). Registry search is real, not
stubbed: `search_npm` hits `registry.npmjs.org/-/v1/search`, `fetch_details` hits
the full packument + the downloads API. Deviations from the plan & source, in
order:

- **§4.3 (security aggregator): dropped**, as recommended — only the local
  per-package `npm audit` list is surfaced.
- **§6.2 tab identity:** one `NpmManagerPanel` per workspace, last project root
  wins (the plan's "per distinct `project_root`" refinement is recorded as a
  possible follow-up, not implemented);
  `npm_manager_panel::open(root, workspace, window, cx)` is the §6.3 seam and
  is what `node_panel`'s re-added "npm mgr" quick action calls.
- **§4.2 (Script Runner wiring): implemented** in `kick_run`
  (`crates/npm_manager_panel/src/npm_manager_panel.rs`): install/remove/update
  look the live dock panel up via `workspace.panel::<ScriptRunnerPanel>(cx)`,
  open the Script Runner dock, stream the npm command through
  `ScriptRunnerPanel::run_external` (new `is_running()` accessor added), and
  reload the lists when `cx.observe` sees it leave `running`. Falls back to a
  silent background run (spinner only) when the dock panel isn't loaded yet,
  and refuses to pile a second run onto a busy runner.
- **Silent-no-op guard:** the quick-action and `OpenNpmManager` action resolve
  the project root as "first worktree, else process cwd" (same convention as
  `NodePanel`/`ScriptRunnerPanel`) instead of doing nothing when no worktree is
  open yet.
- **§1 registry fetches collapsed:** the source's three registry calls
  (`/latest` + abbreviated packument + downloads) are collapsed into one full
  packument fetch plus the downloads API — `/latest` is redundant with the full
  packument, which already carries the latest version's metadata. Deliberate,
  disclosed simplification.
- **Search compat badge:** `node_engine_compatible` reuses `semver::VersionReq`
  (already a dependency) instead of the source's hand-rolled range parser for
  the one engine-compat check. Disclosed deviation.
- **"Update All Patch" bug fixed:** the Updates page's second bulk button passes
  `"patch"` to `update_all`; the source passed `"all"` to both buttons. The
  first button is "safe" (patch + minor), the second patch-only.
- **Search "Load more" made visible:** the source had the paging mechanism but
  no trigger; the Search page renders a "Load more" button when the current page
  results `len < total`.

One real-world gotcha worth recording for future sessions: on Windows the dev
build is two binaries (`zed.exe` + the `cli` crate's `cli.exe`). Launching
`target\debug\zed.exe` without ever building `cargo build -p cli` prints
`could not find zed-cli from any of: bin/zed.exe, ./cli.exe` at startup (and
in a shipped-style `ZED_BUNDLE` build that error is **fatal**). Build both.

## Open questions before starting

- §4.3: drop the security-findings integration (recommended), or scope in a
  minimal version now?
- Confirm `node_backend` doesn't already cover something §2 lists, to avoid
  duplicating a helper across two crates.
- `Item` tab identity (§6.2): confirm one tab per distinct `project_root` is
  the right behavior, not "singleton, last-opened-project-root wins."
