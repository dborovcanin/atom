# POST /authz/check/bulk

## Priority: 2 (Should-have)

---

## Problem

UIs and API gateways commonly need to check multiple actions for the same entity and resource in a single request. For example, rendering a resource detail page requires knowing: can the user read, write, delete, and share this resource?

Today this requires N separate `POST /authz/check` calls — one per action. Each call independently loads the entity, resource, and policy bindings, duplicating work.

---

## Endpoint

```
POST /authz/check/bulk
```

**Authentication:** Bearer token required.

---

## Request

```json
{
  "subject_id":  "uuid",
  "resource_id": "uuid",
  "actions":     ["read", "write", "delete", "publish", "manage"],
  "context":     {}
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `subject_id` | UUID | Yes | The entity attempting the actions |
| `resource_id` | UUID | Yes | The target resource |
| `actions` | string[] | Yes | List of action names to check (max 20) |
| `context` | object | No | Additional ABAC context (default `{}`) |

---

## Response

```json
{
  "subject_id": "aaa-...",
  "resource_id": "r1-...",
  "results": {
    "read":    { "allowed": true,  "reason": "allowed" },
    "write":   { "allowed": true,  "reason": "allowed" },
    "delete":  { "allowed": false, "reason": "no matching allow policy" },
    "publish": { "allowed": false, "reason": "explicitly denied by policy p1-..." },
    "manage":  { "allowed": false, "reason": "no matching allow policy" }
  }
}
```

---

## Response fields

| Field | Type | Description |
|---|---|---|
| `subject_id` | UUID | Echo of the request |
| `resource_id` | UUID | Echo of the request |
| `results` | object | Map of action name → `{allowed, reason}` (same shape as `/authz/check` response) |

---

## Validation

- `actions` must contain at least 1 and at most 20 entries.
- Duplicate action names are deduplicated — each action appears once in the response.
- If an action name doesn't match any capability, its result is `{ "allowed": false, "reason": "unknown action '<name>'" }`.

---

## Use cases

### 1. UI permission buttons

A dashboard renders a resource detail page and needs to know which action buttons to show:

```json
{
  "subject_id": "current-user-id",
  "resource_id": "resource-being-viewed",
  "actions": ["read", "write", "delete", "share"]
}
```

### 2. API gateway pre-check

An API gateway receives a request and checks all relevant permissions in one call before routing:

```json
{
  "subject_id": "calling-service-id",
  "resource_id": "target-resource",
  "actions": ["read", "execute"]
}
```

---

## Implementation notes

- Load entity, resource, and policy bindings **once**.
- Resolve each action to its capability ID.
- Evaluate each action against the same set of bindings.
- This turns N database round-trips into 1 (or 2 if role capabilities need loading).
- Each action in `results` writes its own audit log entry (same as individual check).
- The order of keys in `results` matches the order of `actions` in the request.
