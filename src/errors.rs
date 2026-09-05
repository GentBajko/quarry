//! The error type and the exit-code contract.

use std::fmt;

pub(crate) type Result<T> = std::result::Result<T, QuarryError>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum QuarryError {
    Refusal(String),
    External(String),
}

impl fmt::Display for QuarryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refusal(m) | Self::External(m) => f.write_str(m),
        }
    }
}

impl QuarryError {
    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            Self::Refusal(_) => 1,
            Self::External(_) => 2,
        }
    }

    pub(crate) fn refusal(message: impl Into<String>) -> Self {
        Self::Refusal(message.into())
    }

    pub(crate) fn external(message: impl Into<String>) -> Self {
        Self::External(message.into())
    }
}

impl From<std::io::Error> for QuarryError {
    fn from(e: std::io::Error) -> Self {
        Self::External(e.to_string())
    }
}

impl From<rusqlite::Error> for QuarryError {
    fn from(e: rusqlite::Error) -> Self {
        Self::External(format!("index: {e}"))
    }
}

impl From<serde_json::Error> for QuarryError {
    fn from(e: serde_json::Error) -> Self {
        Self::External(format!("json: {e}"))
    }
}
