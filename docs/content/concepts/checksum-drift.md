+++
title = "Checksum drift"
description = "What the SHA-256 checksum covers, when drift errors fire, and how to recover."
weight = 20
+++

<div class="callout callout-drift"><span class="callout-icon">⚠</span><p>Editing any file that has already been applied — even a comment — will trigger <code>ChecksumDrift</code> on the next <code>up</code>, <code>down</code>, or <code>status</code> run. The fix is to revert the edit or write a new migration.</p></div>

## What is covered

The SHA-256 checksum is computed over the raw bytes of the entire `.sql` file — UP and DOWN sections together, including comments and whitespace. It is computed once on first apply and stored in the `checksum` column of the tracking table.

On every subsequent run of `up`, `down`, or `status`, soma-schema recomputes the checksum for each file on disk and compares it against the stored value. If they differ, the command aborts with a `ChecksumDrift` error before any migration executes.

## Why the whole file

Tools that hash only the UP section miss changes to the DOWN section. This matters in practice: a developer edits a deployed DOWN section to fix a rollback bug, the change is not caught, and later a rollback runs the changed (possibly broken) SQL against production — with no audit record that the file was modified after deployment.

soma-schema catches any change to either section. If the DOWN SQL was edited after deployment, you will know before you run anything.

## When drift fires

`ChecksumDrift` fires at the start of the first command run after the file was edited. It does not fire at the moment of editing — soma-schema only reads files when you invoke a command. The error includes the filename, the expected checksum (from the tracking table), and the actual checksum (recomputed from disk).

## Recovering from drift

`ChecksumDrift` is a deployment error, not a recoverable runtime condition. The correct responses are:

1. **Revert the edit.** If the file was changed accidentally, restore it to the deployed version. The checksum will match again.

2. **Write a new migration.** If the change was intentional — you want to alter the schema further — write a new migration that applies the additional change. Never edit an applied file to change what it does.

3. **Manual intervention (last resort).** If the file was changed and cannot be reverted (e.g. the original is gone), you can update the `checksum` column in the tracking table to match the current file. This should be treated as an incident-level action with a clear audit trail — it effectively tells soma-schema "trust this version of the file going forward."

## Comments and whitespace

Comments count. Adding or removing a comment in an applied migration changes the checksum. This is intentional — the checksum is over the raw file bytes, not parsed SQL. It means there is no safe way to edit an applied migration file, which is the correct constraint: the file is the audit record of what was deployed.
