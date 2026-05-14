//! Task 2.10 gate tests — MESSAGE root and ParsedDeal.
//!
//! Every test builds a MismoMessage in memory, calls parse_all(), and
//! asserts against the FHA purchase reference values for Kyle TX.

use mismo::schema::message::{
    ClosingCostAmounts, MismoClosingCostContainer, MismoCollateral, MismoCollaterals, MismoDeal,
    MismoDealSet, MismoDealSets, MismoDeals, MismoLoan, MismoLoans, MismoMessage, MismoParties,
    MismoParty,
};
use mismo::schema::{
    aus::{AusSystems, MismoAus, MismoQualification},
    closing_cost::MismoClosingCostFee,
    collateral::{MismoAddress, PropertyDetail, SubjectProperty},
    lender_comp::LenderComp,
    loan_terms::{Amortization, MortgageTerms},
    mi::MiDataDetail,
    party::BorrowerDetail,
};
use types::{BasisPoints, Cents, DtiBasisPoints, StateCode};

// ── Fixture helpers ───────────────────────────────────────────────────────────

fn fha_mortgage_terms() -> MortgageTerms {
    MortgageTerms {
        base_loan_amount: "434443.00".into(),
        loan_amount_with_financed_mi: Some("442046.00".into()),
        note_rate_percent: "6.375".into(),
        loan_term_months_count: "360".into(),
        mortgage_type: "FHA".into(),
        lien_priority_type: "FirstLien".into(),
        loan_purpose_type: "Purchase".into(),
        holding_period_months: None,
        days_until_closing: None,
        seller_concession_amount: None,
        seller_pays_owners_title: None,
        waive_escrow: None,
        temp_buydown: None,
        subordinate_financing: None,
        high_balance: None,
    }
}

fn fixed_amortization() -> Amortization {
    Amortization {
        amortization_type: "Fixed".into(),
    }
}

fn fha_mi() -> MiDataDetail {
    MiDataDetail {
        mi_type: "BorrowerPaid".into(),
        mi_program_type: "FHAUpfrontMIP".into(),
        upfront_rate_percent: Some("1.75".into()),
        upfront_amount: Some("7602.7525".into()),
        financed: Some("true".into()),
        monthly_rate_percent: Some("0.55".into()),
        cancellation_ltv_percent: Some("0.0".into()),
        required_months: Some("24".into()),
        calculation_method: Some("Declining".into()),
        remittance_type: None,
    }
}

fn broker_comp() -> LenderComp {
    LenderComp {
        amount: "4899.24".into(),
        comp_bps: Some("112.76".into()),
        comp_type: "BorrowerPaid".into(),
        disclose_in_section_a: Some("true".into()),
        cap_amount: None,
    }
}

fn du_aus() -> AusSystems {
    AusSystems {
        systems: vec![MismoAus {
            system_type: "DesktopUnderwriter".into(),
            recommendation: Some("Approve/Eligible".into()),
            case_id: Some("DU-2025-99999".into()),
        }],
    }
}

fn fha_qualification() -> MismoQualification {
    MismoQualification {
        qualifying_rate: Some("6.375".into()),
        housing_ratio: Some("28.50".into()),
        total_dti: Some("43.00".into()),
    }
}

fn kyle_tx_address() -> MismoAddress {
    MismoAddress {
        city: "Kyle".into(),
        state_code: "TX".into(),
        postal_code: "78640".into(),
        address_line: None,
        street_number: None,
        street_name: None,
        street_type: None,
        street_dir_prefix: None,
        street_dir_suffix: None,
        county_name: Some("Hays".into()),
        fips_code: Some("48209".into()),
        fips_state: None,
        fips_county: None,
    }
}

fn fha_property_detail() -> PropertyDetail {
    PropertyDetail {
        property_structure_type: "Detached".into(),
        property_usage_type: "PrimaryResidence".into(),
        year_built: None,
        financed_unit_count: None,
        gross_living_area: None,
    }
}

fn kyle_tx_subject_property() -> SubjectProperty {
    SubjectProperty {
        address: kyle_tx_address(),
        detail: fha_property_detail(),
        tax: None,
        hoa: None,
        estimated_value: Some("459000.00".into()),
        sales_contract_amount: Some("459000.00".into()),
        annual_hoi: None,
        hoi_zip_lookup: None,
    }
}

fn fha_borrower() -> BorrowerDetail {
    BorrowerDetail {
        credit_score: Some("720".into()),
        monthly_income: Some("8500.00".into()),
        first_time_homebuyer: None,
        experienced_homebuyer: None,
        self_employed: None,
        va_eligible: None,
        va_first_use: None,
        va_full_entitlement: None,
        va_outstanding_balance: None,
        va_fee_exempt: None,
        usda_household_size: None,
        usda_adult_household_income: None,
        affordable_lending_eligible: None,
        affordable_lending_program: None,
        max_cash_to_close: None,
        max_monthly_pitia: None,
    }
}

fn fha_message() -> MismoMessage {
    MismoMessage {
        deal_sets: MismoDealSets {
            deal_set: MismoDealSet {
                deals: MismoDeals {
                    deal: MismoDeal {
                        loans: MismoLoans {
                            loan: MismoLoan {
                                mortgage_terms: fha_mortgage_terms(),
                                amortization: fixed_amortization(),
                                mi_data_detail: Some(fha_mi()),
                                origination_fee_detail: Some(broker_comp()),
                                closing_cost: None,
                                aus_systems: Some(du_aus()),
                                qualification: Some(fha_qualification()),
                            },
                        },
                        parties: MismoParties {
                            parties: vec![MismoParty {
                                borrower_detail: Some(fha_borrower()),
                            }],
                            closing_context: None,
                        },
                        collaterals: MismoCollaterals {
                            collateral: MismoCollateral {
                                subject_property: kyle_tx_subject_property(),
                            },
                        },
                    },
                },
            },
        },
    }
}

// ── Loan terms ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_loan_base_amount() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(43_444_300));
}

#[test]
fn test_parse_all_loan_adjusted_amount() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(
        deal.loan_terms.adjusted_loan_amount,
        Some(Cents(44_204_600))
    );
}

#[test]
fn test_parse_all_note_rate() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.loan_terms.note_rate, BasisPoints(6375));
}

#[test]
fn test_parse_all_term_months() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.loan_terms.term.0, 360);
}

// ── Collateral ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_state_tx() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.collateral.state, StateCode::TX);
}

#[test]
fn test_parse_all_appraised_value() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.collateral.appraised_value, Cents(45_900_000));
}

#[test]
fn test_parse_all_fips_code() {
    let deal = fha_message().parse_all().unwrap();
    let fips = deal.collateral.fips_code.unwrap();
    assert_eq!(fips.to_string(), "48209");
}

// ── Parties ───────────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_credit_score() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.parties.qualifying_credit_score.unwrap().0, 720);
}

#[test]
fn test_parse_all_borrower_count() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.parties.borrower_count, 1);
}

#[test]
fn test_parse_all_coborrower_lower_of_two_credit_score() {
    let mut msg = fha_message();
    msg.deal_sets
        .deal_set
        .deals
        .deal
        .parties
        .parties
        .push(MismoParty {
            borrower_detail: Some(BorrowerDetail {
                credit_score: Some("700".into()),
                monthly_income: Some("3000.00".into()),
                first_time_homebuyer: None,
                experienced_homebuyer: None,
                self_employed: None,
                va_eligible: None,
                va_first_use: None,
                va_full_entitlement: None,
                va_outstanding_balance: None,
                va_fee_exempt: None,
                usda_household_size: None,
                usda_adult_household_income: None,
                affordable_lending_eligible: None,
                affordable_lending_program: None,
                max_cash_to_close: None,
                max_monthly_pitia: None,
            }),
        });
    let deal = msg.parse_all().unwrap();
    assert_eq!(deal.parties.borrower_count, 2);
    // Lower-of-two: min(720, 700) = 700
    assert_eq!(deal.parties.qualifying_credit_score.unwrap().0, 700);
}

#[test]
fn test_parse_all_no_borrower_returns_error() {
    let mut msg = fha_message();
    msg.deal_sets.deal_set.deals.deal.parties.parties.clear();
    assert!(msg.parse_all().is_err());
}

// ── MI ────────────────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_mi_present() {
    let deal = fha_message().parse_all().unwrap();
    assert!(deal.mi.is_some());
}

#[test]
fn test_parse_all_mi_upfront_amount() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.mi.unwrap().upfront_amount, Some(Cents(760_275)));
}

#[test]
fn test_parse_all_mi_monthly_rate() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.mi.unwrap().monthly_annual_rate, Some(BasisPoints(55)));
}

#[test]
fn test_parse_all_mi_life_of_loan() {
    let deal = fha_message().parse_all().unwrap();
    assert!(deal.mi.unwrap().is_life_of_loan);
}

#[test]
fn test_parse_all_mi_absent_is_none() {
    let mut msg = fha_message();
    msg.deal_sets.deal_set.deals.deal.loans.loan.mi_data_detail = None;
    assert!(msg.parse_all().unwrap().mi.is_none());
}

// ── Lender comp ───────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_lender_comp_amount() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.lender_comp.unwrap().amount, Cents(489_924));
}

#[test]
fn test_parse_all_lender_comp_absent_is_none() {
    let mut msg = fha_message();
    msg.deal_sets
        .deal_set
        .deals
        .deal
        .loans
        .loan
        .origination_fee_detail = None;
    assert!(msg.parse_all().unwrap().lender_comp.is_none());
}

// ── Closing costs ─────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_closing_costs_empty_when_absent() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.closing_costs.section_a.len(), 0);
}

#[test]
fn test_parse_all_closing_costs_with_fee() {
    let mut msg = fha_message();
    msg.deal_sets.deal_set.deals.deal.loans.loan.closing_cost = Some(MismoClosingCostContainer {
        amounts: Some(ClosingCostAmounts {
            fees: vec![MismoClosingCostFee {
                section_type: "LoanCosts_OriginationCharges".into(),
                description: "Application Fee".into(),
                total_amount: Some("1095.00".into()),
                borrower_amount: Some("1095.00".into()),
                seller_amount: None,
                lender_amount: None,
                paid_by: Some("Borrower".into()),
                financed: Some("false".into()),
                apr_affected: Some("true".into()),
                sequence_number: Some("1".into()),
                fee_type_code: Some("ApplicationFee".into()),
            }],
        }),
    });
    let deal = msg.parse_all().unwrap();
    assert_eq!(deal.closing_costs.section_a.len(), 1);
    assert_eq!(deal.closing_costs.section_a_borrower(), Cents(109_500));
}

// ── AUS ───────────────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_aus_approve_eligible() {
    let deal = fha_message().parse_all().unwrap();
    assert!(deal.aus.unwrap().is_approvable());
}

#[test]
fn test_parse_all_aus_case_id_preserved() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(deal.aus.unwrap().case_id.as_deref(), Some("DU-2025-99999"));
}

#[test]
fn test_parse_all_aus_absent_is_none() {
    let mut msg = fha_message();
    msg.deal_sets.deal_set.deals.deal.loans.loan.aus_systems = None;
    assert!(msg.parse_all().unwrap().aus.is_none());
}

// ── Qualification ─────────────────────────────────────────────────────────────

#[test]
fn test_parse_all_qualifying_rate() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(
        deal.qualification.as_ref().unwrap().qualifying_rate,
        Some(BasisPoints(6375))
    );
}

#[test]
fn test_parse_all_total_dti() {
    let deal = fha_message().parse_all().unwrap();
    assert_eq!(
        deal.qualification.as_ref().unwrap().total_dti,
        Some(DtiBasisPoints::new(4300))
    );
}

#[test]
fn test_parse_all_qualification_absent_is_none() {
    let mut msg = fha_message();
    msg.deal_sets.deal_set.deals.deal.loans.loan.qualification = None;
    assert!(msg.parse_all().unwrap().qualification.is_none());
}

// ── XML round-trips ───────────────────────────────────────────────────────────

#[test]
fn test_message_xml_roundtrip_loan_amount() {
    let xml = fha_message().to_xml().unwrap();
    assert!(xml.contains("434443.00"));
    assert!(xml.contains("Kyle"));
    assert!(xml.contains("48209"));
    let deal = MismoMessage::from_xml(&xml).unwrap().parse_all().unwrap();
    assert_eq!(deal.loan_terms.base_loan_amount, Cents(43_444_300));
}

#[test]
fn test_message_xml_roundtrip_preserves_mi() {
    let xml = fha_message().to_xml().unwrap();
    let deal = MismoMessage::from_xml(&xml).unwrap().parse_all().unwrap();
    assert_eq!(deal.mi.unwrap().upfront_amount, Some(Cents(760_275)));
}

#[test]
fn test_message_xml_roundtrip_preserves_state() {
    let xml = fha_message().to_xml().unwrap();
    let deal = MismoMessage::from_xml(&xml).unwrap().parse_all().unwrap();
    assert_eq!(deal.collateral.state, StateCode::TX);
}
