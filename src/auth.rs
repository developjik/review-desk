use crate::domain::{Result, ReviewDeskError};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    GitHub,
    ChatGpt,
}

impl TokenKind {
    fn account_name(self) -> &'static str {
        match self {
            Self::GitHub => "github-oauth",
            Self::ChatGpt => "chatgpt-oauth",
        }
    }
}

pub trait TokenStore {
    fn save(&mut self, kind: TokenKind, token: &str) -> Result<()>;
    fn load(&self, kind: TokenKind) -> Result<Option<String>>;
    fn delete(&mut self, kind: TokenKind) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct MemoryTokenStore {
    tokens: HashMap<TokenKind, String>,
}

impl TokenStore for MemoryTokenStore {
    fn save(&mut self, kind: TokenKind, token: &str) -> Result<()> {
        self.tokens.insert(kind, token.to_string());
        Ok(())
    }

    fn load(&self, kind: TokenKind) -> Result<Option<String>> {
        Ok(self.tokens.get(&kind).cloned())
    }

    fn delete(&mut self, kind: TokenKind) -> Result<()> {
        self.tokens.remove(&kind);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KeyringTokenStore {
    service: String,
}

impl KeyringTokenStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, kind: TokenKind) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, kind.account_name())
            .map_err(|error| ReviewDeskError::Keychain(error.to_string()))
    }
}

impl Default for KeyringTokenStore {
    fn default() -> Self {
        Self::new("reviewdesk")
    }
}

impl TokenStore for KeyringTokenStore {
    fn save(&mut self, kind: TokenKind, token: &str) -> Result<()> {
        self.entry(kind)?
            .set_password(token)
            .map_err(|error| ReviewDeskError::Keychain(error.to_string()))
    }

    fn load(&self, kind: TokenKind) -> Result<Option<String>> {
        match self.entry(kind)?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(ReviewDeskError::Keychain(error.to_string())),
        }
    }

    fn delete(&mut self, kind: TokenKind) -> Result<()> {
        match self.entry(kind)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(ReviewDeskError::Keychain(error.to_string())),
        }
    }
}
