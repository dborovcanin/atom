# Admin Hygiene Endpoints

## Priority: 3 (Nice-to-have)

---

## Problem

Over time, an authorization system accumulates stale data:
- Policies that reference entities or roles that have been deleted.
- Resources that no policy covers — either intentionally public or accidentally unprotected.
- Credentials that are about to expire, which could break integrations silently.

Without hygiene endpoints, an administrator must write custom SQL queries to surface these issues.

---

## Endpoints

```
GET /admin/orphan-policies
GET /admin/unprotected-resources
GET /admin/expiring-credentials
```

**Authentication:** Bearer token required. All admin endpoints require `RequireManage` (the caller must hold the `manage` capability with `scope_kind = all`).

---

## 1. GET /admin/orphan-policies

Returns policy bindings where the referenced subject (entity or group) or grant (capability or role) no longer exists.

### Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `limit` | int | 50 | Results per page (1-200) |
| `offset` | int | 0 | Pagination offset |

### Response

```json
{
  "items": [
    {
      "id": "p1-...",
      "subject_kind": "entity",
      "subject_id": "aaa-...",
      "grant_kind": "role",
      "grant_id": "v1-...",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "effect": "allow",
      "conditions": {},
      "created_at": "2026-03-15T12:00:00Z",
      "orphan_reason": "subject_not_found"
    },
    {
      "id": "p2-...",
      "subject_kind": "group",
      "subject_id": "g1-...",
      "grant_kind": "capability",
      "grant_id": "c99-...",
      "scope_kind": "all",
      "scope_ref": null,
      "effect": "allow",
      "conditions": {},
      "created_at": "2026-02-01T08:00:00Z",
      "orphan_reason": "grant_not_found"
    }
  ],
  "total": 2
}
```

### `orphan_reason` values

| Value | Meaning |
|---|---|
| `subject_not_found` | The referenced entity or group has been deleted |
| `grant_not_found` | The referenced capability or role has been deleted |

### Implementation notes

```sql
-- Subject orphans (entity)
SELECT pb.* FROM policy_bindings pb
LEFT JOIN entities e ON pb.subject_kind = 'entity' AND pb.subject_id = e.id
LEFT JOIN groups g ON pb.subject_kind = 'group' AND pb.subject_id = g.id
WHERE
  (pb.subject_kind = 'entity' AND e.id IS NULL)
  OR (pb.subject_kind = 'group' AND g.id IS NULL);

-- Grant orphans
SELECT pb.* FROM policy_bindings pb
LEFT JOIN capabilities c ON pb.grant_kind = 'capability' AND pb.grant_id = c.id
LEFT JOIN roles r ON pb.grant_kind = 'role' AND pb.grant_id = r.id
WHERE
  (pb.grant_kind = 'capability' AND c.id IS NULL)
  OR (pb.grant_kind = 'role' AND r.id IS NULL);
```

These can be combined into a single query that checks both subject and grant orphans.

---

## 2. GET /admin/unprotected-resources

Returns resources that have **no policy bindings** covering them — either directly (`scope_kind = resource`), by kind (`scope_kind = resource_kind`), or globally (`scope_kind = all`).

### Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `tenant_id` | UUID | — | Filter by tenant |
| `kind` | string | — | Filter by resource kind |
| `limit` | int | 50 | Results per page (1-200) |
| `offset` | int | 0 | Pagination offset |

### Response

```json
{
  "items": [
    {
      "id": "r5-...",
      "kind": "secret",
      "name": "db-password",
      "tenant_id": "t1-...",
      "owner_id": null,
      "created_at": "2026-04-20T14:00:00Z"
    },
    {
      "id": "r6-...",
      "kind": "device",
      "name": "test-sensor",
      "tenant_id": "t2-...",
      "owner_id": "aaa-...",
      "created_at": "2026-04-22T09:00:00Z"
    }
  ],
  "total": 2
}
```

### Implementation notes

A resource is "unprotected" if no policy binding's scope covers it:

```sql
SELECT r.* FROM resources r
WHERE NOT EXISTS (
  -- Direct resource binding
  SELECT 1 FROM policy_bindings pb
  WHERE pb.scope_kind = 'resource' AND pb.scope_ref = r.id::text
)
AND NOT EXISTS (
  -- Resource kind binding
  SELECT 1 FROM policy_bindings pb
  WHERE pb.scope_kind = 'resource_kind' AND pb.scope_ref = r.kind
)
AND NOT EXISTS (
  -- Global binding
  SELECT 1 FROM policy_bindings pb
  WHERE pb.scope_kind = 'all'
);
```

Note: if **any** `scope_kind = all` binding exists in the system, no resource is unprotected (every resource is covered). This is expected and the endpoint will return an empty list.

---

## 3. GET /admin/expiring-credentials

Returns credentials that will expire within a specified number of days. Useful for proactive rotation before integrations break.

### Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `days` | int | 30 | Show credentials expiring within this many days |
| `entity_id` | UUID | — | Filter by entity |
| `kind` | `password` \| `api_key` \| `certificate` | — | Filter by credential kind |
| `limit` | int | 50 | Results per page (1-200) |
| `offset` | int | 0 | Pagination offset |

### Response

```json
{
  "items": [
    {
      "id": "cr1-...",
      "entity_id": "bbb-...",
      "entity_name": "sensor-01",
      "entity_kind": "device",
      "kind": "api_key",
      "status": "active",
      "expires_at": "2026-05-10T00:00:00Z",
      "days_remaining": 16,
      "created_at": "2026-01-10T00:00:00Z"
    },
    {
      "id": "cr2-...",
      "entity_id": "ccc-...",
      "entity_name": "billing-service",
      "entity_kind": "service",
      "kind": "api_key",
      "status": "active",
      "expires_at": "2026-04-28T00:00:00Z",
      "days_remaining": 4,
      "created_at": "2025-10-28T00:00:00Z"
    }
  ],
  "total": 2
}
```

### Implementation notes

```sql
SELECT c.*, e.name AS entity_name, e.kind AS entity_kind
FROM credentials c
JOIN entities e ON c.entity_id = e.id
WHERE c.status = 'active'
  AND c.expires_at IS NOT NULL
  AND c.expires_at <= now() + ($1 || ' days')::interval
ORDER BY c.expires_at ASC;
```

`days_remaining` is computed as `expires_at - now()` in the application layer.

Note: `secret_hash` and `identifier` are **never** included in the response — only metadata about the credential.

---

## Authorization

All three endpoints require `RequireManage`. These are administrative endpoints — regular entities should not be able to enumerate system-wide orphans, unprotected resources, or credentials.

---

## Use cases

### 1. Regular cleanup

Run `GET /admin/orphan-policies` weekly. Delete any orphaned bindings to keep the policy set clean.

### 2. Security audit

Run `GET /admin/unprotected-resources` to verify that every resource in a tenant is covered by at least one policy.

### 3. Credential rotation alerts

Run `GET /admin/expiring-credentials?days=7` daily. Alert on any credentials expiring within the next week so teams can rotate before services break.
