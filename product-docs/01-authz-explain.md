# POST /authz/explain

## Priority: 1 (Must-have)

---

## Problem

The current `POST /authz/check` returns `{"allowed": true/false, "reason": "..."}`. When access is denied, operators have no way to understand **why** — which bindings were evaluated, which ones matched, which one caused the denial, and whether the access came through a direct binding or a group.

Debugging authorization issues currently requires reading the database directly.

---

## Endpoint

```
POST /authz/explain
```

**Authentication:** Bearer token required.

---

## Request

Same as `POST /authz/check`:

```json
{
  "subject_id":  "uuid",
  "action":      "read",
  "resource_id": "uuid",
  "context":     {}
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `subject_id` | UUID | Yes | The entity attempting the action |
| `action` | string | Yes | The action name (e.g. `read`, `publish`, `manage`) |
| `resource_id` | UUID | Yes | The target resource |
| `context` | object | No | Additional ABAC context (default `{}`) |

---

## Response

```json
{
  "allowed": false,
  "reason": "explicitly denied by policy 9f3a...",

  "subject": {
    "id": "aaa-...",
    "name": "sensor-01",
    "kind": "device",
    "status": "active"
  },

  "resource": {
    "id": "bbb-...",
    "kind": "channel",
    "name": "temperature-feed",
    "tenant_id": "t1-..."
  },

  "capability": {
    "id": "ccc-...",
    "name": "read",
    "resource_kind": null
  },

  "matched_binding": {
    "id": "9f3a-...",
    "effect": "deny",
    "grant_kind": "role",
    "grant_id": "fff-...",
    "role_name": "restricted-viewer",
    "scope_kind": "resource_kind",
    "scope_ref": "channel",
    "conditions": {},
    "via": "group:ops-team"
  },

  "evaluated_bindings": [
    {
      "id": "aaa1-...",
      "effect": "allow",
      "grant_kind": "role",
      "grant_id": "ggg-...",
      "role_name": "viewer",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "via": "direct",
      "result": "matched",
      "skip_reason": null
    },
    {
      "id": "9f3a-...",
      "effect": "deny",
      "grant_kind": "role",
      "grant_id": "fff-...",
      "role_name": "restricted-viewer",
      "scope_kind": "resource_kind",
      "scope_ref": "channel",
      "via": "group:ops-team",
      "result": "matched",
      "skip_reason": null
    },
    {
      "id": "bbb2-...",
      "effect": "allow",
      "grant_kind": "capability",
      "grant_id": "ddd-...",
      "role_name": null,
      "scope_kind": "resource",
      "scope_ref": "eee-...",
      "via": "direct",
      "result": "skipped",
      "skip_reason": "scope_mismatch"
    },
    {
      "id": "ccc3-...",
      "effect": "allow",
      "grant_kind": "role",
      "grant_id": "hhh-...",
      "role_name": "admin",
      "scope_kind": "all",
      "scope_ref": null,
      "via": "group:admins",
      "result": "skipped",
      "skip_reason": "conditions_mismatch"
    }
  ]
}
```

---

## Response fields

### Top level

| Field | Type | Description |
|---|---|---|
| `allowed` | bool | Final decision |
| `reason` | string | Human-readable reason |
| `subject` | object | Resolved subject entity (id, name, kind, status) |
| `resource` | object | Resolved resource (id, kind, name, tenant_id) |
| `capability` | object \| null | Resolved capability for the action, or null if action not found |
| `matched_binding` | object \| null | The binding that determined the final decision (null if no match) |
| `evaluated_bindings` | array | All bindings that were evaluated, in evaluation order |

### `matched_binding` and `evaluated_bindings[]`

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Policy binding ID |
| `effect` | `allow` \| `deny` | The binding's effect |
| `grant_kind` | `capability` \| `role` | What the binding grants |
| `grant_id` | UUID | ID of the capability or role |
| `role_name` | string \| null | Role name if `grant_kind = role` |
| `scope_kind` | `all` \| `resource_kind` \| `resource` | Scope type |
| `scope_ref` | string \| null | Scope reference |
| `conditions` | object | ABAC conditions on the binding |
| `via` | string | `"direct"` or `"group:<group-name>"` — how the entity received this binding |
| `result` | `matched` \| `skipped` | Whether this binding matched |
| `skip_reason` | string \| null | Why it was skipped: `scope_mismatch`, `grant_mismatch`, `conditions_mismatch`, or null |

---

## Early exit responses

If evaluation stops before reaching bindings:

**Subject not found:**
```json
{
  "allowed": false,
  "reason": "subject not found",
  "subject": null,
  "resource": null,
  "capability": null,
  "matched_binding": null,
  "evaluated_bindings": []
}
```

**Subject not active:**
```json
{
  "allowed": false,
  "reason": "subject is not active",
  "subject": { "id": "...", "name": "...", "kind": "device", "status": "suspended" },
  "resource": null,
  "capability": null,
  "matched_binding": null,
  "evaluated_bindings": []
}
```

**Unknown action:**
```json
{
  "allowed": false,
  "reason": "unknown action 'foo'",
  "subject": { "id": "...", "name": "...", "kind": "human", "status": "active" },
  "resource": { "id": "...", "kind": "channel", "name": "...", "tenant_id": "..." },
  "capability": null,
  "matched_binding": null,
  "evaluated_bindings": []
}
```

---

## Implementation notes

- Reuses the same evaluation logic as `engine::evaluate()`, but collects diagnostic information at each step instead of short-circuiting silently.
- The `via` field requires joining `group_members` → `groups` to resolve group names for bindings where `subject_kind = group`.
- Performance: this endpoint does more work than `/authz/check` (resolving names, collecting all binding evaluations). It should be used for debugging, not in hot paths.
- This endpoint also writes an audit log entry with event `authz.explain`.

---

## Example scenarios

### Scenario 1: Allowed via role through a group

Alice is in group `engineers`. Group `engineers` has a policy: role `editor` on `resource_kind = channel`, effect `allow`. Alice tries to `write` to a channel.

Response: `allowed: true`, `matched_binding.via = "group:engineers"`, `matched_binding.role_name = "editor"`.

### Scenario 2: Denied — deny overrides allow

Device `sensor-01` has a direct allow binding for `read` on all channels. But group `restricted-devices` (which `sensor-01` is a member of) has a deny binding for `read` on `resource_kind = channel`.

Response: `allowed: false`, `matched_binding.effect = "deny"`, `matched_binding.via = "group:restricted-devices"`. The allow binding appears in `evaluated_bindings` with `result: "matched"` but was overridden.

### Scenario 3: Denied — no matching policy

A new service entity has no policy bindings at all.

Response: `allowed: false`, `reason: "no matching allow policy"`, `matched_binding: null`, `evaluated_bindings: []`.
