-- UP: Seed two sample organizations and two users.
-- Seeds use fixed UUIDs so FKs resolve and ON CONFLICT makes re-runs safe.
INSERT INTO vault.organizations (id, name, slug)
VALUES
    ('00000000-0000-0000-0000-0000000000a1', 'Acme Corp',  'acme'),
    ('00000000-0000-0000-0000-0000000000a2', 'Globex Inc', 'globex')
ON CONFLICT (id) DO NOTHING;

INSERT INTO vault.users (id, email, name)
VALUES
    ('00000000-0000-0000-0000-0000000000b1', 'alice@acme.example', 'Alice Nguyen'),
    ('00000000-0000-0000-0000-0000000000b2', 'bob@globex.example', 'Bob Patel')
ON CONFLICT (id) DO NOTHING;

-- DOWN ==
DELETE FROM vault.users WHERE id IN (
    '00000000-0000-0000-0000-0000000000b1',
    '00000000-0000-0000-0000-0000000000b2'
);
DELETE FROM vault.organizations WHERE id IN (
    '00000000-0000-0000-0000-0000000000a1',
    '00000000-0000-0000-0000-0000000000a2'
);
