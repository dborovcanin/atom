# GET /resources/:id/access

## Priority: 1 (Must-have)

---

## Problem

The inverse of entity access: **"Who can access this resource?"**

This is the fundamental security audit question. When reviewing a production resource, an operator needs to see every entity and group that has any level of access to it — through direct bindings, role-based grants, or group memberships.

Today, answering this requires scanning all policy bindings, filtering by scope, expanding roles, and resolving group members — all manually.

---

## Endpoint

```
GET /resources/:id/access
```

**Authentication:** Bearer token required.

---

## Path parameters

| Parameter | Type | Description |
|---|---|---|
| `id` | UUID | Resource ID |

---

## Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `action` | string | — | Filter by specific action/capability name (e.g. `read`, `write`) |
| `entity_kind` | string | — | Filter by entity kind (e.g. `human`, `device`, `service`) |
| `effect` | `allow` \| `deny` | — | Filter by effect |
| `limit` | int | 20 | Results per page (1-100) |
| `offset` | int | 0 | Pagination offset |

---

## Response

```json
{
  "resource_id": "r1-...",
  "resource": {
    "kind": "channel",
    "name": "temperature-feed",
    "tenant_id": "t1-..."
  },
  "items": [
    {
      "entity": {
        "id": "aaa-...",
        "name": "alice",
        "kind": "human",
        "tenant_id": "t1-..."
      },
      "effect": "allow",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "policy_id": "p1-...",
      "grant": {
        "kind": "role",
        "role": { "id": "v1-...", "name": "viewer" },
        "capabilities": [
          { "id": "c1-...", "name": "read" },
          { "id": "c2-...", "name": "subscribe" }
        ]
      },
      "conditions": {},
      "via": "direct"
    },
    {
      "entity": {
        "id": "bbb-...",
        "name": "sensor-01",
        "kind": "device",
        "tenant_id": "t1-..."
      },
      "effect": "allow",
      "scope_kind": "all",
      "scope_ref": null,
      "policy_id": "p2-...",
      "grant": {
        "kind": "capability",
        "role": null,
        "capabilities": [
          { "id": "c3-...", "name": "publish" }
        ]
      },
      "conditions": {},
      "via": "group:field-devices"
    }
  ],
  "total": 2
}
```

---

## Response fields

### Top level

| Field | Type | Description |
|---|---|---|
| `resource_id` | UUID | The resource being queried |
| `resource` | object | Resource details (kind, name, tenant_id) |
| `items` | array | Access entries — one per entity-policy combination |
| `total` | int | Total count (before pagination) |

### Each item in `items`

| Field | Type | Description |
|---|---|---|
| `entity` | object | The entity that has access (id, name, kind, tenant_id) |
| `effect` | `allow` \| `deny` | Whether this is an allow or deny |
| `scope_kind` | `all` \| `resource_kind` \| `resource` | How broad the scope is |
| `scope_ref` | string \| null | Scope reference value |
| `policy_id` | UUID | The policy binding ID |
| `grant.kind` | `capability` \| `role` | Grant type |
| `grant.role` | object \| null | Role info if applicable |
| `grant.capabilities` | array | Capabilities granted |
| `conditions` | object | ABAC conditions |
| `via` | string | `"direct"` or `"group:<group-name>"` |

---

## How policies are matched to this resource

A policy binding covers this resource if:

1. `scope_kind = all` — covers every resource.
2. `scope_kind = resource_kind` AND `scope_ref` equals this resource's `kind`.
3. `scope_kind = resource` AND `scope_ref` equals this resource's `id`.

For bindings with `subject_kind = group`, the query expands group membership to show individual entities. Each entity in the group appears as a separate item with `via: "group:<name>"`.

---

## Use cases

### 1. "Who can access this resource?"

```
GET /resources/r1/access
```

### 2. "Which humans can write to this resource?"

```
GET /resources/r1/access?action=write&entity_kind=human
```

### 3. "What deny policies apply to this resource?"

```
GET /resources/r1/access?effect=deny
```

### 4. "Which devices can publish to this channel?"

```
GET /resources/r1/access?action=publish&entity_kind=device
```

---

## Implementation notes

- The query finds all policy bindings where the scope covers this resource (by `all`, `resource_kind`, or direct `resource` reference).
- For `subject_kind = group` bindings, join through `group_members` to expand to individual entities.
- For `grant_kind = role` bindings, join through `role_capabilities` to expand capabilities.
- The `action` filter resolves the action to a capability ID, then filters bindings whose grant covers that capability.
- If the resource is not found, return `404`.
- If the resource exists but no policies cover it, return `200` with `items: []` and `total: 0`.
- Pagination applies to the final expanded entity list.
