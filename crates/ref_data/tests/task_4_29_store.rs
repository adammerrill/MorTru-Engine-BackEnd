//! Task 4.29 — store-wiring tests for `llpa_price`.
//!
//! The `task_4_29.rs` suite validates the pricing *logic* via the bare
//! `llpa::price()` function. This suite validates the **store wiring**: that
//! `RefDataStore::llpa_price` correctly resolves the versioned GSE dataset and
//! optional lender overlay via `read_versioned_json`, composes them, threads
//! year-fallback into provenance, and propagates ineligibility. Under
//! `--features sqlite` the same assertions run against `SqliteStore`, covering
//! the delegating impl.

use ref_data::llpa::*;
use ref_data::{JsonFileStore, RefDataStore};
use types::{Cents, CreditScore, LtvBasisPoints};

/// Path to the crate's data dir (where the versioned JSON catalogs live).
fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn store() -> JsonFileStore {
    JsonFileStore::new(data_dir())
}

fn scenario(score: u16, ltv_pct: f64, purpose: LlpaPurpose) -> LlpaScenario {
    LlpaScenario {
        purpose,
        occupancy: LlpaOccupancy::Primary,
        property_type: LlpaPropertyType::Detached,
        indicator_score: CreditScore::new(score).unwrap(),
        ltv: LtvBasisPoints::new((ltv_pct * 100.0) as u32).unwrap(),
        loan_amount: Cents::from_dollars(300_000),
        is_arm: false,
        is_high_balance: false,
        is_super_conforming: false,
        has_subordinate_financing: false,
        heloc_balance_at_closing: Cents::ZERO,
        has_affordable_second: false,
        state: "FL".to_owned(),
        ami_percent: None,
        is_first_time_homebuyer: false,
        is_high_cost_area: false,
        is_duty_to_serve: false,
        is_home_ready_or_possible: false,
    }
}

// ── Store resolves the GSE dataset and prices through the trait ─────────────

#[test]
fn store_prices_freddie_purchase_via_read_versioned_json() {
    let s = store();
    let p = s
        .llpa_price(
            GseAgency::Freddie,
            &scenario(800, 78.0, LlpaPurpose::Purchase),
            None,
            2026,
        )
        .expect("dataset resolves")
        .expect("scenario is eligible");
    // Exhibit 19 purchase >=780, 75-80 => 0.375% = 38 bps
    assert_eq!(p.value.gse_subtotal_bps, 38);
    // provenance must name the resolved file the store actually read
    assert_eq!(p.provenance.source_file, "freddie_credit_fees_2026.json");
    assert_eq!(p.provenance.resolved_version, 2026);
}

#[test]
fn store_prices_fannie_purchase() {
    let s = store();
    let p = s
        .llpa_price(
            GseAgency::Fannie,
            &scenario(745, 79.0, LlpaPurpose::Purchase),
            None,
            2026,
        )
        .unwrap()
        .unwrap();
    assert_eq!(p.value.gse_subtotal_bps, 88);
    assert_eq!(p.provenance.source_file, "fannie_llpa_2026.json");
}

// ── Store resolves + composes the lender overlay ────────────────────────────

#[test]
fn store_composes_uwm_overlay_on_no_cash_out_refi() {
    let s = store();
    let p = s
        .llpa_price(
            GseAgency::Freddie,
            &scenario(780, 78.0, LlpaPurpose::NoCashOutRefi),
            Some("uwm"),
            2026,
        )
        .unwrap()
        .unwrap();
    // gse no-cash-out >=780, 75-80 => 0.500% = 50 bps
    assert_eq!(p.value.gse_subtotal_bps, 50);
    // refi incentive -75 + TRAC band ($300k => -40)
    assert_eq!(p.value.lender_subtotal_bps, -75 - 40);
    assert_eq!(p.value.total_bps, 50 - 75 - 40);
}

// ── Ineligibility propagates through the store as Ok(Err(_)) ─────────────────

#[test]
fn store_propagates_ineligible_cash_out_over_80() {
    let s = store();
    let outer = s
        .llpa_price(
            GseAgency::Freddie,
            &scenario(760, 85.0, LlpaPurpose::CashOutRefi),
            None,
            2026,
        )
        .expect("dataset resolves");
    assert!(outer.is_err(), "cash-out over 80% LTV must be ineligible");
    let reason = outer.unwrap_err().reason;
    assert!(
        reason.contains("cutoff") || reason.contains("Not Eligible"),
        "unexpected reason: {reason}"
    );
}

#[test]
fn store_propagates_uwm_investment_ltv_overlay_rejection() {
    let s = store();
    let mut sc = scenario(760, 90.0, LlpaPurpose::Purchase);
    sc.occupancy = LlpaOccupancy::Investment;
    let outer = s
        .llpa_price(GseAgency::Freddie, &sc, Some("uwm"), 2026)
        .expect("dataset resolves");
    assert!(outer.is_err());
    assert!(outer.unwrap_err().reason.contains("investment_property"));
}

// ── Year-fallback is exercised by the real resolver ─────────────────────────

#[test]
fn store_year_fallback_to_2026_when_requesting_2030() {
    let s = store();
    let p = s
        .llpa_price(
            GseAgency::Freddie,
            &scenario(800, 78.0, LlpaPurpose::Purchase),
            None,
            2030,
        )
        .unwrap()
        .unwrap();
    assert_eq!(p.provenance.requested_version, 2030);
    assert_eq!(p.provenance.resolved_version, 2026);
    assert!(p.provenance.is_fallback());
    assert!(p.explain().contains("FALLBACK"));
}

// ── Missing dataset surfaces as the outer RefDataError, not a panic ─────────

#[test]
fn store_missing_overlay_is_refdata_error() {
    let s = store();
    let r = s.llpa_price(
        GseAgency::Freddie,
        &scenario(760, 78.0, LlpaPurpose::Purchase),
        Some("nonexistent_lender"),
        2026,
    );
    assert!(r.is_err(), "missing overlay file must be a RefDataError");
}

// ── SqliteStore delegation (same assertions, behind the feature) ────────────

#[cfg(feature = "sqlite")]
mod sqlite_delegation {
    use super::*;
    use ref_data::SqliteStore;

    fn sqlite_store() -> SqliteStore {
        SqliteStore::new_from_dir(&data_dir()).expect("open sqlite store")
    }

    #[test]
    fn sqlite_prices_freddie_purchase() {
        let s = sqlite_store();
        let p = s
            .llpa_price(
                GseAgency::Freddie,
                &scenario(800, 78.0, LlpaPurpose::Purchase),
                None,
                2026,
            )
            .unwrap()
            .unwrap();
        assert_eq!(p.value.gse_subtotal_bps, 38);
    }

    #[test]
    fn sqlite_composes_overlay_and_matches_jsonfilestore() {
        let sc = scenario(780, 78.0, LlpaPurpose::NoCashOutRefi);
        let json = JsonFileStore::new(data_dir())
            .llpa_price(GseAgency::Freddie, &sc, Some("uwm"), 2026)
            .unwrap()
            .unwrap();
        let sqlite = sqlite_store()
            .llpa_price(GseAgency::Freddie, &sc, Some("uwm"), 2026)
            .unwrap()
            .unwrap();
        assert_eq!(json.value.total_bps, sqlite.value.total_bps);
    }

    #[test]
    fn sqlite_propagates_ineligible() {
        let s = sqlite_store();
        let outer = s
            .llpa_price(
                GseAgency::Freddie,
                &scenario(760, 85.0, LlpaPurpose::CashOutRefi),
                None,
                2026,
            )
            .unwrap();
        assert!(outer.is_err());
    }
}
