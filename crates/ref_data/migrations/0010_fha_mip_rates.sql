-- Migration 0010: FHA mortgage insurance premium rates.
-- Stub: full implementation in Phase 4 (Task 4.13).
-- UFMIP: 1.75% of base loan (all loans, financed). Annual MIP by LTV / term / size.

CREATE TABLE IF NOT EXISTS fha_mip_rates (
    ltv_max_bps         INTEGER     NOT NULL,  -- e.g. 9500 = "LTV > 90% applies"
    term_months_max     SMALLINT    NOT NULL,  -- 180 or 360
    loan_amount_tier    TEXT        NOT NULL,  -- 'standard' | 'high_balance'
    annual_mip_bps      SMALLINT    NOT NULL,  -- e.g. 55 = 0.55%
    ufmip_bps           SMALLINT    NOT NULL DEFAULT 175,
    cancellation_months SMALLINT,              -- NULL = life of loan
    effective_date      DATE        NOT NULL,
    PRIMARY KEY (ltv_max_bps, term_months_max, loan_amount_tier, effective_date)
);
