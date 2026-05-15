-- Migration 0008: Agency/GSE program eligibility rules.
-- Source: FHA, VA, USDA, Fannie Mae, Freddie Mac guidelines (as published).
-- Lender overlays (Task 4.16) tighten but never loosen these values.

CREATE TABLE IF NOT EXISTS program_rules (
    program                         TEXT        NOT NULL PRIMARY KEY,
    min_credit_score                SMALLINT    NOT NULL,
    min_credit_score_alt            SMALLINT,   -- FHA: 500 (requires >= 10% down)
    alt_credit_min_down_payment_bps SMALLINT,   -- FHA: 1000 (10%)
    max_ltv_bps                     INTEGER     NOT NULL,
    max_ltv_bps_alt_credit          INTEGER,    -- FHA: 9000 for 500-579 credit
    max_ltv_bps_high_balance        INTEGER,    -- Conventional: 9000 for HB
    front_end_dti_max_bps           INTEGER     NOT NULL,  -- VA: 9999 (not used)
    requires_primary_residence      BOOLEAN     NOT NULL DEFAULT FALSE,
    requires_first_time_buyer       BOOLEAN     NOT NULL DEFAULT FALSE,
    requires_va_entitlement         BOOLEAN     NOT NULL DEFAULT FALSE,
    requires_usda_eligibility       BOOLEAN     NOT NULL DEFAULT FALSE,
    requires_ami_income_check       BOOLEAN     NOT NULL DEFAULT FALSE,
    effective_date                  DATE        NOT NULL
);
