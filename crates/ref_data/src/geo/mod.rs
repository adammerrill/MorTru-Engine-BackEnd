//! Geographic and program eligibility data types.
//!
//! All data here is keyed by FIPS code (county-level, 5-digit) and/or
//! census tract GEOID (11-digit). ZIP codes are never used for eligibility —
//! they are non-jurisdictional, change frequently, and can span county lines.

use serde::{Deserialize, Serialize};
use types::Cents;

// ── FHA Loan Limits ───────────────────────────────────────────────────────────

/// FHA maximum insurable loan amounts by county and unit count.
///
/// Source: HUD FHA Mortgage Limits database.
/// URL: <https://www.huduser.gov/portal/datasets/fha/fha_limits/>
/// Update: Annually, effective January 1.
///
/// The FHA floor and ceiling are set at 65% and 150% of the national
/// conforming loan limit respectively. High-cost counties receive a higher
/// limit up to the ceiling.
///
/// Engine use: `adjusted_loan_amount ≤ limit_for(fips_code, unit_count)`
/// Note: The UFMIP-financed adjusted loan amount (not the base) must fit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhaLoanLimits {
    pub fips_code: String,
    pub state_abbr: String,
    pub county_name: String,
    /// Limit type: "STANDARD", "HIGH_COST", or "FLOOR".
    pub limit_type: FhaLimitType,
    /// Max insurable amount for a single-unit (SFR / condo / townhouse).
    pub limit_1_unit: Cents,
    /// Max insurable amount for a 2-unit property (duplex).
    pub limit_2_unit: Cents,
    /// Max insurable amount for a 3-unit property (triplex).
    pub limit_3_unit: Cents,
    /// Max insurable amount for a 4-unit property (quadruplex).
    pub limit_4_unit: Cents,
    pub effective_year: u16,
}

/// How the FHA limit was determined for this county.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FhaLimitType {
    /// National floor: 65% of the national conforming limit.
    Floor,
    /// County-specific limit: between floor and ceiling.
    Standard,
    /// National ceiling: 150% of the national conforming limit.
    /// High-cost areas (Alaska, Hawaii, Guam, USVI always; some MSAs).
    HighCost,
}

impl FhaLoanLimits {
    /// Return the limit for the given unit count (1–4).
    ///
    /// # Panics
    /// Panics if `unit_count` is 0 or > 4 — the engine should have
    /// rejected the property type before reaching this code.
    #[must_use]
    pub fn limit_for(&self, unit_count: u8) -> Cents {
        match unit_count {
            1 => self.limit_1_unit,
            2 => self.limit_2_unit,
            3 => self.limit_3_unit,
            4 => self.limit_4_unit,
            _ => panic!("unit_count must be 1–4, got {unit_count}"),
        }
    }
}

// ── GSE Conforming Loan Limits ────────────────────────────────────────────────

/// FNMA / FHLMC conforming loan limits by county and unit count.
///
/// Source: FHFA Conforming Loan Limit data.
/// URL: <https://www.fhfa.gov/data/conforming-loan-limit>
/// Update: Annually, effective January 1.
///
/// A loan is "high-balance conforming" (aka "super conforming") if the
/// amount exceeds the national standard limit but is ≤ the county limit.
/// High-balance loans receive specific LLPA adjustments.
///
/// VA use (post–Blue Water Navy Act, 2020):
/// Veterans with FULL entitlement have no loan limit.
/// Veterans with REDUCED entitlement: limit = `limit_1_unit` for county.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GseLoanLimits {
    pub fips_code: String,
    pub state_abbr: String,
    pub county_name: String,
    pub cbsa_name: Option<String>,
    pub limit_1_unit: Cents,
    pub limit_2_unit: Cents,
    pub limit_3_unit: Cents,
    pub limit_4_unit: Cents,
    /// True when `limit_1_unit > STANDARD_NATIONAL_LIMIT`.
    /// Triggers high-balance pricing treatment and specific LLPAs.
    pub is_high_cost: bool,
    pub effective_year: u16,
}

impl GseLoanLimits {
    /// Standard national conforming limit for 2025 (1-unit).
    pub const STANDARD_1_UNIT_2025: Cents = Cents(80_650_000); // $806,500

    /// Return the limit for the given unit count (1–4).
    #[must_use]
    pub fn limit_for(&self, unit_count: u8) -> Cents {
        match unit_count {
            1 => self.limit_1_unit,
            2 => self.limit_2_unit,
            3 => self.limit_3_unit,
            4 => self.limit_4_unit,
            _ => panic!("unit_count must be 1–4, got {unit_count}"),
        }
    }

    /// True if this loan amount would be classified as high-balance
    /// (above standard national limit but within county limit).
    #[must_use]
    pub fn is_high_balance_amount(&self, loan_amount: Cents, year: u16) -> bool {
        let standard = Self::STANDARD_1_UNIT_2025; // expand with historical years as needed
        loan_amount > standard && loan_amount <= self.limit_1_unit
    }
}

// ── USDA Rural Housing Eligibility ───────────────────────────────────────────

/// Pre-computed USDA Single Family Housing rural eligibility by census tract.
///
/// # Source data
///
/// `SFH_MFH_Ineligible20180823.zip` — 2,607 polygons of urban/suburban
/// areas INELIGIBLE for USDA rural housing programs.
///
/// # Critical logic (from USDA eligibility.js source code)
///
/// ```text
/// identifyResults = point-in-polygon check vs ineligible shapefile
/// if identifyResults.length == 0 → "IS" eligible
/// if identifyResults.length > 0  → "IS NOT" eligible
/// ```
///
/// A property is USDA-eligible if it does NOT fall within any of the
/// 2,607 ineligible urban/suburban polygons. The map shows INELIGIBLE
/// areas — green on the public USDA eligibility map is ineligible, not
/// the other way around.
///
/// # Layer reference
///
/// USDA ArcGIS layer: `layerID = 4` → `"RHS_SFH_MFH_IELG_2011"`
/// (Rural Housing Service, Single Family Housing / Multi-Family Housing,
/// Ineligible, 2011 vintage with 2018-08-23 update)
///
/// # How pre-computation works
///
/// For each of the ~74,000 2020 Census tracts:
/// 1. Get tract centroid (lat/lon)
/// 2. Check if centroid falls within any ineligible polygon
/// 3. Compute percentage of tract area outside ineligible zones
/// 4. Store result: `is_sfh_eligible = (point not in any ineligible polygon)`
///
/// Tracts that straddle a boundary get `pct_eligible < 100`. The engine
/// uses the centroid check for fast eligibility determination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdaruralEligibility {
    /// 11-digit census tract GEOID (state2 + county3 + tract6).
    pub geoid: String,
    /// 5-digit county FIPS code.
    pub fips_code: String,
    pub state_abbr: String,
    /// True if the tract centroid is NOT within any USDA ineligible polygon.
    pub is_sfh_eligible: bool,
    /// True for Multi-Family Housing programs (usually same as SFH).
    pub is_mfh_eligible: bool,
    /// Percentage of tract area outside all ineligible polygons (0.0–100.0).
    /// `None` if computation was not performed (default to centroid result).
    pub pct_eligible: Option<f64>,
    /// Source dataset version string (e.g. "2018-08-23").
    pub source_version: String,
}

// ── USDA Income Limits (SFGH) ─────────────────────────────────────────────────

/// USDA Section 502 Single Family Guaranteed Housing (SFGH) income limits.
///
/// Source: USDA Rural Development.
/// URL: <https://www.rd.usda.gov/programs-services/single-family-housing-programs>
/// Update: Annually (October/November effective date).
///
/// # Critical distinctions
///
/// There are THREE distinct USDA SFH programs with DIFFERENT income limits:
///
/// | Program | Code | Income limit | Who qualifies |
/// |---|---|---|---|
/// | Guaranteed (market rate) | Section 502 SFGH | **115% of AMI** | Middle income |
/// | Direct (subsidized) | Section 502 SFHD | 50–80% of AMI | Low income |
/// | Repair/Rehab | Section 504 | 50% of AMI | Very low income |
///
/// This struct covers **SFGH (Guaranteed) only** — the program used for
/// normal mortgage origination. SFHD (Direct) is a government lending
/// program outside the scope of the mortgage engine.
///
/// # Household income definition
///
/// USDA household income = ALL adults residing in the home, not just
/// borrowers on the loan note. A non-borrower adult child living at home
/// counts. This differs from conventional/FHA/VA which use only borrower income.
///
/// # Family size (1–8)
///
/// HUD defines 8 standard family sizes. Sizes 1–4 use the base limit.
/// Sizes 5–8 get a 115% adjustment of the base for each additional person
/// above 4. The `limit_size_N` fields are the FINAL limits after all
/// adjustments — no further computation required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdaIncomeLimit {
    pub fips_code: String,
    pub state_abbr: String,
    pub county_name: String,
    pub msa_name: Option<String>,
    /// Program: always "SFGH" for Section 502 Guaranteed.
    pub program: String,
    /// Maximum annual household income for 1-person household.
    pub limit_size_1: Cents,
    /// Maximum annual household income for 2-person household.
    pub limit_size_2: Cents,
    /// Maximum annual household income for 3-person household.
    pub limit_size_3: Cents,
    /// Maximum annual household income for 4-person household.
    pub limit_size_4: Cents,
    /// Maximum annual household income for 5-person household (115% adj).
    pub limit_size_5: Cents,
    /// Maximum annual household income for 6-person household.
    pub limit_size_6: Cents,
    /// Maximum annual household income for 7-person household.
    pub limit_size_7: Cents,
    /// Maximum annual household income for 8-person household.
    pub limit_size_8: Cents,
    pub effective_date: chrono::NaiveDate,
}

impl UsdaIncomeLimit {
    /// Return the income limit for a household of `size` persons (1–8).
    ///
    /// Returns `Err` if size is 0 or > 8.
    pub fn limit_for_size(&self, size: u8) -> crate::RefDataResult<Cents> {
        match size {
            1 => Ok(self.limit_size_1),
            2 => Ok(self.limit_size_2),
            3 => Ok(self.limit_size_3),
            4 => Ok(self.limit_size_4),
            5 => Ok(self.limit_size_5),
            6 => Ok(self.limit_size_6),
            7 => Ok(self.limit_size_7),
            8 => Ok(self.limit_size_8),
            n => Err(crate::RefDataError::InvalidHouseholdSize(n)),
        }
    }
}

// ── Area Median Income (AMI) by Census Tract ──────────────────────────────────

/// Area Median Income data by census tract.
///
/// Source: HUD AMI data and FFIEC Census flat file.
/// Update: Annually.
///
/// # Key thresholds
///
/// | Threshold | Programs |
/// |---|---|
/// | 50% AMI | Very Low Income (USDA Direct, HUD programs) |
/// | 80% AMI | Low Income — HomeReady / Home Possible income limit |
/// | 100% AMI | Area median income |
/// | 115% AMI | USDA SFGH income limit (computed: ami_100pct × 1.15) |
/// | 120% AMI | Some state bond programs |
/// | 140% AMI | Some state bond programs |
///
/// # HomeReady / Home Possible income limit rule
///
/// Borrower qualifying income must be ≤ 80% of AMI **for the tract
/// where the property is located**.
///
/// EXCEPTION: If `is_low_income_tract = true`, there is NO income limit.
/// Low-income tract = tract median income ≤ 80% of the MSA median income.
/// Source: FFIEC Census Tract median income data.
///
/// Both FNMA (HomeReady) and FHLMC (Home Possible) use identical rules
/// per their voluntary alignment as of 2023.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmiTractData {
    /// 11-digit census tract GEOID.
    pub geoid: String,
    /// 5-digit county FIPS.
    pub fips_code: String,
    pub state_abbr: String,
    pub county_name: String,
    pub tract_name: Option<String>,
    /// Area Median Income at 100% (the benchmark for all threshold computations).
    pub ami_100pct: Option<Cents>,
    /// Very Low Income threshold (50% of AMI).
    pub ami_50pct: Option<Cents>,
    /// Low Income threshold (80% of AMI). HomeReady/HP income limit.
    pub ami_80pct: Option<Cents>,
    /// USDA SFGH limit (115% of AMI). Computed: ami_100pct × 1.15.
    pub ami_115pct: Option<Cents>,
    /// State bond program threshold (120% of AMI).
    pub ami_120pct: Option<Cents>,
    /// Some state bond program threshold (140% of AMI).
    pub ami_140pct: Option<Cents>,
    /// True when tract median income ≤ 80% of MSA median.
    /// When true: HomeReady/HomePossible have NO income limit for this tract.
    pub is_low_income_tract: bool,
    /// Convenience flag: is_low_income_tract + any required HP eligibility checks.
    pub hp_income_limit_waived: bool,
    pub effective_year: u16,
}

// ── USDA Multi-Family Housing by Census Tract ─────────────────────────────────

/// USDA-financed multi-family housing projects by census tract.
///
/// Source: Uploaded file `USDA_Rural_Housing_by_Tract_6332152967305738155.csv`
/// Records: 7,303 census tracts with active USDA MFH projects.
///
/// # Column definitions
///
/// | Col | Program | Description |
/// |---|---|---|
/// | EL | Elderly housing | Section 515/521 elderly projects |
/// | FA | Family housing | Section 515/521 family projects |
/// | CG | Congregate/Group | Congregate housing projects |
/// | GH | Group Home | Group home projects |
/// | MX | Mixed | Mixed-use projects |
///
/// # Engine use
///
/// 1. Confirms USDA rural status: tracts with active USDA projects are
///    confirmed non-urban by USDA's own approval history.
/// 2. Supplements the ineligible shapefile check for edge cases where
///    a tract centroid falls in an ineligible polygon but USDA has
///    approved existing projects in the tract.
/// 3. Used in USDA MFH program eligibility for 5+ unit properties
///    (outside scope of SFH engine but preserved for future expansion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdaMfhByTract {
    /// 11-digit census tract GEOID (as provided in the source file).
    pub geoid: String,
    /// 5-digit county FIPS (state2 + county3).
    pub fips_code: String,
    pub state_fips: String,
    pub county_fips: String,
    pub tract_number: String,
    pub tract_name: Option<String>,
    pub el_projects: u16,
    pub el_units: u16,
    pub fa_projects: u16,
    pub fa_units: u16,
    pub cg_projects: u16,
    pub cg_units: u16,
    pub gh_projects: u16,
    pub gh_units: u16,
    pub mx_projects: u16,
    pub mx_units: u16,
    pub total_projects: u16,
    pub total_units: u16,
}

impl UsdaMfhByTract {
    /// True if this tract has any active USDA housing projects.
    #[must_use]
    pub fn has_usda_projects(&self) -> bool {
        self.total_projects > 0
    }

    /// True if this tract has any elderly housing projects.
    #[must_use]
    pub fn has_elderly_housing(&self) -> bool {
        self.el_projects > 0
    }

    /// True if this tract has any family housing projects.
    #[must_use]
    pub fn has_family_housing(&self) -> bool {
        self.fa_projects > 0
    }
}

// ── GeoEligibility: unified per-property eligibility record ──────────────────

/// All geographic and program eligibility data for one property, assembled
/// from a single store query keyed by FIPS code + census tract GEOID.
///
/// The eligibility engine (Epic 7) uses this struct to run all four agency
/// program checks without additional data fetches.
///
/// # Data provenance
///
/// Each field derives from a specific source. The `version` in the
/// [`DataVersionManifest`] records which vintage of each source was used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoEligibility {
    // ── Identifiers ───────────────────────────────────────────────────────────
    pub fips_code: String,
    /// 11-digit census tract GEOID. None if FCC resolution not yet performed.
    pub tract_geoid: Option<String>,
    pub effective_year: u16,

    // ── FHA Loan Limits (HUD) ─────────────────────────────────────────────────
    /// Max FHA-insurable amount for 1-unit. Adjusted loan must fit.
    pub fha_limit_1_unit: Cents,
    pub fha_limit_2_unit: Cents,
    pub fha_limit_3_unit: Cents,
    pub fha_limit_4_unit: Cents,
    pub fha_limit_type: FhaLimitType,

    // ── GSE Conforming Limits (FHFA) ──────────────────────────────────────────
    /// Max conventional/VA conforming loan for 1-unit.
    pub gse_limit_1_unit: Cents,
    pub gse_limit_2_unit: Cents,
    pub gse_limit_3_unit: Cents,
    pub gse_limit_4_unit: Cents,
    /// True if county limit > standard national limit (high-balance county).
    pub gse_is_high_cost: bool,

    // ── USDA Rural Housing (USDA RD / shapefile) ──────────────────────────────
    /// True if property is NOT within a USDA ineligible urban/suburban polygon.
    /// Source: `SFH_MFH_Ineligible20180823.shp` point-in-polygon check.
    /// Logic: eligible = NOT in any of the 2,607 ineligible polygons.
    pub usda_sfh_eligible: bool,
    /// True for Multi-Family Housing (usually same as SFH).
    pub usda_mfh_eligible: bool,
    /// % of tract area outside ineligible zones (None = centroid check only).
    pub usda_pct_eligible: Option<f64>,

    // ── USDA Income Limits — SFGH 115% AMI (USDA RD) ─────────────────────────
    /// Max household income for 1-person family. All 8 sizes stored.
    /// Index: `usda_income_limits[0]` = size 1, `[7]` = size 8.
    pub usda_income_limits: [Cents; 8],

    // ── Area Median Income (HUD/FFIEC) ────────────────────────────────────────
    pub ami_100pct: Option<Cents>,
    pub ami_50pct: Option<Cents>,
    pub ami_80pct: Option<Cents>,
    pub ami_115pct: Option<Cents>,
    /// True if this census tract is a low-income tract per FFIEC.
    /// When true: HomeReady and Home Possible have NO income limit.
    pub is_low_income_tract: bool,
    /// Convenience: is_low_income_tract + any HP-specific waiver logic.
    pub hp_income_limit_waived: bool,
}

impl GeoEligibility {
    /// Check FHA eligibility for a given adjusted loan amount and unit count.
    ///
    /// Returns `true` if `adjusted_loan_amount ≤ county FHA limit`.
    /// The UFMIP-financed loan amount must be used, not the base amount.
    #[must_use]
    pub fn fha_loan_within_limit(&self, adjusted_loan: Cents, unit_count: u8) -> bool {
        let limit = match unit_count {
            1 => self.fha_limit_1_unit,
            2 => self.fha_limit_2_unit,
            3 => self.fha_limit_3_unit,
            4 => self.fha_limit_4_unit,
            _ => return false,
        };
        adjusted_loan <= limit
    }

    /// Check GSE conforming loan eligibility.
    ///
    /// Returns `(is_eligible, is_high_balance)`.
    /// High-balance = eligible but above standard national limit.
    #[must_use]
    pub fn gse_loan_status(&self, loan_amount: Cents, unit_count: u8) -> (bool, bool) {
        let county_limit = match unit_count {
            1 => self.gse_limit_1_unit,
            2 => self.gse_limit_2_unit,
            3 => self.gse_limit_3_unit,
            4 => self.gse_limit_4_unit,
            _ => return (false, false),
        };

        if loan_amount > county_limit {
            return (false, false); // jumbo
        }

        let is_high_balance =
            self.gse_is_high_cost && loan_amount > GseLoanLimits::STANDARD_1_UNIT_2025;

        (true, is_high_balance)
    }

    /// Check USDA income eligibility for a household of `size` persons.
    ///
    /// `annual_household_income` = ALL adult income in the home (not just
    /// borrower income — this is the USDA-specific household definition).
    #[must_use]
    pub fn usda_income_eligible(&self, annual_household_income: Cents, household_size: u8) -> bool {
        if household_size == 0 || household_size > 8 {
            return false;
        }
        let limit = self.usda_income_limits[(household_size - 1) as usize];
        annual_household_income <= limit
    }

    /// Check HomeReady / Home Possible income eligibility.
    ///
    /// Returns `true` if eligible for the program at this property's location.
    /// Eligibility = income limit waived (low-income tract) OR income ≤ 80% AMI.
    #[must_use]
    pub fn hp_income_eligible(&self, annual_borrower_income: Cents) -> bool {
        if self.hp_income_limit_waived {
            return true; // low-income census tract — no income limit
        }
        match self.ami_80pct {
            Some(limit) => annual_borrower_income <= limit,
            None => false, // no AMI data → conservative: not eligible
        }
    }
}
