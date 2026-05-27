# Atom Access Model

## Status: Authoritative Draft
## Date: 2026-05-26

This document is the product source of truth for Atom authorization.

## One-line Model

```text
Subject gets a Role.
Role contains Permission Blocks.
Direct Policy gives a Subject one Permission Block directly.
Permission Block is the only place where scope + actions are defined.
```

## Vocabulary

| Term | Meaning |
|---|---|
| Action | Atomic operation, formerly called Capability. |
| Action Applicability | Which object kinds/types support an action. |
| Permission Block | Scope + actions + effect + conditions. Single source of permission logic. |
| Role | Named collection of Permission Blocks. |
| Role Assignment | Gives a Role to an Entity or Principal Group. Has no scope. |
| Direct Policy | Gives one Permission Block directly to an Entity or Principal Group. Has no duplicated scope/actions. |
| Principal Group | Who-container: users, services, devices, applications, workloads. |
| Object Group | Where-container: clients, channels, resources, child Object Groups. |

## Core Rules

- Scope lives only in Permission Blocks.
- Actions live only in Permission Blocks through `permission_block_actions`.
- Roles do not have scope or direct action rows.
- Role Assignments do not have scope or direct action rows.
- Direct Policies do not duplicate scope/actions; they reference a Permission Block.
- Permission Block is the single source of truth for runtime authorization, role-based access, direct grants, deny rules, and conditions.

## Actions

Action is a global operation name.

Examples:

```text
read
write
delete
publish
subscribe
execute
manage
role.manage
policy.manage
credential.manage
audit.read
tenant.manage
tenant.create
signing_key.rotate
authz.check
```

Do not create object-specific action names such as:

```text
client_read
channel_publish
report_execute
```

Action Applicability validates where an action is valid. It does not grant access.

Examples:

```text
publish -> resource:channel
subscribe -> resource:channel
execute -> resource:rule, resource:report
read/write/delete -> supported protected objects
```

## Permission Blocks

Permission Block is the atomic permission unit.

```text
permission_block =
  tenant boundary +
  scope mode +
  optional object kind/type/id +
  optional object group boundary +
  effect +
  conditions +
  actions
```

Permission Block fields:

```text
tenant_id
scope_mode
object_kind
object_type
object_id
group_id
effect
conditions
```

Actions are linked through:

```text
permission_block_actions(permission_block_id, action_id)
```

## Scope Modes

Scope modes live only in Permission Blocks.

| Scope mode | Meaning |
|---|---|
| `platform` | Global/platform. |
| `tenant` | Tenant/domain object itself. |
| `object_kind` | All objects of one kind in a tenant. |
| `object_type` | All objects of one type in a tenant. |
| `object` | One exact entity/resource/object. |
| `group` | Object Group itself. |
| `group_direct_objects` | Entities/resources directly inside an Object Group. |
| `group_descendant_objects` | Entities/resources inside child/deeper Object Groups. |
| `group_child_groups` | Immediate child Object Groups themselves. |
| `group_descendant_groups` | Child/deeper Object Groups themselves. |

Examples:

```text
tenant_id = d1
scope_mode = tenant
=> tenant/domain d1 itself
```

```text
tenant_id = d1
scope_mode = object_type
object_kind = resource
object_type = resource:channel
=> all channels in tenant d1
```

```text
tenant_id = d1
scope_mode = group_direct_objects
object_kind = resource
object_type = resource:channel
group_id = g1
=> channels directly inside Object Group g1
```

## Roles

Role is a business-facing name for a set of Permission Blocks.

```text
Role: Plant-A Operator

Permission Block 1:
  Scope: clients in Object Group Plant-A
  Actions: read, write

Permission Block 2:
  Scope: channels in Object Group Plant-A
  Actions: read, publish, subscribe
```

Role links Permission Blocks:

```text
role_permission_blocks(role_id, permission_block_id)
```

Roles have no scope columns and no direct action columns.

## Role Assignments

Role Assignment grants a Role to a subject.

Subject can be:

- Entity
- Principal Group

Role Assignment fields:

```text
tenant_id
subject_kind
subject_id
role_id
```

Role Assignment has no scope and no action rows.

## Direct Policies

Direct Policy grants one Permission Block directly to a subject.

It exists for advanced/internal cases:

- client-channel publish/subscribe links
- service grants
- explicit deny rules
- temporary or conditional grants
- break-glass access

Direct Policy fields:

```text
tenant_id
subject_kind
subject_id
permission_block_id
```

Direct Policy does not duplicate scope, actions, effect, or conditions. Those come from the referenced Permission Block.

## Principal Groups

Principal Group is a who-container.

It can contain:

- humans
- services
- applications
- workloads
- devices if needed

Principal Group is a subject for Role Assignments and Direct Policies.

Principal Group is not an Object Group and is not used as a scope boundary.

## Object Groups

Object Group is a where-container.

It can contain:

- entities such as clients/devices
- resources such as channels, rules, reports, alarms
- child Object Groups

Object Group containment alone grants no access. It only affects whether a Permission Block scope matches a protected object.

One object belongs to one Object Group in V1.

## Effective Authorization

Atom evaluates both access paths into one effective permission shape:

```text
Role path:
  subject -> role_assignment -> role -> permission_block -> actions

Direct path:
  subject -> direct_policy -> permission_block -> actions
```

The PDP must treat these paths as one logical source:

```text
effective_permissions =
  role assignment path
  UNION ALL
  direct policy path
```

Decision:

1. Resolve the requested action and protected object.
2. Find matching effective permissions.
3. If any matching deny exists, deny.
4. If any matching allow exists, allow.
5. Otherwise deny.

## Listing Authorization Semantics

All normal list queries are read-filtered.

A caller does not need a separate `list` action to list normal objects. A record appears in a list response only if the caller has `read` access to that specific record.

Examples:

- `channels` query returns channels the caller can `read`.
- `entities` query returns entities the caller can `read`.
- `objectGroups` query returns Object Groups the caller can `read`.

Listing must not:

- fetch all tenant rows and call PDP once per row.
- compute total count before authorization filtering.
- leak unreadable object IDs, names, counts, or ordering position.

Listing must:

- apply authorization inside SQL or equivalent DB-side query logic.
- include direct policies.
- include reusable permissions through role assignments.
- include entity assignments.
- include Principal Group assignments.
- include nested Principal Group membership if hierarchy is enabled.
- include Object Group scopes.
- support deny-overrides-allow.
- apply search/sort/pagination after authorization filtering.

Separate actions such as `policy.manage`, `role.manage`, or `assignment.manage` are only for managing access-control objects themselves, not for listing ordinary domain objects.

## Validation Rules

- Platform Permission Blocks require `tenant_id = NULL`.
- Non-platform Permission Blocks require `tenant_id`.
- Role and Permission Block tenants must match, except platform roles/blocks use `NULL`.
- Role Assignment tenant must match Role tenant.
- Direct Policy tenant must match Permission Block tenant.
- Subject Entity tenant must match assignment/direct-policy tenant, except global entities may receive tenant access through active tenant membership.
- Subject Principal Group must belong to the same tenant.
- Object Group scope must reference an Object Group in the same tenant.
- Object target must belong to the Permission Block tenant.
- Every action in a Permission Block must be valid for that Permission Block scope using Action Applicability.

## Product UI Language

Normal UI should expose:

```text
Actions
Permission Blocks
Roles
Role Assignments
Principal Groups
Object Groups
```

Advanced/security UI may expose:

```text
Direct Policies
Deny Permission Blocks
Conditional Permission Blocks
```

Avoid normal user-facing terms:

```text
Capability
PolicyBinding
scope_kind
scope_ref
overloaded Group
```

