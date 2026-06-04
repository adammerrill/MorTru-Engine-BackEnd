//! `RefDataStore` trait + `JsonFileStore` + `SqliteStore` implementations.

use std::path::PathBuf;

use crate::{
    cbsa::CbsaEntry,
    condo_approval::{FhaCondoApprovedFile, FhaCondoProject},
    conv_mi::{
        ConvMiCoverage, ConvMiCoverageTable, ConvMiInput, MiMonthlyTable, MiRateInput,
        UsdaGuaranteeFees,
    },
    dpa_catalog::{DpaCatalogFile, DpaEligibilityInput, DpaOutcome, DpaProgram},
    error::{RefDataError, RefDataResult},
    fha_mip::{FhaMipInput, FhaMipResult, FhaMipTable},
    geo::{
        AmiTractData, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
        UsdaMfhByTract, UsdaruralEligibility,
    },
    hoi_rates::StateHoiRate,
    lender::{LenderOverlays, LenderProfile, LenderProfileFile},
    mcc_catalog::{MccCatalogFile, MccEligibilityInput, MccOutcome, MccProgram},
    program_rules::{AllProgramRules, ProgramEligibilityRules},
    rate_sheet::{LlpaInput, LlpaMatrix, RateSheet, RateSheetFile},
    va_fee::{VaFeeInput, VaFeeTable},
    versioning::{VersionId, Versioned},
    zip_hoi::ZipHoiRate,
};
use types::{Derived, ProgramCode};

// ── RefDataStore trait ────────────────────────────────────────────────────────

/// Single interface for all reference data access.
///
/// Implementations: [`JsonFileStore`] (dev/CI), [`SqliteStore`] (integration).
pub trait RefDataStore: Send + Sync {
    // ── Loan limits ───────────────────────────────────────────────────────────
    fn fha_loan_limits(
        &self,
        fips_code: &str,
        year: u16,
    ) -> RefDataResult<Versioned<FhaLoanLimits>>;
    fn gse_loan_limits(
        &self,
        fips_code: &str,
        year: u16,
    ) -> RefDataResult<Versioned<GseLoanLimits>>;

    // ── USDA ──────────────────────────────────────────────────────────────────
    fn usda_rural_eligibility(&self, geoid: &str) -> RefDataResult<Option<UsdaruralEligibility>>;
    fn usda_income_limits(
        &self,
        fips_code: &str,
        effective_date: chrono::NaiveDate,
    ) -> RefDataResult<Versioned<UsdaIncomeLimit>>;
    fn usda_mfh_by_tract(&self, geoid: &str) -> RefDataResult<Option<UsdaMfhByTract>>;

    // ── AMI ───────────────────────────────────────────────────────────────────
    fn ami_tract_data(
        &self,
        geoid: &str,
        year: u16,
    ) -> RefDataResult<Option<Versioned<AmiTractData>>>;

    // ── Program eligibility rules ─────────────────────────────────────────────
    fn program_rules(&self, program: ProgramCode) -> RefDataResult<ProgramEligibilityRules>;

    // ── HOI estimation ────────────────────────────────────────────────────────
    fn state_hoi_rate(&self, state_abbr: &str, year: u16) -> RefDataResult<StateHoiRate>;

    // ── ZIP-level HOI (Task 4.12) — falls through to state rate if None ───────
    fn zip_hoi_rate(&self, zip5: &str, year: u16) -> RefDataResult<Option<ZipHoiRate>>;

    // ── CBSA/MSA crosswalk (Task 4.11) ───────────────────────────────────────
    fn cbsa_for_county(&self, fips_code: &str) -> RefDataResult<Option<CbsaEntry>>;

    // ── Unified geo-eligibility query ─────────────────────────────────────────
    fn geo_eligibility(
        &self,
        fips_code: &str,
        tract_geoid: Option<&str>,
        year: u16,
    ) -> RefDataResult<GeoEligibility>;

    // ── Version tracking ──────────────────────────────────────────────────────
    fn current_version(&self, dataset: &str) -> RefDataResult<VersionId>;

    // ── FHA / VA / Conv MI / USDA fee tables (Phase 4) ──────────────────
    fn fha_mip(&self, input: &FhaMipInput, year: u16) -> RefDataResult<FhaMipResult>;
    fn va_funding_fee(&self, input: &VaFeeInput, year: u16) -> RefDataResult<u32>;
    fn conv_mi_coverage(&self, input: &ConvMiInput, year: u16) -> RefDataResult<ConvMiCoverage>;
    fn mi_monthly_rate(&self, provider: &str, input: &MiRateInput, year: u16)
        -> RefDataResult<u16>;
    fn usda_guarantee_fees(&self, year: u16) -> RefDataResult<UsdaGuaranteeFees>;

    // ── MCC program catalog (Task 4.24) ──
    fn mcc_program(&self, state: &str, year: u16) -> RefDataResult<Option<Derived<MccProgram>>>;
    fn mcc_evaluate(
        &self,
        input: &MccEligibilityInput,
        year: u16,
    ) -> RefDataResult<Option<Derived<MccOutcome>>>;

    // ── DPA program catalog (Task 4.25) ──
    fn dpa_programs_for_state(
        &self,
        state: &str,
        year: u16,
    ) -> RefDataResult<Vec<Derived<DpaProgram>>>;
    fn dpa_program(
        &self,
        program_id: &str,
        year: u16,
    ) -> RefDataResult<Option<Derived<DpaProgram>>>;
    fn dpa_evaluate(
        &self,
        program_id: &str,
        input: &DpaEligibilityInput,
        year: u16,
    ) -> RefDataResult<Option<Derived<DpaOutcome>>>;

    // ── Lender profiles + overlays (Task 4.16) ───────────────────────────────
    fn lender_profile(&self, lender_id: &str) -> RefDataResult<Option<LenderProfile>>;
    fn lender_overlays(
        &self,
        lender_id: &str,
        program: ProgramCode,
    ) -> RefDataResult<Option<LenderOverlays>>;

    // ── MI provider metadata (Task 4.17) ─────────────────────────────────────
    /// Returns annual rate in bps for the single-premium borrower-paid plan.
    fn mi_single_premium_bps(
        &self,
        provider: &str,
        input: &MiRateInput,
        year: u16,
    ) -> RefDataResult<u16>;

    // ── LLPA matrix (Task 4.18) ──────────────────────────────────────────────
    fn llpa_total(&self, agency: &str, input: &LlpaInput, year: u16) -> RefDataResult<i32>;

    // ── Rate sheet (Task 4.18) ───────────────────────────────────────────────
    fn rate_sheet(&self, lender_id: &str) -> RefDataResult<Option<RateSheet>>;

    // ── FHA condo project approval (Task 4.23) ───────────────────────────────
    fn fha_condo_project(&self, fha_project_id: &str) -> RefDataResult<Option<FhaCondoProject>>;
}

// ── JsonFileStore ─────────────────────────────────────────────────────────────

/// JSON-file-backed store for development and CI.
#[derive(Debug)]
pub struct JsonFileStore {
    pub data_dir: PathBuf,
}

impl JsonFileStore {
    #[must_use]
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub(crate) fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        filename: &str,
    ) -> RefDataResult<T> {
        let path = self.data_dir.join(filename);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| RefDataError::Storage(format!("cannot read {}: {e}", path.display())))?;
        serde_json::from_str(&content).map_err(|e| RefDataError::Json {
            file: filename.to_owned(),
            source: e,
        })
    }

    pub(crate) fn read_versioned_json<T: serde::de::DeserializeOwned>(
        &self,
        prefix: &str,
        year: u16,
    ) -> RefDataResult<(u16, T)> {
        let entries = std::fs::read_dir(&self.data_dir)
            .map_err(|e| RefDataError::Storage(format!("cannot read data_dir: {e}")))?;

        let mut best_year: Option<u16> = None;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix(&format!("{prefix}_")) {
                if let Some(yr_str) = rest.strip_suffix(".json") {
                    if let Ok(yr) = yr_str.parse::<u16>() {
                        if yr <= year {
                            match best_year {
                                None => best_year = Some(yr),
                                Some(b) if yr > b => best_year = Some(yr),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let found_year = best_year.ok_or_else(|| {
            RefDataError::Storage(format!("no {prefix} file found for year ≤ {year}"))
        })?;
        let filename = format!("{prefix}_{found_year}.json");
        let data = self.read_json(&filename)?;
        Ok((found_year, data))
    }
}

impl RefDataStore for JsonFileStore {
    fn fha_loan_limits(
        &self,
        fips_code: &str,
        year: u16,
    ) -> RefDataResult<Versioned<FhaLoanLimits>> {
        let (found_year, records): (u16, Vec<FhaLoanLimits>) =
            self.read_versioned_json("fha_limits", year)?;
        let data = records
            .into_iter()
            .find(|r| r.fips_code == fips_code)
            .ok_or_else(|| RefDataError::NotFound {
                data_type: "FhaLoanLimits",
                fips: fips_code.to_owned(),
                year: found_year,
            })?;
        let eff = chrono::NaiveDate::from_ymd_opt(i32::from(found_year), 1, 1).unwrap_or_default();
        Ok(Versioned::new("fha_loan_limits", eff, data))
    }

    fn gse_loan_limits(
        &self,
        fips_code: &str,
        year: u16,
    ) -> RefDataResult<Versioned<GseLoanLimits>> {
        let (found_year, records): (u16, Vec<GseLoanLimits>) =
            self.read_versioned_json("gse_limits", year)?;
        let data = records
            .into_iter()
            .find(|r| r.fips_code == fips_code)
            .ok_or_else(|| RefDataError::NotFound {
                data_type: "GseLoanLimits",
                fips: fips_code.to_owned(),
                year: found_year,
            })?;
        let eff = chrono::NaiveDate::from_ymd_opt(i32::from(found_year), 1, 1).unwrap_or_default();
        Ok(Versioned::new("gse_loan_limits", eff, data))
    }

    fn usda_rural_eligibility(&self, geoid: &str) -> RefDataResult<Option<UsdaruralEligibility>> {
        let records: Vec<UsdaruralEligibility> = self.read_json("usda_rural_eligibility.json")?;
        Ok(records.into_iter().find(|r| r.geoid == geoid))
    }

    fn usda_income_limits(
        &self,
        fips_code: &str,
        effective_date: chrono::NaiveDate,
    ) -> RefDataResult<Versioned<UsdaIncomeLimit>> {
        let year = effective_date
            .format("%Y")
            .to_string()
            .parse::<u16>()
            .unwrap_or(2025);
        let (found_year, records): (u16, Vec<UsdaIncomeLimit>) =
            self.read_versioned_json("usda_income_limits", year)?;
        let data = records
            .into_iter()
            .find(|r| r.fips_code == fips_code)
            .ok_or_else(|| RefDataError::NotFound {
                data_type: "UsdaIncomeLimit",
                fips: fips_code.to_owned(),
                year: found_year,
            })?;
        let eff = chrono::NaiveDate::from_ymd_opt(i32::from(found_year), 10, 1).unwrap_or_default();
        Ok(Versioned::new("usda_income_limits", eff, data))
    }

    fn usda_mfh_by_tract(&self, geoid: &str) -> RefDataResult<Option<UsdaMfhByTract>> {
        let records: Vec<UsdaMfhByTract> = self.read_json("usda_mfh_by_tract.json")?;
        Ok(records.into_iter().find(|r| r.geoid == geoid))
    }

    fn ami_tract_data(
        &self,
        geoid: &str,
        year: u16,
    ) -> RefDataResult<Option<Versioned<AmiTractData>>> {
        let (found_year, records): (u16, Vec<AmiTractData>) =
            self.read_versioned_json("ami_tract_data", year)?;
        let eff = chrono::NaiveDate::from_ymd_opt(i32::from(found_year), 1, 1).unwrap_or_default();
        let data = records.into_iter().find(|r| r.geoid == geoid);
        Ok(data.map(|d| Versioned::new("ami_tract_data", eff, d)))
    }

    fn program_rules(&self, program: ProgramCode) -> RefDataResult<ProgramEligibilityRules> {
        let all: AllProgramRules = self.read_json("program_rules.json")?;
        all.for_program(program).cloned()
    }

    fn state_hoi_rate(&self, state_abbr: &str, year: u16) -> RefDataResult<StateHoiRate> {
        let (_, records): (u16, Vec<StateHoiRate>) =
            self.read_versioned_json("state_hoi_rates", year)?;
        records
            .into_iter()
            .find(|r| r.state_abbr.eq_ignore_ascii_case(state_abbr))
            .ok_or_else(|| RefDataError::NotFound {
                data_type: "StateHoiRate",
                fips: state_abbr.to_owned(),
                year,
            })
    }

    fn zip_hoi_rate(&self, zip5: &str, year: u16) -> RefDataResult<Option<ZipHoiRate>> {
        // Scan for zip_hoi_rates_*_{year}.json files; state-prefixed names supported
        let entries = std::fs::read_dir(&self.data_dir)
            .map_err(|e| RefDataError::Storage(format!("cannot read data_dir: {e}")))?;

        let mut best: Option<(u16, Vec<ZipHoiRate>)> = None;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("zip_hoi_rates_") && name.ends_with(".json") {
                // Extract trailing year from name like zip_hoi_rates_tx_2025.json
                if let Some(yr_str) = name.trim_end_matches(".json").rsplit('_').next() {
                    if let Ok(yr) = yr_str.parse::<u16>() {
                        if yr <= year {
                            let keep = match &best {
                                None => true,
                                Some((b, _)) => yr > *b,
                            };
                            if keep {
                                if let Ok(records) =
                                    self.read_json::<Vec<ZipHoiRate>>(name.as_ref())
                                {
                                    best = Some((yr, records));
                                }
                            }
                        }
                    }
                }
            }
        }

        match best {
            Some((_, records)) => Ok(records.into_iter().find(|r| r.zip5 == zip5)),
            None => Ok(None), // no ZIP HOI data at all → caller falls through to state rate
        }
    }

    fn cbsa_for_county(&self, fips_code: &str) -> RefDataResult<Option<CbsaEntry>> {
        let records: Vec<CbsaEntry> = self.read_json("cbsa_crosswalk.json")?;
        Ok(records.into_iter().find(|r| r.fips_code == fips_code))
    }

    fn geo_eligibility(
        &self,
        fips_code: &str,
        tract_geoid: Option<&str>,
        year: u16,
    ) -> RefDataResult<GeoEligibility> {
        use types::Cents;

        let fha = self.fha_loan_limits(fips_code, year)?.data;
        let gse = self.gse_loan_limits(fips_code, year)?.data;

        let (usda_sfh, usda_mfh, usda_pct) = match tract_geoid {
            Some(geoid) => match self.usda_rural_eligibility(geoid)? {
                Some(r) => (r.is_sfh_eligible, r.is_mfh_eligible, r.pct_eligible),
                None => (false, false, None),
            },
            None => (false, false, None),
        };

        let effective_date =
            chrono::NaiveDate::from_ymd_opt(i32::from(year), 10, 1).unwrap_or_default();
        let usda_income = self.usda_income_limits(fips_code, effective_date).ok();
        let usda_income_limits = match usda_income {
            Some(v) => [
                v.data.limit_size_1,
                v.data.limit_size_2,
                v.data.limit_size_3,
                v.data.limit_size_4,
                v.data.limit_size_5,
                v.data.limit_size_6,
                v.data.limit_size_7,
                v.data.limit_size_8,
            ],
            None => [Cents(0); 8],
        };

        let ami = tract_geoid
            .and_then(|g| self.ami_tract_data(g, year).ok().flatten())
            .map(|v| v.data);

        Ok(GeoEligibility {
            fips_code: fips_code.to_owned(),
            tract_geoid: tract_geoid.map(str::to_owned),
            effective_year: year,
            fha_limit_1_unit: fha.limit_1_unit,
            fha_limit_2_unit: fha.limit_2_unit,
            fha_limit_3_unit: fha.limit_3_unit,
            fha_limit_4_unit: fha.limit_4_unit,
            fha_limit_type: fha.limit_type,
            gse_limit_1_unit: gse.limit_1_unit,
            gse_limit_2_unit: gse.limit_2_unit,
            gse_limit_3_unit: gse.limit_3_unit,
            gse_limit_4_unit: gse.limit_4_unit,
            gse_is_high_cost: gse.is_high_cost,
            usda_sfh_eligible: usda_sfh,
            usda_mfh_eligible: usda_mfh,
            usda_pct_eligible: usda_pct,
            usda_income_limits,
            ami_100pct: ami.as_ref().and_then(|a| a.ami_100pct),
            ami_50pct: ami.as_ref().and_then(|a| a.ami_50pct),
            ami_80pct: ami.as_ref().and_then(|a| a.ami_80pct),
            ami_115pct: ami.as_ref().and_then(|a| a.ami_115pct),
            is_low_income_tract: ami.as_ref().map(|a| a.is_low_income_tract).unwrap_or(false),
            hp_income_limit_waived: ami
                .as_ref()
                .map(|a| a.hp_income_limit_waived)
                .unwrap_or(false),
        })
    }

    fn current_version(&self, dataset: &str) -> RefDataResult<VersionId> {
        let entries = std::fs::read_dir(&self.data_dir)
            .map_err(|e| RefDataError::Storage(format!("cannot read data_dir: {e}")))?;
        let mut best_year: Option<u16> = None;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix(&format!("{dataset}_")) {
                if let Some(yr_str) = rest.strip_suffix(".json") {
                    if let Ok(yr) = yr_str.parse::<u16>() {
                        match best_year {
                            None => best_year = Some(yr),
                            Some(b) if yr > b => best_year = Some(yr),
                            _ => {}
                        }
                    }
                }
            }
        }
        let year = best_year.ok_or_else(|| {
            RefDataError::Storage(format!("no versioned file found for dataset '{dataset}'"))
        })?;
        let eff = chrono::NaiveDate::from_ymd_opt(i32::from(year), 1, 1).unwrap_or_default();
        Ok(VersionId::new(dataset, eff))
    }

    fn fha_mip(&self, input: &FhaMipInput, year: u16) -> RefDataResult<FhaMipResult> {
        let (_, table): (u16, FhaMipTable) = self.read_versioned_json("fha_mip_rates", year)?;
        table.lookup(input).ok_or_else(|| {
            RefDataError::Storage(format!(
                "no FHA MIP row matched ltv={}, term={}, high_bal={}, streamline={}",
                input.ltv_bps,
                input.term_months,
                input.base_loan_cents,
                input.is_streamline_pre_2009
            ))
        })
    }

    fn va_funding_fee(&self, input: &VaFeeInput, year: u16) -> RefDataResult<u32> {
        let (_, table): (u16, VaFeeTable) = self.read_versioned_json("va_funding_fees", year)?;
        table.lookup(input).ok_or_else(|| {
            RefDataError::Storage("no VA funding fee row matched input parameters".to_owned())
        })
    }

    fn conv_mi_coverage(&self, input: &ConvMiInput, year: u16) -> RefDataResult<ConvMiCoverage> {
        let (_, table): (u16, ConvMiCoverageTable) =
            self.read_versioned_json("conv_mi_coverage", year)?;
        table.lookup(input).ok_or_else(|| {
            RefDataError::Storage(format!(
                "no conv MI coverage row matched ltv={}, term={}, program={:?}",
                input.ltv_bps, input.term_months, input.program
            ))
        })
    }

    fn mi_monthly_rate(
        &self,
        provider: &str,
        input: &MiRateInput,
        year: u16,
    ) -> RefDataResult<u16> {
        let filename = format!("mi_rates_{provider}_monthly");
        let (_, table): (u16, MiMonthlyTable) = self.read_versioned_json(&filename, year)?;
        table.lookup_annual_bps(input).ok_or_else(|| {
            RefDataError::Storage(format!(
                "no MI rate row matched provider={provider}, ltv={}, coverage={}%, fico={}",
                input.ltv_bps, input.coverage_pct, input.fico
            ))
        })
    }

    fn usda_guarantee_fees(&self, year: u16) -> RefDataResult<UsdaGuaranteeFees> {
        let (_, fees): (u16, UsdaGuaranteeFees) =
            self.read_versioned_json("usda_guarantee_fees", year)?;
        Ok(fees)
    }

    fn mcc_program(&self, state: &str, year: u16) -> RefDataResult<Option<Derived<MccProgram>>> {
        let (resolved, file): (u16, MccCatalogFile) =
            self.read_versioned_json("mcc_catalog", year)?;
        let fname = format!("mcc_catalog_{resolved}.json");
        Ok(file.lookup(state, &fname, year, resolved))
    }

    fn mcc_evaluate(
        &self,
        input: &MccEligibilityInput,
        year: u16,
    ) -> RefDataResult<Option<Derived<MccOutcome>>> {
        let (resolved, file): (u16, MccCatalogFile) =
            self.read_versioned_json("mcc_catalog", year)?;
        let fname = format!("mcc_catalog_{resolved}.json");
        Ok(file.evaluate(input, &fname, year, resolved))
    }

    fn dpa_programs_for_state(
        &self,
        state: &str,
        year: u16,
    ) -> RefDataResult<Vec<Derived<DpaProgram>>> {
        let (resolved, file): (u16, DpaCatalogFile) =
            self.read_versioned_json("dpa_catalog", year)?;
        let fname = format!("dpa_catalog_{resolved}.json");
        Ok(file.programs_for_state(state, &fname, year, resolved))
    }

    fn dpa_program(
        &self,
        program_id: &str,
        year: u16,
    ) -> RefDataResult<Option<Derived<DpaProgram>>> {
        let (resolved, file): (u16, DpaCatalogFile) =
            self.read_versioned_json("dpa_catalog", year)?;
        let fname = format!("dpa_catalog_{resolved}.json");
        Ok(file.program_by_id(program_id, &fname, year, resolved))
    }

    fn dpa_evaluate(
        &self,
        program_id: &str,
        input: &DpaEligibilityInput,
        year: u16,
    ) -> RefDataResult<Option<Derived<DpaOutcome>>> {
        let (resolved, file): (u16, DpaCatalogFile) =
            self.read_versioned_json("dpa_catalog", year)?;
        let fname = format!("dpa_catalog_{resolved}.json");
        Ok(file.evaluate(program_id, input, &fname, year, resolved))
    }

    fn lender_profile(&self, lender_id: &str) -> RefDataResult<Option<LenderProfile>> {
        let file: LenderProfileFile = self.read_json("lender_profiles.json")?;
        Ok(file.lenders.into_iter().find(|l| l.lender_id == lender_id))
    }

    fn lender_overlays(
        &self,
        lender_id: &str,
        program: ProgramCode,
    ) -> RefDataResult<Option<LenderOverlays>> {
        let file: LenderProfileFile = self.read_json("lender_profiles.json")?;
        Ok(file
            .overlays
            .into_iter()
            .find(|o| o.lender_id == lender_id && o.program == program))
    }

    fn mi_single_premium_bps(
        &self,
        provider: &str,
        input: &MiRateInput,
        year: u16,
    ) -> RefDataResult<u16> {
        let filename = format!("mi_rates_{provider}_sp_bp_nr");
        let (_, table): (u16, MiMonthlyTable) = self.read_versioned_json(&filename, year)?;
        table.lookup_annual_bps(input).ok_or_else(|| {
            RefDataError::Storage(format!(
                "no SP row: provider={provider}, ltv={}, cov={}%, fico={}",
                input.ltv_bps, input.coverage_pct, input.fico
            ))
        })
    }

    fn llpa_total(&self, agency: &str, input: &LlpaInput, year: u16) -> RefDataResult<i32> {
        let filename = format!("llpa_matrix_{agency}");
        let (_, matrix): (u16, LlpaMatrix) = self.read_versioned_json(&filename, year)?;
        Ok(matrix.total_llpa(input))
    }

    fn rate_sheet(&self, lender_id: &str) -> RefDataResult<Option<RateSheet>> {
        let entries = std::fs::read_dir(&self.data_dir)
            .map_err(|e| RefDataError::Storage(format!("read data_dir: {e}")))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("rate_sheet_") && name.ends_with(".json") {
                if let Ok(file) = self.read_json::<RateSheetFile>(&name) {
                    if let Some(sheet) = file.sheets.into_iter().find(|s| s.lender_id == lender_id)
                    {
                        return Ok(Some(sheet));
                    }
                }
            }
        }
        Ok(None)
    }

    fn fha_condo_project(&self, fha_project_id: &str) -> RefDataResult<Option<FhaCondoProject>> {
        let file: FhaCondoApprovedFile = self.read_json("fha_condo_approved.json")?;
        Ok(file
            .projects
            .into_iter()
            .find(|p| p.fha_project_id == fha_project_id))
    }
}

// SqliteStore lives in its own module, gated behind the "sqlite" feature.
pub mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;
