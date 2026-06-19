-- Aliases (human-friendly unique handles over UUIDs) for tenants, entities, resources.
--
-- Renames the legacy tenant `route` column to the neutral, IAM-friendly `alias`
-- and extends the same primitive one level down to entities (clients/devices)
-- and resources (channels). Tenant aliases are globally unique; entity/resource
-- aliases are unique *per tenant* (the same alias may be reused across tenants).
--
-- An alias stays an alias, NOT a replacement: the UUID remains the canonical
-- identity (authz key, audit key, FK target). Aliases are renameable and
-- case-folded.

----------------------------------------------------------------------
-- 1. Rename the existing tenant route -> alias (carry data; drop old index)
----------------------------------------------------------------------

ALTER TABLE tenants RENAME COLUMN route TO alias;
DROP INDEX IF EXISTS idx_tenants_route;

----------------------------------------------------------------------
-- 2. New alias columns
----------------------------------------------------------------------

ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS alias TEXT;

ALTER TABLE resources
    ADD COLUMN IF NOT EXISTS alias TEXT;

----------------------------------------------------------------------
-- 3. Hardening of existing tenant aliases
--
-- The legacy normalize only trimmed, so existing tenant aliases may not be
-- slug-shaped or may collide once compared case-insensitively. Case-fold first,
-- then fail LOUD (rather than silently dropping data) if anything still violates
-- the slug rule or collides. The operator must clean offending rows first.
----------------------------------------------------------------------

UPDATE tenants
   SET alias = lower(alias)
 WHERE alias IS NOT NULL
   AND alias <> lower(alias);

DO $$
DECLARE
    bad_pattern bigint;
    case_dupes  bigint;
BEGIN
    SELECT count(*) INTO bad_pattern
      FROM tenants
     WHERE alias IS NOT NULL
       AND alias !~ '^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$';
    IF bad_pattern > 0 THEN
        RAISE EXCEPTION
            'Migration 004: % tenant alias(es) are not valid slugs (lowercase, no leading/trailing dash, max 63). Fix or clear them before migrating.',
            bad_pattern;
    END IF;

    SELECT count(*) INTO case_dupes FROM (
        SELECT lower(alias)
          FROM tenants
         WHERE alias IS NOT NULL
         GROUP BY lower(alias)
        HAVING count(*) > 1
    ) d;
    IF case_dupes > 0 THEN
        RAISE EXCEPTION
            'Migration 004: % tenant alias(es) collide case-insensitively. Resolve before migrating.',
            case_dupes;
    END IF;
END $$;

----------------------------------------------------------------------
-- 4. Slug shape constraints (belt-and-suspenders to the app-layer validator)
----------------------------------------------------------------------

ALTER TABLE tenants
    ADD CONSTRAINT chk_tenants_alias_slug
    CHECK (alias IS NULL OR alias ~ '^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$');

ALTER TABLE entities
    ADD CONSTRAINT chk_entities_alias_slug
    CHECK (alias IS NULL OR alias ~ '^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$');

ALTER TABLE resources
    ADD CONSTRAINT chk_resources_alias_slug
    CHECK (alias IS NULL OR alias ~ '^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$');

----------------------------------------------------------------------
-- 5. Uniqueness
--   tenants:            global single namespace (case-folded)
--   entities/resources: scoped per tenant; NULL tenant_id folded to the zero
--                       UUID so global (NULL-tenant) objects share one namespace
--                       (matching the roles / 003 guardrail pattern).
----------------------------------------------------------------------

CREATE UNIQUE INDEX idx_tenants_alias
    ON tenants (lower(alias))
    WHERE alias IS NOT NULL;

CREATE UNIQUE INDEX idx_entities_alias
    ON entities (COALESCE(tenant_id, '00000000-0000-0000-0000-000000000000'::uuid), lower(alias))
    WHERE alias IS NOT NULL;

CREATE UNIQUE INDEX idx_resources_alias
    ON resources (COALESCE(tenant_id, '00000000-0000-0000-0000-000000000000'::uuid), lower(alias))
    WHERE alias IS NOT NULL;
