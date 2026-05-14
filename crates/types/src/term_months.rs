//! `TermMonths` — actual loan term in months, validated to 120..=360.
//!
//! `TermBand` is the rate-sheet lookup key; `TermMonths` is the specific
//! term the engine is pricing. A 360-month loan and a 350-month loan both
//! fall in `Band21To30` and use the same rate, but produce different monthly
//! payments, different total interest, and different APRs. The engine
//! analyses every `TermMonths` within a band independently.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;
use crate::term_band::TermBand;

/// Actual loan term in months. Valid range: 120 (10-year) to 360 (30-year),
/// inclusive. Stored as `pub u16` so it can be constructed directly in
/// trusted contexts (e.g., computed inner loops); use [`Self::new`] for
/// untrusted input.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct TermMonths(pub u16);

impl TermMonths {
    /// Minimum valid term: 10 years (120 months).
    pub const MIN: Self = TermMonths(120);

    /// Maximum valid term: 30 years (360 months).
    pub const MAX: Self = TermMonths(360);

    /// Standard 15-year fixed term sentinel.
    pub const FIFTEEN_YEAR: Self = TermMonths(180);

    /// Standard 30-year fixed term sentinel.
    pub const THIRTY_YEAR: Self = TermMonths(360);

    /// Validating constructor. Returns `Err(ParseError::TermMonthsOutOfRange)`
    /// if `months` is outside `120..=360`.
    pub fn new(months: u16) -> Result<Self, ParseError> {
        if !(120..=360).contains(&months) {
            return Err(ParseError::TermMonthsOutOfRange(months));
        }
        Ok(TermMonths(months))
    }

    /// Construct from a whole number of years. Equivalent to `new(years * 12)`.
    /// Returns `None` if the resulting month count is outside `120..=360`.
    #[must_use]
    pub fn from_years(years: u16) -> Option<Self> {
        let months = years.checked_mul(12)?;
        Self::new(months).ok()
    }

    /// Whole years, truncating any remainder. A 350-month term returns 29.
    #[must_use]
    pub const fn years_floor(self) -> u16 {
        self.0 / 12
    }

    /// Remaining months after subtracting whole years.
    #[must_use]
    pub const fn months_remainder(self) -> u16 {
        self.0 % 12
    }

    /// True if this term is an exact multiple of 12 (whole years).
    #[must_use]
    pub const fn is_whole_year(self) -> bool {
        self.0 % 12 == 0
    }

    /// Conventional rate-sheet band for this term. Returns `None` if the
    /// term is outside the conventional band range (96–360). For all valid
    /// `TermMonths` (120–360) this always returns `Some`.
    #[must_use]
    pub fn band_for_conv(self) -> Option<TermBand> {
        match self.0 {
            96..=120 => Some(TermBand::Band8To10),
            121..=180 => Some(TermBand::Band11To15),
            181..=240 => Some(TermBand::Band16To20),
            241..=360 => Some(TermBand::Band21To30),
            _ => None,
        }
    }

    /// Government (FHA / VA) rate-sheet band for this term. Returns `None`
    /// if outside 96–360. For all valid `TermMonths` (120–360) this always
    /// returns `Some`.
    #[must_use]
    pub fn band_for_govt(self) -> Option<TermBand> {
        match self.0 {
            96..=180 => Some(TermBand::GovtBand8To15),
            181..=360 => Some(TermBand::GovtBand16To30),
            _ => None,
        }
    }

    /// USDA band for this term. Only 360 months qualifies.
    #[must_use]
    pub fn band_for_usda(self) -> Option<TermBand> {
        if self.0 == 360 {
            Some(TermBand::Usda30Only)
        } else {
            None
        }
    }

    /// Iterate over every valid `TermMonths` from `MIN` (120) to `MAX` (360).
    /// The engine calls this to enumerate every term it must price within a
    /// band. Total: 241 terms.
    pub fn all_valid() -> impl Iterator<Item = TermMonths> {
        (Self::MIN.0..=Self::MAX.0).map(TermMonths)
    }
}

impl fmt::Display for TermMonths {
    /// Display as `"360"` (bare month count). Callers that need "30-year"
    /// labels can use `years_floor()` / `months_remainder()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_months_new_valid_range() {
        assert!(TermMonths::new(120).is_ok());
        assert!(TermMonths::new(180).is_ok());
        assert!(TermMonths::new(360).is_ok());
        assert!(TermMonths::new(241).is_ok());
    }

    #[test]
    fn test_term_months_new_invalid_range() {
        assert!(matches!(
            TermMonths::new(119),
            Err(ParseError::TermMonthsOutOfRange(119))
        ));
        assert!(matches!(
            TermMonths::new(361),
            Err(ParseError::TermMonthsOutOfRange(361))
        ));
        assert!(matches!(
            TermMonths::new(0),
            Err(ParseError::TermMonthsOutOfRange(0))
        ));
    }

    #[test]
    fn test_term_months_from_years() {
        assert_eq!(TermMonths::from_years(10), Some(TermMonths(120)));
        assert_eq!(TermMonths::from_years(15), Some(TermMonths(180)));
        assert_eq!(TermMonths::from_years(20), Some(TermMonths(240)));
        assert_eq!(TermMonths::from_years(30), Some(TermMonths(360)));
        assert_eq!(TermMonths::from_years(9), None); // 108 < 120
        assert_eq!(TermMonths::from_years(31), None); // 372 > 360
    }

    #[test]
    fn test_term_months_years_floor() {
        assert_eq!(TermMonths(360).years_floor(), 30);
        assert_eq!(TermMonths(180).years_floor(), 15);
        assert_eq!(TermMonths(350).years_floor(), 29);
        assert_eq!(TermMonths(121).years_floor(), 10);
    }

    #[test]
    fn test_term_months_remainder() {
        assert_eq!(TermMonths(360).months_remainder(), 0);
        assert_eq!(TermMonths(121).months_remainder(), 1);
        assert_eq!(TermMonths(181).months_remainder(), 1);
        assert_eq!(TermMonths(241).months_remainder(), 1);
    }

    #[test]
    fn test_term_months_is_whole_year() {
        assert!(TermMonths(360).is_whole_year());
        assert!(TermMonths(180).is_whole_year());
        assert!(!TermMonths(121).is_whole_year());
        assert!(!TermMonths(181).is_whole_year());
    }

    #[test]
    fn test_term_months_band_for_conv_boundaries() {
        // Band8To10: 96–120 (only 120 is reachable via TermMonths::new)
        assert_eq!(TermMonths(120).band_for_conv(), Some(TermBand::Band8To10));
        // First month of Band11To15
        assert_eq!(TermMonths(121).band_for_conv(), Some(TermBand::Band11To15));
        assert_eq!(TermMonths(180).band_for_conv(), Some(TermBand::Band11To15));
        // First month of Band16To20
        assert_eq!(TermMonths(181).band_for_conv(), Some(TermBand::Band16To20));
        assert_eq!(TermMonths(240).band_for_conv(), Some(TermBand::Band16To20));
        // First month of Band21To30
        assert_eq!(TermMonths(241).band_for_conv(), Some(TermBand::Band21To30));
        assert_eq!(TermMonths(360).band_for_conv(), Some(TermBand::Band21To30));
    }

    #[test]
    fn test_term_months_band_for_govt_boundaries() {
        assert_eq!(
            TermMonths(120).band_for_govt(),
            Some(TermBand::GovtBand8To15)
        );
        assert_eq!(
            TermMonths(180).band_for_govt(),
            Some(TermBand::GovtBand8To15)
        );
        assert_eq!(
            TermMonths(181).band_for_govt(),
            Some(TermBand::GovtBand16To30)
        );
        assert_eq!(
            TermMonths(360).band_for_govt(),
            Some(TermBand::GovtBand16To30)
        );
    }

    #[test]
    fn test_term_months_band_for_usda() {
        assert_eq!(TermMonths(360).band_for_usda(), Some(TermBand::Usda30Only));
        assert_eq!(TermMonths(359).band_for_usda(), None);
        assert_eq!(TermMonths(120).band_for_usda(), None);
    }

    #[test]
    fn test_term_months_all_valid_count() {
        assert_eq!(TermMonths::all_valid().count(), 241); // 120..=360
    }

    #[test]
    fn test_term_months_all_valid_bounds() {
        let mut all: Vec<_> = TermMonths::all_valid().collect();
        assert_eq!(all.first(), Some(&TermMonths::MIN));
        assert_eq!(all.last(), Some(&TermMonths::MAX));
        all.sort_unstable_by_key(|t| t.0);
        assert_eq!(all.first(), Some(&TermMonths::MIN));
    }

    #[test]
    fn test_term_months_display() {
        assert_eq!(TermMonths(360).to_string(), "360");
        assert_eq!(TermMonths(180).to_string(), "180");
    }

    #[test]
    fn test_term_months_serde_json() {
        let t = TermMonths(360);
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "360");
        let back: TermMonths = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn test_term_months_repr_transparent() {
        assert_eq!(size_of::<TermMonths>(), size_of::<u16>());
    }
}
