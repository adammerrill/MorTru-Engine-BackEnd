-- Migration 0007: Area Median Income (AMI) by census tract and year.
-- Source: HUD / FFIEC (annual). AMI thresholds gate HomeReady/HP eligibility.
-- hp_income_limit_waived: true when is_low_income_tract; no income ceiling for HR/HP.

CREATE TABLE IF NOT EXISTS ami_tract_data (
    geoid                   TEXT        NOT NULL,
    fips_code               TEXT        NOT NULL,
    state_abbr              TEXT        NOT NULL,
    county_name             TEXT        NOT NULL,
    tract_name              TEXT,
    ami_100pct              BIGINT,     -- Cents; base AMI
    ami_50pct               BIGINT,     -- Very Low Income threshold
    ami_80pct               BIGINT,     -- Low Income / HomeReady gate
    ami_115pct              BIGINT,     -- USDA SFGH gate
    ami_120pct              BIGINT,     -- State bond program threshold
    ami_140pct              BIGINT,     -- Some state bond programs
    is_low_income_tract     BOOLEAN     NOT NULL DEFAULT FALSE,
    hp_income_limit_waived  BOOLEAN     NOT NULL DEFAULT FALSE,
    effective_year          SMALLINT    NOT NULL,
    PRIMARY KEY (geoid, effective_year)
);

CREATE INDEX IF NOT EXISTS idx_ami_fips
    ON ami_tract_data (fips_code);
