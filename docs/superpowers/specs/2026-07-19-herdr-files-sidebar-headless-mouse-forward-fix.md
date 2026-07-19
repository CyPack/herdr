# Herdr Files Sidebar Headless Mouse Forward-Fix

Date: 2026-07-19
Status: implemented (`dbfa55be` RED / `72cdce83` GREEN)
Scope: visible Files shortcut rows (`Home`, `Desktop`, `Downloads`, configured
pins, media folders, `Trash`, and `Root`)

## Problem

The Files sidebar renders accessible shortcut rows and direct `App` mouse tests
prove that a plain primary click prepares an exact, model-revalidated path.
Those tests do not cover the server-owned runtime used by remote clients.

The server has a separate scheduled-task loop. The monolithic loop calls
`sync_file_manager_sidebar_navigation()`, while
`HeadlessServer::handle_scheduled_tasks_headless()` does not. A remote click
therefore reaches the sidebar handler and prepares the one-shot request, but
the request remains unconsumed. Miller row clicks still work because they
mutate their typed navigation state through a different path.

## Product Contract

1. A plain primary press on any visible, accessible Files shortcut opens that
   exact directory in the existing Native Files generation.
2. The same contract holds for local/monolithic and remote/headless app
   clients.
3. Modified, non-primary, stale, inaccessible, hidden, collapsed, and
   overlay-blocked hits remain inert.
4. The consumer revalidates current sidebar authority and live directory kind;
   no coordinate alone authorizes filesystem navigation.
5. The change is client-presentation behavior hosted by the server App. It adds
   no protocol field, socket message, dependency, or shared runtime fact.

## Dependency Chain

```text
host SGR mouse bytes
  -> per-client RawInputFramer
  -> HeadlessServer::handle_client_input_events
  -> App::route_client_events
  -> AppState::handle_mouse
  -> file_manager_sidebar_path_at
  -> request_file_manager_sidebar_navigation
  -> HeadlessServer::handle_scheduled_tasks_headless
  -> App::sync_file_manager_sidebar_navigation
  -> FmState::open_trail_to
  -> next render_and_stream projection
```

The missing edge is the headless scheduled loop to the existing typed
consumer. Parser, hit geometry, request preparation, filesystem revalidation,
and Trail loading already exist.

## Test Points

| ID | Test | Expected result | Reason |
|---|---|---|---|
| `TP-FMR-SIDEBAR-HL-01` | raw SGR primary-down enters a headless app client at the rendered Home row | exact typed request is prepared | covers the real host-byte and client framing path |
| `TP-FMR-SIDEBAR-HL-02` | headless scheduled tick follows that input | request is consumed once; cwd, Trail root, and snapshot root equal the target | reproduces the live missing consumer |
| `TP-FMR-SIDEBAR-HL-03` | existing Files generation before the click | generation is unchanged | shortcut navigation must reuse the singleton Files surface |
| `TP-FMR-SIDEBAR-HL-04` | modified/non-primary/stale/inaccessible/collapsed/overlay cases | no navigation mutation | preserves fail-closed authority |
| `TP-FMR-SIDEBAR-HL-05` | plain headless scheduled tick without a pending request | returns no false file-manager change | prevents redraw churn |
| `TP-FMR-SIDEBAR-VIS-15` | Playwright Chromium Files fixture after navigation | selected destination and Trail root remain visually coherent | mandatory human-visible regression gate; it does not claim native mouse delivery |

## Non-Goals

- Miller horizontal-scroll ranking or behavior changes.
- New shortcut rows or shortcut configuration UX.
- Replacing the typed request with direct filesystem I/O in the mouse handler.
- Stable Herdr installation, stable sockets, release assets, upstream delivery,
  or plugin adoption.

## Acceptance

- The server-level test fails before the product fix at the scheduled consumer
  assertion and passes after the minimal seam is restored.
- Existing sidebar adversarial families remain green.
- Playwright Chromium, full Rust, Linux clippy, Windows clippy, formatting,
  maintenance-script, and repository hygiene gates pass.
- Codebase graph is reindexed single-worker and both CyPack branches are
  fast-forwarded to the verified commit.
