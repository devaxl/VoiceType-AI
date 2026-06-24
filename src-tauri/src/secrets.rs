use crate::error::{AppError, Result};

const SERVICE: &str = "com.devaxl.voicetype";
const ACCOUNT: &str = "openai-api-key";

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn store_api_key(key: &str) -> Result<()> {
    entry()?
        .set_password(key)
        .map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn get_api_key() -> Result<String> {
    match entry()?.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => Err(AppError::MissingApiKey),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn has_api_key() -> bool {
    get_api_key().is_ok()
}

#[allow(dead_code)]
pub fn delete_api_key() -> Result<()> {
    entry()?
        .delete_credential()
        .map_err(|e| AppError::Keyring(e.to_string()))
}
