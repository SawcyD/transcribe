use crate::{brand::CREDENTIAL_SERVICE, errors::AppError};

#[derive(Debug, Clone, Copy)]
pub enum CredentialKind {
    Deepgram,
    Cleanup,
}

impl CredentialKind {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "deepgram" => Ok(Self::Deepgram),
            "cleanup" => Ok(Self::Cleanup),
            _ => Err(AppError::Configuration(
                "unknown credential provider".into(),
            )),
        }
    }
    fn account(self) -> &'static str {
        match self {
            Self::Deepgram => "deepgram-api-key",
            Self::Cleanup => "cleanup-api-key",
        }
    }
}

pub fn set(kind: CredentialKind, secret: &str) -> Result<(), AppError> {
    if secret.trim().len() < 8 || secret.len() > 4_096 || secret.contains(['\r', '\n', '\0']) {
        return Err(AppError::Credential(
            "credential has an invalid length or contains forbidden characters".into(),
        ));
    }
    entry(kind)?
        .set_password(secret.trim())
        .map_err(|error| AppError::Credential(error.to_string()))
}

pub fn get(kind: CredentialKind) -> Result<String, AppError> {
    entry(kind)?.get_password().map_err(|error| {
        AppError::Credential(match error {
            keyring::Error::NoEntry => format!("{} is not configured", kind.account()),
            other => other.to_string(),
        })
    })
}

pub fn exists(kind: CredentialKind) -> bool {
    entry(kind)
        .and_then(|value| {
            value
                .get_password()
                .map_err(|error| AppError::Credential(error.to_string()))
        })
        .is_ok()
}

pub fn delete(kind: CredentialKind) -> Result<(), AppError> {
    match entry(kind)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::Credential(error.to_string())),
    }
}

fn entry(kind: CredentialKind) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(CREDENTIAL_SERVICE, kind.account())
        .map_err(|error| AppError::Credential(error.to_string()))
}
