-- Migration 0003: Fannie Mae / Freddie Mac conforming loan limits by county and year.
-- Source: FHFA (annual, effective January 1).
-- is_high_cost = true when county limit exceeds the national standard baseline.

CREATE TABLE IF NOT EXISTS gse_loan_limits (
    fips_code       TEXT        NOT NULL,
    state_abbr      TEXT        NOT NULL,
    county_name     TEXT        NOT NULL,
    cbsa_name       TEXT,
    limit_1_unit    BIGINT      NOT NULL,
    limit_2_unit    BIGINT      NOT NULL,
    limit_3_unit    BIGINT      NOT NULL,
    limit_4_unit    BIGINT      NOT NULL,
    is_high_cost    BOOLEAN     NOT NULL DEFAULT FALSE,
    effective_year  SMALLINT    NOT NULL,
    PRIMARY KEY (fips_code, effective_year)
);

CREATE INDEX IF NOT EXISTS idx_gse_limits_fips
    ON gse_loan_limits (fips_code);
