//! Task 4.15 — Conventional MI coverage requirements and National MI monthly rates.

use ref_data::{ConvMiInput, ConvMiProgram, JsonFileStore, MiRateInput, RefDataStore};
use types::Cents;

fn store() -> JsonFileStore {
    JsonFileStore::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

fn conv_std(term_months: u16, ltv_bps: u32) -> ConvMiInput {
    ConvMiInput {
        program: ConvMiProgram::Standard,
        term_months,
        ltv_bps,
        is_arm: false,
        is_standard_manufactured: false,
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Conventional MI Coverage Requirements (Fannie Mae / Freddie Mac)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_conv_mi_30yr_standard_ltv_82_needs_12pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(360, 8200), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 12);
}

#[test]
fn test_conv_mi_30yr_standard_ltv_88_needs_25pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(360, 8800), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 25);
}

#[test]
fn test_conv_mi_30yr_standard_ltv_92_needs_30pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(360, 9200), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 30);
}

#[test]
fn test_conv_mi_30yr_standard_ltv_96_needs_35pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(360, 9600), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 35);
}

#[test]
fn test_conv_mi_15yr_fixed_ltv_82_needs_6pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(180, 8200), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 6, "≤20yr term has lower standard coverage");
}

#[test]
fn test_conv_mi_15yr_fixed_ltv_88_needs_12pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(180, 8800), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 12);
}

#[test]
fn test_conv_mi_15yr_fixed_ltv_92_needs_25pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(180, 9200), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 25);
}

#[test]
fn test_conv_mi_15yr_fixed_ltv_96_needs_35pct() {
    let c = store()
        .conv_mi_coverage(&conv_std(180, 9600), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 35);
}

#[test]
fn test_conv_mi_homeready_30yr_ltv_96_only_needs_25pct() {
    let input = ConvMiInput {
        program: ConvMiProgram::HomeReady,
        term_months: 360,
        ltv_bps: 9600,
        is_arm: false,
        is_standard_manufactured: false,
    };
    let c = store().conv_mi_coverage(&input, 2025).unwrap();
    assert_eq!(
        c.standard_pct, 25,
        "HomeReady 95-97% LTV: 25% standard, not 35%"
    );
}

#[test]
fn test_conv_mi_home_possible_30yr_ltv_96_only_needs_25pct() {
    let input = ConvMiInput {
        program: ConvMiProgram::HomePossible,
        term_months: 360,
        ltv_bps: 9600,
        is_arm: false,
        is_standard_manufactured: false,
    };
    let c = store().conv_mi_coverage(&input, 2025).unwrap();
    assert_eq!(c.standard_pct, 25);
}

#[test]
fn test_conv_mi_minimum_coverage_with_llpa() {
    let c = store()
        .conv_mi_coverage(&conv_std(360, 9200), 2025)
        .unwrap();
    assert_eq!(c.standard_pct, 30);
    assert_eq!(c.minimum_pct, 16, "minimum coverage option: 16% with LLPA");
    assert!(
        c.llpa_with_minimum,
        "minimum coverage triggers LLPA for 90-95% LTV"
    );
}

#[test]
fn test_conv_mi_arm_follows_gt20yr_fixed_table() {
    let arm = ConvMiInput {
        program: ConvMiProgram::Standard,
        term_months: 360,
        ltv_bps: 9600,
        is_arm: true,
        is_standard_manufactured: false,
    };
    let arm_cov = store().conv_mi_coverage(&arm, 2025).unwrap();
    let fixed_gt20 = conv_std(360, 9600);
    let fixed_cov = store().conv_mi_coverage(&fixed_gt20, 2025).unwrap();
    assert_eq!(
        arm_cov.standard_pct, fixed_cov.standard_pct,
        "ARMs use same coverage table as >20yr fixed"
    );
}

#[test]
fn test_conv_mi_manufactured_home_30pct_at_90_95() {
    let input = ConvMiInput {
        program: ConvMiProgram::Standard,
        term_months: 360,
        ltv_bps: 9200,
        is_arm: false,
        is_standard_manufactured: true,
    };
    let c = store().conv_mi_coverage(&input, 2025).unwrap();
    assert_eq!(
        c.standard_pct, 30,
        "standard manufactured home: same as >20yr fixed 90-95%"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// National MI Monthly Premium Rates
// ════════════════════════════════════════════════════════════════════════════

fn nmi_rate(ltv_bps: u32, coverage_pct: u8, fico: u16, term_months: u16) -> MiRateInput {
    MiRateInput {
        ltv_bps,
        coverage_pct,
        fico,
        term_months,
        is_non_fixed: false,
    }
}

#[test]
fn test_nmi_monthly_30yr_ltv96_cov35_fico760_is_58bps() {
    let r = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 760, 360), 2025)
        .unwrap();
    assert_eq!(
        r, 58,
        "NMI >20yr, 95.01-97%, 35% coverage, 760+ FICO → 58 bps"
    );
}

#[test]
fn test_nmi_monthly_30yr_ltv96_cov35_fico639_is_186bps() {
    let r = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 639, 360), 2025)
        .unwrap();
    assert_eq!(r, 186, "620-639 FICO bracket highest rate");
}

#[test]
fn test_nmi_monthly_30yr_ltv92_cov25_fico760_is_34bps() {
    let r = store()
        .mi_monthly_rate("nmi", &nmi_rate(9200, 25, 760, 360), 2025)
        .unwrap();
    assert_eq!(r, 34);
}

#[test]
fn test_nmi_monthly_30yr_ltv88_cov25_fico700_is_55bps() {
    let r = store()
        .mi_monthly_rate("nmi", &nmi_rate(8800, 25, 700, 360), 2025)
        .unwrap();
    assert_eq!(r, 55);
}

#[test]
fn test_nmi_monthly_15yr_lt20yr_lower_than_30yr() {
    let rate_30yr = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 760, 360), 2025)
        .unwrap();
    let rate_15yr = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 760, 180), 2025)
        .unwrap();
    assert!(
        rate_15yr < rate_30yr,
        "≤20yr fixed rate ({rate_15yr}) should be lower than >20yr ({rate_30yr})"
    );
}

#[test]
fn test_nmi_monthly_non_fixed_multiplier_125pct() {
    let fixed_input = nmi_rate(9600, 35, 760, 360);
    let non_fixed_input = MiRateInput {
        is_non_fixed: true,
        ..fixed_input.clone()
    };
    let fixed_rate = store().mi_monthly_rate("nmi", &fixed_input, 2025).unwrap();
    let nf_rate = store()
        .mi_monthly_rate("nmi", &non_fixed_input, 2025)
        .unwrap();
    let expected_nf = ((u32::from(fixed_rate) * 125 + 50) / 100) as u16;
    assert_eq!(nf_rate, expected_nf, "non-fixed = fixed × 1.25, rounded");
}

#[test]
fn test_nmi_monthly_floor_is_14bps() {
    // Low LTV / low coverage — rate should still be at least 14 bps
    let r = store()
        .mi_monthly_rate("nmi", &nmi_rate(8200, 6, 760, 180), 2025)
        .unwrap();
    assert!(r >= 14, "NMI floor is 0.14% (14 bps), got {r}");
}

#[test]
fn test_nmi_monthly_estimate_from_table() {
    use ref_data::conv_mi::MiMonthlyTable;
    let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let json_path = data_dir.join("mi_rates_nmi_monthly_2022.json");
    let table: MiMonthlyTable =
        serde_json::from_str(&std::fs::read_to_string(json_path).unwrap()).unwrap();
    let input = nmi_rate(9200, 25, 760, 360);
    // monthly on $300,000: 34 bps annual → $300,000 × 0.0034 / 12 = $85/mo
    let monthly = table.monthly_mi(&input, Cents(30_000_000)).unwrap();
    assert!(monthly.0 > 0);
    let annual = 30_000_000i128 * 34 / 10_000;
    let expected_monthly = ((annual + 11) / 12) as i64;
    assert_eq!(monthly.0, expected_monthly);
}

#[test]
fn test_nmi_monthly_fico_band_760_is_index_0() {
    // 760+ is the best FICO band; ensure we correctly index it
    let r760 = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 760, 360), 2025)
        .unwrap();
    let r800 = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 800, 360), 2025)
        .unwrap();
    assert_eq!(r760, r800, "760 and 800 should use same 760+ FICO band");
}

#[test]
fn test_nmi_rate_year_fallback() {
    let r2022 = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 760, 360), 2022)
        .unwrap();
    let r2030 = store()
        .mi_monthly_rate("nmi", &nmi_rate(9600, 35, 760, 360), 2030)
        .unwrap();
    assert_eq!(r2022, r2030, "year fallback uses latest available data");
}
