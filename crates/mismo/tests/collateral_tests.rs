//! Task 2.4 gate tests — collateral/property schema.
//!
//! Verifies `SubjectProperty::parse()` against the spreadsheet FHA scenario:
//!   Austin TX 78640, $459,000 appraised, PrimaryResidence, Detached SFR.

use mismo::{
    schema::collateral::{
        HoaDetail, MismoAddress, PropertyDetail, PropertyTaxDetail, SubjectProperty,
    },
    MismoError,
};
use types::{BasisPoints, Cents, FipsCode, Occupancy, PropertyType, StateCode};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn spreadsheet_address() -> MismoAddress {
    MismoAddress {
        street_number: Some("100".into()),
        street_dir_prefix: None,
        street_name: Some("Mockingbird".into()),
        street_type: Some("Ln".into()),
        street_dir_suffix: None,
        address_line: None,
        city: "Kyle".into(),
        state_code: "TX".into(),
        postal_code: "78640".into(),
        county_name: Some("Hays".into()),
        fips_code: None,
        fips_state: None,
        fips_county: None,
    }
}

fn spreadsheet_detail() -> PropertyDetail {
    PropertyDetail {
        property_structure_type: "Detached".into(),
        property_usage_type: "PrimaryResidence".into(),
        year_built: None,
        financed_unit_count: None,
    }
}

fn spreadsheet_subject_property() -> SubjectProperty {
    SubjectProperty {
        address: spreadsheet_address(),
        detail: spreadsheet_detail(),
        tax: None,
        hoa: None,
        estimated_value: Some("459000.00".into()),
        sales_contract_amount: Some("459000.00".into()),
        annual_hoi: None,
        hoi_zip_lookup: None,
    }
}

// ── State code ────────────────────────────────────────────────────────────────

#[test]
fn test_texas_state_code_parses() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.state, StateCode::TX);
}

#[test]
fn test_invalid_state_code_returns_error() {
    let mut sp = spreadsheet_subject_property();
    sp.address.state_code = "ZZ".into();
    let err = sp.parse().unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "StateCode",
            ..
        }
    ));
}

#[test]
fn test_state_code_case_insensitive() {
    let mut sp = spreadsheet_subject_property();
    sp.address.state_code = "tx".into();
    let p = sp.parse().unwrap();
    assert_eq!(p.state, StateCode::TX);
}

// ── FIPS code ─────────────────────────────────────────────────────────────────

#[test]
fn test_fips_code_5digit_string_parses() {
    let mut sp = spreadsheet_subject_property();
    sp.address.fips_code = Some("48209".into()); // TX=48, Hays=209
    let p = sp.parse().unwrap();
    use std::str::FromStr;
    assert_eq!(p.fips_code, FipsCode::from_str("48209").ok());
}

#[test]
fn test_fips_code_absent_is_none() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert!(p.fips_code.is_none());
}

#[test]
fn test_fips_state_and_county_components_derive_fips() {
    let mut sp = spreadsheet_subject_property();
    sp.address.fips_state = Some("48".into());
    sp.address.fips_county = Some("209".into());
    let p = sp.parse().unwrap();
    use std::str::FromStr;
    assert_eq!(p.fips_code, FipsCode::from_str("48209").ok());
}

// ── Valuation ─────────────────────────────────────────────────────────────────

#[test]
fn test_appraised_value_to_cents() {
    // $459,000.00 = 45,900,000 cents
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.appraised_value, Cents(45_900_000));
}

#[test]
fn test_sales_contract_amount_to_cents() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.sales_price, Some(Cents(45_900_000)));
}

#[test]
fn test_sales_price_absent_is_none() {
    let mut sp = spreadsheet_subject_property();
    sp.sales_contract_amount = None;
    let p = sp.parse().unwrap();
    assert!(p.sales_price.is_none());
}

#[test]
fn test_missing_appraised_value_returns_missing_element_error() {
    let mut sp = spreadsheet_subject_property();
    sp.estimated_value = None;
    let err = sp.parse().unwrap_err();
    assert!(matches!(
        err,
        MismoError::MissingElement {
            element: "PropertyEstimatedValueAmount"
        }
    ));
}

// ── Property type ─────────────────────────────────────────────────────────────

#[test]
fn test_detached_single_family_property_type() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.property_type, PropertyType::SingleFamilyDetached);
}

#[test]
fn test_attached_defaults_to_single_family_attached() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.property_structure_type = "Attached".into();
    let p = sp.parse().unwrap();
    assert_eq!(p.property_type, PropertyType::SingleFamilyAttached);
}

#[test]
fn test_condominium_property_type() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.property_structure_type = "Condominium".into();
    let p = sp.parse().unwrap();
    assert_eq!(p.property_type, PropertyType::Condominium);
}

#[test]
fn test_two_unit_property_type() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.property_structure_type = "2-Unit".into();
    let p = sp.parse().unwrap();
    assert_eq!(p.property_type, PropertyType::TwoUnit);
}

#[test]
fn test_invalid_property_type_returns_error() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.property_structure_type = "Treehouse".into();
    let err = sp.parse().unwrap_err();
    assert!(matches!(
        err,
        MismoError::InvalidEnum {
            element: "PropertyStructureType",
            ..
        }
    ));
}

// ── Occupancy ─────────────────────────────────────────────────────────────────

#[test]
fn test_primary_residence_occupancy() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.occupancy, Occupancy::PrimaryResidence);
}

#[test]
fn test_second_home_occupancy() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.property_usage_type = "SecondHome".into();
    let p = sp.parse().unwrap();
    assert_eq!(p.occupancy, Occupancy::SecondHome);
}

#[test]
fn test_investment_occupancy() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.property_usage_type = "Investor".into();
    let p = sp.parse().unwrap();
    assert_eq!(p.occupancy, Occupancy::Investment);
}

// ── Unit count ────────────────────────────────────────────────────────────────

#[test]
fn test_unit_count_defaults_to_one_when_absent() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.unit_count, 1);
}

#[test]
fn test_unit_count_two_parses() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.financed_unit_count = Some("2".into());
    let p = sp.parse().unwrap();
    assert_eq!(p.unit_count, 2);
}

#[test]
fn test_unit_count_zero_returns_error() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.financed_unit_count = Some("0".into());
    let err = sp.parse().unwrap_err();
    assert!(matches!(
        err,
        MismoError::OutOfRange {
            element: "FinancedUnitCount",
            ..
        }
    ));
}

#[test]
fn test_unit_count_five_returns_error() {
    let mut sp = spreadsheet_subject_property();
    sp.detail.financed_unit_count = Some("5".into());
    let err = sp.parse().unwrap_err();
    assert!(matches!(
        err,
        MismoError::OutOfRange {
            element: "FinancedUnitCount",
            ..
        }
    ));
}

// ── Address decomposition ─────────────────────────────────────────────────────

#[test]
fn test_address_decomposed_components_preserved() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert_eq!(p.city, "Kyle");
    assert_eq!(p.postal_code, "78640");
    assert_eq!(p.county_name.as_deref(), Some("Hays"));
    assert_eq!(p.street_number.as_deref(), Some("100"));
    assert_eq!(p.street_name.as_deref(), Some("Mockingbird"));
    assert_eq!(p.street_type.as_deref(), Some("Ln"));
}

#[test]
fn test_display_line_from_components() {
    let addr = spreadsheet_address();
    let line = addr.display_line();
    assert!(line.contains("100"), "should contain street number");
    assert!(line.contains("Mockingbird"), "should contain street name");
}

#[test]
fn test_display_line_falls_back_to_address_line_field() {
    let mut addr = spreadsheet_address();
    addr.street_number = None;
    addr.street_name = None;
    addr.address_line = Some("123 Oak St".into());
    assert_eq!(addr.display_line(), "123 Oak St");
}

// ── Tax data ──────────────────────────────────────────────────────────────────

#[test]
fn test_annual_tax_to_cents() {
    let mut sp = spreadsheet_subject_property();
    sp.tax = Some(PropertyTaxDetail {
        annual_amount: Some("10523.40".into()),
        tax_rate: None,
        tax_year: None,
        paid_in_arrears: Some("true".into()),
        seller_arrears_amount: None,
    });
    let p = sp.parse().unwrap();
    assert_eq!(p.annual_tax, Some(Cents(1_052_340)));
    assert!(p.taxes_in_arrears);
}

#[test]
fn test_tax_rate_to_basis_points() {
    let mut sp = spreadsheet_subject_property();
    sp.tax = Some(PropertyTaxDetail {
        annual_amount: None,
        tax_rate: Some("1.9".into()),
        tax_year: Some("2024".into()),
        paid_in_arrears: None,
        seller_arrears_amount: None,
    });
    let p = sp.parse().unwrap();
    assert_eq!(p.tax_rate, Some(BasisPoints(1900)));
    assert_eq!(p.tax_year, Some(2024));
}

#[test]
fn test_taxes_in_arrears_defaults_false_when_no_tax_block() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert!(!p.taxes_in_arrears);
    assert!(p.annual_tax.is_none());
}

#[test]
fn test_seller_tax_arrears_to_cents() {
    let mut sp = spreadsheet_subject_property();
    sp.tax = Some(PropertyTaxDetail {
        annual_amount: None,
        tax_rate: None,
        tax_year: None,
        paid_in_arrears: Some("true".into()),
        seller_arrears_amount: Some("5261.70".into()),
    });
    let p = sp.parse().unwrap();
    assert_eq!(p.seller_tax_arrears, Some(Cents(526_170)));
}

// ── HOI data ──────────────────────────────────────────────────────────────────

#[test]
fn test_annual_hoi_to_cents() {
    let mut sp = spreadsheet_subject_property();
    sp.annual_hoi = Some("1840.00".into());
    let p = sp.parse().unwrap();
    assert_eq!(p.annual_hoi, Some(Cents(184_000)));
}

#[test]
fn test_annual_hoi_absent_is_none() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert!(p.annual_hoi.is_none());
}

// ── HOA data ──────────────────────────────────────────────────────────────────

#[test]
fn test_hoa_not_present_returns_defaults() {
    let p = spreadsheet_subject_property().parse().unwrap();
    assert!(!p.hoa_yn);
    assert!(p.hoa_monthly.is_none());
    assert!(p.hoa_annual.is_none());
    assert!(p.hoa_transfer_fee.is_none());
    assert!(p.hoa_working_capital.is_none());
}

#[test]
fn test_hoa_monthly_to_cents() {
    let mut sp = spreadsheet_subject_property();
    sp.hoa = Some(HoaDetail {
        hoa_yn: Some("true".into()),
        monthly_fee: Some("250.00".into()),
        fee_frequency: Some("Monthly".into()),
        transfer_fee: None,
        working_capital_fee: None,
        annual_dues_corrected: None,
    });
    let p = sp.parse().unwrap();
    assert!(p.hoa_yn);
    assert_eq!(p.hoa_monthly, Some(Cents(25_000)));
}

#[test]
fn test_hoa_annual_computed_from_monthly() {
    let mut sp = spreadsheet_subject_property();
    sp.hoa = Some(HoaDetail {
        hoa_yn: Some("true".into()),
        monthly_fee: Some("250.00".into()),
        fee_frequency: None,
        transfer_fee: None,
        working_capital_fee: None,
        annual_dues_corrected: None,
    });
    let p = sp.parse().unwrap();
    // 250.00 × 12 = 3000.00 = 300_000 cents
    assert_eq!(p.hoa_annual, Some(Cents(300_000)));
}

#[test]
fn test_hoa_annual_explicit_overrides_monthly_calculation() {
    let mut sp = spreadsheet_subject_property();
    sp.hoa = Some(HoaDetail {
        hoa_yn: Some("true".into()),
        monthly_fee: Some("250.00".into()),
        fee_frequency: None,
        transfer_fee: None,
        working_capital_fee: None,
        annual_dues_corrected: Some("2800.00".into()), // override
    });
    let p = sp.parse().unwrap();
    assert_eq!(p.hoa_annual, Some(Cents(280_000)));
}

#[test]
fn test_hoa_transfer_fee_to_cents() {
    let mut sp = spreadsheet_subject_property();
    sp.hoa = Some(HoaDetail {
        hoa_yn: Some("true".into()),
        monthly_fee: None,
        fee_frequency: None,
        transfer_fee: Some("175.00".into()),
        working_capital_fee: Some("0.00".into()),
        annual_dues_corrected: None,
    });
    let p = sp.parse().unwrap();
    assert_eq!(p.hoa_transfer_fee, Some(Cents(17_500)));
    assert_eq!(p.hoa_working_capital, Some(Cents(0)));
}

// ── XML round-trip ────────────────────────────────────────────────────────────

#[test]
fn test_subject_property_xml_roundtrip() {
    let mut sp = spreadsheet_subject_property();
    sp.tax = Some(PropertyTaxDetail {
        annual_amount: Some("10523.40".into()),
        tax_rate: Some("1.9".into()),
        tax_year: Some("2024".into()),
        paid_in_arrears: Some("true".into()),
        seller_arrears_amount: None,
    });
    sp.hoa = Some(HoaDetail {
        hoa_yn: Some("true".into()),
        monthly_fee: Some("250.00".into()),
        fee_frequency: Some("Monthly".into()),
        transfer_fee: Some("175.00".into()),
        working_capital_fee: None,
        annual_dues_corrected: None,
    });
    sp.annual_hoi = Some("1840.00".into());

    let xml = mismo::xml::serialize::to_xml(&sp).unwrap();
    assert!(xml.contains("459000.00"));
    assert!(xml.contains("TX"));
    assert!(xml.contains("78640"));
    assert!(xml.contains("10523.40"));
    assert!(xml.contains("250.00"));
    assert!(xml.contains("175.00"));

    let restored: SubjectProperty = mismo::xml::parse::from_xml(&xml).unwrap();
    let parsed = restored.parse().unwrap();
    assert_eq!(parsed.state, StateCode::TX);
    assert_eq!(parsed.appraised_value, Cents(45_900_000));
    assert_eq!(parsed.annual_tax, Some(Cents(1_052_340)));
    assert!(parsed.hoa_yn);
    assert_eq!(parsed.hoa_monthly, Some(Cents(25_000)));
}

#[test]
fn test_parse_subject_property_from_xml_string() {
    let xml = r#"<SUBJECT_PROPERTY>
        <ADDRESS>
            <StreetNumberText>100</StreetNumberText>
            <StreetNameText>Mockingbird</StreetNameText>
            <StreetSuffixText>Ln</StreetSuffixText>
            <CityName>Kyle</CityName>
            <StateCode>TX</StateCode>
            <PostalCode>78640</PostalCode>
            <CountyName>Hays</CountyName>
            <FIPSCode>48209</FIPSCode>
        </ADDRESS>
        <PROPERTY_DETAIL>
            <GSEProjectClassificationType>Detached</GSEProjectClassificationType>
            <PropertyUsageType>PrimaryResidence</PropertyUsageType>
        </PROPERTY_DETAIL>
        <PropertyEstimatedValueAmount>459000.00</PropertyEstimatedValueAmount>
        <SalesContractAmount>459000.00</SalesContractAmount>
    </SUBJECT_PROPERTY>"#;

    let sp: SubjectProperty = mismo::xml::parse::from_xml(xml).unwrap();
    let p = sp.parse().unwrap();

    assert_eq!(p.state, StateCode::TX);
    assert_eq!(p.postal_code, "78640");
    assert_eq!(p.county_name.as_deref(), Some("Hays"));
    assert_eq!(p.property_type, PropertyType::SingleFamilyDetached);
    assert_eq!(p.occupancy, Occupancy::PrimaryResidence);
    assert_eq!(p.appraised_value, Cents(45_900_000));
    assert_eq!(p.sales_price, Some(Cents(45_900_000)));
    assert_eq!(p.unit_count, 1);
    assert!(!p.taxes_in_arrears);
    assert!(!p.hoa_yn);
    // FIPS from "48209"
    assert!(p.fips_code.is_some());
}
