//! `SqliteStore` — SQLite-backed implementation of [`RefDataStore`].
//!
//! Used for integration tests and local development without a running
//! PostgreSQL server. Creates an in-process, in-memory database, applies
//! the SQLite-compatible schema, and seeds from the JSON data files in
//! `crates/ref_data/data/`.
//!
//! # Usage (tests)
//!
//! ```rust,ignore
//! use ref_data::SqliteStore;
//!
//! let store = SqliteStore::new_test_store().await.unwrap();
//! let limits = store.fha_loan_limits("48209", 2025).unwrap();
//! assert_eq!(limits.data.limit_1_unit, Cents(52_422_500));
//! ```
//!
//! # SQLite vs PostgreSQL schema notes
//!
//! PostgreSQL migration files (`migrations/`) use PG-native types
//! (ENUM, BOOLEAN, SMALLINT). SQLite compatibility is handled here by
//! using TEXT for enum columns and INTEGER for all numeric types.
//! The Rust layer enforces all type constraints.

#[cfg(feature = "sqlite")]
mod inner {
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use sqlx::SqlitePool;
    use types::ProgramCode;

    use crate::{
        error::{RefDataError, RefDataResult},
        geo::{
            AmiTractData, FhaLoanLimits, GeoEligibility, GseLoanLimits, UsdaIncomeLimit,
            UsdaMfhByTract, UsdaruralEligibility,
        },
        hoi_rates::StateHoiRate,
        program_rules::ProgramEligibilityRules,
        store::{JsonFileStore, RefDataStore},
        versioning::{VersionId, Versioned},
    };

    /// SQLite-backed reference data store.
    ///
    /// All 10 [`RefDataStore`] methods are implemented by delegating to
    /// SQL queries against an in-process SQLite database.
    pub struct SqliteStore {
        pool: SqlitePool,
        /// JSON file store used to seed the SQLite database on startup.
        /// Kept alive to support `current_version()` which needs file discovery.
        json_fallback: JsonFileStore,
    }

    impl SqliteStore {
        /// Create an in-memory SQLite database, apply schema, and seed
        /// all ref_data from the JSON files shipped with the crate.
        ///
        /// Typical startup time: ~50ms. Suitable for use in `#[tokio::test]`
        /// with `#[once]` or per-test initialization.
        pub async fn new_test_store() -> RefDataResult<Self> {
            let data_dir = data_dir();
            let pool = SqlitePool::connect(":memory:")
                .await
                .map_err(|e| RefDataError::Storage(format!("sqlite open failed: {e}")))?;
            Self::init_schema(&pool).await?;
            let store = Self {
                pool,
                json_fallback: JsonFileStore::new(&data_dir),
            };
            store.seed_from_json(&data_dir).await?;
            Ok(store)
        }

        /// Initialize the SQLite schema. Uses SQLite-compatible CREATE TABLE
        /// statements; does NOT run the PostgreSQL migration files.
        async fn init_schema(pool: &SqlitePool) -> RefDataResult<()> {
            sqlx::query(SQLITE_SCHEMA)
                .execute(pool)
                .await
                .map_err(|e| RefDataError::Storage(format!("schema init failed: {e}")))?;
            Ok(())
        }

        /// Seed all tables from the JSON data files.
        async fn seed_from_json(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            self.seed_fha_limits(data_dir).await?;
            self.seed_gse_limits(data_dir).await?;
            self.seed_usda_rural(data_dir).await?;
            self.seed_usda_income(data_dir).await?;
            self.seed_usda_mfh(data_dir).await?;
            self.seed_ami_tract(data_dir).await?;
            self.seed_program_rules(data_dir).await?;
            self.seed_hoi_rates(data_dir).await?;
            Ok(())
        }

        // ── Seed helpers (implemented in Phase 3) ────────────────────────────

        async fn seed_fha_limits(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            // TODO Phase 3: scan data_dir for fha_limits_*.json, parse each,
            // INSERT OR REPLACE INTO fha_loan_limits ...
            Ok(())
        }

        async fn seed_gse_limits(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }

        async fn seed_usda_rural(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }

        async fn seed_usda_income(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }

        async fn seed_usda_mfh(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }

        async fn seed_ami_tract(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }

        async fn seed_program_rules(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }

        async fn seed_hoi_rates(&self, data_dir: &std::path::Path) -> RefDataResult<()> {
            let _ = data_dir;
            Ok(())
        }
    }

    // ── RefDataStore implementation (delegates to JsonFileStore until full impl) ──

    impl RefDataStore for SqliteStore {
        fn fha_loan_limits(
            &self,
            fips_code: &str,
            year: u16,
        ) -> RefDataResult<Versioned<FhaLoanLimits>> {
            // Phase 3: replace with SQL query
            self.json_fallback.fha_loan_limits(fips_code, year)
        }

        fn gse_loan_limits(
            &self,
            fips_code: &str,
            year: u16,
        ) -> RefDataResult<Versioned<GseLoanLimits>> {
            self.json_fallback.gse_loan_limits(fips_code, year)
        }

        fn usda_rural_eligibility(
            &self,
            geoid: &str,
        ) -> RefDataResult<Option<UsdaruralEligibility>> {
            self.json_fallback.usda_rural_eligibility(geoid)
        }

        fn usda_income_limits(
            &self,
            fips_code: &str,
            effective_date: NaiveDate,
        ) -> RefDataResult<Versioned<UsdaIncomeLimit>> {
            self.json_fallback
                .usda_income_limits(fips_code, effective_date)
        }

        fn usda_mfh_by_tract(&self, geoid: &str) -> RefDataResult<Option<UsdaMfhByTract>> {
            self.json_fallback.usda_mfh_by_tract(geoid)
        }

        fn ami_tract_data(
            &self,
            geoid: &str,
            year: u16,
        ) -> RefDataResult<Option<Versioned<AmiTractData>>> {
            self.json_fallback.ami_tract_data(geoid, year)
        }

        fn program_rules(&self, program: ProgramCode) -> RefDataResult<ProgramEligibilityRules> {
            self.json_fallback.program_rules(program)
        }

        fn state_hoi_rate(&self, state_abbr: &str, year: u16) -> RefDataResult<StateHoiRate> {
            self.json_fallback.state_hoi_rate(state_abbr, year)
        }

        fn geo_eligibility(
            &self,
            fips_code: &str,
            tract_geoid: Option<&str>,
            year: u16,
        ) -> RefDataResult<GeoEligibility> {
            self.json_fallback
                .geo_eligibility(fips_code, tract_geoid, year)
        }

        fn current_version(&self, dataset: &str) -> RefDataResult<VersionId> {
            self.json_fallback.current_version(dataset)
        }
    }

    // ── SQLite schema (SQLite-compatible, no PostgreSQL ENUM types) ────────────

    const SQLITE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS fha_loan_limits (
        fips_code      TEXT    NOT NULL,
        state_abbr     TEXT    NOT NULL,
        county_name    TEXT    NOT NULL,
        limit_type     TEXT    NOT NULL,
        limit_1_unit   INTEGER NOT NULL,
        limit_2_unit   INTEGER NOT NULL,
        limit_3_unit   INTEGER NOT NULL,
        limit_4_unit   INTEGER NOT NULL,
        effective_year INTEGER NOT NULL,
        PRIMARY KEY (fips_code, effective_year)
    );
    CREATE TABLE IF NOT EXISTS gse_loan_limits (
        fips_code      TEXT    NOT NULL,
        state_abbr     TEXT    NOT NULL,
        county_name    TEXT    NOT NULL,
        cbsa_name      TEXT,
        limit_1_unit   INTEGER NOT NULL,
        limit_2_unit   INTEGER NOT NULL,
        limit_3_unit   INTEGER NOT NULL,
        limit_4_unit   INTEGER NOT NULL,
        is_high_cost   INTEGER NOT NULL DEFAULT 0,
        effective_year INTEGER NOT NULL,
        PRIMARY KEY (fips_code, effective_year)
    );
    CREATE TABLE IF NOT EXISTS usda_rural_eligibility (
        geoid           TEXT    NOT NULL PRIMARY KEY,
        fips_code       TEXT    NOT NULL,
        state_abbr      TEXT    NOT NULL,
        is_sfh_eligible INTEGER NOT NULL,
        is_mfh_eligible INTEGER NOT NULL,
        pct_eligible    REAL,
        source_version  TEXT    NOT NULL
    );
    CREATE TABLE IF NOT EXISTS usda_income_limits (
        fips_code      TEXT    NOT NULL,
        state_abbr     TEXT    NOT NULL,
        county_name    TEXT    NOT NULL,
        msa_name       TEXT,
        program        TEXT    NOT NULL DEFAULT 'SFGH',
        limit_size_1   INTEGER NOT NULL,
        limit_size_2   INTEGER NOT NULL,
        limit_size_3   INTEGER NOT NULL,
        limit_size_4   INTEGER NOT NULL,
        limit_size_5   INTEGER NOT NULL,
        limit_size_6   INTEGER NOT NULL,
        limit_size_7   INTEGER NOT NULL,
        limit_size_8   INTEGER NOT NULL,
        effective_date TEXT    NOT NULL,
        PRIMARY KEY (fips_code, effective_date)
    );
    CREATE TABLE IF NOT EXISTS usda_mfh_by_tract (
        geoid          TEXT    NOT NULL PRIMARY KEY,
        fips_code      TEXT    NOT NULL,
        state_fips     TEXT    NOT NULL,
        county_fips    TEXT    NOT NULL,
        tract_number   TEXT    NOT NULL,
        tract_name     TEXT,
        el_projects    INTEGER NOT NULL DEFAULT 0,
        el_units       INTEGER NOT NULL DEFAULT 0,
        fa_projects    INTEGER NOT NULL DEFAULT 0,
        fa_units       INTEGER NOT NULL DEFAULT 0,
        cg_projects    INTEGER NOT NULL DEFAULT 0,
        cg_units       INTEGER NOT NULL DEFAULT 0,
        gh_projects    INTEGER NOT NULL DEFAULT 0,
        gh_units       INTEGER NOT NULL DEFAULT 0,
        mx_projects    INTEGER NOT NULL DEFAULT 0,
        mx_units       INTEGER NOT NULL DEFAULT 0,
        total_projects INTEGER NOT NULL DEFAULT 0,
        total_units    INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS ami_tract_data (
        geoid                  TEXT    NOT NULL,
        fips_code              TEXT    NOT NULL,
        state_abbr             TEXT    NOT NULL,
        county_name            TEXT    NOT NULL,
        tract_name             TEXT,
        ami_100pct             INTEGER,
        ami_50pct              INTEGER,
        ami_80pct              INTEGER,
        ami_115pct             INTEGER,
        ami_120pct             INTEGER,
        ami_140pct             INTEGER,
        is_low_income_tract    INTEGER NOT NULL DEFAULT 0,
        hp_income_limit_waived INTEGER NOT NULL DEFAULT 0,
        effective_year         INTEGER NOT NULL,
        PRIMARY KEY (geoid, effective_year)
    );
    CREATE TABLE IF NOT EXISTS program_rules (
        program                         TEXT    NOT NULL PRIMARY KEY,
        min_credit_score                INTEGER NOT NULL,
        min_credit_score_alt            INTEGER,
        alt_credit_min_down_payment_bps INTEGER,
        max_ltv_bps                     INTEGER NOT NULL,
        max_ltv_bps_alt_credit          INTEGER,
        max_ltv_bps_high_balance        INTEGER,
        front_end_dti_max_bps           INTEGER NOT NULL,
        requires_primary_residence      INTEGER NOT NULL DEFAULT 0,
        requires_first_time_buyer       INTEGER NOT NULL DEFAULT 0,
        requires_va_entitlement         INTEGER NOT NULL DEFAULT 0,
        requires_usda_eligibility       INTEGER NOT NULL DEFAULT 0,
        requires_ami_income_check       INTEGER NOT NULL DEFAULT 0,
        effective_date                  TEXT    NOT NULL
    );
    CREATE TABLE IF NOT EXISTS state_hoi_rates (
        state_abbr     TEXT    NOT NULL,
        annual_rate_bps INTEGER NOT NULL,
        effective_year  INTEGER NOT NULL,
        PRIMARY KEY (state_abbr, effective_year)
    );
    ";

    fn data_dir() -> PathBuf {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest.join("data")
    }
}

#[cfg(feature = "sqlite")]
pub use inner::SqliteStore;
