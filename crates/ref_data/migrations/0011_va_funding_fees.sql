-- Migration 0011: VA loan funding fee schedule.
-- Stub: full implementation in Phase 4 (Task 4.14).
-- Disability-exempt veterans: fee_bps = 0.
-- IRRRL (streamline refi): fee_bps = 50 (0.50%) regardless of down payment.

CREATE TABLE IF NOT EXISTS va_funding_fees (
    veteran_status          TEXT        NOT NULL,
    loan_purpose            TEXT        NOT NULL,  -- 'purchase' | 'cash_out_refinance' | 'irrrl'
    down_payment_min_bps    INTEGER     NOT NULL,
    down_payment_max_bps    INTEGER     NOT NULL,  -- 9999999 = no upper bound
    fee_bps                 SMALLINT    NOT NULL,
    effective_date          DATE        NOT NULL,
    PRIMARY KEY (veteran_status, loan_purpose, down_payment_min_bps, effective_date)
);
