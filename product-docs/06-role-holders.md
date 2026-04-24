# GET /roles/:id/holders

## Priority: 2 (Should-have)

---

## Problem

Before modifying or deleting a role, an administrator needs to know: **"Who holds this role, and how will they be affected?"**

Today there is no way to answer this without scanning all policy bindings with `grant_kind = role`, filtering by `grant_id`, and then resolving the subject IDs to entities and groups.

---

## Endpoint

```
GET /roles/:id/holders
```

**Authentication:** Bearer token required.

---

## Path parameters

| Parameter | Type | Description |
|---|---|---|
| `id` | UUID | Role ID |

---

## Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `tenant_id` | UUID | — | Filter subjects by tenant |
| `subject_kind` | `entity` \| `group` | — | Filter by subject kind |
| `limit` | int | 20 | Results per page (1-100) |
| `offset` | int | 0 | Pagination offset |

---

## Response

```json
{
  "role": {
    "id": "v1-...",
    "name": "viewer",
    "tenant_id": "t1-...",
    "description": "Read-only access",
    "capabilities": [
      { "id": "c1-...", "name": "read" },
      { "id": "c2-...", "name": "subscribe" }
    ]
  },
  "items": [
    {
      "subject_kind": "entity",
      "entity": {
        "id": "aaa-...",
        "name": "alice",
        "kind": "human",
        "tenant_id": "t1-..."
      },
      "group": null,
      "policy_id": "p1-...",
      "effect": "allow",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "conditions": {}
    },
    {
      "subject_kind": "entity",
      "entity": {
        "id": "bbb-...",
        "name": "sensor-01",
        "kind": "device",
        "tenant_id": "t1-..."
      },
      "group": null,
      "policy_id": "p2-...",
      "effect": "allow",
      "scope_kind": "all",
      "scope_ref": null,
      "conditions": {}
    },
    {
      "subject_kind": "group",
      "entity": null,
      "group": {
        "id": "g1-...",
        "name": "ops-team",
        "tenant_id": "t1-...",
        "member_count": 12
      },
      "policy_id": "p3-...",
      "effect": "allow",
      "scope_kind": "resource_kind",
      "scope_ref": "device",
      "conditions": { "resource.attributes.env": "prod" }
    }
  ],
  "total": 3
}
```

---

## Response fields

### `role`

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Role ID |
| `name` | string | Role name |
| `tenant_id` | UUID \| null | Tenant the role belongs to |
| `description` | string \| null | Role description |
| `capabilities` | array | Capabilities in this role (id, name) |

### Each item in `items`

| Field | Type | Description |
|---|---|---|
| `subject_kind` | `entity` \| `group` | Who holds the role |
| `entity` | object \| null | Entity details (if subject_kind = entity) |
| `group` | object \| null | Group details with member_count (if subject_kind = group) |
| `policy_id` | UUID | The policy binding that assigns this role |
| `effect` | `allow` \| `deny` | Binding effect |
| `scope_kind` | `all` \| `resource_kind` \| `resource` | How broad the scope is |
| `scope_ref` | string \| null | Scope reference |
| `conditions` | object | ABAC conditions |

---

## Use cases

### 1. "Who holds the editor role?"

```
GET /roles/v2/holders
```

### 2. "Before deleting this role, who will lose access?"

```
GET /roles/v1/holders
```

The response shows every entity and group that holds this role. The administrator can review and reassign before deleting.

### 3. "Which groups hold this role?"

```
GET /roles/v1/holders?subject_kind=group
```

---

## Implementation notes

- Query: `SELECT * FROM policy_bindings WHERE grant_kind = 'role' AND grant_id = $1`, then join entities/groups for details.
- For group subjects, `member_count` is computed via `SELECT COUNT(*) FROM group_members WHERE group_id = $1`.
- The `capabilities` array on the role object uses the existing `role_capabilities` join.
- If the role is not found, return `404`.
