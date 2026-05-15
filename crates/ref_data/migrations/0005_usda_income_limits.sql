-- Migration 0005: USDA Section 502 Single Family Guaranteed Housing (SFGH) income limits.
-- Source: USDA Rural Development (annual, typically October/November effective date).
-- All 8 household sizes stored; sizes 1-4 share a limit, sizes 5-8 share a higher limit.

CREATE TABLE IF NOT EXISTS usda_income_limits (
    fips_code       TEXT        NOT NULL,
    state_abbr      TEXT        NOT NULL,
    county_name     TEXT        NOT NULL,
    msa_name        TEXT,
    program         TEXT        NOT NULL DEFAULT 'SFGH',
    limit_size_1    BIGINT      NOT NULL,
    limit_size_2    BIGINT      NOT NULL,
    limit_size_3    BIGINT      NOT NULL,
    limit_size_4    BIGINT      NOT NULL,
    limit_size_5    BIGINT      NOT NULL,  -- 115% AMI tier
    limit_size_6    BIGINT      NOT NULL,
    limit_size_7    BIGINT      NOT NULL,
    limit_size_8    BIGINT      NOT NULL,
    effective_date  DATE        NOT NULL,
    PRIMARY KEY (fips_code, effective_date)
);
