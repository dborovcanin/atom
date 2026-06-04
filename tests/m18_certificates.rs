mod common;

use atom::{
    certs::service,
    config::{self, Config},
};
use ocsp::{
    common::asn1::{CertId, Oid},
    oid::ALGO_SHA1_DOT,
    request::OneReq,
};
use rcgen::{CertificateParams, DnType, KeyPair};
use ring::digest;
use uuid::Uuid;
use x509_parser::pem::parse_x509_pem;

fn cert_config() -> Config {
    Config {
        database_url: String::new(),
        listen_addr: String::new(),
        grpc_addr: String::new(),
        jwt_expiry_secs: 3600,
        jwt_issuer: "http://localhost:8080".into(),
        jwt_audience: "magistrala".into(),
        admin_entity_id: config::ADMIN_ENTITY_ID,
        admin_secret: None,
        service_secret: None,
        service_entity_id: config::SERVICE_ENTITY_ID,
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
        certs_root_common_name: "Atom Test Root CA".into(),
        certs_intermediate_common_name: "Atom Test Intermediate CA".into(),
    }
}

#[tokio::test]
#[ignore]
async fn certificate_lifecycle_with_database() {
    let pool = common::pool().await;
    let cfg = cert_config();
    service::bootstrap_if_needed(&pool, &cfg).await.unwrap();
    service::bootstrap_if_needed(&pool, &cfg).await.unwrap();

    let entity_id = Uuid::new_v4();
    sqlx::query("INSERT INTO entities (id, name, kind) VALUES ($1, $2, 'device')")
        .bind(entity_id)
        .bind(format!("cert-device-{entity_id}"))
        .execute(&pool)
        .await
        .unwrap();

    let issued = service::issue_certificate(
        &pool,
        &cfg,
        service::IssueCertificate {
            entity_id,
            ttl_secs: Some(3600),
            common_name: Some("cert-device".into()),
            dns_names: vec!["cert-device.local".into()],
            ip_addresses: vec![],
        },
    )
    .await
    .unwrap();
    assert!(issued.private_key_pem.is_some());
    let identity = service::resolve_certificate_identity(
        &pool,
        &issued.certificate.serial_number,
        Some(&issued.certificate.fingerprint_sha256),
    )
    .await
    .unwrap();
    assert_eq!(identity.entity_id, entity_id);

    let csr_pem = test_csr_pem();
    let csr_issued = service::issue_certificate_from_csr(
        &pool,
        &cfg,
        service::IssueCertificateFromCsr {
            entity_id,
            ttl_secs: Some(3600),
            csr_pem,
        },
    )
    .await
    .unwrap();
    assert!(csr_issued.private_key_pem.is_none());

    let renewed = service::renew_certificate(
        &pool,
        &cfg,
        service::RenewCertificate {
            serial_number: issued.certificate.serial_number.clone(),
            ttl_secs: Some(3600),
            revoke_old: false,
        },
    )
    .await
    .unwrap();
    assert_ne!(
        renewed.certificate.serial_number,
        issued.certificate.serial_number
    );

    let chain = service::ca_chain(&pool).await.unwrap();
    let good = service::ocsp_response(
        &pool,
        &cfg,
        &ocsp_request_for_serial(&chain, &issued.certificate.serial_number),
    )
    .await
    .unwrap();
    assert_ocsp_success(&good);

    service::revoke_certificate(
        &pool,
        &issued.certificate.serial_number,
        Some("test".into()),
    )
    .await
    .unwrap();
    assert!(
        service::resolve_certificate_identity(&pool, &issued.certificate.serial_number, None,)
            .await
            .is_err()
    );

    let revoked = service::ocsp_response(
        &pool,
        &cfg,
        &ocsp_request_for_serial(&chain, &issued.certificate.serial_number),
    )
    .await
    .unwrap();
    assert_ocsp_success(&revoked);

    let revoked_count =
        service::revoke_entity_certificates(&pool, entity_id, Some("entity".into()))
            .await
            .unwrap();
    assert!(revoked_count >= 2);

    let crl = service::generate_crl(&pool, &cfg).await.unwrap();
    let (_, parsed_crl) = x509_parser::parse_x509_crl(&crl).unwrap();
    let serial = hex::decode(&issued.certificate.serial_number).unwrap();
    assert!(parsed_crl
        .iter_revoked_certificates()
        .any(|cert| cert.raw_serial() == serial.as_slice()));

    let unknown = service::ocsp_response(
        &pool,
        &cfg,
        &ocsp_request_for_serial(&chain, "0102030405060708"),
    )
    .await
    .unwrap();
    assert_ocsp_success(&unknown);
}

fn test_csr_pem() -> String {
    let key_pair = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec!["csr-device.local".into()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "csr-device");
    params.serialize_request(&key_pair).unwrap().pem().unwrap()
}

fn ocsp_request_for_serial(ca_chain_pem: &str, serial_hex: &str) -> Vec<u8> {
    let der = first_certificate_der(ca_chain_pem);
    let (_, cert) = x509_parser::parse_x509_certificate(&der).unwrap();
    let name_hash = digest::digest(
        &digest::SHA1_FOR_LEGACY_USE_ONLY,
        cert.tbs_certificate.subject.as_raw(),
    );
    let key_hash = digest::digest(
        &digest::SHA1_FOR_LEGACY_USE_ONLY,
        cert.tbs_certificate
            .subject_pki
            .subject_public_key
            .data
            .as_ref(),
    );
    let certid = CertId::new(
        Oid::new_from_dot(ALGO_SHA1_DOT).unwrap(),
        name_hash.as_ref(),
        key_hash.as_ref(),
        &hex::decode(serial_hex).unwrap(),
    );
    let one = OneReq {
        certid,
        one_req_ext: None,
    };
    let request_list = der_sequence(one.to_der().unwrap());
    let tbs_request = der_sequence(request_list);
    der_sequence(tbs_request)
}

fn first_certificate_der(pem: &str) -> Vec<u8> {
    parse_x509_pem(pem.as_bytes()).unwrap().1.contents
}

fn assert_ocsp_success(response: &[u8]) {
    assert!(response.starts_with(&[0x30]));
    assert!(response
        .windows(3)
        .any(|window| window == [0x0a, 0x01, 0x00]));
}

fn der_sequence(value: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.len() + 6);
    out.push(0x30);
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
