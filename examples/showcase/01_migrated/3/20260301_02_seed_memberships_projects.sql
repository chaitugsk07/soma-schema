-- UP: Seed memberships and projects linking orgs and users.
INSERT INTO vault.organization_memberships (id, org_id, user_id, role)
VALUES
    ('00000000-0000-0000-0000-0000000000c1',
     '00000000-0000-0000-0000-0000000000a1',
     '00000000-0000-0000-0000-0000000000b1',
     'owner'),
    ('00000000-0000-0000-0000-0000000000c2',
     '00000000-0000-0000-0000-0000000000a2',
     '00000000-0000-0000-0000-0000000000b2',
     'admin')
ON CONFLICT (id) DO NOTHING;

INSERT INTO vault.projects (id, org_id, name, description)
VALUES
    ('00000000-0000-0000-0000-0000000000d1',
     '00000000-0000-0000-0000-0000000000a1',
     'Launchpad', 'Main product project'),
    ('00000000-0000-0000-0000-0000000000d2',
     '00000000-0000-0000-0000-0000000000a2',
     'Reactor',   'Internal tooling')
ON CONFLICT (id) DO NOTHING;

-- DOWN ==
DELETE FROM vault.projects WHERE id IN (
    '00000000-0000-0000-0000-0000000000d1',
    '00000000-0000-0000-0000-0000000000d2'
);
DELETE FROM vault.organization_memberships WHERE id IN (
    '00000000-0000-0000-0000-0000000000c1',
    '00000000-0000-0000-0000-0000000000c2'
);
