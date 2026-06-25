-- UP: Seed environments and secrets for the sample projects.
INSERT INTO vault.environments (id, project_id, name)
VALUES
    ('00000000-0000-0000-0000-0000000000e1',
     '00000000-0000-0000-0000-0000000000d1',
     'production'),
    ('00000000-0000-0000-0000-0000000000e2',
     '00000000-0000-0000-0000-0000000000d2',
     'staging')
ON CONFLICT (id) DO NOTHING;

INSERT INTO vault.secrets (id, environment_id, key, value)
VALUES
    ('00000000-0000-0000-0000-0000000000f1',
     '00000000-0000-0000-0000-0000000000e1',
     'DATABASE_URL', 'postgres://placeholder/prod'),
    ('00000000-0000-0000-0000-0000000000f2',
     '00000000-0000-0000-0000-0000000000e2',
     'STRIPE_KEY',   'sk_test_placeholder')
ON CONFLICT (id) DO NOTHING;

-- DOWN ==
DELETE FROM vault.secrets WHERE id IN (
    '00000000-0000-0000-0000-0000000000f1',
    '00000000-0000-0000-0000-0000000000f2'
);
DELETE FROM vault.environments WHERE id IN (
    '00000000-0000-0000-0000-0000000000e1',
    '00000000-0000-0000-0000-0000000000e2'
);
