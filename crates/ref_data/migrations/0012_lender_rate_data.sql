-- Migration 0012: Lender, MI provider, and rate sheet tables.
-- Stubs only: full implementation in Phase 5 (Tasks 4.16-4.18).

CREATE TABLE IF NOT EXISTS lender_profiles (
    lender_id   TEXT    NOT NULL PRIMARY KEY,
    name        TEXT    NOT NULL,
    nmls_id     TEXT,
    active      BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS lender_overlays (
    lender_id                   TEXT        NOT NULL,
    program                     TEXT        NOT NULL,
    min_credit_score_override   SMALLINT,
    max_ltv_bps_override        INTEGER,
    dti_max_bps_override        INTEGER,
    effective_date              DATE        NOT NULL,
    PRIMARY KEY (lender_id, program, effective_date)
);

CREATE TABLE IF NOT EXISTS mi_rate_cards (
    provider        TEXT        NOT NULL,
    ltv_max_bps     INTEGER     NOT NULL,
    fico_min        SMALLINT    NOT NULL,
    coverage_pct    SMALLINT    NOT NULL,
    term_months     SMALLINT    NOT NULL,
    monthly_rate_bps SMALLINT   NOT NULL,
    effective_date  DATE        NOT NULL,
    PRIMARY KEY (provider, ltv_max_bps, fico_min, coverage_pct, term_months, effective_date)
);

CREATE TABLE IF NOT EXISTS rate_sheets (
    lender_id       TEXT        NOT NULL,
    product         TEXT        NOT NULL,
    lock_days       SMALLINT    NOT NULL,
    rate_bps        INTEGER     NOT NULL,
    price_pp        DECIMAL(8,6),
    as_of           TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (lender_id, product, lock_days, as_of)
);
