# GET /entities/:id/effective-capabilities

## Priority: 2 (Should-have)

---

## Problem

An entity's actual capabilities come from multiple sources — direct capability bindings, roles (which bundle capabilities), and group memberships (which inherit bindings). There is no single view that shows the **flattened, deduplicated list** of what an entity can actually do.

This is different from the entity access endpoint (#2), which shows access per-resource. This endpoint answers: "What actions can this entity perform, regardless of which resource?" — a capability-centric view rather than a resource-centric view.

---

## Endpoint

```
GET /entities/:id/effective-capabilities
```

**Authentication:** Bearer token required.

---

## Path parameters

| Parameter | Type | Description |
|---|---|---|
| `id` | UUID | Entity ID |

---

## Query parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `tenant_id` | UUID | — | Filter by tenant (only consider bindings whose roles/subjects belong to this tenant) |
| `resource_kind` | string | — | Filter capabilities by resource_kind |

---

## Response

```json
{
  "entity_id": "aaa-...",
  "entity_name": "alice",
  "entity_kind": "human",
  "capabilities": [
    {
      "id": "c1-...",
      "name": "read",
      "resource_kind": null,
      "sources": [
        {
          "kind": "role",
          "role_id": "v1-...",
          "role_name": "viewer",
          "policy_id": "p1-...",
          "scope_kind": "resource_kind",
          "scope_ref": "channel",
          "effect": "allow",
          "via": "direct"
        },
        {
          "kind": "role",
          "role_id": "v2-...",
          "role_name": "editor",
          "policy_id": "p2-...",
          "scope_kind": "all",
          "scope_ref": null,
          "effect": "allow",
          "via": "group:ops-team"
        }
      ]
    },
    {
      "id": "c2-...",
      "name": "write",
      "resource_kind": null,
      "sources": [
        {
          "kind": "role",
          "role_id": "v2-...",
          "role_name": "editor",
          "policy_id": "p2-...",
          "scope_kind": "all",
          "scope_ref": null,
          "effect": "allow",
          "via": "group:ops-team"
        }
      ]
    },
    {
      "id": "c3-...",
      "name": "execute",
      "resource_kind": "device",
      "sources": [
        {
          "kind": "capability",
          "role_id": null,
          "role_name": null,
          "policy_id": "p3-...",
          "scope_kind": "resource",
          "scope_ref": "r5-...",
          "effect": "allow",
          "via": "direct"
        }
      ]
    },
    {
      "id": "c4-...",
      "name": "publish",
      "resource_kind": null,
      "sources": [
        {
          "kind": "role",
          "role_id": "v3-...",
          "role_name": "restricted",
          "policy_id": "p4-...",
          "scope_kind": "resource_kind",
          "scope_ref": "channel",
          "effect": "deny",
          "via": "direct"
        }
      ]
    }
  ]
}
```

---

## Response fields

### Top level

| Field | Type | Description |
|---|---|---|
| `entity_id` | UUID | The entity being queried |
| `entity_name` | string | Entity name |
| `entity_kind` | string | Entity kind |
| `capabilities` | array | Deduplicated list of capabilities with their sources |

### Each capability

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Capability ID |
| `name` | string | Capability name (e.g. `read`, `publish`) |
| `resource_kind` | string \| null | Which resource kind this capability applies to (null = all kinds) |
| `sources` | array | Every path through which this entity holds this capability |

### Each source

| Field | Type | Description |
|---|---|---|
| `kind` | `capability` \| `role` | Whether the binding grants this capability directly or via a role |
| `role_id` | UUID \| null | Role ID if kind = role |
| `role_name` | string \| null | Role name if kind = role |
| `policy_id` | UUID | The policy binding |
| `scope_kind` | `all` \| `resource_kind` \| `resource` | Scope of the binding |
| `scope_ref` | string \| null | Scope reference |
| `effect` | `allow` \| `deny` | Effect — deny sources are included so admins can see blocks |
| `via` | string | `"direct"` or `"group:<group-name>"` |

---

## Why `sources` matters

The same capability can reach an entity through multiple independent paths:

```
read ← viewer role ← direct binding (scope: channels)
read ← editor role ← group:ops-team binding (scope: all)
```

If an admin revokes the direct viewer binding, the entity still has `read` through the ops-team group. The `sources` array makes this visible — an admin knows exactly what to revoke to fully remove a capability.

---

## Use cases

### 1. "What can Alice do?"

```
GET /entities/aaa/effective-capabilities
```

### 2. "What can this device do with channel resources?"

```
GET /entities/bbb/effective-capabilities?resource_kind=channel
```

### 3. "What capabilities does this service have in tenant 1?"

```
GET /entities/ccc/effective-capabilities?tenant_id=t1
```

### 4. "Before revoking a role, will the entity still have access?"

Check if the capability appears with multiple sources. If yes, revoking one source won't fully remove the capability.

---

## Implementation notes

- Load all bindings for the entity (direct + group), same query as the PDP engine.
- For role bindings, expand via `role_capabilities` to get individual capability IDs.
- For direct capability bindings, the capability is the `grant_id` itself.
- Group capability IDs by capability, deduplicate, and attach sources.
- Deny-effect sources are included in the response — they don't grant access but show that a deny exists for that capability.
- The response is not paginated — it's a flat list of distinct capabilities, which is typically small (< 50 even in complex setups).
- If the entity is not found, return `404`.
