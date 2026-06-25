ALTER TABLE vault.projects ADD COLUMN description text;

-- DOWN ==
ALTER TABLE vault.projects DROP COLUMN IF EXISTS description;
