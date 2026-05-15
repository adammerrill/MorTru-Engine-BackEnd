-- Migration 0006: USDA Multi-Family Housing (MFH) project counts by census tract.
-- Source: USDA RD (annual update). Used to identify tracts with active USDA housing.
-- Project type codes: EL=Elderly, FA=Family, CG=Congregate, GH=Group Home, MX=Mixed.

CREATE TABLE IF NOT EXISTS usda_mfh_by_tract (
    geoid           TEXT        NOT NULL PRIMARY KEY,
    fips_code       TEXT        NOT NULL,
    state_fips      TEXT        NOT NULL,
    county_fips     TEXT        NOT NULL,
    tract_number    TEXT        NOT NULL,
    tract_name      TEXT,
    el_projects     SMALLINT    NOT NULL DEFAULT 0,
    el_units        SMALLINT    NOT NULL DEFAULT 0,
    fa_projects     SMALLINT    NOT NULL DEFAULT 0,
    fa_units        SMALLINT    NOT NULL DEFAULT 0,
    cg_projects     SMALLINT    NOT NULL DEFAULT 0,
    cg_units        SMALLINT    NOT NULL DEFAULT 0,
    gh_projects     SMALLINT    NOT NULL DEFAULT 0,
    gh_units        SMALLINT    NOT NULL DEFAULT 0,
    mx_projects     SMALLINT    NOT NULL DEFAULT 0,
    mx_units        SMALLINT    NOT NULL DEFAULT 0,
    total_projects  SMALLINT    NOT NULL DEFAULT 0,
    total_units     SMALLINT    NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_usda_mfh_fips
    ON usda_mfh_by_tract (fips_code);
