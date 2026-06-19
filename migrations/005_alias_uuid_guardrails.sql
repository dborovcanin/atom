-- Keep database enforcement aligned with the application alias validator.
-- Aliases use the same character set as UUID text, so the slug CHECK alone
-- would otherwise allow canonical or compact UUID strings.

----------------------------------------------------------------------
-- Pre-flight: fail LOUD if any existing alias is UUID-shaped.
--
-- A UUID-shaped string (e.g. a legacy tenant route, or a value from a
-- direct SQL/import path) is a valid slug, so it survives migration 004
-- but would collide with id-addressing. Catch it here with operator
-- guidance rather than letting the ADD CONSTRAINT below abort with a raw
-- check-violation error. Matches validate_alias() in the app layer.
----------------------------------------------------------------------

DO $$
DECLARE
    uuid_re   text := '^([0-9a-f]{32}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$';
    bad_count bigint;
BEGIN
    SELECT
        (SELECT count(*) FROM tenants   WHERE alias IS NOT NULL AND lower(alias) ~ uuid_re)
      + (SELECT count(*) FROM entities  WHERE alias IS NOT NULL AND lower(alias) ~ uuid_re)
      + (SELECT count(*) FROM resources WHERE alias IS NOT NULL AND lower(alias) ~ uuid_re)
    INTO bad_count;

    IF bad_count > 0 THEN
        RAISE EXCEPTION
            'Migration 005: % alias(es) are UUID-shaped, which collides with id-addressing. Rename or clear them before migrating.',
            bad_count;
    END IF;
END $$;

ALTER TABLE tenants
    ADD CONSTRAINT chk_tenants_alias_not_uuid
    CHECK (
        alias IS NULL OR alias !~ (
            '^([0-9a-f]{32}|'
            '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$'
        )
    );

ALTER TABLE entities
    ADD CONSTRAINT chk_entities_alias_not_uuid
    CHECK (
        alias IS NULL OR alias !~ (
            '^([0-9a-f]{32}|'
            '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$'
        )
    );

ALTER TABLE resources
    ADD CONSTRAINT chk_resources_alias_not_uuid
    CHECK (
        alias IS NULL OR alias !~ (
            '^([0-9a-f]{32}|'
            '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$'
        )
    );
