//! Alias slug validation.
//!
//! Aliases are human-friendly, case-folded handles over UUIDs (tenant domains,
//! resource channels, entity clients). They stay an alias, never a replacement:
//! the UUID remains the canonical identity. A valid alias is a lowercase slug
//! `[a-z0-9][a-z0-9-]{0,62}` that does not end in `-` and is not UUID-shaped, so
//! alias-addressing and id-addressing can never collide.

use crate::error::AppError;

/// Maximum slug length (DNS-label-like).
pub const MAX_ALIAS_LEN: usize = 63;

/// Which addressable table an alias resolves against. Aliases are unique per
/// `(tenant, table)`, so resolution must say whether it wants an entity
/// (client/device) or a resource (channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasObjectClass {
    Entity,
    Resource,
}

/// Validate and normalize a single alias slug. Input is trimmed and lowercased
/// before validation; the normalized form is returned.
pub fn validate_alias(input: &str) -> Result<String, AppError> {
    let alias = input.trim().to_ascii_lowercase();

    if alias.is_empty() {
        return Err(AppError::bad_request("alias must not be empty"));
    }
    if alias.len() > MAX_ALIAS_LEN {
        return Err(AppError::bad_request(format!(
            "alias must be at most {MAX_ALIAS_LEN} characters"
        )));
    }
    if is_uuid_shaped(&alias) {
        return Err(AppError::bad_request(
            "alias must not be UUID-shaped (use the id directly to address by UUID)",
        ));
    }
    if !alias
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AppError::bad_request(
            "alias may contain only lowercase letters, digits, and '-'",
        ));
    }
    if !alias.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err(AppError::bad_request(
            "alias must start with a letter or digit",
        ));
    }
    if alias.ends_with('-') {
        return Err(AppError::bad_request("alias must not end with '-'"));
    }

    Ok(alias)
}

/// Validate an optional alias. Empty/whitespace is treated as absent (`None`);
/// any non-empty value must be a valid slug.
pub fn validate_alias_opt(alias: Option<String>) -> Result<Option<String>, AppError> {
    match alias {
        Some(a) if a.trim().is_empty() => Ok(None),
        Some(a) => validate_alias(&a).map(Some),
        None => Ok(None),
    }
}

fn is_uuid_shaped(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_slugs() {
        assert_eq!(validate_alias("watermeters").unwrap(), "watermeters");
        assert_eq!(validate_alias("Water-Meters").unwrap(), "water-meters");
        assert_eq!(validate_alias("  ultraviolet  ").unwrap(), "ultraviolet");
        assert_eq!(validate_alias("a1-b2-c3").unwrap(), "a1-b2-c3");
    }

    #[test]
    fn rejects_invalid_slugs() {
        assert!(validate_alias("").is_err());
        assert!(validate_alias("-leading").is_err());
        assert!(validate_alias("trailing-").is_err());
        assert!(validate_alias("has space").is_err());
        assert!(validate_alias("emoji🦀").is_err());
        assert!(validate_alias(&"x".repeat(64)).is_err());
    }

    #[test]
    fn rejects_uuid_shaped() {
        assert!(validate_alias("465358f9-07f4-4ea0-8cbb-2abc654442bd").is_err());
    }

    #[test]
    fn optional_treats_blank_as_absent() {
        assert_eq!(validate_alias_opt(None).unwrap(), None);
        assert_eq!(validate_alias_opt(Some("   ".into())).unwrap(), None);
        assert_eq!(
            validate_alias_opt(Some("chan".into())).unwrap(),
            Some("chan".to_string())
        );
    }
}
