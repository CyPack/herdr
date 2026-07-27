# Session guard — registered behaviors

Fork feature (PM-2026-07-27-001 hardening). Upstream has neither the
identity-pinned pane sweep nor the live-agent stop guard, so nothing upstream
preserves these across syncs: every entry here is ours to keep alive.

Origin: on 2026-07-27 a bare `herdr server stop` swept nine live agent
sessions; the surrounding desktop collapse turned a routine stop into the
loss of every running conversation. Two independent protections came out of
the post-mortem: kill-by-identity (a stored pid is not an identity — pid
numbers recycle) and a stop guard (killing live agents must be a deliberate
choice, not a side effect).

Format and rules: [`README.md`](README.md).

---

## Identity-pinned pane sweep (Linux pidfd)

| ID | Behavior | Breaks if lost | Verified by |
| --- | --- | --- | --- |
| SG-SWEEP-1 | Every Linux sweep target is pidfd-pinned: the fd is opened BEFORE the session-id verification, so a pid recycled between scan and signal cannot be hit. | A pid stored at spawn can name a stranger by teardown; the sweep signals unrelated processes (the exact ABA hazard from the incident). | `shutdown_targets_signal_via_pidfd_and_terminate` |
| SG-SWEEP-2 | A stale (already-reaped) child pid yields ZERO targets — the old blind `pids.push(child_pid)` fallback is gone. | Teardown of a long-dead pane signals whatever process now owns the recycled pid. | `stale_child_pid_yields_no_targets` |
| SG-SWEEP-3 | A non-session-leader child pins only itself; the session sweep is skipped (fail-safe: leak grandchildren, never signal strangers). | A mismatched session id silently widens the kill set to a foreign kernel session. | `shutdown_targets_signal_via_pidfd_and_terminate` |
| SG-SWEEP-4 | PTY children are session leaders (sid == pid) — the anchor assumption of the sweep. | The sweep silently degrades to single-child mode on every pane and grandchildren leak on each teardown. | `pty_child_is_its_own_session_leader` |
| SG-SWEEP-5 | On kernels without pidfd (ENOSYS) the legacy pid path still delivers signals; a dead child (ESRCH) delivers none. | Old kernels silently lose pane teardown, or dead panes signal recycled pids. | `child_open_failure_mapping_enosys_vs_esrch` |

## Live-agent stop guard

| ID | Behavior | Breaks if lost | Verified by |
| --- | --- | --- | --- |
| SG-STOP-1 | `server.stop` without `force` is refused while any pane has a detected agent; the server stays up and the error names the agents and the `--force` way out. | A routine stop silently kills live agent sessions and their unsaved context — the incident's trigger repeats. | `stop_without_force_is_refused_while_agent_pane_lives` |
| SG-STOP-2 | `server.stop` with `force: true` proceeds despite live agents (deliberate override). | Operators with agents always running can never stop the server. | `stop_with_force_proceeds_despite_agent_pane` |
| SG-STOP-3 | With no detected agents a bare stop needs no force — the guard produces zero friction in the common case. | Cry-wolf refusals train operators to always pass `--force`, turning the guard into theater. | `stop_without_agents_needs_no_force` |
| SG-STOP-4 | The refusal message counts panes per agent label. | The operator cannot judge what would die without attaching first. | `guard_message_counts_agents` |
| SG-STOP-5 | The self-update flow stops the old server with `force: true` — updates never wedge behind the guard (the flow already warns that stopping exits pane processes). | `herdr update` deadlocks on any machine that always has agents running. | `update_stop_request_forces_past_the_agent_guard` |
