-- 01_migrated/1/20260101_01_init.sql
CREATE TABLE IF NOT EXISTS example_items (
    id   BIGSERIAL    PRIMARY KEY,
    name TEXT         NOT NULL
);

-- DOWN ==
DROP TABLE IF EXISTS example_items;
