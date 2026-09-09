use std::error::Error;
use std::fmt;

use trinity_core::error::DomainError;

/// Use case or adapter runtime error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Domain(DomainError),
    Port(String),
    NotFound(String),
    InvalidState(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(err) => write!(f, "domain error: {err}"),
            Self::Port(message) => write!(f, "adapter error: {message}"),
            Self::NotFound(what) => write!(f, "not found: {what}"),
            Self::InvalidState(message) => write!(f, "invalid state: {message}"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Domain(err) => Some(err),
            _ => None,
        }
    }
}

impl From<DomainError> for AppError {
    fn from(err: DomainError) -> Self {
        Self::Domain(err)
    }
}
