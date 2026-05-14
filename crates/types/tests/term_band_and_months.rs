//! Integration tests for the Task 1.6 term primitives.
//!
//! Spec-required tests (updated for user-specified contiguous boundaries):
//! - `test_band_21_30_contains_241_through_360`
//! - `test_band_21_30_excludes_240_and_361`
//! - `test_band_21_30_all_months_yields_120_terms`
//! - `test_term_120_maps_to_band_8_to_10_conv`
//! - `test_term_120_maps_to_band_8_to_15_govt`
//! - `test_usda_30_only_contains_only_360`
//! - `test_total_term_count_120_to_360_is_241`
//! - `prop_every_term_120_to_360_maps_to_at_least_one_band_per_class`

use types::{TermBand, TermMonths};

// ─────────────────────────────────────────────────────────────────────────────
// Band21To30: 241–360 (120 terms)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_band_21_30_contains_241_through_360() {
    // Lower bound (241 = 20 years 1 month) — first month in the band
    assert!(
        TermBand::Band21To30.contains(TermMonths(241)),
        "241 must be in Band21To30 (20 year 1 month is the band's start)"
    );

    // Upper bound (360 = 30 years) — last month in the band
    assert!(
        TermBand::Band21To30.contains(TermMonths(360)),
        "360 must be in Band21To30"
    );

    // A few interior points
    assert!(TermBand::Band21To30.contains(TermMonths(300)));
    assert!(TermBand::Band21To30.contains(TermMonths(280)));
    assert!(TermBand::Band21To30.contains(TermMonths(241)));
}

#[test]
fn test_band_21_30_excludes_240_and_361() {
    // 240 = 20 years exactly — belongs to Band16To20
    assert!(
        !TermBand::Band21To30.contains(TermMonths(240)),
        "240 (20 years exactly) must NOT be in Band21To30 — it is the last month of Band16To20"
    );
    assert!(
        TermBand::Band16To20.contains(TermMonths(240)),
        "240 must be in Band16To20"
    );

    // 361 is beyond the 30-year maximum — no band covers it
    assert!(
        !TermBand::Band21To30.contains(TermMonths(361)),
        "361 exceeds the maximum 30-year term and must not be in any band"
    );
}

#[test]
fn test_band_21_30_all_months_yields_120_terms() {
    // 241..=360 inclusive = 120 individual terms
    let months: Vec<TermMonths> = TermBand::Band21To30.all_months().collect();
    assert_eq!(
        months.len(),
        120,
        "Band21To30 (241–360) must yield exactly 120 month-terms; \
         the engine prices each one individually even though they share a rate"
    );
    assert_eq!(months.first(), Some(&TermMonths(241)));
    assert_eq!(months.last(), Some(&TermMonths(360)));
}

// ─────────────────────────────────────────────────────────────────────────────
// Term 120 — the join point between Band8To10 and Band11To15
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_term_120_maps_to_band_8_to_10_conv() {
    // 120 months = 10 years exactly → last month of Band8To10
    let term = TermMonths(120);
    assert_eq!(
        term.band_for_conv(),
        Some(TermBand::Band8To10),
        "term 120 (10 years) must map to Band8To10 for conventional"
    );

    // 121 is the first month of the next band
    assert_eq!(
        TermMonths(121).band_for_conv(),
        Some(TermBand::Band11To15),
        "term 121 (10 years 1 month) must be the first month of Band11To15"
    );
}

#[test]
fn test_term_120_maps_to_band_8_to_15_govt() {
    // Same term, different product class
    let term = TermMonths(120);
    assert_eq!(
        term.band_for_govt(),
        Some(TermBand::GovtBand8To15),
        "term 120 maps to GovtBand8To15 for government loans"
    );

    // The govt band boundary is at 180/181
    assert_eq!(
        TermMonths(180).band_for_govt(),
        Some(TermBand::GovtBand8To15)
    );
    assert_eq!(
        TermMonths(181).band_for_govt(),
        Some(TermBand::GovtBand16To30)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// USDA
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_usda_30_only_contains_only_360() {
    assert!(
        TermBand::Usda30Only.contains(TermMonths(360)),
        "Usda30Only must contain 360"
    );
    assert_eq!(TermBand::Usda30Only.all_months().count(), 1);
    assert_eq!(
        TermBand::Usda30Only.all_months().next(),
        Some(TermMonths(360))
    );

    // No other term is in the USDA band
    for m in 120u16..360 {
        assert!(
            !TermBand::Usda30Only.contains(TermMonths(m)),
            "Usda30Only must not contain {m}"
        );
    }

    // band_for_usda is None for everything except 360
    assert_eq!(TermMonths(360).band_for_usda(), Some(TermBand::Usda30Only));
    assert_eq!(TermMonths(359).band_for_usda(), None);
    assert_eq!(TermMonths(120).band_for_usda(), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Total term count
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_total_term_count_120_to_360_is_241() {
    let count = TermMonths::all_valid().count();
    assert_eq!(
        count, 241,
        "TermMonths::all_valid() (120..=360) must yield 241 terms; \
         the engine analyses every one of these individually within each band"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property: every term 120..=360 maps to at least one band per product class
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_every_term_120_to_360_maps_to_at_least_one_band_per_class() {
    for m in 120u16..=360 {
        let term = TermMonths(m);

        assert!(
            term.band_for_conv().is_some(),
            "term {m} has no conventional band — every month 120–360 must map to a band"
        );

        assert!(
            term.band_for_govt().is_some(),
            "term {m} has no government band — every month 120–360 must map to a band"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional: every conv band is covered exactly once across 96–360
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_conv_bands_partition_96_to_360() {
    let conv_bands = [
        TermBand::Band8To10,
        TermBand::Band11To15,
        TermBand::Band16To20,
        TermBand::Band21To30,
    ];

    for m in 96u16..=360 {
        let term = TermMonths(m);
        let matching: Vec<TermBand> = conv_bands
            .iter()
            .copied()
            .filter(|b| b.contains(term))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "term {m} must belong to exactly 1 conventional band; found {matching:?}"
        );
    }
}

#[test]
fn test_govt_bands_partition_96_to_360() {
    let govt_bands = [TermBand::GovtBand8To15, TermBand::GovtBand16To30];

    for m in 96u16..=360 {
        let term = TermMonths(m);
        let matching: Vec<TermBand> = govt_bands
            .iter()
            .copied()
            .filter(|b| b.contains(term))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "term {m} must belong to exactly 1 government band; found {matching:?}"
        );
    }
}

#[test]
fn test_all_band_labels_non_empty() {
    let all_bands = [
        TermBand::Band8To10,
        TermBand::Band11To15,
        TermBand::Band16To20,
        TermBand::Band21To30,
        TermBand::GovtBand8To15,
        TermBand::GovtBand16To30,
        TermBand::Usda30Only,
    ];
    for b in all_bands {
        assert!(
            !b.rate_sheet_label().is_empty(),
            "label must not be empty for {b:?}"
        );
    }
}

#[test]
fn test_term_months_validated_construction() {
    assert!(TermMonths::new(120).is_ok());
    assert!(TermMonths::new(360).is_ok());
    assert!(TermMonths::new(119).is_err());
    assert!(TermMonths::new(361).is_err());
}
