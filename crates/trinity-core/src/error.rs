use std::error::Error;
use std::fmt;

/// Domain invariant violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidButton(u8),
    EmptyProfileName,
    EmptyNode,
    DuplicateModifier,
    UnknownKeyName(String),
    IncompleteCalibration {
        missing: Vec<u8>,
    },
    PhysicalCodeConflict {
        node: String,
        code: u16,
        first: u8,
        second: u8,
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidButton(n) => {
                write!(f, "invalid side button number: {n} (expected 1..=12)")
            }
            Self::EmptyProfileName => write!(f, "profile name must not be empty"),
            Self::EmptyNode => write!(f, "node id must not be empty"),
            Self::DuplicateModifier => {
                write!(f, "duplicate modifier in key combination")
            }
            Self::UnknownKeyName(name) => write!(f, "unknown key name: {name}"),
            Self::IncompleteCalibration { missing } => {
                write!(f, "incomplete calibration, missing buttons: {missing:?}")
            }
            Self::PhysicalCodeConflict {
                node,
                code,
                first,
                second,
            } => {
                write!(
                    f,
                    "physical code {code} on node '{node}' captured for both button {first} and button {second}"
                )
            }
        }
    }
}

impl Error for DomainError {}
