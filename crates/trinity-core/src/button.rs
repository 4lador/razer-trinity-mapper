use std::fmt;

use crate::error::DomainError;

/// Side button of the Trinity 12-button plate, 1..=12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Button(u8);

impl Button {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 12;

    /// The 12 side buttons, in order.
    pub const ALL: [Button; 12] = [
        Button(1),
        Button(2),
        Button(3),
        Button(4),
        Button(5),
        Button(6),
        Button(7),
        Button(8),
        Button(9),
        Button(10),
        Button(11),
        Button(12),
    ];

    pub const fn new(number: u8) -> Result<Self, DomainError> {
        if number >= Self::MIN && number <= Self::MAX {
            Ok(Self(number))
        } else {
            Err(DomainError::InvalidButton(number))
        }
    }

    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "button {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_1_to_12() {
        for n in 1..=12u8 {
            assert_eq!(Button::new(n).unwrap().number(), n);
        }
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(Button::new(0).unwrap_err(), DomainError::InvalidButton(0));
        assert_eq!(Button::new(13).unwrap_err(), DomainError::InvalidButton(13));
        assert_eq!(
            Button::new(255).unwrap_err(),
            DomainError::InvalidButton(255)
        );
    }

    #[test]
    fn all_covers_every_button_exactly_once() {
        let mut numbers: Vec<u8> = Button::ALL.iter().map(|b| b.number()).collect();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers, (1..=12).collect::<Vec<_>>());
    }
}
