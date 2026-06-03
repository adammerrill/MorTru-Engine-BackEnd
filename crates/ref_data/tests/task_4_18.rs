//! Task 4.18 — FNMA LLPA matrix and rate sheet lookup.

use ref_data::{JsonFileStore, LlpaInput, RefDataStore};

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

fn purchase_primary(fico: u16, ltv_bps: u32) -> LlpaInput {
    LlpaInput {
        fico,
        ltv_bps,
        loan_purpose: "purchase".to_owned(),
        occupancy: "primary".to_owned(),
        is_standard_manufactured: false,
        is_high_balance: false,
    }
}

// ════════════════════════════════════════════════════════════════════════════
// FNMA LLPA Matrix
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_llpa_760_fico_low_ltv_is_zero() {
    let llpa = store()
        .llpa_total("fnma", &purchase_primary(760, 7500), 2025)
        .unwrap();
    assert_eq!(llpa, 0, "760+ FICO, ≤75% LTV: no LLPA");
}

#[test]
fn test_llpa_760_fico_85pct_ltv_is_25bps() {
    let llpa = store()
        .llpa_total("fnma", &purchase_primary(760, 8200), 2025)
        .unwrap();
    assert_eq!(llpa, 25);
}

#[test]
fn test_llpa_760_fico_97pct_ltv_is_75bps() {
    let llpa = store()
        .llpa_total("fnma", &purchase_primary(760, 9600), 2025)
        .unwrap();
    assert_eq!(llpa, 75);
}

#[test]
fn test_llpa_720_fico_95pct_ltv() {
    let llpa = store()
        .llpa_total("fnma", &purchase_primary(720, 9200), 2025)
        .unwrap();
    assert_eq!(llpa, 125);
}

#[test]
fn test_llpa_700_fico_88pct_ltv() {
    let llpa = store()
        .llpa_total("fnma", &purchase_primary(700, 8800), 2025)
        .unwrap();
    assert_eq!(llpa, 175);
}

#[test]
fn test_llpa_660_fico_92pct_ltv() {
    let llpa = store()
        .llpa_total("fnma", &purchase_primary(660, 9200), 2025)
        .unwrap();
    assert_eq!(llpa, 325);
}

#[test]
fn test_llpa_lower_fico_higher_cost() {
    let high = store()
        .llpa_total("fnma", &purchase_primary(760, 9200), 2025)
        .unwrap();
    let low = store()
        .llpa_total("fnma", &purchase_primary(660, 9200), 2025)
        .unwrap();
    assert!(high < low, "lower FICO → higher LLPA: {high} vs {low}");
}

#[test]
fn test_llpa_higher_ltv_higher_cost() {
    let lo_ltv = store()
        .llpa_total("fnma", &purchase_primary(720, 8000), 2025)
        .unwrap();
    let hi_ltv = store()
        .llpa_total("fnma", &purchase_primary(720, 9600), 2025)
        .unwrap();
    assert!(lo_ltv < hi_ltv, "higher LTV → higher LLPA");
}

#[test]
fn test_llpa_manufactured_home_adds_50bps() {
    let standard = purchase_primary(760, 8800);
    let manufactured = LlpaInput {
        is_standard_manufactured: true,
        ..standard.clone()
    };
    let base = store().llpa_total("fnma", &standard, 2025).unwrap();
    let with_mfg = store().llpa_total("fnma", &manufactured, 2025).unwrap();
    assert_eq!(with_mfg - base, 50, "manufactured home LLPA adds 50 bps");
}

#[test]
fn test_llpa_second_home_adds_premium() {
    let primary = purchase_primary(760, 8000);
    let second = LlpaInput {
        occupancy: "second_home".to_owned(),
        loan_purpose: "purchase".to_owned(),
        ..primary.clone()
    };
    let prim_llpa = store().llpa_total("fnma", &primary, 2025).unwrap();
    let sec_llpa = store().llpa_total("fnma", &second, 2025).unwrap();
    assert!(sec_llpa > prim_llpa, "second home LLPA > primary LLPA");
}

#[test]
fn test_llpa_cash_out_adds_premium() {
    let purchase = purchase_primary(760, 7800);
    let cash_out = LlpaInput {
        loan_purpose: "cash_out_refi".to_owned(),
        ..purchase.clone()
    };
    let purch_llpa = store().llpa_total("fnma", &purchase, 2025).unwrap();
    let co_llpa = store().llpa_total("fnma", &cash_out, 2025).unwrap();
    assert!(co_llpa > purch_llpa, "cash-out refi LLPA > purchase LLPA");
}

#[test]
fn test_llpa_year_fallback() {
    let input = purchase_primary(720, 9200);
    let r2024 = store().llpa_total("fnma", &input, 2024).unwrap();
    let r2030 = store().llpa_total("fnma", &input, 2030).unwrap();
    assert_eq!(r2024, r2030);
}

// ════════════════════════════════════════════════════════════════════════════
// Rate Sheet
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rate_sheet_found_for_lender() {
    let sheet = store().rate_sheet("lndr_abc_mortgage").unwrap().unwrap();
    assert_eq!(sheet.lender_id, "lndr_abc_mortgage");
    assert!(!sheet.entries.is_empty());
}

#[test]
fn test_rate_sheet_unknown_lender_returns_none() {
    let sheet = store().rate_sheet("lndr_nonexistent").unwrap();
    assert!(sheet.is_none());
}

#[test]
fn test_rate_sheet_find_product() {
    let sheet = store().rate_sheet("lndr_abc_mortgage").unwrap().unwrap();
    let entry = sheet.find("conv_30yr_fixed", 30).unwrap();
    assert_eq!(entry.product, "conv_30yr_fixed");
    assert!(entry.par_rate_bps > 0);
}

#[test]
fn test_rate_sheet_all_products_present() {
    let sheet = store().rate_sheet("lndr_abc_mortgage").unwrap().unwrap();
    for product in [
        "conv_30yr_fixed",
        "fha_30yr_fixed",
        "va_30yr_fixed",
        "usda_30yr_fixed",
    ] {
        assert!(
            sheet.find(product, 30).is_some(),
            "product {product} missing from rate sheet"
        );
    }
}

#[test]
fn test_rate_sheet_longer_lock_has_lower_price() {
    let sheet = store().rate_sheet("lndr_abc_mortgage").unwrap().unwrap();
    let lock_30 = sheet.find("conv_30yr_fixed", 30).unwrap();
    let lock_60 = sheet.find("conv_30yr_fixed", 60).unwrap();
    // Longer lock → lender charges more → lower price (more negative rebate or more discount)
    assert!(
        lock_60.price_at_par < lock_30.price_at_par,
        "60-day lock should be more expensive than 30-day"
    );
}

#[test]
fn test_rate_sheet_fha_rate_below_conv() {
    let sheet = store().rate_sheet("lndr_abc_mortgage").unwrap().unwrap();
    let conv = sheet.find("conv_30yr_fixed", 30).unwrap();
    let fha = sheet.find("fha_30yr_fixed", 30).unwrap();
    assert!(
        fha.par_rate_bps < conv.par_rate_bps,
        "FHA par rate should be below conventional (MIP provides lender coverage)"
    );
}
