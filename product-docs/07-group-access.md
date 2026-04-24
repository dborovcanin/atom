# GET /groups/:id/access

## Priority: 2 (Should-have)

---

## Problem

Groups are a core building block — adding an entity to a group instantly grants it all the group's policy bindings. But there is no way to preview **what access a group grants** before adding a member.

An administrator about to run `POST /groups/:id/members { entity_id }` has no way to answer: "What will this entity gain by joining this group?"

---

## Endpoint

```
GET /groups/:id/access
```

**Authentication:** Bearer token required.

---

## Path parameters

| Parameter | Type | Description |
|---|---|---|
| `id` | UUID | Group ID |

---

## Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `resource_kind` | string | — | Filter by resource kind |
| `action` | string | — | Filter by action/capability name |
| `effect` | `allow` \| `deny` | — | Filter by effect |
| `limit` | int | 20 | Results per page (1-100) |
| `offset` | int | 0 | Pagination offset |

---

## Response

```json
{
  "group_id": "g1-...",
  "group": {
    "name": "field-engineers",
    "tenant_id": "t1-...",
    "member_count": 8
  },
  "items": [
    {
      "resource": {
        "id": "r1-...",
        "kind": "device",
        "name": "pump-1",
        "tenant_id": "t1-..."
      },
      "effect": "allow",
      "scope_kind": "resource_kind",
      "scope_ref": "device",
      "policy_id": "p1-...",
      "grant": {
        "kind": "role",
        "role": { "id": "v1-...", "name": "operator" },
        "capabilities": [
          { "id": "c1-...", "name": "read" },
          { "id": "c2-...", "name": "write" },
          { "id": "c3-...", "name": "execute" }
        ]
      },
      "conditions": {}
    },
    {
      "resource": {
        "id": "r2-...",
        "kind": "channel",
        "name": "telemetry",
        "tenant_id": "t1-..."
      },
      "effect": "allow",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "policy_id": "p2-...",
      "grant": {
        "kind": "capability",
        "role": null,
        "capabilities": [
          { "id": "c4-...", "name": "subscribe" }
        ]
      },
      "conditions": {}
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
| `group_id` | UUID | The group being queried |
| `group` | object | Group details (name, tenant_id, member_count) |
| `items` | array | Access entries — same shape as entity access items but without `via` (always the group) |
| `total` | int | Total count (before pagination) |

### Each item in `items`

| Field | Type | Description |
|---|---|---|
| `resource` | object | Resource details (id, kind, name, tenant_id) |
| `effect` | `allow` \| `deny` | Effect |
| `scope_kind` | `all` \| `resource_kind` \| `resource` | Scope type |
| `scope_ref` | string \| null | Scope reference |
| `policy_id` | UUID | The policy binding ID |
| `grant.kind` | `capability` \| `role` | Grant type |
| `grant.role` | object \| null | Role info if applicable |
| `grant.capabilities` | array | Capabilities granted |
| `conditions` | object | ABAC conditions |

---

## Use cases

### 1. "What does joining this group grant?"

```
GET /groups/g1/access
```

An admin reviews this before adding a new entity to the group.

### 2. "What device access does this group have?"

```
GET /groups/g1/access?resource_kind=device
```

### 3. "Does this group have any deny policies?"

```
GET /groups/g1/access?effect=deny
```

---

## Implementation notes

- Query: find all `policy_bindings WHERE subject_kind = 'group' AND subject_id = $1`, then expand scopes to resources and resolve grants.
- Scope expansion works the same as entity access (see [02-entity-access.md](./02-entity-access.md)).
- `member_count` in the group object is computed via `COUNT(*) FROM group_members`.
- No `via` field needed — all access is from the group itself.
- If the group is not found, return `404`.
