# Integrations: Authorized Search and Listing

## Context

This document captures two production-grade integration paths for using Atom as an authorization layer for third-party search/listing services.

- **Path 2**: Atom-backed authorized listing API (Atom/DB is source of truth for listing authorization).
- **Path 3**: Search index with ACL projection (search engine performs most filtering with projected auth data).

Option 1 (per-item check fanout) is intentionally excluded.

## Path 2: Atom-Backed Authorized Listing API

### Summary

Build a first-class listing endpoint in Atom, e.g. `POST /authz/resources/search`, that:

1. Authenticates caller.
2. Resolves subject from bearer token (server-side).
3. Applies policy semantics in Atom/Postgres.
4. Returns only resources the subject is allowed to see for a given action.

### Strengths

- Strong consistency: policy/group/resource changes are reflected immediately.
- Full semantic fidelity with Atom PDP behavior (deny-overrides-allow, default deny, ABAC conditions).
- Easier auditability and explainability (`reason`, policy traceability).
- Lower operational complexity than maintaining a second policy system in search infra.

### Weaknesses

- Query path is DB-bound and can become expensive for very large search workloads.
- Requires careful SQL design and indexing to keep p95/p99 low.
- Offset-based pagination does not scale well for deep pagination.

### Production Notes

- Prefer keyset/cursor pagination over offset.
- Keep request contract explicit:
  - Required: `action`, filters, page cursor/limit.
  - Optional: `context` for ABAC.
- Enforce subject binding from token, not caller-supplied subject IDs.
- Add explicit metrics: latency, rows scanned, denied/allowed ratio, tenant hot spots.

## Path 3: Search Index with ACL Projection

### Summary

Mirror resources and authorization-relevant data into a search index (Elasticsearch/OpenSearch/etc.) and execute most listing filters there, including ACL filters.

### Strengths

- Best scalability for high-QPS listing/search.
- Better support for relevance ranking, faceting, and large candidate sets.
- Horizontal read scaling is usually simpler than scaling relational authorization joins alone.

### Weaknesses

- Higher operational complexity (CDC/outbox, projector, replay, reconciliation, lag monitoring).
- Eventual consistency risk unless strong synchronization barriers are added.
- Easy to drift from true PDP semantics, especially for:
  - deny precedence
  - group/role churn
  - dynamic ABAC inputs (`context.*`)
- Harder to provide correct "why allowed/denied" explanation unless explicitly engineered.

### Production Notes

- Treat Atom as source of truth; index is a derived projection.
- Build idempotent projector with replay support and tombstone handling.
- Instrument end-to-end lag from Atom write timestamp to searchable state.
- Keep a fallback path to Path 2 for consistency-sensitive flows.

## Findings

1. **Path 2 is the correct first production target** for correctness and policy fidelity.
2. **Path 3 is the scale optimization path**, not the initial trust anchor.
3. The biggest migration risk is semantic drift from Atom PDP behavior.
4. ABAC clauses should be classified:
   - Index-safe/static clauses can be projected.
   - Dynamic/request-context clauses should stay in Atom evaluation or force fallback.
5. Even after Path 3 rollout, Path 2 should remain available as a correctness fallback.

## Migration Path: 2 -> 3

### Phase 0: Stabilize Path 2 contract

- Ship `POST /authz/resources/search` with strict subject-from-token semantics.
- Add deterministic response shape and cursor pagination.
- Add integration tests that compare results against per-resource PDP truth.

### Phase 1: Add change capture

- Implement CDC or outbox for changes affecting listing authorization:
  - resources
  - policy_bindings
  - groups/group_members
  - roles/role_capabilities
  - entity attributes/status relevant to ABAC

### Phase 2: Build projector and index schema

- Define index documents for resource metadata + projected ACL fields.
- Ensure idempotent upsert/delete and replay-from-offset.
- Store version/watermark metadata for drift detection.

### Phase 3: Shadow reads

- Execute Path 3 in shadow mode for sampled requests.
- Compare Path 3 results against Path 2.
- Log divergence with reproducible diagnostics.

### Phase 4: Progressive cutover

- Enable Path 3 by tenant/workload segment.
- Keep auto-fallback to Path 2 when:
  - lag threshold exceeded
  - divergence threshold exceeded
  - unsupported ABAC context is present

### Phase 5: Hybrid steady state

- Path 3 serves high-scale search/listing traffic.
- Path 2 remains canonical fallback and safety path.
- Run periodic reconciliation jobs and alert on drift.

## Decision Guidance

- Choose **Path 2 only** if workload is moderate and strict consistency is required.
- Choose **Path 2 then Path 3** for large-scale search where low latency and advanced search features matter.
- Avoid direct Path 3-first rollout unless you are ready to absorb the complexity of correctness validation and continuous reconciliation.
