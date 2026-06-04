use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    hkdf, rand,
    rand::SecureRandom,
};
use zeroize::Zeroize;

use crate::{config::Config, error::AppError};

const KEY_DERIVATION_SALT: &[u8] = b"atom-certs-key-encryption";
const KEY_DERIVATION_CONTEXT: &[u8] = b"atom/certificates/ca-key-encryption/v1";

pub struct EncryptedBlob {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub fn encrypt_private_key(config: &Config, plaintext: &[u8]) -> Result<EncryptedBlob, AppError> {
    let key = encryption_key(config)?;
    let mut nonce = [0_u8; 12];
    rand::SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to generate certificate nonce")))?;
    let mut ciphertext = plaintext.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::empty(),
        &mut ciphertext,
    )
    .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to encrypt certificate key")))?;
    Ok(EncryptedBlob {
        ciphertext,
        nonce: nonce.to_vec(),
    })
}

pub fn decrypt_private_key(
    config: &Config,
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, AppError> {
    let key = encryption_key(config)?;
    let nonce = Nonce::try_assume_unique_for_key(nonce)
        .map_err(|_| AppError::bad_request("invalid certificate key nonce"))?;
    let mut plaintext = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut plaintext)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to decrypt certificate key")))?;
    let decrypted = plaintext.to_vec();
    plaintext.zeroize();
    Ok(decrypted)
}

fn encryption_key(config: &Config) -> Result<LessSafeKey, AppError> {
    let secret = config
        .certs_key_encryption_secret
        .as_deref()
        .ok_or_else(|| AppError::bad_request("ATOM_CERTS_KEY_ENCRYPTION_SECRET must be set"))?;
    if secret.len() < 32 {
        return Err(AppError::bad_request(
            "ATOM_CERTS_KEY_ENCRYPTION_SECRET must be at least 32 bytes",
        ));
    }
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, KEY_DERIVATION_SALT);
    let prk = salt.extract(secret.as_bytes());
    let okm = prk
        .expand(&[KEY_DERIVATION_CONTEXT], Aes256KeyLen)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid certificate encryption key")))?;
    let mut key_bytes = [0_u8; 32];
    okm.fill(&mut key_bytes)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid certificate encryption key")))?;
    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid certificate encryption key")))?;
    key_bytes.zeroize();
    Ok(LessSafeKey::new(unbound))
}

struct Aes256KeyLen;

impl hkdf::KeyType for Aes256KeyLen {
    fn len(&self) -> usize {
        32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(secret: &str) -> Config {
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
            certs_key_encryption_secret: Some(secret.into()),
            certs_root_ttl_secs: 315_360_000,
            certs_intermediate_ttl_secs: 157_680_000,
            certs_leaf_default_ttl_secs: 2_592_000,
            certs_leaf_max_ttl_secs: 2_592_000,
            certs_root_common_name: "Atom Root CA".into(),
            certs_intermediate_common_name: "Atom Intermediate CA".into(),
        }
    }

    #[test]
    fn encrypts_and_decrypts_private_key() {
        let cfg = config("01234567890123456789012345678901");
        let encrypted = encrypt_private_key(&cfg, b"private-key").expect("encrypt");
        let decrypted =
            decrypt_private_key(&cfg, &encrypted.nonce, &encrypted.ciphertext).expect("decrypt");
        assert_eq!(decrypted, b"private-key");
    }

    #[test]
    fn wrong_secret_cannot_decrypt_private_key() {
        let cfg = config("01234567890123456789012345678901");
        let other = config("abcdef0123456789abcdef0123456789");
        let encrypted = encrypt_private_key(&cfg, b"private-key").expect("encrypt");
        let err = decrypt_private_key(&other, &encrypted.nonce, &encrypted.ciphertext)
            .expect_err("wrong secret must fail");
        assert!(err.to_string().contains("internal error"));
    }
}
