use std::fmt;

use crate::error::DomainError;

/// Stable identifier of an input node (`/dev/input/by-id` name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        if id.trim().is_empty() {
            Err(DomainError::EmptyNode)
        } else {
            Ok(Self(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_ids() {
        assert_eq!(NodeId::new("").unwrap_err(), DomainError::EmptyNode);
        assert_eq!(NodeId::new("   ").unwrap_err(), DomainError::EmptyNode);
    }

    #[test]
    fn keeps_value() {
        let node = NodeId::new("usb-Razer_Razer_Naga_Trinity-event-if01").unwrap();
        assert_eq!(node.as_str(), "usb-Razer_Razer_Naga_Trinity-event-if01");
    }
}
