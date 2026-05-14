//! Integration tests for the Task 1.5 common enumerations.
//!
//! Spec-required tests:
//! - `test_program_code_to_mismo_mortgage_type`
//! - `test_program_code_from_mismo_unknown_returns_error`
//! - `test_property_type_to_reso_lookup`
//! - `test_occupancy_round_trip_mismo`
//! - `test_all_enums_have_exhaustive_match`
//! - `test_loan_product_term_band_consistency`
//! - `prop_serde_roundtrip_all_enums`

use types::{
    AmortizationType, AusType, BalanceType, LienPriority, LoanProduct, LoanPurpose, LockPeriod,
    MiCoverageType, Occupancy, ProgramCode, PropertyType, Tier,
};

// ─────────────────────────────────────────────────────────────────────────────
// Spec tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_program_code_to_mismo_mortgage_type() {
    assert_eq!(
        ProgramCode::Conventional.to_mismo_mortgage_type(),
        "Conventional"
    );
    assert_eq!(
        ProgramCode::HomeReady.to_mismo_mortgage_type(),
        "Conventional"
    );
    assert_eq!(ProgramCode::Fha.to_mismo_mortgage_type(), "FHA");
    assert_eq!(ProgramCode::FhaDpa.to_mismo_mortgage_type(), "FHA");
    assert_eq!(ProgramCode::Va.to_mismo_mortgage_type(), "VA");
    assert_eq!(ProgramCode::VaJumbo.to_mismo_mortgage_type(), "VA");
    assert_eq!(
        ProgramCode::Usda.to_mismo_mortgage_type(),
        "USDARuralDevelopment"
    );
}

#[test]
fn test_program_code_from_mismo_unknown_returns_error() {
    assert!(ProgramCode::from_mismo_mortgage_type("Jumbo").is_err());
    assert!(ProgramCode::from_mismo_mortgage_type("").is_err());
    assert!(ProgramCode::from_mismo_mortgage_type("fha").is_err());
    assert!(ProgramCode::from_mismo_mortgage_type("conventional").is_err());
    assert!(ProgramCode::from_mismo_mortgage_type("HELOC").is_err());
}

#[test]
fn test_property_type_to_reso_lookup() {
    assert_eq!(
        PropertyType::SingleFamilyDetached.to_reso_lookup(),
        "Single Family Residence"
    );
    assert_eq!(PropertyType::Condominium.to_reso_lookup(), "Condominium");
    assert_eq!(PropertyType::TwoUnit.to_reso_lookup(), "Duplex");
    assert_eq!(PropertyType::FourUnit.to_reso_lookup(), "Quadruplex");
    assert_eq!(
        PropertyType::ManufacturedHome.to_reso_lookup(),
        "Manufactured Home"
    );
    assert_eq!(
        PropertyType::Cooperative.to_reso_lookup(),
        "Stock Cooperative"
    );
    assert_eq!(
        PropertyType::PlannedUnitDevelopment.to_reso_lookup(),
        "Planned Unit Development"
    );
}

#[test]
fn test_occupancy_round_trip_mismo() {
    let all = [
        Occupancy::PrimaryResidence,
        Occupancy::SecondHome,
        Occupancy::Investment,
    ];
    for occ in all {
        let mismo_str = occ.to_mismo();
        let back = Occupancy::from_mismo(mismo_str)
            .unwrap_or_else(|_| panic!("from_mismo failed for {mismo_str}"));
        assert_eq!(back, occ, "MISMO roundtrip failed for {occ:?}");
    }
}

/// Exhaustiveness check: a `match` on every enum must compile and cover
/// all variants without a wildcard. If a variant is added and this match
/// is not updated, the compiler will error — that's the point.
#[test]
fn test_all_enums_have_exhaustive_match() {
    // ProgramCode
    let p = ProgramCode::Conventional;
    let _: &str = match p {
        ProgramCode::Conventional => "conventional",
        ProgramCode::HomeReady => "home_ready",
        ProgramCode::HomePossible => "home_possible",
        ProgramCode::HomeOne => "home_one",
        ProgramCode::Fha => "fha",
        ProgramCode::FhaDpa => "fha_dpa",
        ProgramCode::Va => "va",
        ProgramCode::VaJumbo => "va_jumbo",
        ProgramCode::Usda => "usda",
        ProgramCode::Bond => "bond",
        ProgramCode::Jumbo => "jumbo",
        ProgramCode::NonQm => "non_qm",
    };

    // LoanProduct
    let lp = LoanProduct::FixedConv21To30;
    let _: bool = match lp {
        LoanProduct::FixedConv8To10
        | LoanProduct::FixedConv11To15
        | LoanProduct::FixedConv16To20
        | LoanProduct::FixedConv21To30
        | LoanProduct::FixedFha8To15
        | LoanProduct::FixedFha16To30
        | LoanProduct::FixedVa8To15
        | LoanProduct::FixedVa16To30
        | LoanProduct::FixedUsda30 => false,
        LoanProduct::Arm5_6Sofr
        | LoanProduct::Arm7_6Sofr
        | LoanProduct::Arm10_6Sofr
        | LoanProduct::Arm5_1
        | LoanProduct::Arm7_1
        | LoanProduct::Arm10_1 => true,
        LoanProduct::OtcConv30
        | LoanProduct::OtcConv15
        | LoanProduct::OtcVa30
        | LoanProduct::OtcVaJumbo30 => false,
    };

    // PropertyType
    let pt = PropertyType::SingleFamilyDetached;
    let _ = match pt {
        PropertyType::SingleFamilyDetached => 1,
        PropertyType::SingleFamilyAttached => 2,
        PropertyType::Townhouse => 3,
        PropertyType::Condominium => 4,
        PropertyType::Cooperative => 5,
        PropertyType::PlannedUnitDevelopment => 6,
        PropertyType::ManufacturedHome => 7,
        PropertyType::Modular => 8,
        PropertyType::MobileHome => 9,
        PropertyType::TwoUnit => 10,
        PropertyType::ThreeUnit => 11,
        PropertyType::FourUnit => 12,
    };

    // Occupancy
    let o = Occupancy::PrimaryResidence;
    let _ = match o {
        Occupancy::PrimaryResidence | Occupancy::SecondHome | Occupancy::Investment => true,
    };

    // LoanPurpose
    let lpu = LoanPurpose::Purchase;
    let _ = match lpu {
        LoanPurpose::Purchase
        | LoanPurpose::RateAndTermRefinance
        | LoanPurpose::CashOutRefinance
        | LoanPurpose::Construction
        | LoanPurpose::ConstructionToPermanent => true,
    };

    // AmortizationType
    let a = AmortizationType::Fixed;
    let _ = match a {
        AmortizationType::Fixed
        | AmortizationType::Arm
        | AmortizationType::InterestOnly
        | AmortizationType::GraduatedPayment
        | AmortizationType::PaymentOption => true,
    };

    // LockPeriod
    let lk = LockPeriod::Day30;
    let _ = match lk {
        LockPeriod::Day15
        | LockPeriod::Day21
        | LockPeriod::Day30
        | LockPeriod::Day45
        | LockPeriod::Day60
        | LockPeriod::Day75
        | LockPeriod::Day90 => true,
    };

    // LienPriority
    let li = LienPriority::First;
    let _ = match li {
        LienPriority::First | LienPriority::Second | LienPriority::Third => true,
    };

    // BalanceType
    let bt = BalanceType::Conforming;
    let _ = match bt {
        BalanceType::Conforming
        | BalanceType::HighBalance
        | BalanceType::SuperConforming
        | BalanceType::Jumbo => true,
    };

    // Tier
    let t = Tier::Standard;
    let _ = match t {
        Tier::Elite | Tier::Standard => true,
    };

    // MiCoverageType
    let mi = MiCoverageType::None;
    let _ = match mi {
        MiCoverageType::None
        | MiCoverageType::LenderPaid
        | MiCoverageType::BorrowerPaidMonthly
        | MiCoverageType::BorrowerPaidSingle
        | MiCoverageType::BorrowerPaidSplit
        | MiCoverageType::FhaUpfrontAndAnnual
        | MiCoverageType::VaFundingFee
        | MiCoverageType::UsdaUpfrontAndAnnual => true,
    };

    // AusType
    let aus = AusType::DesktopUnderwriter;
    let _ = match aus {
        AusType::DesktopUnderwriter
        | AusType::LoanProductAdvisor
        | AusType::Got
        | AusType::Gus
        | AusType::Manual => true,
    };
}

#[test]
fn test_loan_product_term_band_consistency() {
    assert_eq!(LoanProduct::FixedConv8To10.term_range_months(), (96, 120));
    assert_eq!(LoanProduct::FixedConv11To15.term_range_months(), (121, 180));
    assert_eq!(LoanProduct::FixedConv16To20.term_range_months(), (181, 240));
    assert_eq!(LoanProduct::FixedConv21To30.term_range_months(), (241, 360));
    assert_eq!(LoanProduct::FixedFha8To15.term_range_months(), (96, 180));
    assert_eq!(LoanProduct::FixedFha16To30.term_range_months(), (181, 360));
    assert_eq!(LoanProduct::FixedUsda30.term_range_months(), (360, 360));
    // ARM: amortises over 30 years regardless of initial fixed period
    assert_eq!(LoanProduct::Arm5_6Sofr.term_range_months(), (360, 360));
    assert_eq!(LoanProduct::Arm7_1.term_range_months(), (360, 360));
}

// ─────────────────────────────────────────────────────────────────────────────
// prop_serde_roundtrip_all_enums (proptest-style, but done deterministically
// by iterating every variant — proptest would just be checking the same finite
// set anyway since these are all C-like enums with no fields)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_serde_roundtrip_all_enums() {
    macro_rules! rt_all {
        ($($val:expr),+ $(,)?) => {
            $({
                let json = serde_json::to_string(&$val)
                    .unwrap_or_else(|e| panic!("serialize failed for {:?}: {e}", $val));
                assert!(!json.is_empty(), "serialized form must not be empty for {:?}", $val);
                // Re-deserialize and assert round-trip
                let back = serde_json::from_str(&json)
                    .unwrap_or_else(|e| panic!("deserialize failed for {:?} json={json}: {e}", $val));
                assert_eq!($val, back, "roundtrip mismatch for {:?}", $val);
            })+
        };
    }

    rt_all!(
        ProgramCode::Conventional,
        ProgramCode::HomeReady,
        ProgramCode::HomePossible,
        ProgramCode::HomeOne,
        ProgramCode::Fha,
        ProgramCode::FhaDpa,
        ProgramCode::Va,
        ProgramCode::VaJumbo,
        ProgramCode::Usda,
        ProgramCode::Bond,
        ProgramCode::Jumbo,
        ProgramCode::NonQm,
    );
    rt_all!(
        LoanProduct::FixedConv8To10,
        LoanProduct::FixedConv11To15,
        LoanProduct::FixedConv16To20,
        LoanProduct::FixedConv21To30,
        LoanProduct::FixedFha8To15,
        LoanProduct::FixedFha16To30,
        LoanProduct::FixedVa8To15,
        LoanProduct::FixedVa16To30,
        LoanProduct::FixedUsda30,
        LoanProduct::Arm5_6Sofr,
        LoanProduct::Arm7_6Sofr,
        LoanProduct::Arm10_6Sofr,
        LoanProduct::Arm5_1,
        LoanProduct::Arm7_1,
        LoanProduct::Arm10_1,
        LoanProduct::OtcConv30,
        LoanProduct::OtcConv15,
        LoanProduct::OtcVa30,
        LoanProduct::OtcVaJumbo30,
    );
    rt_all!(
        PropertyType::SingleFamilyDetached,
        PropertyType::SingleFamilyAttached,
        PropertyType::Townhouse,
        PropertyType::Condominium,
        PropertyType::Cooperative,
        PropertyType::PlannedUnitDevelopment,
        PropertyType::ManufacturedHome,
        PropertyType::TwoUnit,
        PropertyType::ThreeUnit,
        PropertyType::FourUnit,
    );
    rt_all!(
        Occupancy::PrimaryResidence,
        Occupancy::SecondHome,
        Occupancy::Investment,
    );
    rt_all!(
        LoanPurpose::Purchase,
        LoanPurpose::RateAndTermRefinance,
        LoanPurpose::CashOutRefinance,
        LoanPurpose::Construction,
        LoanPurpose::ConstructionToPermanent,
    );
    rt_all!(
        AmortizationType::Fixed,
        AmortizationType::Arm,
        AmortizationType::InterestOnly,
        AmortizationType::GraduatedPayment,
        AmortizationType::PaymentOption,
    );
    rt_all!(
        LockPeriod::Day15,
        LockPeriod::Day21,
        LockPeriod::Day30,
        LockPeriod::Day45,
        LockPeriod::Day60,
        LockPeriod::Day75,
        LockPeriod::Day90,
    );
    rt_all!(
        LienPriority::First,
        LienPriority::Second,
        LienPriority::Third
    );
    rt_all!(
        BalanceType::Conforming,
        BalanceType::HighBalance,
        BalanceType::SuperConforming,
        BalanceType::Jumbo,
    );
    rt_all!(Tier::Elite, Tier::Standard);
    rt_all!(
        MiCoverageType::None,
        MiCoverageType::LenderPaid,
        MiCoverageType::BorrowerPaidMonthly,
        MiCoverageType::BorrowerPaidSingle,
        MiCoverageType::BorrowerPaidSplit,
        MiCoverageType::FhaUpfrontAndAnnual,
        MiCoverageType::VaFundingFee,
        MiCoverageType::UsdaUpfrontAndAnnual,
    );
    rt_all!(
        AusType::DesktopUnderwriter,
        AusType::LoanProductAdvisor,
        AusType::Got,
        AusType::Gus,
        AusType::Manual,
    );
}
