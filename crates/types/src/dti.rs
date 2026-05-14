//! `DtiBasisPoints` — debt-to-income ratio in true basis points.
//!
//! **Encoding**: one stored unit equals **0.01%**. So `43.00%` is stored as
//! `DtiBasisPoints(4300)` and `50.00%` is stored as `DtiBasisPoints(5000)`.
//!
//! Unlike LTV there is no hard upper cap — Temporary GSE QM loans with strong
//! compensating factors can legitimately approve at DTI ratios above 50%.
//! Instead, the validating constructor **always succeeds** but emits a
//! `tracing::debug!` log when the value exceeds 60.00% (`DtiBasisPoints(6000)`)
//! so anomalous values are visible in structured logs without breaking
//! valid-but-uncommon scenarios.

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Debt-to-income ratio stored at 2-decimal-place precision.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DtiBasisPoints(pub u32);

impl DtiBasisPoints {
    /// Zero DTI.
    pub const ZERO: Self = DtiBasisPoints(0);

    /// 60.00% — the threshold above which a debug log is emitted on
    /// construction. 60% DTI is the practical ceiling for most QM products;
    /// values above are accepted but flagged for downstream review.
    pub const TYPICAL_MAX: Self = DtiBasisPoints(6000);

    /// Construct, emitting a `tracing::debug!` log if the value exceeds the
    /// typical 60% threshold. **Never fails** — DTI values above 60% are
    /// legitimate for Temporary GSE QM loans with compensating factors.
    #[must_use]
    pub fn new(value: u32) -> Self {
        if value > Self::TYPICAL_MAX.0 {
            tracing::debug!(
                dti_bps = value,
                threshold_bps = Self::TYPICAL_MAX.0,
                "DTI exceeds typical 60% threshold; verify QM eligibility and compensating factors"
            );
        }
        DtiBasisPoints(value)
    }

    /// Convert to a `Decimal` percentage. `DtiBasisPoints(4300)` returns `43.00`.
    #[must_use]
    pub fn to_decimal_percent(self) -> Decimal {
        Decimal::new(i64::from(self.0), 2)
    }

    /// True if this value exceeds the typical 60% maximum.
    #[must_use]
    pub const fn exceeds_typical_max(self) -> bool {
        self.0 > Self::TYPICAL_MAX.0
    }
}

impl fmt::Display for DtiBasisPoints {
    /// Format as `"43.00%"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / 100;
        let frac = self.0 % 100;
        write!(f, "{whole}.{frac:02}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_dti_no_upper_cap_but_warn_above_six_thousand() {
        // Normal values: accepted silently
        assert_eq!(DtiBasisPoints::new(4300).0, 4300); // 43.00%
        assert_eq!(DtiBasisPoints::new(5000).0, 5000); // 50.00%
        assert_eq!(DtiBasisPoints::new(5500).0, 5500); // 55.00%
        assert_eq!(DtiBasisPoints::new(6000).0, 6000); // 60.00% (at threshold)

        // Above threshold: still accepted (no panic, no Err), but a debug log
        // is emitted. Verifying the log content requires a tracing subscriber;
        // here we just confirm construction succeeds and the value is preserved.
        assert_eq!(DtiBasisPoints::new(6001).0, 6001);
        assert_eq!(DtiBasisPoints::new(6500).0, 6500); // 65.00%
        assert_eq!(DtiBasisPoints::new(7000).0, 7000); // 70.00%
        assert_eq!(DtiBasisPoints::new(8000).0, 8000); // 80.00%
    }

    #[test]
    fn test_dti_exceeds_typical_max_predicate() {
        assert!(!DtiBasisPoints::new(4300).exceeds_typical_max());
        assert!(!DtiBasisPoints::new(6000).exceeds_typical_max());
        assert!(DtiBasisPoints::new(6001).exceeds_typical_max());
        assert!(DtiBasisPoints::new(8000).exceeds_typical_max());
    }

    #[test]
    fn test_dti_to_decimal_percent() {
        assert_eq!(DtiBasisPoints::new(4300).to_decimal_percent(), dec!(43.00));
        assert_eq!(DtiBasisPoints::new(5000).to_decimal_percent(), dec!(50.00));
        assert_eq!(DtiBasisPoints::new(0).to_decimal_percent(), dec!(0.00));
    }

    #[test]
    fn test_dti_display() {
        assert_eq!(DtiBasisPoints::new(4300).to_string(), "43.00%");
        assert_eq!(DtiBasisPoints::new(5000).to_string(), "50.00%");
        assert_eq!(DtiBasisPoints::new(0).to_string(), "0.00%");
        assert_eq!(DtiBasisPoints::new(6500).to_string(), "65.00%");
        assert_eq!(DtiBasisPoints::new(99).to_string(), "0.99%");
    }

    #[test]
    fn test_dti_serde_json() {
        let d = DtiBasisPoints::new(4300);
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "4300");
        let back: DtiBasisPoints = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn test_dti_constants() {
        assert_eq!(DtiBasisPoints::ZERO, DtiBasisPoints(0));
        assert_eq!(DtiBasisPoints::TYPICAL_MAX, DtiBasisPoints(6000));
    }

    #[test]
    fn test_dti_repr_transparent() {
        assert_eq!(size_of::<DtiBasisPoints>(), size_of::<u32>());
    }

    #[test]
    fn test_dti_ordering() {
        let mut v = vec![
            DtiBasisPoints::new(5000),
            DtiBasisPoints::new(3500),
            DtiBasisPoints::new(4300),
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                DtiBasisPoints::new(3500),
                DtiBasisPoints::new(4300),
                DtiBasisPoints::new(5000),
            ]
        );
    }
}
