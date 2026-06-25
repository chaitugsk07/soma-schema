CREATE TABLE vault.environments (
    id         uuid        NOT NULL DEFAULT gen_random_uuid(),
    project_id uuid        NOT NULL REFERENCES vault.projects (id),
    name       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT environments_pkey PRIMARY KEY (id)
);

CREATE TABLE vault.secrets (
    id             uuid        NOT NULL DEFAULT gen_random_uuid(),
    environment_id uuid        NOT NULL REFERENCES vault.environments (id),
    key            text        NOT NULL,
    value          text        NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT secrets_pkey PRIMARY KEY (id)
);

-- DOWN ==
DROP TABLE IF EXISTS vault.secrets;
DROP TABLE IF EXISTS vault.environments;
