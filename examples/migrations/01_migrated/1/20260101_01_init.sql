-- UP: Create the widgets table
CREATE TABLE IF NOT EXISTS example.widgets (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT        NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Attach the updated_at trigger.
CREATE OR REPLACE TRIGGER trg_widgets_updated_at
    BEFORE UPDATE ON example.widgets
    FOR EACH ROW EXECUTE FUNCTION example.fn_update_timestamp();

-- DOWN ==
DROP TRIGGER IF EXISTS trg_widgets_updated_at ON example.widgets;
DROP TABLE IF EXISTS example.widgets;
