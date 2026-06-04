# Atom Certificates

## Status: Active v1
## Date: 2026-06-02

This document defines Atom-native certificate credentials. Magistrala's certs service is a reference for capabilities, not a service boundary. Atom owns certificate state, CA custody, certificate lifecycle, and runtime certificate identity lookup.

---

## Architecture Summary

Atom is the certificate authority and certificate source of truth.

- Atom bootstraps and owns the internal root and intermediate CA.
- Atom issues generated certificates or signs CSRs for Atom entities.
- Issued leaf certificates are stored as Atom credentials, so revocation, listing, authorization, and audit stay in the normal Atom credential model.
- Generated leaf private keys are shown once and are never stored; CSR private keys never enter Atom.
- Public PKI artifacts are served by Atom through CA chain, CRL, and OCSP endpoints.
- Runtime services extract the client certificate serial during mTLS and ask Atom gRPC to resolve that serial to an active entity.
- There is no OpenBao dependency, no standalone Magistrala certs service, and no legacy Magistrala certificate storage boundary.

---

## Model

A certificate credential belongs to an Atom entity and is stored in `credentials` with:

- `kind = certificate`
- `identifier = normalized certificate serial number`
- `secret_hash = null`
- `entity_id = owner entity id`
- `expires_at = certificate not-after timestamp`
- `metadata` containing certificate PEM, subject, SANs, issuer CA id, issuer serial, DER SHA-256 fingerprint, validity, revocation time, and revocation reason.

Issued leaf private keys are never stored. When Atom generates a leaf keypair, the private key is returned once in the issuance response.

---

## Storage and Retrieval

Atom stores certificate state in Postgres.

| Table | Purpose |
|---|---|
| `credentials` | Issued leaf certificates. Certificate rows use `kind = certificate`; `identifier` is the normalized serial number; `metadata` stores the certificate PEM and certificate attributes. |
| `certificate_authorities` | Atom root and intermediate CA certificates, CA serials, CA status, CA validity, and encrypted CA private keys. |
| `certificate_crl_state` | Singleton CRL state: CRL number, cached CRL DER, `thisUpdate`, `nextUpdate`, dirty flag, and update timestamp. |
| `entities` | Owner/subject entity for issued certificates through `credentials.entity_id`. |
| `actions` and `action_applicability` | Authorization metadata for credential `read`, `manage`, `rotate`, and `revoke`. |

An issued certificate PEM is retrievable after issuance through GraphQL:

- `certificate(serialNumber)` returns one certificate.
- `certificates(entityId, tenantId, status)` lists matching certificates.
- `caChain` returns the public CA chain.

The same public CA material is available through `GET /certs/ca-chain`. CRL and OCSP material are available through `GET /certs/crl` and `POST /certs/ocsp`.

Generated leaf private keys are not retrievable after issuance. Atom returns the generated leaf private key once as `privateKeyPem`; after that, Atom keeps only the certificate record. CSR-issued certificates never expose a private key to Atom, so there is no private key to return.

CA private keys are stored only as encrypted bytes in `certificate_authorities.encrypted_private_key` with `private_key_nonce`. Atom decrypts them internally using `ATOM_CERTS_KEY_ENCRYPTION_SECRET` only when it must sign certificates, CRLs, or OCSP responses. CA private keys are not returned by UI, GraphQL, HTTP, or gRPC APIs.

Useful operational queries:

```sql
-- List issued certificate credentials.
SELECT id,
       entity_id,
       identifier AS serial_number,
       status,
       expires_at,
       metadata->>'fingerprint_sha256' AS fingerprint_sha256
FROM credentials
WHERE kind = 'certificate'
ORDER BY created_at DESC;

-- Retrieve one issued certificate PEM by serial number.
SELECT metadata->>'certificate_pem' AS certificate_pem
FROM credentials
WHERE kind = 'certificate'
  AND identifier = '<serial-number>';

-- Inspect Atom CA rows without exposing private-key plaintext.
SELECT id,
       kind,
       status,
       serial_number,
       not_before,
       not_after,
       certificate_pem
FROM certificate_authorities
ORDER BY kind, created_at DESC;

-- Inspect cached CRL state.
SELECT crl_number,
       this_update,
       next_update,
       dirty
FROM certificate_crl_state;
```

---

## Internal CA

Atom uses an internal root CA and intermediate CA.

- On startup, if certificates are enabled and no CA exists, Atom generates a root CA and intermediate CA.
- The root signs the intermediate.
- The active intermediate signs entity certificates.
- CA private keys are encrypted before storage in Postgres using an AES-256-GCM key derived with HKDF-SHA256 from `ATOM_CERTS_KEY_ENCRYPTION_SECRET`.
- Atom fails startup if certificate support is enabled but CA key encryption/decryption cannot be configured.
- CA bootstrap uses a Postgres advisory transaction lock so concurrent Atom replicas do not generate competing CAs.
- Issuance uses only the active intermediate CA and refuses leaf validity that extends beyond the active intermediate's validity.

Default lifetimes:

- Root CA: 10 years
- Intermediate CA: 5 years
- Leaf certificate: 30 days
- Maximum leaf TTL: 30 days

Required deployment setting:

```text
ATOM_CERTS_ENABLED=true
ATOM_CERTS_KEY_ENCRYPTION_SECRET=<at least 32 random bytes>
```

The key-encryption secret is required whenever certificates are enabled.

---

## Lifecycle

Atom supports:

- generated certificate issuance for an entity;
- CSR signing for an entity;
- certificate listing and viewing;
- renewal by serial number;
- serial revocation;
- entity-wide certificate revocation;
- CA chain publication;
- CRL publication;
- OCSP responses;
- runtime serial-to-entity lookup.

Renewal creates a new certificate and serial number. The old certificate remains valid until expiry unless the caller requests old-certificate revocation.

CSR signing verifies the CSR and signs the CSR public key. Atom does not store or return a private key for CSR-issued certificates.

CSR-issued certificates are forced to non-CA leaf certificates with `digitalSignature` key usage and `clientAuth` extended key usage. Atom does not trust CSR CA/basic-constraint requests.

Requested TTL values above `ATOM_CERTS_LEAF_MAX_TTL_SECS` are rejected. Generated leaf certificates use a five-minute negative `notBefore` skew to tolerate small clock differences.

Certificate serial numbers are normalized lowercase hex. Atom retries serial generation on unique collisions.

Certificate fingerprints are SHA-256 over certificate DER, not over PEM text.

Full multi-generation CA rotation is not part of v1. v1 supports active/retired/revoked CA rows and active intermediate replacement readiness, but operators must plan CA rotation explicitly.

---

## Interfaces

GraphQL management APIs expose:

- `certificates`
- `certificate`
- `caChain`
- `issueCertificate`
- `issueCertificateFromCsr`
- `renewCertificate`
- `revokeCertificate`
- `revokeEntityCertificates`

Public PKI endpoints expose standard unauthenticated artifacts:

- `GET /certs/ca-chain`
- `GET /certs/crl`
- `POST /certs/ocsp`

Runtime services use Atom gRPC:

- `CertificateService.ResolveCertificate`
- `CertificateService.RevokeEntityCertificates`

HTTP GraphQL remains bearer-token based. Client TLS termination and certificate extraction are handled by the runtime service, which then asks Atom to resolve the serial.

Runtime certificate lookup is authorization-gated. A caller of `ResolveCertificate` must authenticate to Atom and hold `authz.check` on the resolved certificate tenant or platform.

---

## Authorization

Certificate operations use Atom credential authorization:

- Issue: credential `manage` on the target entity.
- View/list: credential `read` or `manage`.
- Renew: exact credential `rotate` or `manage`; target entity credential `manage` also allows it.
- Revoke: exact credential `revoke` or `manage`; target entity credential `manage` also allows it.
- Runtime resolve: `authz.check` on the resolved tenant or platform.
- CA chain, CRL, and OCSP are public.

Credential authority follows the target entity's `tenant_id`, not tenant membership. Tenant admins may manage certificates only for tenant-owned entities in their tenant unless explicit platform policy delegates authority.

The GraphQL certificate list supports optional `entityId`, `tenantId`, and `status` filters. Platform readers/managers can list globally; tenant readers/managers can list tenant-owned certificate credentials.

---

## Revocation, CRL, and OCSP

Revocation updates the credential status to `revoked`, writes revocation metadata, and marks the CRL state dirty.

CRL responses are DER-encoded and cached in Postgres. Atom regenerates the CRL only when:

- a certificate was revoked;
- entity-wide revocation changed certificate state;
- the cached CRL is missing;
- the cached CRL reached `nextUpdate`.

CRL regeneration uses a Postgres advisory transaction lock so concurrent Atom replicas do not race CRL numbers.

OCSP responses validate the request issuer name/key hashes against Atom's active intermediate CA. Requests with mismatched issuer hashes return `unknown` for that certificate. Malformed OCSP requests receive DER OCSP non-success responses rather than JSON API errors.

---

## Magistrala Alignment

Magistrala clients may authenticate with Atom API keys or Atom certificates. Former Magistrala certificate service behavior is moved into Atom:

- certificate issuance and CSR signing happen in Atom;
- revocation state lives in Atom credentials;
- CRL and OCSP are served by Atom;
- runtime services resolve certificate serials through Atom gRPC.

Magistrala should not run a separate certs service or OpenBao deployment for Atom-owned identity.
