-- UP: Seed initial widget registry rows.
-- Seeds are ordinary migrations whose SQL is idempotent (ON CONFLICT DO NOTHING).
INSERT INTO example.widgets (id, name, description)
VALUES
    ('00000000-0000-0000-0000-000000000001', 'alpha', 'First widget'),
    ('00000000-0000-0000-0000-000000000002', 'beta',  'Second widget')
ON CONFLICT (id) DO NOTHING;

-- DOWN ==
DELETE FROM example.widgets
WHERE id IN (
    '00000000-0000-0000-0000-000000000001',
    '00000000-0000-0000-0000-000000000002'
);
