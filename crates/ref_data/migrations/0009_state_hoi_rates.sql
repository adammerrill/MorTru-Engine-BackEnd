-- Migration 0009: State-average homeowner's insurance rates (basis points of value).
-- Source: NAIC homeowners insurance data + state insurance department filings.
-- annual_rate_bps: e.g. TX=56 means 0.56% of property value per year.
-- NATIONAL_FALLBACK_RATE_BPS=85 used when state not found.

CREATE TABLE IF NOT EXISTS state_hoi_rates (
    state_abbr      TEXT        NOT NULL,
    annual_rate_bps SMALLINT    NOT NULL,
    effective_year  SMALLINT    NOT NULL,
    PRIMARY KEY (state_abbr, effective_year)
);
