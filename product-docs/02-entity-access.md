# GET /entities/:id/access

## Priority: 1 (Must-have)

---

## Problem

The most common question an administrator asks is: **"What can this entity access?"**

Today, answering this requires:
1. `GET /policies?subject_id=<id>` — returns binding rows with only UUIDs.
2. For each binding with `grant_kind = role`: `GET /roles/:id/capabilities` — resolve capabilities.
3. For each binding: manually interpret `scope_kind` + `scope_ref` to determine which resources are affected.
4. Separately check if the entity is in any groups: `GET /entities/:id/groups`, then repeat steps 1-3 for each group.

This is unusable for any operator managing more than a handful of entities.

---

## Endpoint

```
GET /entities/:id/access
```

**Authentication:** Bearer token required.

---

## Path parameters

| Parameter | Type | Description |
|---|---|---|
| `id` | UUID | Entity ID |

---

## Query parameters

All filters are optional. When omitted, the endpoint returns all access across all tenants.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `tenant_id` | UUID | — | Filter resources by tenant |
| `resource_kind` | string | — | Filter resources by kind (e.g. `channel`, `device`) |
| `action` | string | — | Filter by specific action/capability name (e.g. `read`, `publish`) |
| `effect` | `allow` \| `deny` | — | Filter by effect (show only allows or only denies) |
| `limit` | int | 20 | Results per page (1-100) |
| `offset` | int | 0 | Pagination offset |

---

## Response

```json
{
  "entity_id": "aaa-...",
  "entity_name": "alice",
  "entity_kind": "human",
  "items": [
    {
      "resource": {
        "id": "r1-...",
        "kind": "channel",
        "name": "temperature-feed",
        "tenant_id": "t1-..."
      },
      "effect": "allow",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "policy_id": "p1-...",
      "grant": {
        "kind": "role",
        "role": {
          "id": "v1-...",
          "name": "viewer"
        },
        "capabilities": [
          { "id": "c1-...", "name": "read" },
          { "id": "c2-...", "name": "subscribe" }
        ]
      },
      "conditions": {},
      "via": "direct"
    },
    {
      "resource": {
        "id": "r2-...",
        "kind": "device",
        "name": "pump-1",
        "tenant_id": "t1-..."
      },
      "effect": "allow",
      "scope_kind": "resource",
      "scope_ref": "r2-...",
      "policy_id": "p2-...",
      "grant": {
        "kind": "capability",
        "role": null,
        "capabilities": [
          { "id": "c3-...", "name": "execute" }
        ]
      },
      "conditions": { "context.ip_trusted": "true" },
      "via": "group:field-engineers"
    },
    {
      "resource": {
        "id": "r3-...",
        "kind": "channel",
        "name": "alerts",
        "tenant_id": "t1-..."
      },
      "effect": "deny",
      "scope_kind": "resource",
      "scope_ref": "r3-...",
      "policy_id": "p3-...",
      "grant": {
        "kind": "capability",
        "role": null,
        "capabilities": [
          { "id": "c4-...", "name": "write" }
        ]
      },
      "conditions": {},
      "via": "direct"
    }
  ],
  "total": 3
}
```

---

## Response fields

### Top level

| Field | Type | Description |
|---|---|---|
| `entity_id` | UUID | The entity being queried |
| `entity_name` | string | Entity name |
| `entity_kind` | string | Entity kind (human, device, service, etc.) |
| `items` | array | Access entries |
| `total` | int | Total count (before pagination) |

### Each item in `items`

| Field | Type | Description |
|---|---|---|
| `resource` | object | The resource this access applies to (id, kind, name, tenant_id) |
| `effect` | `allow` \| `deny` | Whether this is an allow or deny |
| `scope_kind` | `all` \| `resource_kind` \| `resource` | How broad the scope is |
| `scope_ref` | string \| null | Scope reference value |
| `policy_id` | UUID | The policy binding that creates this access |
| `grant.kind` | `capability` \| `role` | Whether the grant is a direct capability or a role |
| `grant.role` | object \| null | Role details if grant is a role (id, name) |
| `grant.capabilities` | array | Capabilities granted (id, name) — expanded from role if applicable |
| `conditions` | object | ABAC conditions on the binding |
| `via` | string | `"direct"` or `"group:<group-name>"` |

---

## Scope expansion

The response expands scopes to show actual resources:

- **`scope_kind = resource`** — one item per resource directly referenced by `scope_ref`.
- **`scope_kind = resource_kind`** — one item per resource matching that kind (within tenant filter if provided).
- **`scope_kind = all`** — one item per resource (within tenant filter if provided).

For `resource_kind` and `all` scopes, the result can be large. Pagination (`limit`/`offset`) applies to the expanded resource list.

---

## How `via` works

Each binding is loaded by querying:
1. Direct bindings: `subject_kind = entity AND subject_id = entity_id`
2. Group bindings: `subject_kind = group AND subject_id IN (entity's groups)`

For group bindings, the `via` field shows which group the access comes from: `"group:<group-name>"`. For direct bindings: `"direct"`.

If the same resource is accessible through multiple paths (e.g. direct + group), each path appears as a separate item.

---

## Use cases

### 1. "What can Alice access in tenant 1?"

```
GET /entities/aaa/access?tenant_id=t1
```

### 2. "What can Alice access across all tenants?"

```
GET /entities/aaa/access
```

### 3. "What channels can Alice access in tenant 1?"

```
GET /entities/aaa/access?tenant_id=t1&resource_kind=channel
```

### 4. "What channels can Alice access across all tenants?"

```
GET /entities/aaa/access?resource_kind=channel
```

### 5. "What can Alice read?"

```
GET /entities/aaa/access?action=read
```

### 6. "What is Alice denied?"

```
GET /entities/aaa/access?effect=deny
```

---

## Implementation notes

- The core query joins `policy_bindings` → `group_members` → resources → roles → role_capabilities → capabilities.
- For `scope_kind = all` and `scope_kind = resource_kind`, the query joins against the `resources` table to expand scope into actual resources. This can produce many rows — pagination is critical.
- The `action` filter works by resolving the action to a capability ID first, then filtering bindings whose grant covers that capability.
- If the entity is not found, return `404`.
- If the entity exists but has no bindings, return `200` with `items: []` and `total: 0`.
