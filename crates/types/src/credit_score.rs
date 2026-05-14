//! `CreditScore` — FICO/credit score validated to the 300-850 range.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// Credit score validated to the FICO/VantageScore range of 300-850.
///
/// The validating constructor [`Self::new`] enforces the range. Direct
/// tuple-struct construction (`CreditScore(720)`) is also allowed for
/// convenience in trusted contexts, but external input should always go
/// through `new()` to surface bad data early.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CreditScore(pub u16);

impl CreditScore {
    /// Minimum valid score. FICO and VantageScore are undefined below 300.
    pub const MIN: Self = CreditScore(300);

    /// Maximum valid score. FICO and VantageScore top out at 850.
    pub const MAX: Self = CreditScore(850);

    /// Validating constructor. Returns `Err` if `value` is outside `300..=850`.
    pub fn new(value: u16) -> Result<Self, ParseError> {
        if !(300..=850).contains(&value) {
            return Err(ParseError::CreditScoreOutOfRange(value));
        }
        Ok(CreditScore(value))
    }

    /// Compute the middle of three credit scores — the industry-standard
    /// representative score used by underwriting. If two scores tie, the
    /// tied value is returned (it is by definition the middle).
    ///
    /// # Example
    /// ```ignore
    /// let middle = CreditScore::middle_of_three(
    ///     CreditScore(720), CreditScore(740), CreditScore(700)
    /// );
    /// assert_eq!(middle, CreditScore(720));
    /// ```
    #[must_use]
    pub fn middle_of_three(a: CreditScore, b: CreditScore, c: CreditScore) -> CreditScore {
        let mut s = [a.0, b.0, c.0];
        s.sort_unstable();
        CreditScore(s[1])
    }

    /// Compute the lower of two credit scores. Used when only two repository
    /// scores are available (industry convention: the lower score is the
    /// representative).
    #[must_use]
    pub fn lower_of_two(a: CreditScore, b: CreditScore) -> CreditScore {
        if a.0 <= b.0 {
            a
        } else {
            b
        }
    }
}

impl fmt::Display for CreditScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_score_range() {
        // Below 300: rejected
        assert!(CreditScore::new(0).is_err());
        assert!(CreditScore::new(100).is_err());
        assert!(CreditScore::new(299).is_err());

        // 300..=850: accepted
        assert_eq!(CreditScore::new(300).unwrap(), CreditScore(300));
        assert_eq!(CreditScore::new(550).unwrap(), CreditScore(550));
        assert_eq!(CreditScore::new(720).unwrap(), CreditScore(720));
        assert_eq!(CreditScore::new(800).unwrap(), CreditScore(800));
        assert_eq!(CreditScore::new(850).unwrap(), CreditScore(850));

        // Above 850: rejected
        assert!(CreditScore::new(851).is_err());
        assert!(CreditScore::new(900).is_err());
        assert!(CreditScore::new(u16::MAX).is_err());

        // The error variant carries the offending value
        match CreditScore::new(299) {
            Err(ParseError::CreditScoreOutOfRange(v)) => assert_eq!(v, 299),
            other => panic!("expected CreditScoreOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn test_credit_score_middle_of_three() {
        // Three distinct scores
        let mid = CreditScore::middle_of_three(
            CreditScore::new(720).unwrap(),
            CreditScore::new(740).unwrap(),
            CreditScore::new(700).unwrap(),
        );
        assert_eq!(mid, CreditScore(720));

        // Already sorted ascending
        let mid = CreditScore::middle_of_three(
            CreditScore::new(600).unwrap(),
            CreditScore::new(700).unwrap(),
            CreditScore::new(800).unwrap(),
        );
        assert_eq!(mid, CreditScore(700));

        // Already sorted descending
        let mid = CreditScore::middle_of_three(
            CreditScore::new(800).unwrap(),
            CreditScore::new(700).unwrap(),
            CreditScore::new(600).unwrap(),
        );
        assert_eq!(mid, CreditScore(700));

        // Two-way tie at low end
        let mid = CreditScore::middle_of_three(
            CreditScore::new(700).unwrap(),
            CreditScore::new(700).unwrap(),
            CreditScore::new(800).unwrap(),
        );
        assert_eq!(mid, CreditScore(700));

        // Two-way tie at high end
        let mid = CreditScore::middle_of_three(
            CreditScore::new(600).unwrap(),
            CreditScore::new(800).unwrap(),
            CreditScore::new(800).unwrap(),
        );
        assert_eq!(mid, CreditScore(800));

        // All three equal
        let mid = CreditScore::middle_of_three(
            CreditScore::new(720).unwrap(),
            CreditScore::new(720).unwrap(),
            CreditScore::new(720).unwrap(),
        );
        assert_eq!(mid, CreditScore(720));
    }

    #[test]
    fn test_credit_score_lower_of_two() {
        let a = CreditScore::new(720).unwrap();
        let b = CreditScore::new(740).unwrap();
        assert_eq!(CreditScore::lower_of_two(a, b), a);
        assert_eq!(CreditScore::lower_of_two(b, a), a);

        // Tie returns either (this impl returns the first)
        assert_eq!(CreditScore::lower_of_two(a, a), a);
    }

    #[test]
    fn test_credit_score_display() {
        assert_eq!(CreditScore::new(720).unwrap().to_string(), "720");
        assert_eq!(CreditScore::new(300).unwrap().to_string(), "300");
        assert_eq!(CreditScore::new(850).unwrap().to_string(), "850");
    }

    #[test]
    fn test_credit_score_serde_json() {
        let c = CreditScore::new(720).unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "720");
        let back: CreditScore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn test_credit_score_constants() {
        assert_eq!(CreditScore::MIN, CreditScore(300));
        assert_eq!(CreditScore::MAX, CreditScore(850));
    }

    #[test]
    fn test_credit_score_repr_transparent() {
        assert_eq!(
            std::mem::size_of::<CreditScore>(),
            std::mem::size_of::<u16>()
        );
    }

    #[test]
    fn test_credit_score_ordering() {
        let mut v = vec![
            CreditScore::new(720).unwrap(),
            CreditScore::new(650).unwrap(),
            CreditScore::new(800).unwrap(),
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                CreditScore::new(650).unwrap(),
                CreditScore::new(720).unwrap(),
                CreditScore::new(800).unwrap(),
            ]
        );
    }

    #[test]
    fn test_credit_score_full_range_accepted() {
        // Every score in 300..=850 must be accepted
        for score in 300..=850_u16 {
            assert!(
                CreditScore::new(score).is_ok(),
                "score {score} should be valid"
            );
        }
    }
}
