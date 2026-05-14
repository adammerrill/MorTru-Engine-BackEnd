//! Task 2.9 gate tests — AUS and qualification schema.
//!
//! Covers DU/LPA/FHA TOTAL/GUS system types, all recommendation variants,
//! qualifying rate to BasisPoints, and housing/total DTI to DtiBasisPoints.

use mismo::{
    enums::aus::AusRecommendation,
    schema::aus::{MismoAus, MismoQualification},
    MismoError,
};
use types::{AusType, BasisPoints, DtiBasisPoints};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn du_approve_eligible() -> MismoAus {
    MismoAus {
        system_type: "DesktopUnderwriter".into(),
        recommendation: Some("Approve/Eligible".into()),
        case_id: Some("DU-2025-12345".into()),
    }
}

fn fha_qualification() -> MismoQualification {
    MismoQualification {
        qualifying_rate: Some("6.375".into()),
        housing_ratio: Some("28.50".into()),
        total_dti: Some("43.00".into()),
    }
}

// ── AUS system type parsing ───────────────────────────────────────────────────

#[test]
fn test_du_system_type_parses() {
    let p = du_approve_eligible().parse().unwrap();
    assert_eq!(p.system, AusType::DesktopUnderwriter);
}

#[test]
fn test_lpa_system_type_parses() {
    let aus = MismoAus {
        system_type: "LoanProductAdvisor".into(),
        recommendation: Some("Accept".into()),
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert_eq!(p.system, AusType::LoanProductAdvisor);
}

#[test]
fn test_fha_total_scorecard_system_type() {
    let aus = MismoAus {
        system_type: "FHATotalScorecard".into(),
        recommendation: Some("ApproveEligible".into()),
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert_eq!(p.system, AusType::Got);
}

#[test]
fn test_gus_usda_system_type() {
    let aus = MismoAus {
        system_type: "USDARuralHousingGUS".into(),
        recommendation: Some("Accept".into()),
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert_eq!(p.system, AusType::Gus);
}

#[test]
fn test_manual_underwriting_system_type() {
    let aus = MismoAus {
        system_type: "Manual".into(),
        recommendation: None,
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert_eq!(p.system, AusType::Manual);
}

#[test]
fn test_unknown_aus_type_returns_error() {
    let aus = MismoAus {
        system_type: "MagicScorecard".into(),
        recommendation: None,
        case_id: None,
    };
    assert!(matches!(
        aus.parse().unwrap_err(),
        MismoError::InvalidEnum {
            element: "AutomatedUnderwritingSystemType",
            ..
        }
    ));
}

// ── AUS recommendation variants ───────────────────────────────────────────────

#[test]
fn test_du_approve_eligible_recommendation() {
    let p = du_approve_eligible().parse().unwrap();
    assert_eq!(p.recommendation, Some(AusRecommendation::ApproveEligible));
}

#[test]
fn test_lpa_accept_maps_to_approve_eligible() {
    let aus = MismoAus {
        system_type: "LoanProductAdvisor".into(),
        recommendation: Some("Accept".into()),
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert_eq!(p.recommendation, Some(AusRecommendation::ApproveEligible));
}

#[test]
fn test_refer_recommendation() {
    let mut aus = du_approve_eligible();
    aus.recommendation = Some("Refer".into());
    let p = aus.parse().unwrap();
    assert_eq!(p.recommendation, Some(AusRecommendation::Refer));
}

#[test]
fn test_refer_with_caution_recommendation() {
    let mut aus = du_approve_eligible();
    aus.recommendation = Some("ReferWithCaution".into());
    let p = aus.parse().unwrap();
    assert_eq!(p.recommendation, Some(AusRecommendation::ReferWithCaution));
}

#[test]
fn test_ineligible_recommendation() {
    let mut aus = du_approve_eligible();
    aus.recommendation = Some("Ineligible".into());
    let p = aus.parse().unwrap();
    assert_eq!(p.recommendation, Some(AusRecommendation::Ineligible));
}

#[test]
fn test_recommendation_absent_is_none() {
    let aus = MismoAus {
        system_type: "DesktopUnderwriter".into(),
        recommendation: None,
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert!(p.recommendation.is_none());
}

#[test]
fn test_unknown_recommendation_returns_error() {
    let mut aus = du_approve_eligible();
    aus.recommendation = Some("FiftyFifty".into());
    assert!(matches!(
        aus.parse().unwrap_err(),
        MismoError::InvalidEnum {
            element: "AUSRecommendationType",
            ..
        }
    ));
}

// ── is_approvable predicate ───────────────────────────────────────────────────

#[test]
fn test_approve_eligible_is_approvable() {
    let p = du_approve_eligible().parse().unwrap();
    assert!(p.is_approvable());
}

#[test]
fn test_refer_is_not_approvable() {
    let mut aus = du_approve_eligible();
    aus.recommendation = Some("Refer".into());
    let p = aus.parse().unwrap();
    assert!(!p.is_approvable());
}

#[test]
fn test_no_recommendation_is_not_approvable() {
    let aus = MismoAus {
        system_type: "DesktopUnderwriter".into(),
        recommendation: None,
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert!(!p.is_approvable());
}

// ── Case ID ───────────────────────────────────────────────────────────────────

#[test]
fn test_case_id_preserved() {
    let p = du_approve_eligible().parse().unwrap();
    assert_eq!(p.case_id.as_deref(), Some("DU-2025-12345"));
}

#[test]
fn test_case_id_absent_is_none() {
    let aus = MismoAus {
        system_type: "DesktopUnderwriter".into(),
        recommendation: Some("Approve/Eligible".into()),
        case_id: None,
    };
    let p = aus.parse().unwrap();
    assert!(p.case_id.is_none());
}

// ── Qualification: qualifying rate ────────────────────────────────────────────

#[test]
fn test_qualifying_rate_6375_to_basis_points() {
    let p = fha_qualification().parse().unwrap();
    // 6.375% note rate → BasisPoints(6375) using ×1000 scale
    assert_eq!(p.qualifying_rate, Some(BasisPoints(6375)));
}

#[test]
fn test_qualifying_rate_absent_is_none() {
    let q = MismoQualification {
        qualifying_rate: None,
        housing_ratio: None,
        total_dti: None,
    };
    let p = q.parse().unwrap();
    assert!(p.qualifying_rate.is_none());
}

#[test]
fn test_invalid_qualifying_rate_returns_error() {
    let q = MismoQualification {
        qualifying_rate: Some("not_a_rate".into()),
        housing_ratio: None,
        total_dti: None,
    };
    assert!(matches!(
        q.parse().unwrap_err(),
        MismoError::OutOfRange { .. }
    ));
}

// ── Qualification: DTI ratios ─────────────────────────────────────────────────

#[test]
fn test_housing_ratio_2850() {
    let p = fha_qualification().parse().unwrap();
    // 28.50% housing ratio → DtiBasisPoints(2850)
    assert_eq!(p.housing_ratio, Some(DtiBasisPoints::new(2850)));
}

#[test]
fn test_total_dti_4300() {
    let p = fha_qualification().parse().unwrap();
    // 43.00% total DTI → DtiBasisPoints(4300)
    assert_eq!(p.total_dti, Some(DtiBasisPoints::new(4300)));
}

#[test]
fn test_dti_absent_is_none() {
    let q = MismoQualification {
        qualifying_rate: None,
        housing_ratio: None,
        total_dti: None,
    };
    let p = q.parse().unwrap();
    assert!(p.housing_ratio.is_none());
    assert!(p.total_dti.is_none());
}

#[test]
fn test_invalid_dti_returns_error() {
    let q = MismoQualification {
        qualifying_rate: None,
        housing_ratio: Some("bad_dti".into()),
        total_dti: None,
    };
    assert!(matches!(
        q.parse().unwrap_err(),
        MismoError::OutOfRange { .. }
    ));
}

// ── XML round-trips ───────────────────────────────────────────────────────────

#[test]
fn test_aus_xml_roundtrip() {
    let aus = du_approve_eligible();
    let xml = mismo::xml::serialize::to_xml(&aus).unwrap();
    assert!(xml.contains("DesktopUnderwriter"));
    assert!(xml.contains("DU-2025-12345"));

    let restored: MismoAus = mismo::xml::parse::from_xml(&xml).unwrap();
    let p = restored.parse().unwrap();
    assert_eq!(p.system, AusType::DesktopUnderwriter);
    assert_eq!(p.recommendation, Some(AusRecommendation::ApproveEligible));
}

#[test]
fn test_qualification_xml_roundtrip() {
    let q = fha_qualification();
    let xml = mismo::xml::serialize::to_xml(&q).unwrap();
    assert!(xml.contains("6.375"));
    assert!(xml.contains("28.50"));
    assert!(xml.contains("43.00"));

    let restored: MismoQualification = mismo::xml::parse::from_xml(&xml).unwrap();
    let p = restored.parse().unwrap();
    assert_eq!(p.qualifying_rate, Some(BasisPoints(6375)));
    assert_eq!(p.total_dti, Some(DtiBasisPoints::new(4300)));
}

#[test]
fn test_parse_aus_from_xml_string() {
    let xml = r#"<AUTOMATED_UNDERWRITING_SYSTEM>
        <AutomatedUnderwritingSystemType>DesktopUnderwriter</AutomatedUnderwritingSystemType>
        <AUSRecommendationType>Approve/Eligible</AUSRecommendationType>
        <AUSCaseIdentifier>12345678</AUSCaseIdentifier>
    </AUTOMATED_UNDERWRITING_SYSTEM>"#;

    let aus: MismoAus = mismo::xml::parse::from_xml(xml).unwrap();
    let p = aus.parse().unwrap();
    assert_eq!(p.system, AusType::DesktopUnderwriter);
    assert!(p.is_approvable());
    assert_eq!(p.case_id.as_deref(), Some("12345678"));
}
