//! MISMO 3.4 collateral/property schema — `SUBJECT_PROPERTY`, `ADDRESS`,
//! `PROPERTY_DETAIL`, `PROPERTY_TAX`, and `HOA_DETAIL`.
//!
//! # Document location
//!
//! ```text
//! MESSAGE/DEAL_SETS/DEAL_SET/DEALS/DEAL/COLLATERALS/COLLATERAL/
//!   └── SUBJECT_PROPERTY
//!         ├── ADDRESS                  ← MismoAddress
//!         ├── PROPERTY_DETAIL         ← PropertyDetail
//!         ├── PROPERTY_TAX            ← PropertyTaxDetail (optional)
//!         └── HOA_DETAIL              ← HoaDetail (optional)
//! ```
//!
//! # Reference values — FHA purchase, Kyle TX, $459k, 6.375%, 30yr fixed
//!
//! | Field | XML value | Parsed value |
//! |---|---|---|
//! | `StateCode` | `"TX"` | `StateCode::TX` |
//! | `PostalCode` | `"78640"` | `"78640"` |
//! | `PropertyEstimatedValueAmount` | `"459000.00"` | `Cents(45_900_000)` |
//! | `SalesContractAmount` | `"459000.00"` | `Cents(45_900_000)` |
//! | `PropertyUsageType` | `"PrimaryResidence"` | `Occupancy::PrimaryResidence` |
//! | `GSEProjectClassificationType` | `"Detached"` | `PropertyType::SingleFamilyDetached` |

use std::str::FromStr;
use types::{BasisPoints, Cents, FipsCode, Occupancy, PropertyType, StateCode};

// ── Parsing helpers (shared with loan_terms) ─────────────────────────────────

fn parse_cents(s: &str, element: &'static str) -> crate::Result<Cents> {
    use rust_decimal::Decimal;
    let decimal = Decimal::from_str(s.trim()).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is not a valid decimal amount"),
    })?;
    Cents::from_decimal_dollars(decimal).map_err(|_| crate::MismoError::OutOfRange {
        element,
        detail: format!("'{s}' is out of range for Cents (i64)"),
    })
}

fn parse_optional_cents(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<Cents>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => parse_cents(s, element).map(Some),
    }
}

fn parse_optional_bps(
    opt: &Option<String>,
    element: &'static str,
) -> crate::Result<Option<BasisPoints>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => BasisPoints::from_percentage_str(s).map(Some).map_err(|_| {
            crate::MismoError::OutOfRange {
                element,
                detail: format!("'{s}' is not a valid rate percentage"),
            }
        }),
    }
}

fn parse_bool_indicator(opt: &Option<String>) -> bool {
    opt.as_deref()
        .map(|s| matches!(s.trim().to_lowercase().as_str(), "true" | "yes" | "1"))
        .unwrap_or(false)
}

fn parse_optional_u16(opt: &Option<String>, element: &'static str) -> crate::Result<Option<u16>> {
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => s
            .trim()
            .parse::<u16>()
            .map(Some)
            .map_err(|_| crate::MismoError::OutOfRange {
                element,
                detail: format!("'{s}' is not a valid 16-bit integer"),
            }),
    }
}

fn parse_unit_count(opt: &Option<String>) -> crate::Result<u8> {
    match opt.as_deref() {
        None | Some("") => Ok(1),
        Some(s) => {
            let n: u8 = s
                .trim()
                .parse()
                .map_err(|_| crate::MismoError::OutOfRange {
                    element: "FinancedUnitCount",
                    detail: format!("'{s}' is not a valid unit count"),
                })?;
            if n == 0 || n > 4 {
                return Err(crate::MismoError::OutOfRange {
                    element: "FinancedUnitCount",
                    detail: format!("{n} is outside the valid range 1–4"),
                });
            }
            Ok(n)
        }
    }
}

// ── ADDRESS ───────────────────────────────────────────────────────────────────

/// MISMO 3.4 `ADDRESS` element — full decomposed address.
///
/// Either the decomposed street components (`street_number`, `street_name`,
/// etc.) or the single `address_line` field may be populated; both are
/// optional so the struct deserializes regardless of which form the sender
/// uses.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "ADDRESS")]
pub struct MismoAddress {
    /// Street number. e.g. `"1234"`.
    #[serde(rename = "StreetNumberText", skip_serializing_if = "Option::is_none")]
    pub street_number: Option<String>,

    /// Pre-directional. e.g. `"N"`, `"SW"`.
    #[serde(
        rename = "StreetPreDirectionalText",
        skip_serializing_if = "Option::is_none"
    )]
    pub street_dir_prefix: Option<String>,

    /// Street name. e.g. `"Main"`.
    #[serde(rename = "StreetNameText", skip_serializing_if = "Option::is_none")]
    pub street_name: Option<String>,

    /// Street type. e.g. `"St"`, `"Ave"`, `"Blvd"`.
    #[serde(rename = "StreetSuffixText", skip_serializing_if = "Option::is_none")]
    pub street_type: Option<String>,

    /// Post-directional. e.g. `"NE"`, `"SW"`.
    #[serde(
        rename = "StreetPostDirectionalText",
        skip_serializing_if = "Option::is_none"
    )]
    pub street_dir_suffix: Option<String>,

    /// Full address line when not decomposed. e.g. `"1234 Main St"`.
    #[serde(rename = "AddressLineText", skip_serializing_if = "Option::is_none")]
    pub address_line: Option<String>,

    /// City name. Required.
    #[serde(rename = "CityName")]
    pub city: String,

    /// Two-letter state abbreviation. e.g. `"TX"`.
    #[serde(rename = "StateCode")]
    pub state_code: String,

    /// ZIP or ZIP+4. e.g. `"78640"` or `"78640-1234"`.
    #[serde(rename = "PostalCode")]
    pub postal_code: String,

    /// County name. e.g. `"Hays"`.
    #[serde(rename = "CountyName", skip_serializing_if = "Option::is_none")]
    pub county_name: Option<String>,

    /// Full 5-digit FIPS code. e.g. `"48209"` (TX=48, Hays County=209).
    /// When present, used directly; otherwise resolved from state+county
    /// in the `ingest` crate (Epic 4).
    #[serde(rename = "FIPSCode", skip_serializing_if = "Option::is_none")]
    pub fips_code: Option<String>,

    /// FIPS state portion only. e.g. `"48"`.
    #[serde(rename = "FIPSStateCode", skip_serializing_if = "Option::is_none")]
    pub fips_state: Option<String>,

    /// FIPS county portion only. e.g. `"209"`.
    #[serde(rename = "FIPSCountyCode", skip_serializing_if = "Option::is_none")]
    pub fips_county: Option<String>,
}

impl MismoAddress {
    /// Derive a `FipsCode` from this address, if possible.
    ///
    /// Priority:
    /// 1. `fips_code` — parse the 5-digit FIPS string directly
    /// 2. `fips_state` + `fips_county` — combine the two components
    ///
    /// Returns `None` if neither is present or neither parses successfully.
    pub fn try_fips_code(&self) -> Option<FipsCode> {
        if let Some(s) = self.fips_code.as_deref().filter(|s| !s.is_empty()) {
            return FipsCode::from_str(s).ok();
        }
        if let (Some(state_s), Some(county_s)) = (
            self.fips_state.as_deref().filter(|s| !s.is_empty()),
            self.fips_county.as_deref().filter(|s| !s.is_empty()),
        ) {
            let state_n: u8 = state_s.trim().parse().ok()?;
            let county_n: u16 = county_s.trim().parse().ok()?;
            return FipsCode::new(state_n, county_n).ok();
        }
        None
    }

    /// Reconstruct a single-line address string from components, or return
    /// `address_line` if present.
    pub fn display_line(&self) -> String {
        if let Some(line) = self.address_line.as_deref().filter(|s| !s.is_empty()) {
            return line.to_owned();
        }
        let parts: Vec<&str> = [
            self.street_dir_prefix.as_deref(),
            self.street_number.as_deref(),
            self.street_name.as_deref(),
            self.street_type.as_deref(),
            self.street_dir_suffix.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect();
        parts.join(" ")
    }
}

// ── PROPERTY_DETAIL ───────────────────────────────────────────────────────────

/// MISMO 3.4 `PROPERTY_DETAIL` element.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "PROPERTY_DETAIL")]
pub struct PropertyDetail {
    /// MISMO property structure type.
    /// e.g. `"Detached"`, `"Attached"`, `"Condominium"`, `"PUD"`,
    /// `"ManufacturedHousing"`, `"2-Unit"`, `"3-Unit"`, `"4-Unit"`.
    #[serde(rename = "GSEProjectClassificationType")]
    pub property_structure_type: String,

    /// Occupancy / usage type.
    /// e.g. `"PrimaryResidence"`, `"SecondHome"`, `"Investor"`.
    #[serde(rename = "PropertyUsageType")]
    pub property_usage_type: String,

    /// Year the structure was built. e.g. `"1998"`.
    #[serde(
        rename = "PropertyStructureBuiltYear",
        skip_serializing_if = "Option::is_none"
    )]
    pub year_built: Option<String>,

    /// Number of units (1–4). Absent implies 1.
    #[serde(rename = "FinancedUnitCount", skip_serializing_if = "Option::is_none")]
    pub financed_unit_count: Option<String>,

    /// Gross living area in square feet. e.g. `"2150"`.
    /// Used for manufactured home minimum size rules and condo project limits.
    #[serde(
        rename = "GrossLivingAreaAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub gross_living_area: Option<String>,
}

// ── PROPERTY_TAX ─────────────────────────────────────────────────────────────

/// MISMO extension element `PROPERTY_TAX`.
///
/// Not part of the MISMO 3.4 core schema; used as an engine extension
/// to carry property tax data alongside the property record.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "PROPERTY_TAX")]
pub struct PropertyTaxDetail {
    /// Annual property tax amount. e.g. `"10523.40"`.
    #[serde(rename = "AnnualTaxAmount", skip_serializing_if = "Option::is_none")]
    pub annual_amount: Option<String>,

    /// Effective tax rate as a percentage of appraised value. e.g. `"1.9"`.
    #[serde(rename = "TaxRatePercent", skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<String>,

    /// Year for which the tax amount applies. e.g. `"2024"`.
    #[serde(rename = "TaxYear", skip_serializing_if = "Option::is_none")]
    pub tax_year: Option<String>,

    /// `"true"` when the jurisdiction collects property taxes in arrears
    /// (Texas = true). Affects seller tax proration at closing.
    #[serde(
        rename = "TaxesCollectedInArrearsIndicator",
        skip_serializing_if = "Option::is_none"
    )]
    pub paid_in_arrears: Option<String>,

    /// Seller prorated tax amount override. e.g. `"5261.70"`.
    #[serde(
        rename = "SellerTaxArrearsAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub seller_arrears_amount: Option<String>,
}

// ── HOA_DETAIL ────────────────────────────────────────────────────────────────

/// MISMO extension element `HOA_DETAIL`.
///
/// HOA data drives two calculations:
/// - Monthly PITIA (principal + interest + taxes + insurance + **association**)
/// - Section H closing costs (transfer fee, working capital fee)
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "HOA_DETAIL")]
pub struct HoaDetail {
    /// `"true"` if an HOA exists.
    #[serde(rename = "HOAIndicator", skip_serializing_if = "Option::is_none")]
    pub hoa_yn: Option<String>,

    /// Monthly HOA dues. e.g. `"250.00"`.
    #[serde(rename = "HOAMonthlyAmount", skip_serializing_if = "Option::is_none")]
    pub monthly_fee: Option<String>,

    /// Fee frequency when not monthly: `"Monthly"` | `"Quarterly"` | `"Annual"`.
    #[serde(
        rename = "HOAFeeFrequencyType",
        skip_serializing_if = "Option::is_none"
    )]
    pub fee_frequency: Option<String>,

    /// Transfer fee charged at closing (Section H). e.g. `"175.00"`.
    #[serde(
        rename = "HOATransferFeeAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub transfer_fee: Option<String>,

    /// Working capital / move-in fee. e.g. `"0.00"`.
    #[serde(
        rename = "HOAWorkingCapitalFeeAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub working_capital_fee: Option<String>,

    /// Annual dues amount (normalized from frequency, if provided).
    #[serde(
        rename = "HOAAnnualDuesAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub annual_dues_corrected: Option<String>,
}

// ── SUBJECT_PROPERTY ─────────────────────────────────────────────────────────

/// MISMO 3.4 `SUBJECT_PROPERTY` element — the property securing the loan.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename = "SUBJECT_PROPERTY")]
pub struct SubjectProperty {
    /// Decomposed property address.
    #[serde(rename = "ADDRESS")]
    pub address: MismoAddress,

    /// Property structure and occupancy details.
    #[serde(rename = "PROPERTY_DETAIL")]
    pub detail: PropertyDetail,

    /// Property tax data (engine extension, optional).
    #[serde(rename = "PROPERTY_TAX", skip_serializing_if = "Option::is_none")]
    pub tax: Option<PropertyTaxDetail>,

    /// HOA data (engine extension, optional).
    #[serde(rename = "HOA_DETAIL", skip_serializing_if = "Option::is_none")]
    pub hoa: Option<HoaDetail>,

    /// Appraised / estimated value. e.g. `"459000.00"`.
    #[serde(
        rename = "PropertyEstimatedValueAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub estimated_value: Option<String>,

    /// Sales contract price. e.g. `"459000.00"`.
    #[serde(
        rename = "SalesContractAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub sales_contract_amount: Option<String>,

    /// Annual hazard/homeowners insurance premium (user override).
    /// e.g. `"1840.00"`.
    #[serde(
        rename = "AnnualHazardInsurancePremiumAmount",
        skip_serializing_if = "Option::is_none"
    )]
    pub annual_hoi: Option<String>,

    /// ZIP-code-based HOI lookup result (informational).
    #[serde(rename = "HOIZipLookupAmount", skip_serializing_if = "Option::is_none")]
    pub hoi_zip_lookup: Option<String>,
}

// ── CollateralParsed ──────────────────────────────────────────────────────────

/// Fully typed collateral data — output of [`SubjectProperty::parse`].
#[derive(Debug, Clone)]
pub struct CollateralParsed {
    // ── Address ───────────────────────────────────────────────────────────────
    /// State code derived from `StateCode` address element.
    pub state: StateCode,
    /// 5-digit FIPS code when present or derivable from state/county codes.
    pub fips_code: Option<FipsCode>,
    /// ZIP code (first 5 digits).
    pub postal_code: String,
    /// County name, if provided.
    pub county_name: Option<String>,
    /// Street number component, if decomposed.
    pub street_number: Option<String>,
    /// Street name component, if decomposed.
    pub street_name: Option<String>,
    /// Street type component, if decomposed.
    pub street_type: Option<String>,
    /// Pre-directional component, if decomposed.
    pub street_dir_prefix: Option<String>,
    /// Post-directional component, if decomposed.
    pub street_dir_suffix: Option<String>,
    /// City name.
    pub city: String,

    // ── Property ──────────────────────────────────────────────────────────────
    /// Property structure type.
    pub property_type: PropertyType,
    /// Occupancy / usage type.
    pub occupancy: Occupancy,
    /// Number of residential units (1–4).
    pub unit_count: u8,

    // ── Valuation ─────────────────────────────────────────────────────────────
    /// Appraised / estimated value.
    pub appraised_value: Cents,
    /// Sales contract price (purchase only).
    pub sales_price: Option<Cents>,
    /// List price from RESO feed (set in ingest; None when parsed from MISMO only).
    pub list_price: Option<Cents>,

    // ── Physical characteristics ───────────────────────────────────────────────
    /// Gross living area in square feet.
    pub sqft: Option<u32>,
    /// Year the structure was built.
    pub year_built: Option<u16>,

    // ── Taxes ─────────────────────────────────────────────────────────────────
    /// Annual property tax amount.
    pub annual_tax: Option<Cents>,
    /// Effective property tax rate (percentage, as basis points).
    pub tax_rate: Option<BasisPoints>,
    /// Tax assessment year.
    pub tax_year: Option<u16>,
    /// `true` when the jurisdiction collects taxes in arrears (Texas = true).
    pub taxes_in_arrears: bool,
    /// Seller prorated tax amount (override).
    pub seller_tax_arrears: Option<Cents>,

    // ── HOI ───────────────────────────────────────────────────────────────────
    /// Annual homeowners insurance premium (user override).
    pub annual_hoi: Option<Cents>,
    /// ZIP-code-based HOI lookup result (from `geo_data` crate, Epic 5).
    pub hoi_zip_lookup: Option<Cents>,

    // ── HOA ───────────────────────────────────────────────────────────────────
    /// `true` if the property has an HOA.
    pub hoa_yn: bool,
    /// Monthly HOA dues.
    pub hoa_monthly: Option<Cents>,
    /// Annual HOA dues (from monthly × 12, or `annual_dues_corrected` field).
    pub hoa_annual: Option<Cents>,
    /// HOA transfer fee paid at closing (Section H).
    pub hoa_transfer_fee: Option<Cents>,
    /// HOA working capital / move-in fee (Section H).
    pub hoa_working_capital: Option<Cents>,
}

// ── Parse implementation ──────────────────────────────────────────────────────

impl SubjectProperty {
    /// Convert raw XML strings into a fully typed [`CollateralParsed`].
    ///
    /// # Errors
    /// Returns [`crate::MismoError`] if required fields are missing or
    /// contain invalid values.
    ///
    /// # Townhouse / Attached disambiguation
    /// MISMO uses `"Attached"` for both `SingleFamilyAttached` and
    /// `Townhouse`. This method defaults `"Attached"` to
    /// `PropertyType::SingleFamilyAttached`. Use the HOA and project
    /// classification fields in the ingest layer (Epic 4) to distinguish
    /// townhouse if needed.
    pub fn parse(&self) -> crate::Result<CollateralParsed> {
        let address = &self.address;
        let detail = &self.detail;

        // ── State code ──────────────────────────────────────────────────────
        let state = StateCode::from_str(&address.state_code).map_err(|_| {
            crate::MismoError::InvalidEnum {
                element: "StateCode",
                value: address.state_code.clone(),
            }
        })?;

        // ── FIPS code ───────────────────────────────────────────────────────
        let fips_code = address.try_fips_code();

        // ── Property type ───────────────────────────────────────────────────
        let property_type =
            crate::enums::property::try_property_type(&detail.property_structure_type)?;

        // ── Occupancy ───────────────────────────────────────────────────────
        let occupancy = crate::enums::property::try_occupancy(&detail.property_usage_type)?;

        // ── Unit count ──────────────────────────────────────────────────────
        let unit_count = parse_unit_count(&detail.financed_unit_count)?;

        // ── Valuation ───────────────────────────────────────────────────────
        let appraised_value = self
            .estimated_value
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| parse_cents(s, "PropertyEstimatedValueAmount"))
            .transpose()?
            .ok_or(crate::MismoError::MissingElement {
                element: "PropertyEstimatedValueAmount",
            })?;
        let sales_price = parse_optional_cents(&self.sales_contract_amount, "SalesContractAmount")?;
        // list_price mirrors sales_price from MISMO; the ingest layer (Epic 6)
        // overwrites it with the RESO currentPrice when available.
        let list_price = sales_price;

        // ── Physical characteristics ─────────────────────────────────────────
        let sqft = match detail.gross_living_area.as_deref() {
            None | Some("") => None,
            Some(s) => {
                Some(
                    s.trim()
                        .parse::<u32>()
                        .map_err(|_| crate::MismoError::OutOfRange {
                            element: "GrossLivingAreaAmount",
                            detail: format!("'{s}' is not a valid square-footage integer"),
                        })?,
                )
            }
        };
        let year_built = match detail.year_built.as_deref() {
            None | Some("") => None,
            Some(s) => {
                Some(
                    s.trim()
                        .parse::<u16>()
                        .map_err(|_| crate::MismoError::OutOfRange {
                            element: "PropertyStructureBuiltYear",
                            detail: format!("'{s}' is not a valid year"),
                        })?,
                )
            }
        };

        // ── Taxes ───────────────────────────────────────────────────────────
        let (annual_tax, tax_rate, tax_year, taxes_in_arrears, seller_tax_arrears) =
            if let Some(tax) = &self.tax {
                (
                    parse_optional_cents(&tax.annual_amount, "AnnualTaxAmount")?,
                    parse_optional_bps(&tax.tax_rate, "TaxRatePercent")?,
                    parse_optional_u16(&tax.tax_year, "TaxYear")?,
                    parse_bool_indicator(&tax.paid_in_arrears),
                    parse_optional_cents(&tax.seller_arrears_amount, "SellerTaxArrearsAmount")?,
                )
            } else {
                (None, None, None, false, None)
            };

        // ── HOI ─────────────────────────────────────────────────────────────
        let annual_hoi =
            parse_optional_cents(&self.annual_hoi, "AnnualHazardInsurancePremiumAmount")?;
        let hoi_zip_lookup = parse_optional_cents(&self.hoi_zip_lookup, "HOIZipLookupAmount")?;

        // ── HOA ─────────────────────────────────────────────────────────────
        let (hoa_yn, hoa_monthly, hoa_annual, hoa_transfer_fee, hoa_working_capital) =
            if let Some(hoa) = &self.hoa {
                let monthly = parse_optional_cents(&hoa.monthly_fee, "HOAMonthlyAmount")?;
                // Annual: prefer explicit field; fall back to monthly × 12
                let annual = if let Ok(Some(a)) =
                    parse_optional_cents(&hoa.annual_dues_corrected, "HOAAnnualDuesAmount")
                {
                    Some(a)
                } else {
                    monthly.map(|m| Cents(m.0 * 12))
                };
                (
                    parse_bool_indicator(&hoa.hoa_yn),
                    monthly,
                    annual,
                    parse_optional_cents(&hoa.transfer_fee, "HOATransferFeeAmount")?,
                    parse_optional_cents(&hoa.working_capital_fee, "HOAWorkingCapitalFeeAmount")?,
                )
            } else {
                (false, None, None, None, None)
            };

        Ok(CollateralParsed {
            state,
            fips_code,
            postal_code: address.postal_code.clone(),
            county_name: address.county_name.clone(),
            street_number: address.street_number.clone(),
            street_name: address.street_name.clone(),
            street_type: address.street_type.clone(),
            street_dir_prefix: address.street_dir_prefix.clone(),
            street_dir_suffix: address.street_dir_suffix.clone(),
            city: address.city.clone(),
            property_type,
            occupancy,
            unit_count,
            appraised_value,
            sales_price,
            list_price,
            sqft,
            year_built,
            annual_tax,
            tax_rate,
            tax_year,
            taxes_in_arrears,
            seller_tax_arrears,
            annual_hoi,
            hoi_zip_lookup,
            hoa_yn,
            hoa_monthly,
            hoa_annual,
            hoa_transfer_fee,
            hoa_working_capital,
        })
    }
}
