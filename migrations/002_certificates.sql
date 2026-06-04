CREATE TABLE certificate_authorities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind TEXT NOT NULL CHECK (kind IN ('root', 'intermediate')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'retired', 'revoked')),
    subject JSONB NOT NULL DEFAULT '{}'::jsonb,
    serial_number TEXT NOT NULL UNIQUE,
    certificate_pem TEXT NOT NULL,
    encrypted_private_key BYTEA NOT NULL,
    private_key_nonce BYTEA NOT NULL,
    not_before TIMESTAMPTZ NOT NULL,
    not_after TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_certificate_authorities_active_kind
    ON certificate_authorities(kind)
    WHERE status = 'active';
CREATE INDEX idx_certificate_authorities_status
    ON certificate_authorities(status, not_after);

CREATE TABLE certificate_crl_state (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    crl_number BIGINT NOT NULL DEFAULT 0,
    crl_der BYTEA,
    this_update TIMESTAMPTZ,
    next_update TIMESTAMPTZ,
    dirty BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO certificate_crl_state (id, crl_number, dirty)
VALUES (TRUE, 0, TRUE)
ON CONFLICT (id) DO NOTHING;

CREATE UNIQUE INDEX idx_credentials_certificate_serial
    ON credentials(identifier)
    WHERE kind = 'certificate' AND identifier IS NOT NULL;
CREATE INDEX idx_credentials_certificate_status_expiry
    ON credentials(kind, status, expires_at)
    WHERE kind = 'certificate';

INSERT INTO action_applicability (action_id, object_kind, object_type)
SELECT id, 'credential', NULL
FROM actions
WHERE name IN ('read', 'rotate')
ON CONFLICT DO NOTHING;
