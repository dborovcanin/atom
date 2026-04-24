# gRPC API Reference

Atom exposes two gRPC services on port **8081** (configurable via `GRPC_ADDR`).

The proto source lives at [`proto/atom.proto`](../proto/atom.proto). All types below are derived directly from it.

---

## Connection

```
host: localhost:8081
protocol: gRPC (HTTP/2)
TLS: none (terminate at service mesh / load balancer in production)
```

### grpcurl

```bash
# List services
grpcurl -plaintext localhost:8081 list

# Describe a service
grpcurl -plaintext localhost:8081 describe atom.v1.AuthzService
```

---

## Services

### `atom.v1.AuthzService`

Authorization decisions. Call `Check` on every protected operation in a downstream service.

#### `Check`

```
rpc Check(CheckRequest) returns (CheckResponse)
```

Evaluates whether a subject may perform an action on a resource. Runs the same PDP algorithm as `POST /authz/check` — same DB queries, same deny-override-allow logic, same ABAC evaluation.

**Request: `CheckRequest`**

| Field | Type | Required | Description |
|---|---|---|---|
| `subject_id` | `string` (UUID) | yes | Entity performing the action |
| `action` | `string` | yes | Capability name, e.g. `"publish"` |
| `resource_id` | `string` (UUID) | yes | Resource being acted on |
| `context` | `map<string, string>` | no | ABAC context injected under the `context` key during evaluation |

> **Context limitation:** The gRPC interface only supports flat `string → string` context values. For nested JSON context (uncommon), use `POST /authz/check` over REST instead.

**Response: `CheckResponse`**

| Field | Type | Description |
|---|---|---|
| `allowed` | `bool` | Authorization decision |
| `reason` | `string` | Human-readable explanation |

Possible `reason` values:

| Value | Meaning |
|---|---|
| `"allowed"` | At least one allow binding matched; no deny matched |
| `"no matching allow policy"` | No allow binding covered this request |
| `"subject not found"` | No entity with `subject_id` exists |
| `"subject is not active"` | Entity status is `inactive` or `suspended` |
| `"resource not found"` | No resource with `resource_id` exists |
| `"unknown action '<name>'"` | No capability with that name exists |
| `"explicitly denied by policy <id>"` | A deny binding matched |

**gRPC status codes**

| Code | Condition |
|---|---|
| `OK` | Decision returned (check `allowed` field — never an error status) |
| `INVALID_ARGUMENT` | `subject_id` or `resource_id` is not a valid UUID |
| `UNAUTHENTICATED` | Should not occur on this RPC (no auth required for the check itself) |
| `INTERNAL` | Database error |

**Example — grpcurl**

```bash
grpcurl -plaintext \
  -d '{
    "subject_id":  "550e8400-e29b-41d4-a716-446655440000",
    "action":      "publish",
    "resource_id": "7c4b7f1e-4b9e-4b7f-8b4b-7f1e4b9e4b7f"
  }' \
  localhost:8081 atom.v1.AuthzService/Check
```

```json
{
  "allowed": true,
  "reason": "allowed"
}
```

**Example — with ABAC context**

```bash
grpcurl -plaintext \
  -d '{
    "subject_id":  "550e8400-e29b-41d4-a716-446655440000",
    "action":      "read",
    "resource_id": "7c4b7f1e-4b9e-4b7f-8b4b-7f1e4b9e4b7f",
    "context": {
      "ip_trusted": "true"
    }
  }' \
  localhost:8081 atom.v1.AuthzService/Check
```

---

### `atom.v1.AuthService`

Token authentication. Use `Authenticate` to validate incoming Bearer tokens in downstream services without decoding JWTs locally.

#### `Authenticate`

```
rpc Authenticate(AuthenticateRequest) returns (AuthenticateResponse)
```

Validates a JWT or API key and returns the caller's identity. Performs the same verification as the HTTP Bearer extractor:
- For JWTs: verifies ES256 signature, checks `kid` against the live key store, validates expiry, confirms the session is not revoked.
- For API keys: looks up credential by embedded ID, verifies argon2 hash, checks status and expiry.

**Request: `AuthenticateRequest`**

| Field | Type | Required | Description |
|---|---|---|---|
| `token` | `string` | yes | JWT (`eyJ...`) or API key (`atom_...`) — same value as the HTTP `Authorization: Bearer <token>` |

**Response: `AuthenticateResponse`**

| Field | Type | Description |
|---|---|---|
| `entity_id` | `string` (UUID) | The authenticated entity |
| `tenant_id` | `string` (UUID) | The entity's tenant; empty string if none |
| `session_id` | `string` (UUID) | The session backing this JWT; empty string for API key authentication |

**gRPC status codes**

| Code | Condition |
|---|---|
| `OK` | Token valid |
| `UNAUTHENTICATED` | Token missing, malformed, expired, signed with unknown key, or session revoked |
| `INTERNAL` | Database error |

**Example — JWT**

```bash
grpcurl -plaintext \
  -d '{"token": "eyJhbGciOiJFUzI1NiIsImtpZCI6Ii4uLiIsInR5cCI6IkpXVCJ9..."}' \
  localhost:8081 atom.v1.AuthService/Authenticate
```

```json
{
  "entity_id":  "550e8400-e29b-41d4-a716-446655440000",
  "tenant_id":  "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "session_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6"
}
```

**Example — API key**

```bash
grpcurl -plaintext \
  -d '{"token": "atom_0a1b2c3d4e5f6789abcdef01_abcdef..."}' \
  localhost:8081 atom.v1.AuthService/Authenticate
```

```json
{
  "entity_id":  "550e8400-e29b-41d4-a716-446655440000",
  "tenant_id":  "",
  "session_id": ""
}
```

---

## Client examples

### Go

```go
import (
    "google.golang.org/grpc"
    "google.golang.org/grpc/credentials/insecure"
    atomv1 "github.com/absmach/atom/gen/go/atom/v1"
)

conn, err := grpc.NewClient("localhost:8081",
    grpc.WithTransportCredentials(insecure.NewCredentials()),
)
if err != nil { ... }
defer conn.Close()

authz := atomv1.NewAuthzServiceClient(conn)
resp, err := authz.Check(ctx, &atomv1.CheckRequest{
    SubjectId:  deviceID,
    Action:     "publish",
    ResourceId: channelID,
})
if err != nil { ... }
if !resp.Allowed {
    return fmt.Errorf("denied: %s", resp.Reason)
}
```

### Python

```python
import grpc
from atom.v1 import atom_pb2, atom_pb2_grpc

channel = grpc.insecure_channel("localhost:8081")
stub = atom_pb2_grpc.AuthzServiceStub(channel)

response = stub.Check(atom_pb2.CheckRequest(
    subject_id  = device_id,
    action      = "publish",
    resource_id = channel_id,
))
if not response.allowed:
    raise PermissionError(response.reason)
```

### Rust (tonic)

```rust
use atom_v1::authz_service_client::AuthzServiceClient;
use atom_v1::CheckRequest;

let mut client = AuthzServiceClient::connect("http://localhost:8081").await?;

let resp = client.check(CheckRequest {
    subject_id:  device_id.to_string(),
    action:      "publish".to_string(),
    resource_id: channel_id.to_string(),
    context:     Default::default(),
}).await?.into_inner();

if !resp.allowed {
    return Err(anyhow::anyhow!("denied: {}", resp.reason));
}
```

---

## Code generation

Generate client stubs from the proto using `buf`:

```bash
# Install buf
brew install bufbuild/buf/buf   # macOS
# or: https://buf.build/docs/installation

# Generate Go client
buf generate --template buf.gen.go.yaml

# Generate Python client
buf generate --template buf.gen.python.yaml

# Lint the proto
buf lint

# Check for breaking changes against the registry
buf breaking --against '.git#branch=main'
```

See [`buf.yaml`](../buf.yaml) at the project root for the module configuration.

---

## Comparison: gRPC vs REST

| | gRPC | REST |
|---|---|---|
| Hot path (`authz/check`, `authenticate`) | ✅ Preferred — persistent connections, binary encoding | ✓ Works |
| Management operations (CRUD) | ✗ Not exposed | ✅ Preferred |
| External / browser clients | Limited | ✅ Preferred |
| ABAC context | Flat `string → string` only | Full JSON object |
| Code generation | Required (protobuf) | Optional (OpenAPI) |
