use funnel::*;
use types::Occupancy;
#[test]
fn partial_json_deserializes_and_gates() {
    // only steps 1-2 present
    let j = r#"{"borrower_count":2,"property_use":"primary_residence"}"#;
    let p: PartialAnalysisInput = serde_json::from_str(j).unwrap();
    assert!(p.step_complete(WizardStep::BorrowerCount));
    assert!(p.step_complete(WizardStep::PropertyUse));
    assert!(!p.step_complete(WizardStep::BorrowerDetails)); // no borrowers yet
    assert_eq!(p.furthest_step(), Some(WizardStep::PropertyUse));
}
#[test]
fn borrower_details_needs_all_rows_complete() {
    let mut p = PartialAnalysisInput {
        borrower_count: Some(2),
        property_use: Some(Occupancy::PrimaryResidence),
        ..Default::default()
    };
    p.borrowers.push(BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: vec![types::CreditScore::new(740).unwrap()],
        va: None,
        annual_income: types::Cents::from_dollars(90_000),
    });
    assert!(!p.step_complete(WizardStep::BorrowerDetails)); // 1 of 2
    p.borrowers.push(BorrowerInput {
        occupancy: Occupancy::PrimaryResidence,
        credit_scores: vec![types::CreditScore::new(700).unwrap()],
        va: None,
        annual_income: types::Cents::from_dollars(60_000),
    });
    assert!(p.step_complete(WizardStep::BorrowerDetails)); // 2 of 2
}
