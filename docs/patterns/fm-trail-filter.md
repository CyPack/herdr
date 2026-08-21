# FM · Trail filter as a snapshot-owned projection (2026-08-21)

`/` filters the active trail column. The load-bearing decisions, for reuse:

- **The filter lives on `TrailSnapshots`, keyed to a DIRECTORY** — the same
  home `show_hidden` has. Columns shift as branches open and close; an
  index-keyed filter silently starts narrowing somebody else's directory
  (TP-FM-FILTER-02, pinned by the cannot-narrow-another-column test).
- **Projection, never re-indexing.** `filtered_indices` returns TRUE indices
  into `entries`; rows, movement (`move_cursor_in_column` walks the allowed
  list), and hit-testing all keep addressing the entry the person sees —
  operations can never target a ghost one off from the highlight
  (TP-FM-FILTER-01).
- **Zero signature ripple.** Because the snapshots own the filter, the six
  `project_trail_view*` wrappers and their twelve call sites never changed —
  the projection is read inside `trail_logical_lines_filtered` and inside
  movement. Prefer state-owned policy over threading a parameter.
- **Cursor is always a member**: after every keystroke a zero-delta move
  through the ordinary road normalizes it onto a match — reusing the road
  brings the operation projection along for free.
- **The echo survives truncation**: the identity line composes the `/pattern`
  suffix AFTER truncating the path, because a filter clipped away with the
  path's tail narrows the listing invisibly (caught by a red test at 69
  columns; TP-FM-FILTER-03).
- Entering a directory clears the filter at the `select_dir` seam; a filter
  whose directory left the visible trail is dropped by `sync`.

Landing: `c848f3e8` + pins `4a969ff5`/`636f7d5a`. Registry: TP-FM-FILTER-01/02/03.
