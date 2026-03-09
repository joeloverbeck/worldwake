//! Shared numeric newtypes for the Worldwake simulation.
//!
//! These types enforce semantic meaning and prevent ad hoc use of raw
//! primitives across the codebase. No floating-point types are used.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Add;

/// Container capacity accounting units.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct LoadUnits(pub u32);

impl fmt::Display for LoadUnits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}lu", self.0)
    }
}

/// Fixed-point value in the range `0..=1000` (per-mille).
///
/// Constructors validate the range. Use `new` for runtime values and
/// `new_unchecked` for compile-time constants where the value is known valid.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct Permille(u16);

impl Permille {
    /// Create a new `Permille` value, returning an error if out of range.
    pub fn new(value: u16) -> Result<Self, &'static str> {
        if value > 1000 {
            Err("Permille value must be in 0..=1000")
        } else {
            Ok(Self(value))
        }
    }

    /// Create a `Permille` without validation. Use only for compile-time constants.
    ///
    /// # Safety (logical)
    /// Caller must ensure `value <= 1000`.
    pub const fn new_unchecked(value: u16) -> Self {
        // We still assert in debug builds.
        assert!(value <= 1000, "Permille value must be in 0..=1000");
        Self(value)
    }

    /// Returns the inner value.
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Permille {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}‰", self.0)
    }
}

/// Conserved lot count with semantic wrapper.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct Quantity(pub u32);

impl Quantity {
    /// Checked subtraction — returns `None` if `rhs > self`.
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl Add for Quantity {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "×{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Permille ---

    #[test]
    fn permille_valid_values() {
        assert!(Permille::new(0).is_ok());
        assert!(Permille::new(500).is_ok());
        assert!(Permille::new(1000).is_ok());
    }

    #[test]
    fn permille_rejects_out_of_range() {
        assert!(Permille::new(1001).is_err());
        assert!(Permille::new(u16::MAX).is_err());
    }

    #[test]
    fn permille_unchecked_compile_time() {
        const P: Permille = Permille::new_unchecked(500);
        assert_eq!(P.value(), 500);
    }

    // --- Quantity ---

    #[test]
    fn quantity_addition() {
        assert_eq!(Quantity(5) + Quantity(3), Quantity(8));
    }

    #[test]
    fn quantity_checked_subtraction_success() {
        assert_eq!(Quantity(8).checked_sub(Quantity(3)), Some(Quantity(5)));
    }

    #[test]
    fn quantity_checked_subtraction_underflow() {
        assert_eq!(Quantity(3).checked_sub(Quantity(5)), None);
    }

    // --- Bincode round-trips ---

    #[test]
    fn load_units_bincode_roundtrip() {
        let val = LoadUnits(42);
        let bytes = bincode::serialize(&val).unwrap();
        let back: LoadUnits = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn permille_bincode_roundtrip() {
        let val = Permille::new(750).unwrap();
        let bytes = bincode::serialize(&val).unwrap();
        let back: Permille = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn quantity_bincode_roundtrip() {
        let val = Quantity(100);
        let bytes = bincode::serialize(&val).unwrap();
        let back: Quantity = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    // --- Trait bound assertions ---

    fn assert_numeric_bounds<
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + std::fmt::Display
            + Serialize
            + serde::de::DeserializeOwned,
    >() {
    }

    #[test]
    fn numeric_types_satisfy_required_traits() {
        assert_numeric_bounds::<LoadUnits>();
        assert_numeric_bounds::<Permille>();
        assert_numeric_bounds::<Quantity>();
    }
}
