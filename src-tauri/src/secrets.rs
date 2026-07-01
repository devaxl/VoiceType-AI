use crate::config::Provider;
use crate::error::{AppError, Result};

const SERVICE: &str = "com.devaxl.voicetype";

/// Keychain account (per provider). OpenAI keeps its original account name so keys stored by
/// pre-multi-provider versions keep working after an upgrade.
fn account(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAI => "openai-api-key",
        Provider::Anthropic => "anthropic-api-key",
        Provider::Groq => "groq-api-key",
    }
}

fn entry(provider: Provider) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, account(provider)).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn store_api_key(provider: Provider, key: &str) -> Result<()> {
    entry(provider)?
        .set_password(key)
        .map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn get_api_key(provider: Provider) -> Result<String> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => Err(AppError::MissingApiKey(provider.label())),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn has_api_key(provider: Provider) -> bool {
    get_api_key(provider).is_ok()
}

#[allow(dead_code)]
pub fn delete_api_key(provider: Provider) -> Result<()> {
    entry(provider)?
        .delete_credential()
        .map_err(|e| AppError::Keyring(e.to_string()))
}
