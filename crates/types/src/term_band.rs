//! `TermBand` — rate-sheet term classification used for rate/price lookup.
//!
//! Rate sheets price by band, but the engine analyses every individual month
//! within the band. The band determines *which rate and price row to read*;
//! the individual `TermMonths` value determines *what the monthly payment and
//! amortisation schedule look like* at that specific term.
//!
//! # User-specified boundaries
//!
//! Bands are contiguous — each starts the month immediately after the
//! previous band ends, so no term is unclassified and no term belongs to two
//! bands:
//!
//! | Variant            | Months    | Label on rate sheet          |
//! |--------------------|-----------|------------------------------|
//! | `Band8To10`        | 96–120    | "8-10 YEAR"                  |
//! | `Band11To15`       | 121–180   | "10 Year 1 Month–15 YEAR"    |
//! | `Band16To20`       | 181–240   | "15 Year 1 Month–20 YEAR"    |
//! | `Band21To30`       | 241–360   | "20 Year 1 Month–30 YEAR"    |
//! | `GovtBand8To15`    | 96–180    | "8-15 YEAR" (Govt)           |
//! | `GovtBand16To30`   | 181–360   | "15 Year 1 Month–30 YEAR"    |
//! | `Usda30Only`       | 360       | "30 YEAR" (USDA only)        |

use serde::{Deserialize, Serialize};

use crate::term_months::TermMonths;

/// Rate-sheet term band. Used as the lookup key for the rate and price rows
/// on a lender's rate sheet. The band is *not* the same as the loan term —
/// every month within the band may produce a different payment and yield.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermBand {
    // ── Conventional bands (contiguous, no gaps 96–360) ──────────────────
    /// 96–120 months ("8-10 YEAR")
    Band8To10,
    /// 121–180 months ("10 Year 1 Month–15 YEAR")
    Band11To15,
    /// 181–240 months ("15 Year 1 Month–20 YEAR")
    Band16To20,
    /// 241–360 months ("20 Year 1 Month–30 YEAR")
    Band21To30,

    // ── Government broader bands ──────────────────────────────────────────
    /// 96–180 months — FHA, VA use a single band for the 8-15 year range.
    GovtBand8To15,
    /// 181–360 months — FHA, VA use a single band for the 15 year 1 month
    /// through 30 year range.
    GovtBand16To30,

    // ── USDA ──────────────────────────────────────────────────────────────
    /// 360 months only — USDA Section 502 is only offered as a 30-year fixed.
    Usda30Only,
}

impl TermBand {
    /// Inclusive `(low, high)` month range for this band.
    #[must_use]
    pub const fn range(self) -> (u16, u16) {
        match self {
            Self::Band8To10 => (96, 120),
            Self::Band11To15 => (121, 180),
            Self::Band16To20 => (181, 240),
            Self::Band21To30 => (241, 360),
            Self::GovtBand8To15 => (96, 180),
            Self::GovtBand16To30 => (181, 360),
            Self::Usda30Only => (360, 360),
        }
    }

    /// True if `term` falls within this band (inclusive on both ends).
    #[must_use]
    pub fn contains(self, term: TermMonths) -> bool {
        let (lo, hi) = self.range();
        (lo..=hi).contains(&term.0)
    }

    /// Iterator over every `TermMonths` value within this band, low-to-high.
    /// This is the set of individual terms the engine must analyse when pricing
    /// a scenario against this band.
    pub fn all_months(self) -> impl Iterator<Item = TermMonths> {
        let (lo, hi) = self.range();
        (lo..=hi).map(TermMonths)
    }

    /// Number of individual month-terms within this band.
    #[must_use]
    pub const fn month_count(self) -> u16 {
        let (lo, hi) = self.range();
        hi - lo + 1
    }

    /// The rate-sheet label as it typically appears on a UWM-style price grid.
    #[must_use]
    pub const fn rate_sheet_label(self) -> &'static str {
        match self {
            Self::Band8To10 => "8-10 YEAR",
            Self::Band11To15 => "10 Year 1 Month-15 YEAR",
            Self::Band16To20 => "15 Year 1 Month-20 YEAR",
            Self::Band21To30 => "20 Year 1 Month-30 YEAR",
            Self::GovtBand8To15 => "8-15 YEAR",
            Self::GovtBand16To30 => "15 Year 1 Month-30 YEAR",
            Self::Usda30Only => "30 YEAR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_band_ranges_are_correct() {
        assert_eq!(TermBand::Band8To10.range(), (96, 120));
        assert_eq!(TermBand::Band11To15.range(), (121, 180));
        assert_eq!(TermBand::Band16To20.range(), (181, 240));
        assert_eq!(TermBand::Band21To30.range(), (241, 360));
        assert_eq!(TermBand::GovtBand8To15.range(), (96, 180));
        assert_eq!(TermBand::GovtBand16To30.range(), (181, 360));
        assert_eq!(TermBand::Usda30Only.range(), (360, 360));
    }

    #[test]
    fn test_conv_bands_are_contiguous() {
        // Each conventional band starts the month after the previous ends
        let (_, hi8) = TermBand::Band8To10.range();
        let (lo11, hi11) = TermBand::Band11To15.range();
        let (lo16, hi16) = TermBand::Band16To20.range();
        let (lo21, _) = TermBand::Band21To30.range();
        assert_eq!(
            hi8 + 1,
            lo11,
            "Band11To15 must start one month after Band8To10 ends"
        );
        assert_eq!(
            hi11 + 1,
            lo16,
            "Band16To20 must start one month after Band11To15 ends"
        );
        assert_eq!(
            hi16 + 1,
            lo21,
            "Band21To30 must start one month after Band16To20 ends"
        );
    }

    #[test]
    fn test_govt_bands_are_contiguous() {
        let (_, hi8) = TermBand::GovtBand8To15.range();
        let (lo16, _) = TermBand::GovtBand16To30.range();
        assert_eq!(
            hi8 + 1,
            lo16,
            "GovtBand16To30 must start one month after GovtBand8To15 ends"
        );
    }

    #[test]
    fn test_band_contains() {
        // Band boundaries
        assert!(TermBand::Band8To10.contains(TermMonths(96)));
        assert!(TermBand::Band8To10.contains(TermMonths(120)));
        assert!(!TermBand::Band8To10.contains(TermMonths(121)));

        assert!(TermBand::Band11To15.contains(TermMonths(121)));
        assert!(TermBand::Band11To15.contains(TermMonths(180)));
        assert!(!TermBand::Band11To15.contains(TermMonths(120)));
        assert!(!TermBand::Band11To15.contains(TermMonths(181)));

        assert!(TermBand::Band21To30.contains(TermMonths(241)));
        assert!(TermBand::Band21To30.contains(TermMonths(360)));
        assert!(!TermBand::Band21To30.contains(TermMonths(240)));
        assert!(!TermBand::Band21To30.contains(TermMonths(361)));
    }

    #[test]
    fn test_band_month_count() {
        assert_eq!(TermBand::Band8To10.month_count(), 25); // 96–120
        assert_eq!(TermBand::Band11To15.month_count(), 60); // 121–180
        assert_eq!(TermBand::Band16To20.month_count(), 60); // 181–240
        assert_eq!(TermBand::Band21To30.month_count(), 120); // 241–360
        assert_eq!(TermBand::GovtBand8To15.month_count(), 85); // 96–180
        assert_eq!(TermBand::GovtBand16To30.month_count(), 180); // 181–360
        assert_eq!(TermBand::Usda30Only.month_count(), 1);
    }

    #[test]
    fn test_band_all_months_yields_correct_count() {
        assert_eq!(TermBand::Band8To10.all_months().count(), 25);
        assert_eq!(TermBand::Band11To15.all_months().count(), 60);
        assert_eq!(TermBand::Band16To20.all_months().count(), 60);
        assert_eq!(TermBand::Band21To30.all_months().count(), 120);
        assert_eq!(TermBand::Usda30Only.all_months().count(), 1);
    }

    #[test]
    fn test_band_all_months_boundaries() {
        let mut conv8: Vec<_> = TermBand::Band8To10.all_months().collect();
        assert_eq!(conv8.first(), Some(&TermMonths(96)));
        assert_eq!(conv8.last(), Some(&TermMonths(120)));

        let band21: Vec<_> = TermBand::Band21To30.all_months().collect();
        assert_eq!(band21.first(), Some(&TermMonths(241)));
        assert_eq!(band21.last(), Some(&TermMonths(360)));

        // Must be ascending
        conv8.sort_unstable_by_key(|t| t.0);
        assert_eq!(conv8, TermBand::Band8To10.all_months().collect::<Vec<_>>());
    }

    #[test]
    fn test_term_band_serde_json() {
        let b = TermBand::Band21To30;
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "\"band21_to30\"");
        let back: TermBand = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }
}
