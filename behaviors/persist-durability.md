# Crash-durable state files — registered behaviors

Fork feature. Every state file herdr owns (`session.json`, `session-history.json`,
`workspace-chats.json`, `closed-agents.json`, `plugins.json`) is written and read
through one road, `src/persist/durable.rs`.

Why (measured 2026-08-26 on a btrfs home): the files were `fs::write(tmp)` +
`rename` with no `fsync`. A system crash landed the renames before the bytes,
every file came back empty, every loader "degraded to empty" as designed, and
the next save overwrote the last good copy. 28 panes, 350 filed chats and every
module row were lost with nothing on disk to recover from.

Two ideas run through the family:

- **The rename is the commit.** Bytes are synced before the rename and the
  directory after it, so the visible file is always the previous one or the
  complete new one.
- **A file that does not parse is evidence, not a slot to overwrite.** It is
  moved aside with a timestamp and the previous save stands in.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-PERSIST-01 | A save is `tmp → fsync(file) → rename → fsync(dir)`; a failed sync leaves the previous file untouched and no temp file behind. | A crash between the rename and the data reaching disk produces an empty file that the next start reads as corrupt — the exact 2026-08-26 loss. | `a_save_syncs_the_bytes_before_the_rename_and_the_directory_after`, `a_failed_file_sync_leaves_the_previous_file_untouched` |
| TP-PERSIST-02 | Before a save replaces a readable (non-empty) previous file, that file is kept as `<name>.bak`; a zero-byte file is never promoted to backup. | One save back is the only copy that exists after a torn primary; promoting the torn file would destroy it. | `a_save_keeps_the_previous_readable_file_as_a_backup`, `an_empty_previous_file_is_not_promoted_to_backup` |
| TP-PERSIST-03 | A file that fails to parse (torn, malformed, or from a newer version) is moved aside as `<name>.corrupt-<secs>` and the `.bak` copy is loaded instead; with no backup the loader starts empty but the evidence survives; a missing primary with a backup restores the backup. | The loader's "degrade to empty" contract silently turns into "destroy on next save". | `a_corrupt_file_is_quarantined_and_the_backup_is_restored`, `a_corrupt_file_without_backup_is_quarantined_not_overwritten`, `a_newer_version_file_is_preserved_the_same_way`, `a_missing_primary_with_a_backup_restores_the_backup`, `a_missing_file_with_no_backup_is_a_first_run` |
| TP-PERSIST-04 | The session snapshot, the workspace-chat ledger and the closed-agent graveyard all take that road: a torn file of each is quarantined and its previous save restored, and a session save leaves a `.bak`. | One store drifting back to a bare `fs::write` reopens the loss for that store alone, invisibly. | `a_corrupt_session_file_is_quarantined_and_the_backup_restored`, `a_session_save_leaves_a_backup_of_the_previous_session`, `a_torn_ledger_is_quarantined_and_the_previous_save_restored`, `a_torn_graveyard_is_quarantined_and_the_previous_save_restored` |
