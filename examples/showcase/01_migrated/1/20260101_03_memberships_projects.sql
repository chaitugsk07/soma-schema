CREATE TABLE vault.organization_memberships (
    id         uuid        NOT NULL DEFAULT gen_random_uuid(),
    org_id     uuid        NOT NULL REFERENCES vault.organizations (id),
    user_id    uuid        NOT NULL REFERENCES vault.users (id),
    role       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT organization_memberships_pkey PRIMARY KEY (id)
);

CREATE TABLE vault.projects (
    id         uuid        NOT NULL DEFAULT gen_random_uuid(),
    org_id     uuid        NOT NULL REFERENCES vault.organizations (id),
    name       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT projects_pkey PRIMARY KEY (id)
);

-- DOWN ==
DROP TABLE IF EXISTS vault.projects;
DROP TABLE IF EXISTS vault.organization_memberships;
