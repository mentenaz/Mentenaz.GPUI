# Database Panel Specification

## Status

Planning document. This specification describes the target architecture and
delivery phases for a database panel ported from the Forge database panel.
The initial milestone is connection reliability and schema discovery.
The SQL workbench and schema graph are intentionally later phases and must
not block the connection milestone.

## Goals

The finished panel should provide:

- SQLite, PostgreSQL, MySQL/MariaDB, and MSSQL connections.
- Secure credential storage through Zed's existing credentials provider.
- Saved connection metadata and stable database identifiers.
- Connection, database, and schema lifecycle management.
- A typed schema tree showing tables, columns, types, nullability, and keys.
- A SQL workbench with query execution, results, errors, timing, and history.
- A foreign-key schema graph with table nodes and relationship edges.
- Consistent Zed theming and interaction behavior through `gpui_component`.

The panel must be implemented as native GPUI code. Flat `div` rows that merely
imitate settings, tabs, trees, or resizable panels do not satisfy this
specification.

## Non-goals for the first milestone

The first milestone does **not** include:

- SQL workbench editing or query execution.
- Query results grids.
- Query history.
- Foreign-key graph rendering.
- AI-generated query suggestions.
- Table sample previews or export actions.
- Full schema editing/migration tooling.

Those features are deliberately deferred until connection, disconnect,
reconnect, credential, and schema metadata behavior is proven.

## Crate boundaries

### `database_backend`

Create a backend crate for database-specific behavior once implementation
begins. It should not depend on `gpui_component`.

Responsibilities:

- Connection URL/type parsing.
- Driver-specific connection lifecycle.
- Connection registry keyed by stable connection ID.
- Connect, disconnect, and reconnect.
- Database listing and creation where supported.
- Table, column, and foreign-key metadata queries.
- Query execution and result conversion in later phases.
- Typed errors, cancellation, and bounded result policies.

Suggested modules:

```text
database_backend/
  src/
    database_backend.rs
    connection.rs
    drivers/
      sqlite.rs
      postgres.rs
      mysql.rs
      mssql.rs
    metadata.rs
    query.rs
    models.rs
    errors.rs
```

SQLite may initially reuse `sqlez` directly. The backend abstraction should
still be established before adding network drivers so UI code does not become
coupled to a single driver.

### `database_panel`

Create a UI crate for the panel and its GPUI state.

Responsibilities:

- Panel state and persistence coordination.
- Connection forms and connection status.
- Server/database selectors.
- Schema tree.
- Workbench and result UI in later phases.
- Schema graph in a later phase.

The panel should receive or resolve the backend service through an application
entity/global rather than opening driver connections directly from render code.

## Credential and persistence model

No additional keychain crate is required.

Use the existing workspace abstractions:

- `credentials_provider::CredentialsProvider`
- `zed_credentials_provider::global`
- GPUI platform credential methods on `App`

Store passwords and other secrets under a stable, normalized key such as:

```text
database://postgres/{normalized-host}:{port}/{database}
```

The exact key format must be centralized in the backend and versioned if it
changes. Credentials must never be written to workspace JSON, schema cache
files, query history, logs, error messages, or serialized panel state.

Persist only non-secret connection metadata:

- Stable server/database IDs.
- Database type.
- Display title.
- Host and port.
- Username, if needed for reconnection lookup.
- SSL mode/configuration.
- Known database names.
- Active selections.

Development may use the existing development credentials provider behavior.
Production and preview builds must use the platform keychain through
`zed_credentials_provider`.

## `gpui_component` UI stack

### Panel shell and split layout

Use:

- `v_flex` for the vertical panel structure.
- `h_flex` for toolbars, selectors, and action rows.
- `h_resizable` and `resizable_panel` for the schema/content split.
- `scroll::Scrollbar` with explicit scroll handles for long content.

The initial layout should be:

```text
DatabasePanel
├── database type selector
├── server selector and connection actions
├── database selector and database actions
└── h_resizable
    ├── schema sidebar
    └── deferred workbench/content area
```

### Connection forms

Use the settings system for connection configuration:

- `setting::Settings`
- `setting::SettingPage`
- `setting::SettingGroup`
- `setting::SettingItem`

Use:

- `Input` and `InputState` for host, port, username, password, SQLite path,
  database name, and display title.
- Password input configuration appropriate to `Input`.
- `Button` with existing variants for connect, cancel, disconnect, and retry.
- `Spinner` for active connection operations.
- Theme status colors for errors and connection state.

Forms must have explicit loading, validation, error, and success states.
Network calls and driver work must run outside render methods.

### Server and database selectors

Use the component tab/menu primitives rather than hand-built tab rows:

- Tab bar components where selection is persistent and horizontal.
- `PopupMenu` for overflow, disconnect, and database actions.
- `ContextMenuExt` for table/server contextual actions.
- `Icon`/`IconName` for database type and action affordances.

Selectors must expose active, disconnected, connecting, and failed states.

### Connection and database navigation

The panel uses a shared connection model. Every connection supports the same
fields, actions, lifecycle states, and database navigation regardless of
whether it is SQLite, PostgreSQL, MySQL/MariaDB, or MSSQL.

The target layout is:

```text
[ DATABASES ]                                              
                                                             
  Connections                                      [ + ]     
                                                             
  [▼] [●] Local SQLite                            [⋮]        
      ├─ [●] main                                 [⋮]        
      ├─ [ ] analytics                             [⋮]        
      └─ [ + ] Database                                      
          ├─ [ Create database ]                            
          └─ [ Register existing database ]                 
                                                             
  [▶] [●] Development PostgreSQL                  [⋮]        
                                                             
  [▶] [ ] Production MySQL                        [⋮]        
                                                             
  [▶] [ ] Reporting MSSQL                         [⋮]        
```

Selecting another connection changes the expanded section. Only the selected
connection is expanded; multiple connections may remain connected:

```text
[ DATABASES ]                                              
                                                             
  Connections                                      [ + ]     
                                                             
  [▶] [●] Local SQLite                            [⋮]        
                                                             
  [▼] [●] Development PostgreSQL                  [⋮]        
      ├─ [ ] postgres                             [⋮]        
      ├─ [●] application_db                        [⋮]        
      ├─ [ ] audit                                 [⋮]        
      └─ [ + ] Database                                      
          ├─ [ Create database ]                            
          └─ [ Register existing database ]                 
                                                             
  [▶] [ ] Production MySQL                        [⋮]        
                                                             
  [▶] [ ] Reporting MSSQL                         [⋮]        
```

Interaction rules:

- The top-level `[ + ]` opens a blank connection form.
- Each connection discovers its databases automatically after connecting.
- The database list is rediscovered on startup and can also be refreshed.
- The database `[ + ]` opens a menu containing `Create database` and
  `Register existing database`.
- Selecting a database connects to it and opens its schema view.
- Connection and database overflow menus use `PopupMenu` or `ContextMenuExt`.
- Connection metadata persists across restarts; credentials remain keychain
  backed and are never serialized with the navigation state.

Status and affordance indicators are represented by component icons and
themed states:

```text
[●] Connected
[◐] Connecting
[ ] Disconnected
[!] Connection error
[▼] Selected connection expanded
[▶] Connection collapsed
[⋮] Connection or database actions
[+] Add connection or database
```

After a database is selected, the schema content appears beside the
navigation sidebar:

```text
[ DATABASES ]                    [ application_db ]        
  [▼] Development PostgreSQL                                 
      ├─ postgres                 [ Tables ] [ Views ]       
      ├─ [●] application_db                                  
      └─ [ + ] Database            [▼] users                 
                                      id          integer    
                                  email       varchar        
                                  created_at  timestamp      
                                                             
                                  [▶] projects               
                                  [▶] orders                 
```

The sidebar/content boundary must use `h_resizable` and
`resizable_panel`. The ASCII layout describes behavior and hierarchy only;
production rendering must use the `gpui_component` widgets listed in this
specification.

### Schema tree

Use the component tree/list primitives:

- `Tree` or `uniform_list` for virtualized table/column rows.
- `Scrollbar` for independent schema scrolling.
- `Input` if schema filtering is added.
- `ContextMenuExt` / `PopupMenu` for table actions.
- `Collapsible` only where it matches the component's intended tree behavior;
  do not recreate expansion state with unrelated nested `div`s.

Each table row should show its name and column count. Expanded columns should
show name, type, nullable state, and primary-key/foreign-key indicators.

### Workbench (later phase)

The workbench must be implemented only after connection and schema discovery
are stable.

Use:

- Existing editor/input infrastructure for SQL editing.
- `InputState` only for a lightweight first editor if the existing editor is
  not yet integrated.
- `Button` and `ButtonGroup` for Run, Cancel, Clear, and Load SQL.
- `Tag`/status styling for row count, affected count, execution time, and
  connection state.
- `uniform_list` or table components for result rows.
- `Scrollbar` for both horizontal and vertical result navigation.
- `ContextMenuExt` for copy/export/reuse query actions.

Workbench state must support:

- Draft SQL.
- Running/cancelled state.
- Result set or affected-row count.
- Error display.
- Execution duration.
- Query history.
- Persistence per database ID.

### Schema graph (later phase)

The graph is also deferred until the connection and metadata phases are
complete.

Preferred implementation:

- Reuse `gpui_flow` if its node/edge semantics remain suitable.
- Use `FlowNode` for tables and `FlowEdge` for foreign-key relationships.
- Use the existing fit, zoom, minimap, selection, and nested hit-testing
  behavior.
- Render table nodes with `gpui_component` typography, borders, tags, and
  themed surfaces.
- Use graph context menus for preview, copy table name, generate SQL, and
  focus-related-tables actions.

Graph layout and positions should be persisted separately from connection
metadata, keyed by database ID.

## Connection navigation phase plan

The connection/database navigation is delivered before the workbench and
schema graph. Each phase must preserve the shared UI model so adding a new
driver does not create a separate panel implementation.

### Navigation Phase 1: Shared models and empty states

- Define connection, database, status, selection, and expansion models.
- Define stable connection and database IDs.
- Implement the panel shell and the top-level `Connections` list.
- Add the top-level `[ + ]` action and blank connection form entry point.
- Render loading, empty, disconnected, and error states.
- Keep the schema/content area as an explicit deferred state.

Exit criteria: the panel renders the approved hierarchy with no backend
connection required, and only one connection can be expanded at a time.

### Navigation Phase 2: Connection creation and persistence

- Implement the shared connection form for all supported database types.
- Validate required fields and driver-specific fields.
- Persist non-secret connection metadata.
- Load saved connections on startup.
- Store passwords and secrets only through the existing credentials provider.
- Add connection row actions for edit, reconnect, disconnect, and delete.
- Allow multiple connection sessions to remain active simultaneously.

Exit criteria: users can create, edit, persist, reconnect, disconnect, and
delete connections without credentials appearing in serialized state.

### Navigation Phase 3: Database discovery and selection

- Discover databases after a connection succeeds.
- Rediscover databases on startup and expose a refresh action.
- Render each connection's own database list.
- Add the database `[ + ]` menu with `Create database` and
  `Register existing database`.
- Connect to the selected database.
- Open the schema view beside the sidebar after database selection.
- Preserve the selected connection/database across panel re-renders and
  application restarts where the selection is still valid.

Exit criteria: the approved interaction flow works end to end:
connection selection expands its databases, database selection connects and
opens schema content, and another connection can be selected without closing
existing active sessions.

### Navigation Phase 4: Schema sidebar

- Load tables and columns for the selected database.
- Replace the schema placeholder with the component tree/list UI.
- Add table expansion, column details, key indicators, refresh, and filtering.
- Add table-level context menus.
- Handle loading, empty schema, permission errors, stale connections, and
  reconnect states.

Exit criteria: the connection navigation and schema sidebar are reliable
independently of the later workbench and graph.

### Navigation Phase 5: Workbench and graph integration

- Add the SQL workbench only after Navigation Phases 1-4 are proven.
- Add query results, query history, and workbench persistence.
- Add the schema graph only after foreign-key metadata is reliable.
- Keep workbench and graph selection scoped to the active database ID.
- Preserve the navigation sidebar while switching between schema, workbench,
  and graph views.

Exit criteria: the later workbench and graph consume the same selected
connection/database state without duplicating connection lifecycle logic.

## Delivery phases

### Phase 0: Architecture and dependency validation

- Confirm the application startup path initializes `gpui_component`.
- Confirm theme bridging is available for the target panel.
- Confirm `credentials_provider` and `zed_credentials_provider` can be used
  from the new backend/panel crates.
- Decide whether SQLite uses `sqlez` directly or a backend adapter.
- Add crate skeletons and typed model definitions only.

Exit criteria: crates compile and no UI or driver code owns credentials
directly.

### Phase 1: SQLite connection proof

- Implement SQLite connect/disconnect.
- Read/write credentials through the existing provider where applicable.
- Add connection status/error types.
- Add stable connection IDs.
- Implement basic persistence for non-secret connection metadata.
- Build the first `Settings`-based SQLite connection form.
- Add focused backend tests for valid paths, invalid paths, duplicate IDs,
  disconnect, and missing connections.

Exit criteria: a user can connect to and disconnect from a SQLite database,
restart the application, and reconnect without a plaintext password in normal
state.

### Phase 2: Metadata proof

- Implement SQLite table/column metadata.
- Build the schema sidebar using the component tree/list stack.
- Add refresh, loading, empty, and error states.
- Add table expansion and column details.
- Add schema cache persistence keyed by database ID.

Exit criteria: connection and schema metadata remain reliable across panel
re-rendering, tab switching, refresh, reconnect, and application restart.

### Phase 3: Network connection support

- Add PostgreSQL, MySQL/MariaDB, and MSSQL drivers.
- Add normalized connection configuration and SSL options.
- Add network connection forms using the same settings components.
- Add reconnect-on-stale-connection behavior.
- Add per-driver metadata queries.
- Add integration tests where local test services are available; otherwise
  retain deterministic parser/unit tests and explicit manual validation.

Exit criteria: all supported connection types share the same backend contract
and UI state model.

### Phase 4: Workbench and query execution

- Implement query execution and cancellation.
- Add SQL editor integration.
- Add result/affected-row models.
- Add results table with bounded rendering.
- Add query errors, timing, and empty states.
- Add query history and copy/reuse actions.
- Persist draft SQL, last result metadata, and history separately from
  credentials.

Exit criteria: a user can run safe read/write SQL, see typed stringified
results and errors, cancel long work where supported, and restore the draft
and history for a database.

### Phase 5: Schema graph

- Add foreign-key metadata for each supported driver.
- Build table nodes and relationship edges.
- Add fit view, zoom, pan, minimap, and selection.
- Add table context menus and focus behavior.
- Cache foreign-key metadata and persist graph layout.

Exit criteria: graph relationships match backend metadata and remain usable
with large schemas without blocking the main panel.

### Phase 6: Advanced database UX

- Inline database browser and database creation.
- Table sample previews.
- Export to JSON/SQL/CSV where supported.
- SQL completion based on current schema.
- Optional AI query suggestions.
- Performance profiling and large-schema virtualization.

These features must not weaken credential isolation or make connection errors
silent.

## Testing strategy

Backend tests should cover URL/config normalization, secret-key generation,
connection lifecycle, reconnect behavior, metadata conversion, and driver
errors.

GPUI tests should cover:

- Opening and submitting connection forms.
- Validation and error presentation.
- Switching server/database selections.
- Refreshing schema.
- Expanding/collapsing tables.
- Workbench query state transitions in the later phase.
- Graph selection and context menus in the later phase.

Use the repository's existing GPUI test conventions and scheduler
reproduction controls for asynchronous connection/query tests.

## Definition of done

The full feature is complete when:

- All supported drivers use the common backend contract.
- Passwords are keychain-backed and absent from ordinary persistence/logging.
- The connection/schema milestone works independently of the workbench and
  graph.
- The workbench is a later, independently testable layer.
- The graph is a later, independently testable layer.
- All visible UI uses `gpui_component` widgets and layout primitives.
- Loading, empty, error, reconnect, cancellation, and large-result states are
  explicit rather than silently ignored.
