use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::error::{db_err, AppError};

#[derive(Debug, Clone, FromRow)]
pub struct CertificateAuthority {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub subject: Value,
    pub serial_number: String,
    pub certificate_pem: String,
    pub encrypted_private_key: Vec<u8>,
    pub private_key_nonce: Vec<u8>,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CertificateCredential {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub identifier: String,
    pub status: String,
    pub metadata: Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CrlState {
    pub crl_number: i64,
    pub crl_der: Option<Vec<u8>>,
    pub this_update: Option<DateTime<Utc>>,
    pub next_update: Option<DateTime<Utc>>,
    pub dirty: bool,
}

pub struct NewAuthority<'a> {
    pub kind: &'a str,
    pub subject: Value,
    pub serial_number: &'a str,
    pub certificate_pem: &'a str,
    pub encrypted_private_key: &'a [u8],
    pub private_key_nonce: &'a [u8],
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
}

pub async fn active_authority(
    pool: &PgPool,
    kind: &str,
) -> Result<Option<CertificateAuthority>, AppError> {
    sqlx::query_as::<_, CertificateAuthority>(
        r#"
        SELECT id, kind, status, subject, serial_number, certificate_pem,
               encrypted_private_key, private_key_nonce, not_before, not_after
        FROM certificate_authorities
        WHERE kind = $1 AND status = 'active'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(kind)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn active_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
) -> Result<Option<CertificateAuthority>, AppError> {
    sqlx::query_as::<_, CertificateAuthority>(
        r#"
        SELECT id, kind, status, subject, serial_number, certificate_pem,
               encrypted_private_key, private_key_nonce, not_before, not_after
        FROM certificate_authorities
        WHERE kind = $1 AND status = 'active'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(kind)
    .fetch_optional(&mut **tx)
    .await
    .map_err(AppError::Database)
}

pub async fn active_authority_count(pool: &PgPool) -> Result<i64, AppError> {
    sqlx::query_scalar("SELECT COUNT(*) FROM certificate_authorities WHERE status = 'active'")
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn insert_authority(pool: &PgPool, ca: NewAuthority<'_>) -> Result<Uuid, AppError> {
    sqlx::query_scalar(
        r#"
        INSERT INTO certificate_authorities (
            kind, subject, serial_number, certificate_pem, encrypted_private_key,
            private_key_nonce, not_before, not_after
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(ca.kind)
    .bind(ca.subject)
    .bind(ca.serial_number)
    .bind(ca.certificate_pem)
    .bind(ca.encrypted_private_key)
    .bind(ca.private_key_nonce)
    .bind(ca.not_before)
    .bind(ca.not_after)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn insert_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    ca: NewAuthority<'_>,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar(
        r#"
        INSERT INTO certificate_authorities (
            kind, subject, serial_number, certificate_pem, encrypted_private_key,
            private_key_nonce, not_before, not_after
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(ca.kind)
    .bind(ca.subject)
    .bind(ca.serial_number)
    .bind(ca.certificate_pem)
    .bind(ca.encrypted_private_key)
    .bind(ca.private_key_nonce)
    .bind(ca.not_before)
    .bind(ca.not_after)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::Database)
}

pub async fn entity_tenant_id(pool: &PgPool, entity_id: Uuid) -> Result<Option<Uuid>, AppError> {
    sqlx::query_scalar(
        r#"
        SELECT e.tenant_id
        FROM entities e
        LEFT JOIN tenants t ON t.id = e.tenant_id
        WHERE e.id = $1
          AND e.status = 'active'
          AND (e.tenant_id IS NULL OR t.status = 'active')
        "#,
    )
    .bind(entity_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("entity not found"))
}

pub async fn insert_certificate_credential(
    pool: &PgPool,
    entity_id: Uuid,
    serial_number: &str,
    metadata: Value,
    expires_at: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar(
        r#"
        INSERT INTO credentials (id, entity_id, kind, identifier, metadata, expires_at)
        VALUES ($1, $2, 'certificate', $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(entity_id)
    .bind(serial_number)
    .bind(metadata)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn certificate_by_serial(
    pool: &PgPool,
    serial_number: &str,
) -> Result<CertificateCredential, AppError> {
    sqlx::query_as::<_, CertificateCredential>(
        r#"
        SELECT c.id, c.entity_id, e.tenant_id, c.identifier, c.status, c.metadata,
               c.expires_at, c.created_at
        FROM credentials c
        JOIN entities e ON e.id = c.entity_id
        WHERE c.kind = 'certificate' AND c.identifier = $1
        "#,
    )
    .bind(serial_number)
    .fetch_one(pool)
    .await
    .map_err(db_err)
}

pub async fn certificate_by_id(
    pool: &PgPool,
    credential_id: Uuid,
) -> Result<CertificateCredential, AppError> {
    sqlx::query_as::<_, CertificateCredential>(
        r#"
        SELECT c.id, c.entity_id, e.tenant_id, c.identifier, c.status, c.metadata,
               c.expires_at, c.created_at
        FROM credentials c
        JOIN entities e ON e.id = c.entity_id
        WHERE c.kind = 'certificate' AND c.id = $1
        "#,
    )
    .bind(credential_id)
    .fetch_one(pool)
    .await
    .map_err(db_err)
}

pub async fn list_certificates(
    pool: &PgPool,
    entity_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<CertificateCredential>, AppError> {
    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        SELECT c.id, c.entity_id, e.tenant_id, c.identifier, c.status, c.metadata,
               c.expires_at, c.created_at
        FROM credentials c
        JOIN entities e ON e.id = c.entity_id
        WHERE c.kind = 'certificate'
        "#,
    );
    if let Some(entity_id) = entity_id {
        query.push(" AND c.entity_id = ");
        query.push_bind(entity_id);
    }
    if let Some(tenant_id) = tenant_id {
        query.push(" AND e.tenant_id = ");
        query.push_bind(tenant_id);
    }
    if let Some(status) = status {
        query.push(" AND c.status = ");
        query.push_bind(status);
    }
    query.push(" ORDER BY c.created_at DESC LIMIT ");
    query.push_bind(limit);
    query.push(" OFFSET ");
    query.push_bind(offset);

    query
        .build_query_as::<CertificateCredential>()
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn revoke_certificate(
    pool: &PgPool,
    credential_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE credentials
        SET status = 'revoked', metadata = $2
        WHERE id = $1 AND kind = 'certificate'
        "#,
    )
    .bind(credential_id)
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn active_entity_certificates(
    pool: &PgPool,
    entity_id: Uuid,
) -> Result<Vec<CertificateCredential>, AppError> {
    sqlx::query_as::<_, CertificateCredential>(
        r#"
        SELECT c.id, c.entity_id, e.tenant_id, c.identifier, c.status, c.metadata,
               c.expires_at, c.created_at
        FROM credentials c
        JOIN entities e ON e.id = c.entity_id
        WHERE c.kind = 'certificate' AND c.entity_id = $1 AND c.status = 'active'
        "#,
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn revoked_certificates(pool: &PgPool) -> Result<Vec<CertificateCredential>, AppError> {
    sqlx::query_as::<_, CertificateCredential>(
        r#"
        SELECT c.id, c.entity_id, e.tenant_id, c.identifier, c.status, c.metadata,
               c.expires_at, c.created_at
        FROM credentials c
        JOIN entities e ON e.id = c.entity_id
        WHERE c.kind = 'certificate' AND c.status = 'revoked'
        ORDER BY c.created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn next_crl_number(pool: &PgPool) -> Result<i64, AppError> {
    sqlx::query_scalar(
        r#"
        UPDATE certificate_crl_state
        SET crl_number = crl_number + 1, updated_at = now()
        WHERE id = TRUE
        RETURNING crl_number
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn crl_state_tx(tx: &mut Transaction<'_, Postgres>) -> Result<CrlState, AppError> {
    sqlx::query_as::<_, CrlState>(
        r#"
        SELECT crl_number, crl_der, this_update, next_update, dirty
        FROM certificate_crl_state
        WHERE id = TRUE
        "#,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::Database)
}

pub async fn store_crl_tx(
    tx: &mut Transaction<'_, Postgres>,
    crl_number: i64,
    crl_der: &[u8],
    this_update: DateTime<Utc>,
    next_update: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE certificate_crl_state
        SET crl_number = $1,
            crl_der = $2,
            this_update = $3,
            next_update = $4,
            dirty = FALSE,
            updated_at = now()
        WHERE id = TRUE
        "#,
    )
    .bind(crl_number)
    .bind(crl_der)
    .bind(this_update)
    .bind(next_update)
    .execute(&mut **tx)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn mark_crl_dirty(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE certificate_crl_state
        SET dirty = TRUE, updated_at = now()
        WHERE id = TRUE
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}
