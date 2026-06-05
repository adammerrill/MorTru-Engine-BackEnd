//! Epic 4.5.2 — store-wiring tests for `mi_rate_quote`.
//!
//! `conv_pmi.rs` validates the `quote()` logic via direct calls. This suite
//! validates the **store wiring**: that `RefDataStore::mi_rate_quote` resolves
//! the versioned card via `read_versioned_json`, threads version-fallback into
//! provenance, and propagates "not offered" as `Ok(Err(MiUnavailable))`. Under
//! `--features sqlite` the same assertions run against `SqliteStore`.

use ref_data::conv_pmi::*;
use ref_data::{JsonFileStore, RefDataStore};
use types::{Cents, CreditScore, LtvBasisPoints};

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}
fn store() -> JsonFileStore {
    JsonFileStore::new(data_dir())
}

fn scn(plan: MiPlan, fixed: bool, score: u16, ltv_pct: f64, coverage: u8) -> MiScenario {
    MiScenario {
        plan,
        refundability: Refundability::NonRefundable,
        is_fixed: fixed,
        amortization_term_months: 360,
        indicator_score: CreditScore::new(score).unwrap(),
        ltv: LtvBasisPoints::new((ltv_pct * 100.0) as u32).unwrap(),
        coverage_percent: coverage,
        loan_amount: Cents::from_dollars(300_000),
        property_type: MiPropertyType::SingleFamilyDetached,
        occupancy: MiOccupancy::Primary,
        purpose: MiPurpose::Purchase,
        borrower_count: 1,
        is_relocation: false,
        dti_tier: DtiTier::Qualified,
    }
}

#[test]
fn store_prices_enact_via_read_versioned_json() {
    let q = store()
        .mi_rate_quote(
            MiCompany::Enact,
            &scn(MiPlan::MonthlyBpmi, true, 800, 96.0, 35),
            2025,
        )
        .expect("card resolves")
        .expect("offered");
    assert_eq!(q.value.net_milli_pct, 580);
    assert_eq!(q.provenance.source_file, "enact_pmi_2025.json");
    assert_eq!(q.provenance.resolved_version, 2025);
}

#[test]
fn store_prices_radian_lt620() {
    let q = store()
        .mi_rate_quote(
            MiCompany::Radian,
            &scn(MiPlan::SingleBpmi, true, 610, 96.0, 35),
            2025,
        )
        .unwrap()
        .unwrap();
    assert_eq!(q.value.net_milli_pct, 8940);
    assert_eq!(q.provenance.source_file, "radian_pmi_2021.json"); // fallback to 2021
}

#[test]
fn store_essent_version_fallback() {
    // Essent card is 2019; requesting 2025 resolves back to 2019.
    let q = store()
        .mi_rate_quote(
            MiCompany::Essent,
            &scn(MiPlan::MonthlyBpmi, true, 800, 96.0, 35),
            2025,
        )
        .unwrap()
        .unwrap();
    assert_eq!(q.provenance.requested_version, 2025);
    assert_eq!(q.provenance.resolved_version, 2019);
    assert!(q.provenance.is_fallback());
}

#[test]
fn store_propagates_unavailable_plan() {
    let outer = store()
        .mi_rate_quote(
            MiCompany::Radian,
            &scn(MiPlan::SplitPremium, true, 800, 96.0, 35),
            2025,
        )
        .expect("card resolves");
    assert!(outer.is_err());
}

#[test]
fn store_missing_company_is_refdata_error() {
    // Arch card not shipped this task => outer RefDataError, not a panic.
    let r = store().mi_rate_quote(
        MiCompany::Arch,
        &scn(MiPlan::MonthlyBpmi, true, 800, 96.0, 35),
        2025,
    );
    assert!(r.is_err());
}

#[cfg(feature = "sqlite")]
mod sqlite_delegation {
    use super::*;
    use ref_data::SqliteStore;

    fn sqlite_store() -> SqliteStore {
        SqliteStore::new_from_dir(&data_dir()).expect("open sqlite store")
    }

    #[test]
    fn sqlite_prices_enact() {
        let q = sqlite_store()
            .mi_rate_quote(
                MiCompany::Enact,
                &scn(MiPlan::MonthlyBpmi, true, 800, 96.0, 35),
                2025,
            )
            .unwrap()
            .unwrap();
        assert_eq!(q.value.net_milli_pct, 580);
    }

    #[test]
    fn sqlite_matches_jsonfilestore() {
        let s = scn(MiPlan::SingleBpmi, true, 740, 90.0, 25);
        let j = JsonFileStore::new(data_dir())
            .mi_rate_quote(MiCompany::Enact, &s, 2025)
            .unwrap()
            .unwrap();
        let q = sqlite_store()
            .mi_rate_quote(MiCompany::Enact, &s, 2025)
            .unwrap()
            .unwrap();
        assert_eq!(j.value.net_milli_pct, q.value.net_milli_pct);
    }

    #[test]
    fn sqlite_propagates_unavailable() {
        let outer = sqlite_store()
            .mi_rate_quote(
                MiCompany::Radian,
                &scn(MiPlan::SplitPremium, true, 800, 96.0, 35),
                2025,
            )
            .unwrap();
        assert!(outer.is_err());
    }
}
