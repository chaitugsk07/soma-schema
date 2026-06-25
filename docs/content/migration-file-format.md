+++
title = "Migration file format"
description = "The SQL file format — UP section, DOWN section, delimiter, seeds, and naming."
weight = 30
+++

Each migration is a plain `.sql` file that holds both the apply and revert SQL, separated by a delimiter line.

## Structure

```sql
-- UP section (everything before the delimiter)
CREATE TABLE myapp.widgets (
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);

-- DOWN ==
DROP TABLE IF EXISTS myapp.widgets;
```

The delimiter line must be exactly `-- DOWN ==` when trimmed of leading and trailing whitespace. Everything before the first occurrence of this line is the UP section; everything after it is the DOWN section.

## Rules

- The DOWN section is optional in the file, but if you try to roll back a migration that lacks one, soma-schema returns an error. Write a DOWN section for every migration unless it is genuinely irreversible.
- The checksum is SHA-256 of the **entire file** — UP and DOWN together. Editing either section after the migration has been applied to any environment triggers a `ChecksumDrift` error on the next command run. To change deployed schema, write a new migration.
- File names have no required format, but `<YYYYMMDD>_<NN>_<descriptive_name>.sql` is the conventional pattern used across soma services.

## Seeds

Seeds are ordinary migration files whose UP SQL is idempotent:

```sql
INSERT INTO myapp.reference_data (code, label)
VALUES ('USD', 'US Dollar'), ('EUR', 'Euro')
ON CONFLICT (code) DO NOTHING;

-- DOWN ==
DELETE FROM myapp.reference_data WHERE code IN ('USD', 'EUR');
```

Using `ON CONFLICT DO NOTHING` (or `INSERT OR IGNORE` on SQLite) means the seed can be safely re-run after a rollback.

## File naming

soma-schema does not impose a file naming convention. The `migration-order.yaml` manifest is the authoritative apply order — filename sort is not used. That said, `<YYYYMMDD>_<NN>_<name>.sql` keeps the directory visually sorted and avoids collisions when multiple developers add migrations on the same day.

## What the checksum covers

The checksum is SHA-256 of the raw bytes of the file as stored on disk. It is computed once on first apply and stored in the [tracking table](@/concepts/tracking-table.md). On every subsequent command run, soma-schema recomputes the checksum and compares it. The checksum covers comments — even changing a comment in an applied file is drift.

See [Checksum drift](@/concepts/checksum-drift.md) for how drift errors work and how to recover from them.
