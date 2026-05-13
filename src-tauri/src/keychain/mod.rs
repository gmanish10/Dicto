use serde::{Deserialize, Serialize};

const SERVICE: &str = "com.dicto.app";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKey {
    Groq,
    Openai,
    Anthropic,
}

impl ApiKey {
    pub fn all() -> Vec<Self> {
        vec![Self::Groq, Self::Openai, Self::Anthropic]
    }

    fn account(self) -> &'static str {
        match self {
            ApiKey::Groq => "groq_api_key",
            ApiKey::Openai => "openai_api_key",
            ApiKey::Anthropic => "anthropic_api_key",
        }
    }
}

fn entry(key: ApiKey) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, key.account())
}

pub fn get(key: ApiKey) -> Option<String> {
    entry(key).ok().and_then(|e| e.get_password().ok())
}

pub fn set(key: ApiKey, value: &str) -> anyhow::Result<()> {
    let e = entry(key)?;
    e.set_password(value)?;
    Ok(())
}

pub fn delete(key: ApiKey) -> anyhow::Result<()> {
    let e = entry(key)?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(other) => Err(other.into()),
    }
}

pub fn exists(key: ApiKey) -> bool {
    get(key).is_some()
}
