CREATE TABLE vault.users (
    id         uuid        NOT NULL DEFAULT gen_random_uuid(),
    email      text        NOT NULL,
    name       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT users_pkey PRIMARY KEY (id),
    CONSTRAINT users_email_key UNIQUE (email)
);

-- DOWN ==
DROP TABLE IF EXISTS vault.users;
