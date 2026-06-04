//! `SqliteStore` — synchronous SQLite-backed implementation of [`RefDataStore`].
//!
//! Uses `rusqlite` with a bundled SQLite for zero external dependencies.
//! All [`RefDataStore`] methods are implemented with direct SQL queries.
//!
//! # Test usage
//!
//! ```rust,ignore
//! use ref_data::SqliteStore;
//! let store = SqliteStore::new_test_store().unwrap();
//! let limits = store.fha_loan_limits("48209", 2025).unwrap();
//! assert_eq!(limits.data.limit_1_unit, Cents(52_422_500));
//! ```

#[cfg(feature = "sqlite")]
mod inner {
    #![allow(clippy::type_complexity)]
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use chrono::NaiveDate;
    use rusqlite::{params, Connection, OptionalExtension};
    use types::{Cents, ProgramCode};

    use crate::{
        cbsa::{CbsaDesignation, CbsaEntry},
        condo_approval::FhaCondoProject,
        conv_mi::{ConvMiCoverage, ConvMiInput, MiRateInput, UsdaGuaranteeFees},
        dpa_catalog::{DpaEligibilityInput, DpaOutcome, DpaProgram},
        error::{RefDataError, RefDataResult},
        fha_mip::{FhaMipInput, FhaMipResult},
        geo::{
            AmiTractData, FhaLimitType, FhaLoanLimits, GeoEligibility, GseLoanLimits,
            UsdaIncomeLimit, UsdaMfhByTract, UsdaruralEligibility,
        },
        hoi_rates::StateHoiRate,
        lender::{LenderOverlays, LenderProfile},
        mcc_catalog::{MccEligibilityInput, MccOutcome, MccProgram},
        program_rules::ProgramEligibilityRules,
        rate_sheet::{LlpaInput, RateSheet},
        store::{JsonFileStore, RefDataStore},
        va_fee::VaFeeInput,
        versioning::{VersionId, Versioned},
        zip_hoi::ZipHoiRate,
    };
    use types::Derived;

    /// SQLite-backed reference data store using `rusqlite` (bundled SQLite).
    ///
    /// Thread-safe via `Arc<Mutex<Connection>>`. All queries acquire the lock
    /// for the duration of the call. For tests, create one store per test
    /// or share across tests via `Arc<SqliteStore>`.
    pub struct SqliteStore {
        conn: Arc<Mutex<Connection>>,
        /// Retained to serve `current_version()` and to seed the database.
        data_dir: PathBuf,
    }

    impl std::fmt::Debug for SqliteStore {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SqliteStore")
                .field("data_dir", &self.data_dir)
                .finish()
        }
    }

    impl SqliteStore {
        /// Create an in-memory SQLite database, apply schema, and seed
        /// all ref_data tables from the JSON files in `data_dir`.
        pub fn new_test_store() -> RefDataResult<Self> {
            let data_dir = default_data_dir();
            Self::new_from_dir(&data_dir)
        }

        /// Create from an explicit data directory (useful when the default
        /// CARGO_MANIFEST_DIR path is wrong in a test binary).
        pub fn new_from_dir(data_dir: &std::path::Path) -> RefDataResult<Self> {
            let conn = Connection::open_in_memory()
                .map_err(|e| RefDataError::Storage(format!("sqlite open failed: {e}")))?;
            init_schema(&conn)?;
            let store = Self {
                conn: Arc::new(Mutex::new(conn)),
                data_dir: data_dir.to_owned(),
            };
            store.seed_from_json(data_dir)?;
            Ok(store)
        }

        fn seed_from_json(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let json = JsonFileStore::new(data_dir);
            let conn = self.conn.lock().unwrap();

            // ── FHA loan limits ───────────────────────────────────────────
            for yr in find_years(data_dir, "fha_limits") {
                let (_, records): (u16, Vec<FhaLoanLimits>) =
                    json.read_versioned_json("fha_limits", yr)?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO fha_loan_limits
                         (fips_code,state_abbr,county_name,limit_type,
                          limit_1_unit,limit_2_unit,limit_3_unit,limit_4_unit,effective_year)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                        params![
                            r.fips_code,
                            r.state_abbr,
                            r.county_name,
                            limit_type_str(r.limit_type),
                            r.limit_1_unit.0,
                            r.limit_2_unit.0,
                            r.limit_3_unit.0,
                            r.limit_4_unit.0,
                            yr
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("fha insert: {e}")))?;
                }
            }

            // ── GSE loan limits ───────────────────────────────────────────
            for yr in find_years(data_dir, "gse_limits") {
                let (_, records): (u16, Vec<GseLoanLimits>) =
                    json.read_versioned_json("gse_limits", yr)?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO gse_loan_limits
                         (fips_code,state_abbr,county_name,cbsa_name,
                          limit_1_unit,limit_2_unit,limit_3_unit,limit_4_unit,
                          is_high_cost,effective_year)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                        params![
                            r.fips_code,
                            r.state_abbr,
                            r.county_name,
                            r.cbsa_name,
                            r.limit_1_unit.0,
                            r.limit_2_unit.0,
                            r.limit_3_unit.0,
                            r.limit_4_unit.0,
                            r.is_high_cost as i32,
                            yr
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("gse insert: {e}")))?;
                }
            }

            // ── USDA rural eligibility ────────────────────────────────────
            {
                let records: Vec<UsdaruralEligibility> =
                    json.read_json("usda_rural_eligibility.json")?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO usda_rural_eligibility
                         (geoid,fips_code,state_abbr,is_sfh_eligible,is_mfh_eligible,
                          pct_eligible,source_version)
                         VALUES (?1,?2,?3,?4,?5,?6,?7)",
                        params![
                            r.geoid,
                            r.fips_code,
                            r.state_abbr,
                            r.is_sfh_eligible as i32,
                            r.is_mfh_eligible as i32,
                            r.pct_eligible,
                            r.source_version
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("usda_rural insert: {e}")))?;
                }
            }

            // ── USDA income limits ────────────────────────────────────────
            for yr in find_years(data_dir, "usda_income_limits") {
                let eff_date = format!("{yr}-10-01");
                let d = NaiveDate::from_ymd_opt(i32::from(yr), 10, 1).unwrap_or_default();
                let (_, records): (u16, Vec<UsdaIncomeLimit>) =
                    json.read_versioned_json("usda_income_limits", yr)?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO usda_income_limits
                         (fips_code,state_abbr,county_name,msa_name,program,
                          limit_size_1,limit_size_2,limit_size_3,limit_size_4,
                          limit_size_5,limit_size_6,limit_size_7,limit_size_8,
                          effective_date)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                        params![
                            r.fips_code,
                            r.state_abbr,
                            r.county_name,
                            r.msa_name,
                            r.program,
                            r.limit_size_1.0,
                            r.limit_size_2.0,
                            r.limit_size_3.0,
                            r.limit_size_4.0,
                            r.limit_size_5.0,
                            r.limit_size_6.0,
                            r.limit_size_7.0,
                            r.limit_size_8.0,
                            eff_date
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("usda_income insert: {e}")))?;
                    let _ = d; // suppress unused warning
                }
            }

            // ── USDA MFH by tract ─────────────────────────────────────────
            {
                let records: Vec<UsdaMfhByTract> = json.read_json("usda_mfh_by_tract.json")?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO usda_mfh_by_tract
                         (geoid,fips_code,state_fips,county_fips,tract_number,tract_name,
                          el_projects,el_units,fa_projects,fa_units,
                          cg_projects,cg_units,gh_projects,gh_units,
                          mx_projects,mx_units,total_projects,total_units)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
                        params![
                            r.geoid,
                            r.fips_code,
                            r.state_fips,
                            r.county_fips,
                            r.tract_number,
                            r.tract_name,
                            r.el_projects,
                            r.el_units,
                            r.fa_projects,
                            r.fa_units,
                            r.cg_projects,
                            r.cg_units,
                            r.gh_projects,
                            r.gh_units,
                            r.mx_projects,
                            r.mx_units,
                            r.total_projects,
                            r.total_units
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("usda_mfh insert: {e}")))?;
                }
            }

            // ── AMI tract data ────────────────────────────────────────────
            for yr in find_years(data_dir, "ami_tract_data") {
                let (_, records): (u16, Vec<AmiTractData>) =
                    json.read_versioned_json("ami_tract_data", yr)?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO ami_tract_data
                         (geoid,fips_code,state_abbr,county_name,tract_name,
                          ami_100pct,ami_50pct,ami_80pct,ami_115pct,ami_120pct,ami_140pct,
                          is_low_income_tract,hp_income_limit_waived,effective_year)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                        params![
                            r.geoid,
                            r.fips_code,
                            r.state_abbr,
                            r.county_name,
                            r.tract_name,
                            r.ami_100pct.map(|c| c.0),
                            r.ami_50pct.map(|c| c.0),
                            r.ami_80pct.map(|c| c.0),
                            r.ami_115pct.map(|c| c.0),
                            r.ami_120pct.map(|c| c.0),
                            r.ami_140pct.map(|c| c.0),
                            r.is_low_income_tract as i32,
                            r.hp_income_limit_waived as i32,
                            yr
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("ami insert: {e}")))?;
                }
            }

            // ── Program rules ─────────────────────────────────────────────
            {
                let all: crate::program_rules::AllProgramRules =
                    json.read_json("program_rules.json")?;
                for r in &all.0 {
                    conn.execute(
                        "INSERT OR REPLACE INTO program_rules
                         (program,min_credit_score,min_credit_score_alt,
                          alt_credit_min_down_payment_bps,
                          max_ltv_bps,max_ltv_bps_alt_credit,max_ltv_bps_high_balance,
                          front_end_dti_max_bps,requires_primary_residence,
                          requires_first_time_buyer,requires_va_entitlement,
                          requires_usda_eligibility,requires_ami_income_check,
                          effective_date)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                        params![
                            program_code_str(r.program),
                            r.min_credit_score,
                            r.min_credit_score_alt,
                            r.alt_credit_min_down_payment_bps,
                            r.max_ltv_bps,
                            r.max_ltv_bps_alt_credit,
                            r.max_ltv_bps_high_balance,
                            r.front_end_dti_max_bps,
                            r.requires_primary_residence as i32,
                            r.requires_first_time_buyer as i32,
                            r.requires_va_entitlement as i32,
                            r.requires_usda_eligibility as i32,
                            r.requires_ami_income_check as i32,
                            r.effective_date.format("%Y-%m-%d").to_string()
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("program_rules insert: {e}")))?;
                }
            }

            // ── State HOI rates ───────────────────────────────────────────
            for yr in find_years(data_dir, "state_hoi_rates") {
                let (_, records): (u16, Vec<StateHoiRate>) =
                    json.read_versioned_json("state_hoi_rates", yr)?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO state_hoi_rates
                         (state_abbr,annual_rate_bps,effective_year)
                         VALUES (?1,?2,?3)",
                        params![r.state_abbr, r.annual_rate_bps, yr],
                    )
                    .map_err(|e| RefDataError::Storage(format!("hoi insert: {e}")))?;
                }
            }

            // ── ZIP HOI rates ─────────────────────────────────────────────
            {
                let entries = std::fs::read_dir(data_dir)
                    .map_err(|e| RefDataError::Storage(format!("read data_dir: {e}")))?;
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name = name.to_string_lossy().to_string();
                    if name.starts_with("zip_hoi_rates_") && name.ends_with(".json") {
                        if let Some(yr_str) = name.trim_end_matches(".json").rsplit('_').next() {
                            if let Ok(yr) = yr_str.parse::<u16>() {
                                let records: Vec<ZipHoiRate> = json.read_json(&name)?;
                                for r in records {
                                    conn.execute(
                                        "INSERT OR REPLACE INTO zip_hoi_rates
                                         (zip5,state_abbr,annual_rate_bps,
                                          median_annual_premium_cents,sample_size,effective_year)
                                         VALUES (?1,?2,?3,?4,?5,?6)",
                                        params![
                                            r.zip5,
                                            r.state_abbr,
                                            r.annual_rate_bps,
                                            r.median_annual_premium_cents.map(|c| c.0),
                                            r.sample_size,
                                            yr
                                        ],
                                    )
                                    .map_err(|e| {
                                        RefDataError::Storage(format!("zip_hoi insert: {e}"))
                                    })?;
                                }
                            }
                        }
                    }
                }
            }

            // ── CBSA crosswalk ────────────────────────────────────────────
            {
                let records: Vec<CbsaEntry> = json.read_json("cbsa_crosswalk.json")?;
                for r in records {
                    conn.execute(
                        "INSERT OR REPLACE INTO cbsa_crosswalk
                         (fips_code,cbsa_code,cbsa_name,designation,is_metro)
                         VALUES (?1,?2,?3,?4,?5)",
                        params![
                            r.fips_code,
                            r.cbsa_code,
                            r.cbsa_name,
                            designation_str(r.designation),
                            r.is_metro as i32
                        ],
                    )
                    .map_err(|e| RefDataError::Storage(format!("cbsa insert: {e}")))?;
                }
            }

            Ok(())
        }
    }

    // ── RefDataStore implementation ───────────────────────────────────────────

    impl RefDataStore for SqliteStore {
        fn fha_loan_limits(
            &self,
            fips_code: &str,
            year: u16,
        ) -> RefDataResult<Versioned<FhaLoanLimits>> {
            let conn = self.conn.lock().unwrap();
            let row: Option<(String, String, String, i64, i64, i64, i64, i32)> = conn
                .query_row(
                    "SELECT state_abbr,county_name,limit_type,
                             limit_1_unit,limit_2_unit,limit_3_unit,limit_4_unit,effective_year
                     FROM fha_loan_limits
                     WHERE fips_code=?1 AND effective_year<=?2
                     ORDER BY effective_year DESC LIMIT 1",
                    params![fips_code, year as i32],
                    |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            let (state, county, lt, l1, l2, l3, l4, found_yr) =
                row.ok_or_else(|| RefDataError::NotFound {
                    data_type: "FhaLoanLimits",
                    fips: fips_code.to_owned(),
                    year,
                })?;

            let data = FhaLoanLimits {
                fips_code: fips_code.to_owned(),
                state_abbr: state,
                county_name: county,
                limit_type: parse_limit_type(&lt),
                limit_1_unit: Cents(l1),
                limit_2_unit: Cents(l2),
                limit_3_unit: Cents(l3),
                limit_4_unit: Cents(l4),
                effective_year: found_yr as u16,
            };
            let eff = NaiveDate::from_ymd_opt(i32::from(found_yr as u16), 1, 1).unwrap_or_default();
            Ok(Versioned::new("fha_loan_limits", eff, data))
        }

        fn gse_loan_limits(
            &self,
            fips_code: &str,
            year: u16,
        ) -> RefDataResult<Versioned<GseLoanLimits>> {
            let conn = self.conn.lock().unwrap();
            let row: Option<(String, String, Option<String>, i64, i64, i64, i64, i32, i32)> = conn
                .query_row(
                    "SELECT state_abbr,county_name,cbsa_name,
                             limit_1_unit,limit_2_unit,limit_3_unit,limit_4_unit,
                             is_high_cost,effective_year
                     FROM gse_loan_limits
                     WHERE fips_code=?1 AND effective_year<=?2
                     ORDER BY effective_year DESC LIMIT 1",
                    params![fips_code, year as i32],
                    |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            let (state, county, cbsa, l1, l2, l3, l4, hc, found_yr) =
                row.ok_or_else(|| RefDataError::NotFound {
                    data_type: "GseLoanLimits",
                    fips: fips_code.to_owned(),
                    year,
                })?;

            let data = GseLoanLimits {
                fips_code: fips_code.to_owned(),
                state_abbr: state,
                county_name: county,
                cbsa_name: cbsa,
                limit_1_unit: Cents(l1),
                limit_2_unit: Cents(l2),
                limit_3_unit: Cents(l3),
                limit_4_unit: Cents(l4),
                is_high_cost: hc != 0,
                effective_year: found_yr as u16,
            };
            let eff = NaiveDate::from_ymd_opt(i32::from(found_yr as u16), 1, 1).unwrap_or_default();
            Ok(Versioned::new("gse_loan_limits", eff, data))
        }

        fn usda_rural_eligibility(
            &self,
            geoid: &str,
        ) -> RefDataResult<Option<UsdaruralEligibility>> {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT fips_code,state_abbr,is_sfh_eligible,is_mfh_eligible,
                         pct_eligible,source_version
                 FROM usda_rural_eligibility WHERE geoid=?1",
                params![geoid.to_owned()],
                |r| {
                    Ok(UsdaruralEligibility {
                        geoid: geoid.to_owned(),
                        fips_code: r.get(0)?,
                        state_abbr: r.get(1)?,
                        is_sfh_eligible: r.get::<_, i32>(2)? != 0,
                        is_mfh_eligible: r.get::<_, i32>(3)? != 0,
                        pct_eligible: r.get(4)?,
                        source_version: r.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|e| RefDataError::Storage(e.to_string()))
        }

        fn usda_income_limits(
            &self,
            fips_code: &str,
            effective_date: NaiveDate,
        ) -> RefDataResult<Versioned<UsdaIncomeLimit>> {
            let conn = self.conn.lock().unwrap();
            let date_str = effective_date.format("%Y-%m-%d").to_string();
            let row: Option<(
                String,
                String,
                Option<String>,
                String,
                i64,
                i64,
                i64,
                i64,
                i64,
                i64,
                i64,
                i64,
                String,
            )> = conn
                .query_row(
                    "SELECT state_abbr,county_name,msa_name,program,
                             limit_size_1,limit_size_2,limit_size_3,limit_size_4,
                             limit_size_5,limit_size_6,limit_size_7,limit_size_8,
                             effective_date
                     FROM usda_income_limits
                     WHERE fips_code=?1 AND effective_date<=?2
                     ORDER BY effective_date DESC LIMIT 1",
                    params![fips_code.to_owned(), date_str],
                    |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                            r.get(9)?,
                            r.get(10)?,
                            r.get(11)?,
                            r.get(12)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            let (state, county, msa, prog, l1, l2, l3, l4, l5, l6, l7, l8, eff_str) = row
                .ok_or_else(|| RefDataError::NotFound {
                    data_type: "UsdaIncomeLimit",
                    fips: fips_code.to_owned(),
                    year: effective_date.format("%Y").to_string().parse().unwrap_or(0),
                })?;

            let eff = NaiveDate::parse_from_str(&eff_str, "%Y-%m-%d").unwrap_or_default();
            let data = UsdaIncomeLimit {
                fips_code: fips_code.to_owned(),
                state_abbr: state,
                county_name: county,
                msa_name: msa,
                program: prog,
                limit_size_1: Cents(l1),
                limit_size_2: Cents(l2),
                limit_size_3: Cents(l3),
                limit_size_4: Cents(l4),
                limit_size_5: Cents(l5),
                limit_size_6: Cents(l6),
                limit_size_7: Cents(l7),
                limit_size_8: Cents(l8),
                effective_date: eff,
            };
            Ok(Versioned::new("usda_income_limits", eff, data))
        }

        fn usda_mfh_by_tract(&self, geoid: &str) -> RefDataResult<Option<UsdaMfhByTract>> {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT fips_code,state_fips,county_fips,tract_number,tract_name,
                         el_projects,el_units,fa_projects,fa_units,
                         cg_projects,cg_units,gh_projects,gh_units,
                         mx_projects,mx_units,total_projects,total_units
                 FROM usda_mfh_by_tract WHERE geoid=?1",
                params![geoid.to_owned()],
                |r| {
                    Ok(UsdaMfhByTract {
                        geoid: geoid.to_owned(),
                        fips_code: r.get(0)?,
                        state_fips: r.get(1)?,
                        county_fips: r.get(2)?,
                        tract_number: r.get(3)?,
                        tract_name: r.get(4)?,
                        el_projects: r.get(5)?,
                        el_units: r.get(6)?,
                        fa_projects: r.get(7)?,
                        fa_units: r.get(8)?,
                        cg_projects: r.get(9)?,
                        cg_units: r.get(10)?,
                        gh_projects: r.get(11)?,
                        gh_units: r.get(12)?,
                        mx_projects: r.get(13)?,
                        mx_units: r.get(14)?,
                        total_projects: r.get(15)?,
                        total_units: r.get(16)?,
                    })
                },
            )
            .optional()
            .map_err(|e| RefDataError::Storage(e.to_string()))
        }

        fn ami_tract_data(
            &self,
            geoid: &str,
            year: u16,
        ) -> RefDataResult<Option<Versioned<AmiTractData>>> {
            let conn = self.conn.lock().unwrap();
            let row: Option<(
                String,
                String,
                String,
                Option<String>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                i32,
                i32,
                i32,
            )> = conn
                .query_row(
                    "SELECT fips_code,state_abbr,county_name,tract_name,
                             ami_100pct,ami_50pct,ami_80pct,ami_115pct,ami_120pct,ami_140pct,
                             is_low_income_tract,hp_income_limit_waived,effective_year
                     FROM ami_tract_data
                     WHERE geoid=?1 AND effective_year<=?2
                     ORDER BY effective_year DESC LIMIT 1",
                    params![geoid, year as i32],
                    |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                            r.get(9)?,
                            r.get(10)?,
                            r.get(11)?,
                            r.get(12)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            match row {
                None => Ok(None),
                Some((
                    fips,
                    state,
                    county,
                    tract_name,
                    a100,
                    a50,
                    a80,
                    a115,
                    a120,
                    a140,
                    low,
                    hp,
                    found_yr,
                )) => {
                    let eff = NaiveDate::from_ymd_opt(i32::from(found_yr as u16), 1, 1)
                        .unwrap_or_default();
                    let data = AmiTractData {
                        geoid: geoid.to_owned(),
                        fips_code: fips,
                        state_abbr: state,
                        county_name: county,
                        tract_name,
                        ami_100pct: a100.map(Cents),
                        ami_50pct: a50.map(Cents),
                        ami_80pct: a80.map(Cents),
                        ami_115pct: a115.map(Cents),
                        ami_120pct: a120.map(Cents),
                        ami_140pct: a140.map(Cents),
                        is_low_income_tract: low != 0,
                        hp_income_limit_waived: hp != 0,
                        effective_year: found_yr as u16,
                    };
                    Ok(Some(Versioned::new("ami_tract_data", eff, data)))
                }
            }
        }

        fn program_rules(&self, program: ProgramCode) -> RefDataResult<ProgramEligibilityRules> {
            let prog_str = program_code_str(program);
            let conn = self.conn.lock().unwrap();
            let row: Option<(
                i32,
                Option<i32>,
                Option<i32>,
                i32,
                Option<i32>,
                Option<i32>,
                i32,
                i32,
                i32,
                i32,
                i32,
                i32,
                i32,
                String,
            )> = conn
                .query_row(
                    "SELECT min_credit_score,min_credit_score_alt,
                             alt_credit_min_down_payment_bps,
                             max_ltv_bps,max_ltv_bps_alt_credit,max_ltv_bps_high_balance,
                             front_end_dti_max_bps,requires_primary_residence,
                             requires_first_time_buyer,requires_va_entitlement,
                             requires_usda_eligibility,requires_ami_income_check,
                             0,effective_date
                     FROM program_rules WHERE program=?1",
                    params![prog_str.to_owned()],
                    |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                            r.get(9)?,
                            r.get(10)?,
                            r.get(11)?,
                            r.get(12)?,
                            r.get(13)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            let (
                min_cs,
                min_cs_alt,
                alt_dp,
                max_ltv,
                max_ltv_alt,
                max_ltv_hb,
                dti,
                prim,
                ftib,
                va,
                usda,
                ami,
                _,
                eff_str,
            ) = row.ok_or_else(|| RefDataError::NotFound {
                data_type: "ProgramEligibilityRules",
                fips: prog_str.to_owned(),
                year: 0,
            })?;

            let eff = NaiveDate::parse_from_str(&eff_str, "%Y-%m-%d").unwrap_or_default();
            Ok(ProgramEligibilityRules {
                program,
                min_credit_score: min_cs as u16,
                min_credit_score_alt: min_cs_alt.map(|v| v as u16),
                alt_credit_min_down_payment_bps: alt_dp.map(|v| v as u16),
                max_ltv_bps: max_ltv as u32,
                max_ltv_bps_alt_credit: max_ltv_alt.map(|v| v as u32),
                max_ltv_bps_high_balance: max_ltv_hb.map(|v| v as u32),
                front_end_dti_max_bps: dti as u32,
                requires_primary_residence: prim != 0,
                requires_first_time_buyer: ftib != 0,
                requires_va_entitlement: va != 0,
                requires_usda_eligibility: usda != 0,
                requires_ami_income_check: ami != 0,
                effective_date: eff,
            })
        }

        fn state_hoi_rate(&self, state_abbr: &str, year: u16) -> RefDataResult<StateHoiRate> {
            let conn = self.conn.lock().unwrap();
            let row: Option<(i32, i32)> = conn
                .query_row(
                    "SELECT annual_rate_bps,effective_year FROM state_hoi_rates
                     WHERE state_abbr=?1 AND effective_year<=?2
                     ORDER BY effective_year DESC LIMIT 1",
                    params![state_abbr, year as i32],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            let (rate, found_yr) = row.ok_or_else(|| RefDataError::NotFound {
                data_type: "StateHoiRate",
                fips: state_abbr.to_owned(),
                year,
            })?;

            Ok(StateHoiRate {
                state_abbr: state_abbr.to_uppercase(),
                annual_rate_bps: rate as u16,
                effective_year: found_yr as u16,
            })
        }

        fn zip_hoi_rate(&self, zip5: &str, year: u16) -> RefDataResult<Option<ZipHoiRate>> {
            let conn = self.conn.lock().unwrap();
            let row: Option<(String, i32, Option<i64>, Option<i64>, i32)> = conn
                .query_row(
                    "SELECT state_abbr,annual_rate_bps,median_annual_premium_cents,
                             sample_size,effective_year
                     FROM zip_hoi_rates
                     WHERE zip5=?1 AND effective_year<=?2
                     ORDER BY effective_year DESC LIMIT 1",
                    params![zip5, year],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            Ok(row.map(|(state, rate, med, sample, found_yr)| ZipHoiRate {
                zip5: zip5.to_owned(),
                state_abbr: state,
                annual_rate_bps: rate as u16,
                median_annual_premium_cents: med.map(Cents),
                sample_size: sample.map(|v| v as u32),
                effective_year: found_yr as u16,
            }))
        }

        fn cbsa_for_county(&self, fips_code: &str) -> RefDataResult<Option<CbsaEntry>> {
            let conn = self.conn.lock().unwrap();
            let row: Option<(Option<String>, Option<String>, String, i32)> = conn
                .query_row(
                    "SELECT cbsa_code,cbsa_name,designation,is_metro
                     FROM cbsa_crosswalk WHERE fips_code=?1",
                    params![fips_code.to_owned()],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()
                .map_err(|e| RefDataError::Storage(e.to_string()))?;

            Ok(row.map(|(code, name, desig, metro)| CbsaEntry {
                fips_code: fips_code.to_owned(),
                cbsa_code: code,
                cbsa_name: name,
                designation: parse_designation(&desig),
                is_metro: metro != 0,
            }))
        }

        fn geo_eligibility(
            &self,
            fips_code: &str,
            tract_geoid: Option<&str>,
            year: u16,
        ) -> RefDataResult<GeoEligibility> {
            // Delegate to the standard composition — same logic as JsonFileStore
            let fha = self.fha_loan_limits(fips_code, year)?.data;
            let gse = self.gse_loan_limits(fips_code, year)?.data;

            let (usda_sfh, usda_mfh, usda_pct) = match tract_geoid {
                Some(geoid) => match self.usda_rural_eligibility(geoid)? {
                    Some(r) => (r.is_sfh_eligible, r.is_mfh_eligible, r.pct_eligible),
                    None => (false, false, None),
                },
                None => (false, false, None),
            };

            let eff_date = NaiveDate::from_ymd_opt(i32::from(year), 10, 1).unwrap_or_default();
            let usda_income = self.usda_income_limits(fips_code, eff_date).ok();
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
            JsonFileStore::new(&self.data_dir).current_version(dataset)
        }

        fn fha_mip(&self, input: &FhaMipInput, year: u16) -> RefDataResult<FhaMipResult> {
            JsonFileStore::new(&self.data_dir).fha_mip(input, year)
        }

        fn va_funding_fee(&self, input: &VaFeeInput, year: u16) -> RefDataResult<u32> {
            JsonFileStore::new(&self.data_dir).va_funding_fee(input, year)
        }

        fn conv_mi_coverage(
            &self,
            input: &ConvMiInput,
            year: u16,
        ) -> RefDataResult<ConvMiCoverage> {
            JsonFileStore::new(&self.data_dir).conv_mi_coverage(input, year)
        }

        fn mi_monthly_rate(
            &self,
            provider: &str,
            input: &MiRateInput,
            year: u16,
        ) -> RefDataResult<u16> {
            JsonFileStore::new(&self.data_dir).mi_monthly_rate(provider, input, year)
        }

        fn usda_guarantee_fees(&self, year: u16) -> RefDataResult<UsdaGuaranteeFees> {
            JsonFileStore::new(&self.data_dir).usda_guarantee_fees(year)
        }

        fn lender_profile(&self, lender_id: &str) -> RefDataResult<Option<LenderProfile>> {
            JsonFileStore::new(&self.data_dir).lender_profile(lender_id)
        }

        fn lender_overlays(
            &self,
            lender_id: &str,
            program: ProgramCode,
        ) -> RefDataResult<Option<LenderOverlays>> {
            JsonFileStore::new(&self.data_dir).lender_overlays(lender_id, program)
        }

        fn mi_single_premium_bps(
            &self,
            provider: &str,
            input: &MiRateInput,
            year: u16,
        ) -> RefDataResult<u16> {
            JsonFileStore::new(&self.data_dir).mi_single_premium_bps(provider, input, year)
        }

        fn llpa_total(&self, agency: &str, input: &LlpaInput, year: u16) -> RefDataResult<i32> {
            JsonFileStore::new(&self.data_dir).llpa_total(agency, input, year)
        }

        fn rate_sheet(&self, lender_id: &str) -> RefDataResult<Option<RateSheet>> {
            JsonFileStore::new(&self.data_dir).rate_sheet(lender_id)
        }

        fn fha_condo_project(
            &self,
            fha_project_id: &str,
        ) -> RefDataResult<Option<FhaCondoProject>> {
            JsonFileStore::new(&self.data_dir).fha_condo_project(fha_project_id)
        }

        fn mcc_program(
            &self,
            state: &str,
            year: u16,
        ) -> RefDataResult<Option<Derived<MccProgram>>> {
            JsonFileStore::new(&self.data_dir).mcc_program(state, year)
        }

        fn mcc_evaluate(
            &self,
            input: &MccEligibilityInput,
            year: u16,
        ) -> RefDataResult<Option<Derived<MccOutcome>>> {
            JsonFileStore::new(&self.data_dir).mcc_evaluate(input, year)
        }

        fn dpa_programs_for_state(
            &self,
            state: &str,
            year: u16,
        ) -> RefDataResult<Vec<Derived<DpaProgram>>> {
            JsonFileStore::new(&self.data_dir).dpa_programs_for_state(state, year)
        }

        fn dpa_program(
            &self,
            program_id: &str,
            year: u16,
        ) -> RefDataResult<Option<Derived<DpaProgram>>> {
            JsonFileStore::new(&self.data_dir).dpa_program(program_id, year)
        }

        fn dpa_evaluate(
            &self,
            program_id: &str,
            input: &DpaEligibilityInput,
            year: u16,
        ) -> RefDataResult<Option<Derived<DpaOutcome>>> {
            JsonFileStore::new(&self.data_dir).dpa_evaluate(program_id, input, year)
        }
    }

    // ── Schema ────────────────────────────────────────────────────────────────

    fn init_schema(conn: &Connection) -> RefDataResult<()> {
        conn.execute_batch(SQLITE_SCHEMA)
            .map_err(|e| RefDataError::Storage(format!("schema init: {e}")))
    }

    const SQLITE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS fha_loan_limits (
        fips_code TEXT NOT NULL, state_abbr TEXT NOT NULL, county_name TEXT NOT NULL,
        limit_type TEXT NOT NULL, limit_1_unit INTEGER NOT NULL, limit_2_unit INTEGER NOT NULL,
        limit_3_unit INTEGER NOT NULL, limit_4_unit INTEGER NOT NULL, effective_year INTEGER NOT NULL,
        PRIMARY KEY (fips_code, effective_year));
    CREATE TABLE IF NOT EXISTS gse_loan_limits (
        fips_code TEXT NOT NULL, state_abbr TEXT NOT NULL, county_name TEXT NOT NULL,
        cbsa_name TEXT, limit_1_unit INTEGER NOT NULL, limit_2_unit INTEGER NOT NULL,
        limit_3_unit INTEGER NOT NULL, limit_4_unit INTEGER NOT NULL,
        is_high_cost INTEGER NOT NULL DEFAULT 0, effective_year INTEGER NOT NULL,
        PRIMARY KEY (fips_code, effective_year));
    CREATE TABLE IF NOT EXISTS usda_rural_eligibility (
        geoid TEXT NOT NULL PRIMARY KEY, fips_code TEXT NOT NULL, state_abbr TEXT NOT NULL,
        is_sfh_eligible INTEGER NOT NULL, is_mfh_eligible INTEGER NOT NULL,
        pct_eligible REAL, source_version TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS usda_income_limits (
        fips_code TEXT NOT NULL, state_abbr TEXT NOT NULL, county_name TEXT NOT NULL,
        msa_name TEXT, program TEXT NOT NULL DEFAULT 'SFGH',
        limit_size_1 INTEGER NOT NULL, limit_size_2 INTEGER NOT NULL,
        limit_size_3 INTEGER NOT NULL, limit_size_4 INTEGER NOT NULL,
        limit_size_5 INTEGER NOT NULL, limit_size_6 INTEGER NOT NULL,
        limit_size_7 INTEGER NOT NULL, limit_size_8 INTEGER NOT NULL,
        effective_date TEXT NOT NULL, PRIMARY KEY (fips_code, effective_date));
    CREATE TABLE IF NOT EXISTS usda_mfh_by_tract (
        geoid TEXT NOT NULL PRIMARY KEY, fips_code TEXT NOT NULL,
        state_fips TEXT NOT NULL, county_fips TEXT NOT NULL,
        tract_number TEXT NOT NULL, tract_name TEXT,
        el_projects INTEGER NOT NULL DEFAULT 0, el_units INTEGER NOT NULL DEFAULT 0,
        fa_projects INTEGER NOT NULL DEFAULT 0, fa_units INTEGER NOT NULL DEFAULT 0,
        cg_projects INTEGER NOT NULL DEFAULT 0, cg_units INTEGER NOT NULL DEFAULT 0,
        gh_projects INTEGER NOT NULL DEFAULT 0, gh_units INTEGER NOT NULL DEFAULT 0,
        mx_projects INTEGER NOT NULL DEFAULT 0, mx_units INTEGER NOT NULL DEFAULT 0,
        total_projects INTEGER NOT NULL DEFAULT 0, total_units INTEGER NOT NULL DEFAULT 0);
    CREATE TABLE IF NOT EXISTS ami_tract_data (
        geoid TEXT NOT NULL, fips_code TEXT NOT NULL, state_abbr TEXT NOT NULL,
        county_name TEXT NOT NULL, tract_name TEXT,
        ami_100pct INTEGER, ami_50pct INTEGER, ami_80pct INTEGER,
        ami_115pct INTEGER, ami_120pct INTEGER, ami_140pct INTEGER,
        is_low_income_tract INTEGER NOT NULL DEFAULT 0,
        hp_income_limit_waived INTEGER NOT NULL DEFAULT 0,
        effective_year INTEGER NOT NULL, PRIMARY KEY (geoid, effective_year));
    CREATE TABLE IF NOT EXISTS program_rules (
        program TEXT NOT NULL PRIMARY KEY, min_credit_score INTEGER NOT NULL,
        min_credit_score_alt INTEGER, alt_credit_min_down_payment_bps INTEGER,
        max_ltv_bps INTEGER NOT NULL, max_ltv_bps_alt_credit INTEGER,
        max_ltv_bps_high_balance INTEGER, front_end_dti_max_bps INTEGER NOT NULL,
        requires_primary_residence INTEGER NOT NULL DEFAULT 0,
        requires_first_time_buyer INTEGER NOT NULL DEFAULT 0,
        requires_va_entitlement INTEGER NOT NULL DEFAULT 0,
        requires_usda_eligibility INTEGER NOT NULL DEFAULT 0,
        requires_ami_income_check INTEGER NOT NULL DEFAULT 0,
        effective_date TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS state_hoi_rates (
        state_abbr TEXT NOT NULL, annual_rate_bps INTEGER NOT NULL,
        effective_year INTEGER NOT NULL, PRIMARY KEY (state_abbr, effective_year));
    CREATE TABLE IF NOT EXISTS zip_hoi_rates (
        zip5 TEXT NOT NULL, state_abbr TEXT NOT NULL, annual_rate_bps INTEGER NOT NULL,
        median_annual_premium_cents INTEGER, sample_size INTEGER,
        effective_year INTEGER NOT NULL, PRIMARY KEY (zip5, effective_year));
    CREATE TABLE IF NOT EXISTS cbsa_crosswalk (
        fips_code TEXT NOT NULL PRIMARY KEY, cbsa_code TEXT, cbsa_name TEXT,
        designation TEXT NOT NULL, is_metro INTEGER NOT NULL DEFAULT 0);
    ";

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn default_data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
    }

    fn find_years(data_dir: &std::path::Path, prefix: &str) -> Vec<u16> {
        let mut years = Vec::new();
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy().to_string();
                if let Some(rest) = name.strip_prefix(&format!("{prefix}_")) {
                    if let Some(yr_str) = rest.strip_suffix(".json") {
                        if let Ok(yr) = yr_str.parse::<u16>() {
                            years.push(yr);
                        }
                    }
                }
            }
        }
        years
    }

    fn limit_type_str(lt: FhaLimitType) -> &'static str {
        match lt {
            FhaLimitType::Floor => "Floor",
            FhaLimitType::Standard => "Standard",
            FhaLimitType::HighCost => "HighCost",
        }
    }

    fn parse_limit_type(s: &str) -> FhaLimitType {
        match s {
            "Floor" => FhaLimitType::Floor,
            "HighCost" => FhaLimitType::HighCost,
            _ => FhaLimitType::Standard,
        }
    }

    fn program_code_str(p: ProgramCode) -> &'static str {
        match p {
            ProgramCode::Conventional => "conventional",
            ProgramCode::HomeReady => "home_ready",
            ProgramCode::HomePossible => "home_possible",
            ProgramCode::HomeOne => "home_one",
            ProgramCode::Fha => "fha",
            ProgramCode::FhaDpa => "fha_dpa",
            ProgramCode::Va => "va",
            ProgramCode::VaJumbo => "va_jumbo",
            ProgramCode::Usda => "usda",
            ProgramCode::Bond => "bond",
            ProgramCode::Jumbo => "jumbo",
            ProgramCode::NonQm => "non_qm",
        }
    }

    fn designation_str(d: CbsaDesignation) -> &'static str {
        match d {
            CbsaDesignation::Metropolitan => "metropolitan",
            CbsaDesignation::Micropolitan => "micropolitan",
            CbsaDesignation::Rural => "rural",
        }
    }

    fn parse_designation(s: &str) -> CbsaDesignation {
        match s {
            "metropolitan" => CbsaDesignation::Metropolitan,
            "micropolitan" => CbsaDesignation::Micropolitan,
            _ => CbsaDesignation::Rural,
        }
    }
}

#[cfg(feature = "sqlite")]
pub use inner::SqliteStore;
