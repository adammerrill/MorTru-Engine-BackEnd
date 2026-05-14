//! `LoanProduct` — specific product/term-band combination offered on a rate sheet.

use serde::{Deserialize, Serialize};

/// A specific product as it appears on a lender rate sheet.
///
/// Each variant encodes both the amortisation type and the term band so
/// the pricing engine can look up exactly which column to use without
/// additional joins.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum LoanProduct {
    // ── Conventional fixed-rate ───────────────────────────────────────────
    /// 8–10 year conventional fixed (96–120 months).
    FixedConv8To10,
    /// 11–15 year conventional fixed (121–180 months).
    FixedConv11To15,
    /// 16–20 year conventional fixed (181–240 months).
    FixedConv16To20,
    /// 21–30 year conventional fixed (241–360 months).
    FixedConv21To30,

    // ── FHA fixed-rate ────────────────────────────────────────────────────
    /// 8–15 year FHA fixed (96–180 months).
    FixedFha8To15,
    /// 16–30 year FHA fixed (181–360 months).
    FixedFha16To30,

    // ── VA fixed-rate ─────────────────────────────────────────────────────
    /// 8–15 year VA fixed (96–180 months).
    FixedVa8To15,
    /// 16–30 year VA fixed (181–360 months).
    FixedVa16To30,

    // ── USDA ──────────────────────────────────────────────────────────────
    /// USDA 30-year fixed (360 months only).
    FixedUsda30,

    // ── Conventional ARMs (SOFR-indexed, 6-month caps) ───────────────────
    Arm5_6Sofr,
    Arm7_6Sofr,
    Arm10_6Sofr,

    // ── Conventional ARMs (legacy 1-year caps) ────────────────────────────
    Arm5_1,
    Arm7_1,
    Arm10_1,

    // ── One-Time-Close construction ───────────────────────────────────────
    OtcConv30,
    OtcConv15,
    OtcVa30,
    OtcVaJumbo30,
}

impl LoanProduct {
    /// Nominal term band in (min_months, max_months) form used for rate-
    /// sheet column lookup. ARM products use the full 30-year amortisation
    /// period; the initial fixed period is encoded in the variant name.
    #[must_use]
    pub const fn term_range_months(self) -> (u16, u16) {
        match self {
            Self::FixedConv8To10 => (96, 120),
            Self::FixedConv11To15 => (121, 180),
            Self::FixedConv16To20 => (181, 240),
            Self::FixedConv21To30 => (241, 360),
            Self::FixedFha8To15 => (96, 180),
            Self::FixedFha16To30 => (181, 360),
            Self::FixedVa8To15 => (96, 180),
            Self::FixedVa16To30 => (181, 360),
            Self::FixedUsda30 => (360, 360),
            Self::Arm5_6Sofr | Self::Arm7_6Sofr | Self::Arm10_6Sofr => (360, 360),
            Self::Arm5_1 | Self::Arm7_1 | Self::Arm10_1 => (360, 360),
            Self::OtcConv30 | Self::OtcVa30 | Self::OtcVaJumbo30 => (360, 360),
            Self::OtcConv15 => (180, 180),
        }
    }

    /// True if this is an adjustable-rate product.
    #[must_use]
    pub const fn is_arm(self) -> bool {
        matches!(
            self,
            Self::Arm5_6Sofr
                | Self::Arm7_6Sofr
                | Self::Arm10_6Sofr
                | Self::Arm5_1
                | Self::Arm7_1
                | Self::Arm10_1
        )
    }

    /// True if this is a one-time-close construction product.
    #[must_use]
    pub const fn is_construction(self) -> bool {
        matches!(
            self,
            Self::OtcConv30 | Self::OtcConv15 | Self::OtcVa30 | Self::OtcVaJumbo30
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loan_product_term_band_consistency() {
        // Spec: FixedConv11To15 returns TermBand(11..=15), i.e. 121–180 months
        assert_eq!(LoanProduct::FixedConv11To15.term_range_months(), (121, 180));
        assert_eq!(LoanProduct::FixedConv8To10.term_range_months(), (96, 120));
        assert_eq!(LoanProduct::FixedConv16To20.term_range_months(), (181, 240));
        assert_eq!(LoanProduct::FixedConv21To30.term_range_months(), (241, 360));
        assert_eq!(LoanProduct::FixedFha8To15.term_range_months(), (96, 180));
        assert_eq!(LoanProduct::FixedFha16To30.term_range_months(), (181, 360));
        assert_eq!(LoanProduct::FixedUsda30.term_range_months(), (360, 360));
    }

    #[test]
    fn test_loan_product_is_arm() {
        assert!(LoanProduct::Arm5_6Sofr.is_arm());
        assert!(LoanProduct::Arm7_6Sofr.is_arm());
        assert!(LoanProduct::Arm10_6Sofr.is_arm());
        assert!(LoanProduct::Arm5_1.is_arm());
        assert!(!LoanProduct::FixedConv21To30.is_arm());
        assert!(!LoanProduct::OtcConv30.is_arm());
    }

    #[test]
    fn test_loan_product_is_construction() {
        assert!(LoanProduct::OtcConv30.is_construction());
        assert!(LoanProduct::OtcConv15.is_construction());
        assert!(LoanProduct::OtcVa30.is_construction());
        assert!(LoanProduct::OtcVaJumbo30.is_construction());
        assert!(!LoanProduct::FixedConv21To30.is_construction());
        assert!(!LoanProduct::Arm5_6Sofr.is_construction());
    }

    #[test]
    fn test_loan_product_serde_json() {
        let p = LoanProduct::FixedConv21To30;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"fixed_conv21_to30\"");
        let back: LoanProduct = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
