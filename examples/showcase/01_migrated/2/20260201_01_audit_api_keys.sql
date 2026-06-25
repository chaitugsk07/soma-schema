CREATE TABLE vault.audit_logs (
    id         uuid        NOT NULL DEFAULT gen_random_uuid(),
    org_id     uuid        NOT NULL,
    user_id    uuid        NOT NULL,
    action     text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT audit_logs_pkey PRIMARY KEY (id)
);

ALTER TABLE vault.audit_logs
    ADD CONSTRAINT audit_logs_org_id_fkey
    FOREIGN KEY (org_id) REFERENCES vault.organizations (id);

ALTER TABLE vault.audit_logs
    ADD CONSTRAINT audit_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES vault.users (id);

CREATE TABLE vault.api_keys (
    id         uuid        NOT NULL DEFAULT gen_random_uuid(),
    org_id     uuid        NOT NULL REFERENCES vault.organizations (id),
    name       text        NOT NULL,
    hash       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT api_keys_pkey PRIMARY KEY (id)
);

-- DOWN ==
DROP TABLE IF EXISTS vault.api_keys;
DROP TABLE IF EXISTS vault.audit_logs;
