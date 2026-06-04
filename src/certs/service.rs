use chrono::{DateTime, Datelike, Timelike, Utc};
use ocsp::{
    common::asn1::{GeneralizedTime, Oid},
    oid::{ALGO_SHA1_DOT, ALGO_SHA256_WITH_RSA_ENCRYPTION_DOT, OCSP_RESPONSE_BASIC_DOT},
    request::OcspRequest,
    response::{
        CertStatus, CertStatusCode, CrlReason, OcspRespStatus, OcspResponse, OneResp, ResponderId,
        ResponseData, RevokedInfo,
    },
};
use rcgen::{
    BasicConstraints, CertificateParams, CertificateRevocationListParams,
    CertificateSigningRequestParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyIdMethod,
    KeyPair, KeyUsagePurpose, RevocationReason, RevokedCertParams, RsaKeySize, SanType,
    SerialNumber, SigningKey, PKCS_RSA_SHA256,
};
use ring::{digest, rand, rand::SecureRandom};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use x509_parser::pem::parse_x509_pem;
use zeroize::Zeroize;

use crate::{config::Config, error::AppError};

use super::{crypto, repo};

const CA_BOOTSTRAP_LOCK_ID: i64 = 0x0041_544f_4d43_4101;
const CRL_REGEN_LOCK_ID: i64 = 0x0041_544f_4d43_524c;
const LEAF_CLOCK_SKEW_SECS: i64 = 300;
const CRL_TTL_HOURS: i64 = 24;
const SERIAL_INSERT_ATTEMPTS: usize = 3;

#[derive(Debug, Clone)]
pub struct IssueCertificate {
    pub entity_id: Uuid,
    pub ttl_secs: Option<u64>,
    pub common_name: Option<String>,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IssueCertificateFromCsr {
    pub entity_id: Uuid,
    pub ttl_secs: Option<u64>,
    pub csr_pem: String,
}

#[derive(Debug, Clone)]
pub struct RenewCertificate {
    pub serial_number: String,
    pub ttl_secs: Option<u64>,
    pub revoke_old: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateMetadata {
    pub certificate_pem: String,
    pub subject: Value,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub issuer_ca_id: Uuid,
    pub issuer_serial_number: String,
    pub fingerprint_sha256: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub issued_from_csr: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CertificateRecord {
    pub credential_id: Uuid,
    pub entity_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub serial_number: String,
    pub status: String,
    pub certificate_pem: String,
    pub subject: Value,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub fingerprint_sha256: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IssuedCertificate {
    pub certificate: CertificateRecord,
    pub private_key_pem: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CertificateIdentity {
    pub entity_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub credential_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

struct LoadedIssuer {
    ca: repo::CertificateAuthority,
    issuer: Issuer<'static, KeyPair>,
    key_pair: KeyPair,
    certificate_der: Vec<u8>,
    issuer_name_hash_sha1: Vec<u8>,
    issuer_key_hash_sha1: Vec<u8>,
}

struct PersistCertificate {
    entity_id: Uuid,
    serial_number: String,
    certificate_pem: String,
    subject: Value,
    dns_names: Vec<String>,
    ip_addresses: Vec<String>,
    issued_from_csr: bool,
    not_before: DateTime<Utc>,
    not_after: DateTime<Utc>,
}

pub async fn bootstrap_if_needed(pool: &sqlx::PgPool, config: &Config) -> Result<(), AppError> {
    if !config.certs_enabled {
        return Ok(());
    }
    validate_encryption_config(config)?;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(CA_BOOTSTRAP_LOCK_ID)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    let root = repo::active_authority_tx(&mut tx, "root").await?;
    let intermediate = repo::active_authority_tx(&mut tx, "intermediate").await?;
    match (root, intermediate) {
        (Some(_), Some(_)) => {
            tx.commit().await.map_err(AppError::Database)?;
            return Ok(());
        }
        (None, None) => {}
        (Some(_), None) | (None, Some(_)) => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "certificate CA bootstrap is partially initialized"
            )));
        }
    }

    let now = OffsetDateTime::now_utc();
    let root_serial = random_serial()?;
    let intermediate_serial = random_serial()?;

    let mut root_params = ca_params(
        &config.certs_root_common_name,
        root_serial.clone(),
        now,
        config.certs_root_ttl_secs,
    )?;
    root_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let root_key =
        KeyPair::generate_rsa_for(&PKCS_RSA_SHA256, RsaKeySize::_2048).map_err(rcgen_err)?;
    let mut root_private_key = root_key.serialize_pem();
    let root_cert = root_params.self_signed(&root_key).map_err(rcgen_err)?;
    let root_issuer = Issuer::new(root_params.clone(), root_key);

    let mut intermediate_params = ca_params(
        &config.certs_intermediate_common_name,
        intermediate_serial.clone(),
        now,
        config.certs_intermediate_ttl_secs,
    )?;
    intermediate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let intermediate_key =
        KeyPair::generate_rsa_for(&PKCS_RSA_SHA256, RsaKeySize::_2048).map_err(rcgen_err)?;
    let mut intermediate_private_key = intermediate_key.serialize_pem();
    let intermediate_cert = intermediate_params
        .signed_by(&intermediate_key, &root_issuer)
        .map_err(rcgen_err)?;

    let encrypted_root = crypto::encrypt_private_key(config, root_private_key.as_bytes())?;
    root_private_key.zeroize();
    repo::insert_authority_tx(
        &mut tx,
        repo::NewAuthority {
            kind: "root",
            subject: json!({"common_name": config.certs_root_common_name}),
            serial_number: &serial_to_string(&root_serial),
            certificate_pem: &root_cert.pem(),
            encrypted_private_key: &encrypted_root.ciphertext,
            private_key_nonce: &encrypted_root.nonce,
            not_before: to_chrono(now)?,
            not_after: to_chrono(now + Duration::seconds(config.certs_root_ttl_secs as i64))?,
        },
    )
    .await?;

    let encrypted_intermediate =
        crypto::encrypt_private_key(config, intermediate_private_key.as_bytes())?;
    intermediate_private_key.zeroize();
    repo::insert_authority_tx(
        &mut tx,
        repo::NewAuthority {
            kind: "intermediate",
            subject: json!({"common_name": config.certs_intermediate_common_name}),
            serial_number: &serial_to_string(&intermediate_serial),
            certificate_pem: &intermediate_cert.pem(),
            encrypted_private_key: &encrypted_intermediate.ciphertext,
            private_key_nonce: &encrypted_intermediate.nonce,
            not_before: to_chrono(now)?,
            not_after: to_chrono(
                now + Duration::seconds(config.certs_intermediate_ttl_secs as i64),
            )?,
        },
    )
    .await?;

    tx.commit().await.map_err(AppError::Database)?;
    tracing::info!("certificate authorities bootstrapped");
    Ok(())
}

pub async fn issue_certificate(
    pool: &sqlx::PgPool,
    config: &Config,
    input: IssueCertificate,
) -> Result<IssuedCertificate, AppError> {
    ensure_enabled(config)?;
    repo::entity_tenant_id(pool, input.entity_id).await?;
    let loaded = load_intermediate(pool, config).await?;
    let ttl = leaf_ttl(config, input.ttl_secs)?;
    let now = OffsetDateTime::now_utc();
    let not_before = now - Duration::seconds(LEAF_CLOCK_SKEW_SECS);
    let not_after = now + Duration::seconds(ttl as i64);
    ensure_issuer_covers_leaf(&loaded.ca, not_after)?;
    let common_name = input
        .common_name
        .clone()
        .unwrap_or_else(|| input.entity_id.to_string());
    let san_names = input
        .dns_names
        .iter()
        .chain(input.ip_addresses.iter())
        .cloned()
        .collect::<Vec<_>>();

    for attempt in 0..SERIAL_INSERT_ATTEMPTS {
        let serial = random_serial()?;
        let serial_number = serial_to_string(&serial);
        let mut params = CertificateParams::new(san_names.clone()).map_err(rcgen_err)?;
        params.distinguished_name = rcgen::DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, common_name.clone());
        params.serial_number = Some(serial);
        params.not_before = not_before;
        params.not_after = not_after;
        params.use_authority_key_identifier_extension = true;
        params.key_usages.push(KeyUsagePurpose::DigitalSignature);
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ClientAuth);

        let key_pair = KeyPair::generate().map_err(rcgen_err)?;
        let cert = params
            .signed_by(&key_pair, &loaded.issuer)
            .map_err(rcgen_err)?;
        let mut private_key_pem = key_pair.serialize_pem();
        match persist_certificate(
            pool,
            &loaded.ca,
            PersistCertificate {
                entity_id: input.entity_id,
                serial_number,
                certificate_pem: cert.pem(),
                subject: json!({"common_name": common_name}),
                dns_names: input.dns_names.clone(),
                ip_addresses: input.ip_addresses.clone(),
                issued_from_csr: false,
                not_before: to_chrono(not_before)?,
                not_after: to_chrono(not_after)?,
            },
        )
        .await
        {
            Ok(record) => {
                return Ok(IssuedCertificate {
                    certificate: record,
                    private_key_pem: Some(private_key_pem),
                });
            }
            Err(err) if is_unique_violation(&err) && attempt + 1 < SERIAL_INSERT_ATTEMPTS => {
                private_key_pem.zeroize();
            }
            Err(err) => {
                private_key_pem.zeroize();
                return Err(err);
            }
        }
    }

    Err(AppError::conflict(
        "failed to allocate a unique certificate serial number",
    ))
}

pub async fn issue_certificate_from_csr(
    pool: &sqlx::PgPool,
    config: &Config,
    input: IssueCertificateFromCsr,
) -> Result<IssuedCertificate, AppError> {
    ensure_enabled(config)?;
    repo::entity_tenant_id(pool, input.entity_id).await?;
    let loaded = load_intermediate(pool, config).await?;
    let ttl = leaf_ttl(config, input.ttl_secs)?;
    let now = OffsetDateTime::now_utc();
    let not_before = now - Duration::seconds(LEAF_CLOCK_SKEW_SECS);
    let not_after = now + Duration::seconds(ttl as i64);
    ensure_issuer_covers_leaf(&loaded.ca, not_after)?;
    let mut csr_template = CertificateSigningRequestParams::from_pem(&input.csr_pem)
        .map_err(|_| AppError::bad_request("invalid CSR"))?;
    force_leaf_csr_params(&mut csr_template.params);
    let (dns_names, ip_addresses) = san_metadata(&csr_template.params);
    let subject = json!({"csr_subject": format!("{:?}", csr_template.params.distinguished_name)});

    for attempt in 0..SERIAL_INSERT_ATTEMPTS {
        let serial = random_serial()?;
        let serial_number = serial_to_string(&serial);
        let mut csr = csr_template.clone();
        csr.params.serial_number = Some(serial);
        csr.params.not_before = not_before;
        csr.params.not_after = not_after;
        let cert = csr.signed_by(&loaded.issuer).map_err(rcgen_err)?;
        match persist_certificate(
            pool,
            &loaded.ca,
            PersistCertificate {
                entity_id: input.entity_id,
                serial_number,
                certificate_pem: cert.pem(),
                subject: subject.clone(),
                dns_names: dns_names.clone(),
                ip_addresses: ip_addresses.clone(),
                issued_from_csr: true,
                not_before: to_chrono(not_before)?,
                not_after: to_chrono(not_after)?,
            },
        )
        .await
        {
            Ok(record) => {
                return Ok(IssuedCertificate {
                    certificate: record,
                    private_key_pem: None,
                });
            }
            Err(err) if is_unique_violation(&err) && attempt + 1 < SERIAL_INSERT_ATTEMPTS => {}
            Err(err) => return Err(err),
        }
    }

    Err(AppError::conflict(
        "failed to allocate a unique certificate serial number",
    ))
}

pub async fn renew_certificate(
    pool: &sqlx::PgPool,
    config: &Config,
    input: RenewCertificate,
) -> Result<IssuedCertificate, AppError> {
    let serial = normalize_serial(&input.serial_number)?;
    let old = certificate_by_serial(pool, &serial).await?;
    if old.status == "revoked" {
        return Err(AppError::bad_request("cannot renew a revoked certificate"));
    }
    let issued = issue_certificate(
        pool,
        config,
        IssueCertificate {
            entity_id: old.entity_id,
            ttl_secs: input.ttl_secs,
            common_name: old
                .subject
                .get("common_name")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            dns_names: old.dns_names.clone(),
            ip_addresses: old.ip_addresses.clone(),
        },
    )
    .await?;
    if input.revoke_old {
        revoke_certificate(pool, &serial, Some("superseded".into())).await?;
    }
    Ok(issued)
}

pub async fn revoke_certificate(
    pool: &sqlx::PgPool,
    serial_number: &str,
    reason: Option<String>,
) -> Result<CertificateRecord, AppError> {
    let serial = normalize_serial(serial_number)?;
    let current = repo::certificate_by_serial(pool, &serial).await?;
    let mut metadata = current.metadata.clone();
    let now = Utc::now();
    metadata["revoked_at"] = json!(now);
    metadata["revocation_reason"] = json!(reason.clone().unwrap_or_else(|| "unspecified".into()));
    repo::revoke_certificate(pool, current.id, metadata).await?;
    repo::mark_crl_dirty(pool).await?;
    certificate_by_serial(pool, &serial).await
}

pub async fn revoke_entity_certificates(
    pool: &sqlx::PgPool,
    entity_id: Uuid,
    reason: Option<String>,
) -> Result<usize, AppError> {
    let certs = repo::active_entity_certificates(pool, entity_id).await?;
    let count = certs.len();
    for cert in certs {
        let mut metadata = cert.metadata.clone();
        metadata["revoked_at"] = json!(Utc::now());
        metadata["revocation_reason"] =
            json!(reason.clone().unwrap_or_else(|| "entity_revoked".into()));
        repo::revoke_certificate(pool, cert.id, metadata).await?;
    }
    if count > 0 {
        repo::mark_crl_dirty(pool).await?;
    }
    Ok(count)
}

pub async fn certificate_by_serial(
    pool: &sqlx::PgPool,
    serial_number: &str,
) -> Result<CertificateRecord, AppError> {
    repo::certificate_by_serial(pool, &normalize_serial(serial_number)?)
        .await
        .and_then(record_from_row)
}

pub async fn certificate_by_id(
    pool: &sqlx::PgPool,
    credential_id: Uuid,
) -> Result<CertificateRecord, AppError> {
    repo::certificate_by_id(pool, credential_id)
        .await
        .and_then(record_from_row)
}

pub async fn list_certificates(
    pool: &sqlx::PgPool,
    entity_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    status: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<CertificateRecord>, AppError> {
    let status = status.map(validate_certificate_status).transpose()?;
    let rows = repo::list_certificates(
        pool,
        entity_id,
        tenant_id,
        status.as_deref(),
        limit.clamp(1, 100),
        offset.max(0),
    )
    .await?;
    rows.into_iter().map(record_from_row).collect()
}

pub async fn ca_chain(pool: &sqlx::PgPool) -> Result<String, AppError> {
    let intermediate = repo::active_authority(pool, "intermediate")
        .await?
        .ok_or_else(|| AppError::not_found("intermediate CA not found"))?;
    let root = repo::active_authority(pool, "root")
        .await?
        .ok_or_else(|| AppError::not_found("root CA not found"))?;
    Ok(format!(
        "{}{}",
        intermediate.certificate_pem, root.certificate_pem
    ))
}

pub async fn generate_crl(pool: &sqlx::PgPool, config: &Config) -> Result<Vec<u8>, AppError> {
    ensure_enabled(config)?;
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(CRL_REGEN_LOCK_ID)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    let state = repo::crl_state_tx(&mut tx).await?;
    let now_chrono = Utc::now();
    if !should_regenerate_crl(&state, now_chrono) {
        if let Some(crl_der) = state.crl_der {
            tx.commit().await.map_err(AppError::Database)?;
            return Ok(crl_der);
        }
    }

    let loaded = load_intermediate_tx(&mut tx, config).await?;
    let revoked = repo::revoked_certificates(pool).await?;
    let revoked_certs = revoked
        .into_iter()
        .map(|cert| {
            let metadata = metadata_from_value(&cert.metadata)?;
            Ok(RevokedCertParams {
                serial_number: SerialNumber::from(serial_bytes(&cert.identifier)?),
                revocation_time: to_offset(metadata.revoked_at.unwrap_or_else(Utc::now))?,
                reason_code: Some(RevocationReason::Unspecified),
                invalidity_date: None,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let now = OffsetDateTime::now_utc();
    let next_update = now + Duration::hours(CRL_TTL_HOURS);
    let crl_number = state.crl_number + 1;
    let crl = CertificateRevocationListParams {
        this_update: now,
        next_update,
        crl_number: SerialNumber::from(crl_number as u64),
        issuing_distribution_point: None,
        revoked_certs,
        key_identifier_method: KeyIdMethod::Sha256,
    }
    .signed_by(&loaded.issuer)
    .map_err(rcgen_err)?;
    let crl_der = crl.der().as_ref().to_vec();
    repo::store_crl_tx(
        &mut tx,
        crl_number,
        &crl_der,
        to_chrono(now)?,
        to_chrono(next_update)?,
    )
    .await?;
    tx.commit().await.map_err(AppError::Database)?;
    Ok(crl_der)
}

pub async fn ocsp_response(
    pool: &sqlx::PgPool,
    config: &Config,
    request_der: &[u8],
) -> Result<Vec<u8>, AppError> {
    ensure_enabled(config)?;
    let loaded = load_intermediate(pool, config).await?;
    let request = OcspRequest::parse(request_der)
        .map_err(|_| AppError::bad_request("invalid OCSP request"))?;
    let now = Utc::now();
    let this_update = generalized_time(now)?;
    let next_update = Some(generalized_time(now + chrono::Duration::hours(1))?);
    let mut one_responses = Vec::with_capacity(request.tbs_request.request_list.len());
    for one in &request.tbs_request.request_list {
        let issuer_matches = certid_issuer_matches(
            &one.certid,
            &loaded.issuer_name_hash_sha1,
            &loaded.issuer_key_hash_sha1,
        )?;
        let status = if issuer_matches {
            let serial = serial_from_ocsp_request(&one.certid.serial_num)?;
            match repo::certificate_by_serial(pool, &serial).await {
                Ok(cert) if cert.status == "active" => CertStatus::new(CertStatusCode::Good, None),
                Ok(cert) => {
                    let metadata = metadata_from_value(&cert.metadata)?;
                    let revoked_at = metadata.revoked_at.unwrap_or_else(Utc::now);
                    CertStatus::new(
                        CertStatusCode::Revoked,
                        Some(RevokedInfo::new(
                            generalized_time(revoked_at)?,
                            Some(CrlReason::OcspRevokeUnspecified),
                        )),
                    )
                }
                Err(AppError::NotFound(_)) => CertStatus::new(CertStatusCode::Unknown, None),
                Err(err) => return Err(err),
            }
        } else {
            CertStatus::new(CertStatusCode::Unknown, None)
        };
        one_responses.push(OneResp {
            cid: one.certid.clone(),
            cert_status: status,
            this_update,
            next_update,
            one_resp_ext: None,
        });
    }

    let responder = ResponderId::new_key_hash(&loaded.issuer_key_hash_sha1);
    let data = ResponseData::new(responder, this_update, one_responses, None);
    let data_der = data.to_der().map_err(ocsp_err)?;
    let signature = loaded.key_pair.sign(&data_der).map_err(rcgen_err)?;
    let oid = Oid::new_from_dot(ALGO_SHA256_WITH_RSA_ENCRYPTION_DOT).map_err(ocsp_err)?;
    let response_type = Oid::new_from_dot(OCSP_RESPONSE_BASIC_DOT).map_err(ocsp_err)?;
    let basic = basic_ocsp_response_der(
        &data_der,
        &oid,
        &signature,
        &[loaded.certificate_der.as_slice()],
    )?;
    successful_ocsp_response_der(&response_type, &basic)
}

pub async fn resolve_certificate_identity(
    pool: &sqlx::PgPool,
    serial_number: &str,
    fingerprint_sha256: Option<&str>,
) -> Result<CertificateIdentity, AppError> {
    let record = certificate_by_serial(pool, serial_number).await?;
    if record.status != "active" {
        return Err(AppError::Unauthorized("certificate revoked".into()));
    }
    let expires_at = record
        .expires_at
        .ok_or_else(|| AppError::Unauthorized("certificate has no expiry".into()))?;
    if expires_at <= Utc::now() {
        return Err(AppError::Unauthorized("certificate expired".into()));
    }
    if let Some(expected) = fingerprint_sha256 {
        if normalize_fingerprint(expected) != normalize_fingerprint(&record.fingerprint_sha256) {
            return Err(AppError::Unauthorized(
                "certificate fingerprint mismatch".into(),
            ));
        }
    }
    repo::entity_tenant_id(pool, record.entity_id).await?;
    Ok(CertificateIdentity {
        entity_id: record.entity_id,
        tenant_id: record.tenant_id,
        credential_id: record.credential_id,
        expires_at,
    })
}

pub fn normalize_serial(serial_number: &str) -> Result<String, AppError> {
    let normalized = serial_number
        .chars()
        .filter(|ch| *ch != ':' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if normalized.is_empty() || normalized.len() % 2 != 0 || hex::decode(&normalized).is_err() {
        return Err(AppError::bad_request("invalid certificate serial number"));
    }
    Ok(normalized)
}

fn validate_encryption_config(config: &Config) -> Result<(), AppError> {
    if config.certs_leaf_default_ttl_secs > config.certs_leaf_max_ttl_secs {
        return Err(AppError::bad_request(
            "ATOM_CERTS_LEAF_DEFAULT_TTL_SECS must be less than or equal to ATOM_CERTS_LEAF_MAX_TTL_SECS",
        ));
    }
    if config.certs_intermediate_ttl_secs > config.certs_root_ttl_secs {
        return Err(AppError::bad_request(
            "ATOM_CERTS_INTERMEDIATE_TTL_SECS must be less than or equal to ATOM_CERTS_ROOT_TTL_SECS",
        ));
    }
    crypto::encrypt_private_key(config, b"certificate-key-validation").map(|_| ())
}

fn ensure_enabled(config: &Config) -> Result<(), AppError> {
    if config.certs_enabled {
        Ok(())
    } else {
        Err(AppError::bad_request("certificate support is disabled"))
    }
}

fn ca_params(
    common_name: &str,
    serial: SerialNumber,
    now: OffsetDateTime,
    ttl_secs: u64,
) -> Result<CertificateParams, AppError> {
    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(rcgen_err)?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, common_name);
    params.serial_number = Some(serial);
    params.not_before = now;
    params.not_after = now + Duration::seconds(ttl_secs as i64);
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    params.key_usages.push(KeyUsagePurpose::CrlSign);
    Ok(params)
}

async fn load_intermediate(pool: &sqlx::PgPool, config: &Config) -> Result<LoadedIssuer, AppError> {
    let ca = repo::active_authority(pool, "intermediate")
        .await?
        .ok_or_else(|| AppError::not_found("intermediate CA not found"))?;
    load_issuer_from_ca(config, ca)
}

async fn load_intermediate_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    config: &Config,
) -> Result<LoadedIssuer, AppError> {
    let ca = repo::active_authority_tx(tx, "intermediate")
        .await?
        .ok_or_else(|| AppError::not_found("intermediate CA not found"))?;
    load_issuer_from_ca(config, ca)
}

fn load_issuer_from_ca(
    config: &Config,
    ca: repo::CertificateAuthority,
) -> Result<LoadedIssuer, AppError> {
    if ca.not_after <= Utc::now() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "active certificate intermediate CA is expired"
        )));
    }
    let certificate_der = certificate_der_from_pem(&ca.certificate_pem)?;
    let (issuer_name_hash_sha1, issuer_key_hash_sha1) =
        issuer_sha1_hashes_from_der(&certificate_der)?;
    let mut key_pem = String::from_utf8(crypto::decrypt_private_key(
        config,
        &ca.private_key_nonce,
        &ca.encrypted_private_key,
    )?)
    .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid CA key PEM")))?;
    let key_pair = KeyPair::from_pem(&key_pem).map_err(rcgen_err)?;
    let issuer = Issuer::from_ca_cert_pem(
        &ca.certificate_pem,
        KeyPair::from_pem(&key_pem).map_err(rcgen_err)?,
    )
    .map_err(rcgen_err)?;
    key_pem.zeroize();
    Ok(LoadedIssuer {
        ca,
        issuer,
        key_pair,
        certificate_der,
        issuer_name_hash_sha1,
        issuer_key_hash_sha1,
    })
}

async fn persist_certificate(
    pool: &sqlx::PgPool,
    ca: &repo::CertificateAuthority,
    input: PersistCertificate,
) -> Result<CertificateRecord, AppError> {
    let fingerprint_sha256 = certificate_fingerprint_sha256(&input.certificate_pem)?;
    let metadata = CertificateMetadata {
        certificate_pem: input.certificate_pem,
        subject: input.subject,
        dns_names: input.dns_names,
        ip_addresses: input.ip_addresses,
        issuer_ca_id: ca.id,
        issuer_serial_number: ca.serial_number.clone(),
        fingerprint_sha256,
        not_before: input.not_before,
        not_after: input.not_after,
        issued_from_csr: input.issued_from_csr,
        revoked_at: None,
        revocation_reason: None,
    };
    let id = repo::insert_certificate_credential(
        pool,
        input.entity_id,
        &input.serial_number,
        serde_json::to_value(metadata).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?,
        input.not_after,
    )
    .await?;
    certificate_by_id(pool, id).await
}

fn record_from_row(row: repo::CertificateCredential) -> Result<CertificateRecord, AppError> {
    let metadata = metadata_from_value(&row.metadata)?;
    Ok(CertificateRecord {
        credential_id: row.id,
        entity_id: row.entity_id,
        tenant_id: row.tenant_id,
        serial_number: row.identifier,
        status: row.status,
        certificate_pem: metadata.certificate_pem,
        subject: metadata.subject,
        dns_names: metadata.dns_names,
        ip_addresses: metadata.ip_addresses,
        fingerprint_sha256: metadata.fingerprint_sha256,
        expires_at: row.expires_at,
        created_at: row.created_at,
        revoked_at: metadata.revoked_at,
        revocation_reason: metadata.revocation_reason,
    })
}

fn metadata_from_value(value: &Value) -> Result<CertificateMetadata, AppError> {
    serde_json::from_value(value.clone())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid certificate metadata")))
}

fn leaf_ttl(config: &Config, ttl_secs: Option<u64>) -> Result<u64, AppError> {
    let ttl = ttl_secs.unwrap_or(config.certs_leaf_default_ttl_secs);
    if ttl == 0 {
        return Err(AppError::bad_request(
            "certificate TTL must be greater than zero",
        ));
    }
    if ttl > config.certs_leaf_max_ttl_secs {
        return Err(AppError::bad_request(format!(
            "certificate TTL exceeds ATOM_CERTS_LEAF_MAX_TTL_SECS ({})",
            config.certs_leaf_max_ttl_secs
        )));
    }
    Ok(ttl)
}

fn validate_certificate_status(status: String) -> Result<String, AppError> {
    match status.as_str() {
        "active" | "revoked" => Ok(status),
        _ => Err(AppError::bad_request(
            "certificate status must be active or revoked",
        )),
    }
}

fn ensure_issuer_covers_leaf(
    ca: &repo::CertificateAuthority,
    leaf_not_after: OffsetDateTime,
) -> Result<(), AppError> {
    let leaf_not_after = to_chrono(leaf_not_after)?;
    if leaf_not_after > ca.not_after {
        return Err(AppError::bad_request(
            "requested certificate validity exceeds active intermediate CA validity",
        ));
    }
    Ok(())
}

fn force_leaf_csr_params(params: &mut CertificateParams) {
    params.is_ca = IsCa::NoCa;
    params.key_usages.clear();
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.extended_key_usages.clear();
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ClientAuth);
    params.name_constraints = None;
    params.custom_extensions.clear();
    params.use_authority_key_identifier_extension = true;
}

fn san_metadata(params: &CertificateParams) -> (Vec<String>, Vec<String>) {
    let dns_names = params
        .subject_alt_names
        .iter()
        .filter_map(|san| match san {
            SanType::DnsName(name) => Some(name.to_string()),
            SanType::Rfc822Name(_)
            | SanType::URI(_)
            | SanType::IpAddress(_)
            | SanType::OtherName(_)
            | _ => None,
        })
        .collect::<Vec<_>>();
    let ip_addresses = params
        .subject_alt_names
        .iter()
        .filter_map(|san| match san {
            SanType::IpAddress(ip) => Some(ip.to_string()),
            SanType::Rfc822Name(_)
            | SanType::DnsName(_)
            | SanType::URI(_)
            | SanType::OtherName(_)
            | _ => None,
        })
        .collect::<Vec<_>>();
    (dns_names, ip_addresses)
}

fn certificate_fingerprint_sha256(certificate_pem: &str) -> Result<String, AppError> {
    let der = certificate_der_from_pem(certificate_pem)?;
    let fingerprint = digest::digest(&digest::SHA256, &der);
    Ok(hex::encode(fingerprint.as_ref()))
}

fn certificate_der_from_pem(certificate_pem: &str) -> Result<Vec<u8>, AppError> {
    parse_x509_pem(certificate_pem.as_bytes())
        .map(|(_, pem)| pem.contents)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid certificate PEM")))
}

fn issuer_sha1_hashes_from_der(certificate_der: &[u8]) -> Result<(Vec<u8>, Vec<u8>), AppError> {
    let (_, cert) = x509_parser::parse_x509_certificate(certificate_der)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid issuer certificate DER")))?;
    let name_hash = digest::digest(
        &digest::SHA1_FOR_LEGACY_USE_ONLY,
        cert.tbs_certificate.subject.as_raw(),
    )
    .as_ref()
    .to_vec();
    let key_hash = digest::digest(
        &digest::SHA1_FOR_LEGACY_USE_ONLY,
        cert.tbs_certificate
            .subject_pki
            .subject_public_key
            .data
            .as_ref(),
    )
    .as_ref()
    .to_vec();
    Ok((name_hash, key_hash))
}

fn certid_issuer_matches(
    certid: &ocsp::common::asn1::CertId,
    issuer_name_hash_sha1: &[u8],
    issuer_key_hash_sha1: &[u8],
) -> Result<bool, AppError> {
    let sha1 = Oid::new_from_dot(ALGO_SHA1_DOT).map_err(ocsp_err)?;
    Ok(certid.hash_algo == sha1
        && certid.issuer_name_hash == issuer_name_hash_sha1
        && certid.issuer_key_hash == issuer_key_hash_sha1)
}

fn serial_from_ocsp_request(serial: &[u8]) -> Result<String, AppError> {
    let trimmed = serial
        .iter()
        .skip_while(|byte| **byte == 0)
        .copied()
        .collect::<Vec<_>>();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("invalid certificate serial number"));
    }
    normalize_serial(&hex::encode(trimmed))
}

fn should_regenerate_crl(state: &repo::CrlState, now: DateTime<Utc>) -> bool {
    state.dirty
        || state.crl_der.is_none()
        || state
            .next_update
            .map(|next_update| next_update <= now)
            .unwrap_or(true)
}

fn is_unique_violation(err: &AppError) -> bool {
    matches!(
        err,
        AppError::Database(sqlx::Error::Database(db)) if db.code().as_deref() == Some("23505")
    )
}

fn random_serial() -> Result<SerialNumber, AppError> {
    let mut bytes = [0_u8; 16];
    rand::SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to generate serial number")))?;
    bytes[0] &= 0x7f;
    if bytes[0] == 0 {
        bytes[0] = 1;
    }
    Ok(SerialNumber::from(bytes.to_vec()))
}

fn serial_to_string(serial: &SerialNumber) -> String {
    hex::encode(serial.to_bytes())
}

fn serial_bytes(serial_number: &str) -> Result<Vec<u8>, AppError> {
    hex::decode(normalize_serial(serial_number)?)
        .map_err(|_| AppError::bad_request("invalid certificate serial number"))
}

fn to_chrono(value: OffsetDateTime) -> Result<DateTime<Utc>, AppError> {
    DateTime::<Utc>::from_timestamp(value.unix_timestamp(), value.nanosecond())
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("invalid certificate timestamp")))
}

fn to_offset(value: DateTime<Utc>) -> Result<OffsetDateTime, AppError> {
    OffsetDateTime::from_unix_timestamp(value.timestamp())
        .map(|time| time + Duration::nanoseconds(value.timestamp_subsec_nanos() as i64))
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid certificate timestamp")))
}

fn generalized_time(value: DateTime<Utc>) -> Result<GeneralizedTime, AppError> {
    GeneralizedTime::new(
        value.year(),
        value.month(),
        value.day(),
        value.hour(),
        value.minute(),
        value.second(),
    )
    .map_err(ocsp_err)
}

fn normalize_fingerprint(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != ':' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn basic_ocsp_response_der(
    response_data_der: &[u8],
    signature_oid: &Oid,
    signature: &[u8],
    certs: &[&[u8]],
) -> Result<Vec<u8>, AppError> {
    let mut body = response_data_der.to_vec();
    body.extend(signature_oid.to_der_with_null().map_err(ocsp_err)?);
    body.extend(der_bit_string(signature));
    if !certs.is_empty() {
        let cert_list = certs
            .iter()
            .flat_map(|cert| cert.iter().copied())
            .collect::<Vec<_>>();
        body.extend(der_tlv(0xa0, der_tlv(0x30, cert_list)));
    }
    Ok(der_tlv(0x30, body))
}

fn successful_ocsp_response_der(
    response_type: &Oid,
    basic_der: &[u8],
) -> Result<Vec<u8>, AppError> {
    let mut response_bytes = response_type.to_der_raw().map_err(ocsp_err)?;
    response_bytes.extend(der_tlv(0x04, basic_der.to_vec()));
    let mut body = vec![0x0a, 0x01, OcspRespStatus::Successful as u8];
    body.extend(der_tlv(0xa0, der_tlv(0x30, response_bytes)));
    Ok(der_tlv(0x30, body))
}

fn der_bit_string(data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(data.len() + 1);
    body.push(0);
    body.extend_from_slice(data);
    der_tlv(0x03, body)
}

fn der_tlv(tag: u8, value: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 5 + value.len());
    out.push(tag);
    out.extend(der_len(value.len()));
    out.extend(value);
    out
}

fn der_len(len: usize) -> Vec<u8> {
    if len <= 127 {
        return vec![len as u8];
    }
    let bytes = len
        .to_be_bytes()
        .into_iter()
        .skip_while(|byte| *byte == 0)
        .collect::<Vec<_>>();
    let mut out = Vec::with_capacity(bytes.len() + 1);
    out.push(0x80 | bytes.len() as u8);
    out.extend(bytes);
    out
}

fn rcgen_err(err: rcgen::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!("certificate error: {err}"))
}

fn ocsp_err(err: ocsp::err::OcspError) -> AppError {
    AppError::Internal(anyhow::anyhow!("OCSP error: {err:?}"))
}

pub fn unsuccessful_ocsp(status: OcspRespStatus) -> Result<Vec<u8>, AppError> {
    OcspResponse::new_non_success(status)
        .map_err(ocsp_err)?
        .to_der()
        .map_err(ocsp_err)
}

#[cfg(test)]
mod tests {
    use ocsp::common::asn1::CertId;

    use super::*;

    fn config() -> Config {
        Config {
            database_url: String::new(),
            listen_addr: String::new(),
            grpc_addr: String::new(),
            jwt_expiry_secs: 3600,
            jwt_issuer: "http://localhost:8080".into(),
            jwt_audience: "magistrala".into(),
            admin_entity_id: crate::config::ADMIN_ENTITY_ID,
            admin_secret: None,
            service_secret: None,
            service_entity_id: crate::config::SERVICE_ENTITY_ID,
            signup_enabled: false,
            dev_allow_unverified_email_login: false,
            public_base_url: "http://localhost:8080".into(),
            cors_allowed_origins: vec!["http://localhost:8080".into()],
            email_verification_redirect: "http://localhost:8080/auth/email/verify".into(),
            password_reset_redirect: "http://localhost:8080/reset-password".into(),
            invitation_redirect: "http://localhost:8080/invitations/accept".into(),
            oauth_success_redirect: "http://localhost:8080".into(),
            oauth_error_redirect: "http://localhost:8080".into(),
            oidc_providers: vec![],
            smtp: None,
            email_verification_expiry_secs: 86_400,
            invitation_expiry_secs: 604_800,
            oauth_state_expiry_secs: 600,
            auth_exchange_code_expiry_secs: 300,
            certs_enabled: true,
            certs_key_encryption_secret: Some("01234567890123456789012345678901".into()),
            certs_root_ttl_secs: 315_360_000,
            certs_intermediate_ttl_secs: 157_680_000,
            certs_leaf_default_ttl_secs: 2_592_000,
            certs_leaf_max_ttl_secs: 2_592_000,
            certs_root_common_name: "Atom Root CA".into(),
            certs_intermediate_common_name: "Atom Intermediate CA".into(),
        }
    }

    #[test]
    fn normalizes_serial_numbers() {
        assert_eq!(normalize_serial("AA:bb 01").unwrap(), "aabb01");
        assert!(normalize_serial("not-hex").is_err());
    }

    #[test]
    fn certificate_fingerprint_uses_der_not_pem_text() {
        let key = KeyPair::generate().expect("key");
        let mut params =
            CertificateParams::new(vec!["device.example".to_string()]).expect("params");
        params
            .distinguished_name
            .push(DnType::CommonName, "device.example");
        let cert = params.self_signed(&key).expect("cert");
        let pem = cert.pem();
        let fingerprint = certificate_fingerprint_sha256(&pem).expect("fingerprint");
        let der = certificate_der_from_pem(&pem).expect("der");
        let expected = digest::digest(&digest::SHA256, &der);
        let pem_text_hash = digest::digest(&digest::SHA256, pem.as_bytes());

        assert_eq!(fingerprint, hex::encode(expected.as_ref()));
        assert_ne!(fingerprint, hex::encode(pem_text_hash.as_ref()));
    }

    #[test]
    fn leaf_ttl_rejects_values_above_max() {
        let cfg = config();
        assert_eq!(leaf_ttl(&cfg, Some(60)).unwrap(), 60);
        let err = leaf_ttl(&cfg, Some(cfg.certs_leaf_max_ttl_secs + 1)).unwrap_err();
        assert!(err.to_string().contains("exceeds"));
    }

    #[test]
    fn csr_params_are_forced_to_leaf_client_auth() {
        let mut params = CertificateParams::new(Vec::<String>::new()).expect("params");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages.push(KeyUsagePurpose::KeyCertSign);
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ServerAuth);

        force_leaf_csr_params(&mut params);

        assert!(matches!(params.is_ca, IsCa::NoCa));
        assert_eq!(params.key_usages, vec![KeyUsagePurpose::DigitalSignature]);
        assert_eq!(
            params.extended_key_usages,
            vec![ExtendedKeyUsagePurpose::ClientAuth]
        );
    }

    #[test]
    fn ocsp_issuer_hashes_must_match_intermediate() {
        let key = KeyPair::generate_rsa_for(&PKCS_RSA_SHA256, RsaKeySize::_2048).expect("key");
        let mut params = ca_params(
            "Atom Test Intermediate",
            random_serial().unwrap(),
            OffsetDateTime::now_utc(),
            86_400,
        )
        .expect("params");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key).expect("cert");
        let der = certificate_der_from_pem(&cert.pem()).expect("der");
        let (name_hash, key_hash) = issuer_sha1_hashes_from_der(&der).expect("hashes");
        let oid = Oid::new_from_dot(ALGO_SHA1_DOT).expect("oid");
        let serial = vec![1, 2, 3, 4];
        let good = CertId::new(oid.clone(), &name_hash, &key_hash, &serial);
        let bad = CertId::new(oid, &[0; 20], &key_hash, &serial);

        assert!(certid_issuer_matches(&good, &name_hash, &key_hash).unwrap());
        assert!(!certid_issuer_matches(&bad, &name_hash, &key_hash).unwrap());
    }

    #[test]
    fn crl_cache_regenerates_only_when_dirty_missing_or_expired() {
        let now = Utc::now();
        let fresh = repo::CrlState {
            crl_number: 1,
            crl_der: Some(vec![1, 2, 3]),
            this_update: Some(now),
            next_update: Some(now + chrono::Duration::hours(1)),
            dirty: false,
        };
        assert!(!should_regenerate_crl(&fresh, now));

        let mut dirty = fresh.clone();
        dirty.dirty = true;
        assert!(should_regenerate_crl(&dirty, now));

        let mut missing = fresh.clone();
        missing.crl_der = None;
        assert!(should_regenerate_crl(&missing, now));

        let mut expired = fresh;
        expired.next_update = Some(now - chrono::Duration::seconds(1));
        assert!(should_regenerate_crl(&expired, now));
    }
}
