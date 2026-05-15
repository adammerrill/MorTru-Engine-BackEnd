//! Address and location extraction methods for [`PropertyReso`] — Categories 5–6.
//!
//! # Categories covered
//!
//! - Category 5: Address — state parsing, postal code normalization, street
//!   component assembly, county extraction
//! - Category 6: Geographic coordinates — WGS 84 validation, coordinate pair
//!   extraction, FIPS derivation path selection
//!
//! # FIPS derivation priority
//!
//! FIPS code is the engine's jurisdictional authority. The derivation order is:
//!
//! 1. `TaxTract` field (some MLS feeds embed the 11-digit census tract GEOID)
//! 2. FCC Census Geocoder API (Task 3.10) — `lat/lon → FIPS` — most reliable
//! 3. State + county name lookup (approximate, requires reference table)
//!
//! Methods here implement path 1 (field extraction). Path 2 is in `fcc.rs`.

use std::str::FromStr;

use crate::{
    error::{ResoError, ResoResult},
    property::PropertyReso,
};

impl PropertyReso {
    // ── State / Province ──────────────────────────────────────────────────────

    /// Parse `StateOrProvince` into `types::StateCode`.
    ///
    /// RESO 2.0 specifies `StateOrProvince` as a 2-letter US state abbreviation
    /// or Canadian province code. The engine supports all 56 US state/territory
    /// codes defined by `types::StateCode`.
    ///
    /// Returns `Err(MissingField)` if absent, `Err(InvalidLookup)` if
    /// the string is not a recognized code.
    pub fn state_code(&self) -> ResoResult<types::StateCode> {
        let s = self
            .state_or_province
            .as_deref()
            .ok_or(ResoError::MissingField {
                field: "StateOrProvince",
            })?;

        types::StateCode::from_str(s).map_err(|_| ResoError::InvalidLookup {
            field: "StateOrProvince",
            value: s.to_owned(),
        })
    }

    /// True if `StateOrProvince` is present and parses to a valid state code.
    pub fn has_valid_state(&self) -> bool {
        self.state_code().is_ok()
    }

    // ── Postal code ───────────────────────────────────────────────────────────

    /// Return the 5-digit ZIP code portion of `PostalCode`.
    ///
    /// Many RESO feeds send ZIP+4 (e.g. "78640-1234"). This method strips
    /// everything after the first hyphen and trims whitespace.
    /// Returns `None` if the field is absent.
    ///
    /// Note: ZIP codes are display-only in the engine. FIPS code (not ZIP)
    /// is the jurisdictional authority for all fee and eligibility lookups.
    pub fn postal_code_5digit(&self) -> Option<&str> {
        self.postal_code
            .as_deref()
            .map(|s| s.trim())
            .map(|s| s.split('-').next().unwrap_or(s))
            .filter(|s| !s.is_empty())
    }

    // ── County ────────────────────────────────────────────────────────────────

    /// County or parish name from the `CountyOrParish` field.
    /// Trims whitespace. Returns `None` if absent.
    pub fn county_name(&self) -> Option<&str> {
        self.county_or_parish
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    // ── Street address assembly ───────────────────────────────────────────────

    /// Best available single-line street address.
    ///
    /// Priority:
    /// 1. `UnparsedAddress` — already a full string, use directly
    /// 2. Compose from components: `StreetNumber StreetDirPrefix StreetName
    ///    StreetSuffix StreetDirSuffix [Unit UnitNumber]`
    ///
    /// Returns `None` if neither the full address nor any components are present.
    pub fn best_address(&self) -> Option<String> {
        // Prefer the pre-assembled full address
        if let Some(a) = self.unparsed_address.as_deref() {
            let trimmed = a.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }

        // Compose from components
        self.composed_street_address()
    }

    /// Compose street address from parsed RESO address components.
    ///
    /// Returns `None` if `StreetName` is absent (the minimum required component).
    pub fn composed_street_address(&self) -> Option<String> {
        let name = self.street_name.as_deref()?.trim();
        if name.is_empty() {
            return None;
        }

        let mut parts: Vec<&str> = Vec::with_capacity(7);

        if let Some(n) = self.street_number.as_deref() {
            let n = n.trim();
            if !n.is_empty() {
                parts.push(n);
            }
        }
        if let Some(d) = self.street_dir_prefix.as_deref() {
            let d = d.trim();
            if !d.is_empty() {
                parts.push(d);
            }
        }
        parts.push(name);
        if let Some(s) = self.street_suffix.as_deref() {
            let s = s.trim();
            if !s.is_empty() {
                parts.push(s);
            }
        }
        if let Some(d) = self.street_dir_suffix.as_deref() {
            let d = d.trim();
            if !d.is_empty() {
                parts.push(d);
            }
        }

        let mut address = parts.join(" ");

        // Append unit if present
        if let Some(u) = self.unit_number.as_deref() {
            let u = u.trim();
            if !u.is_empty() {
                let designator = self.unit_number_type.as_deref().unwrap_or("Unit").trim();
                address = format!("{address} {designator} {u}");
            }
        }

        Some(address)
    }

    /// Full city/state/zip line (e.g. "Kyle, TX 78640").
    ///
    /// Returns `None` if both `City` and `StateOrProvince` are absent.
    pub fn city_state_zip(&self) -> Option<String> {
        let city = self
            .city
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let state = self
            .state_or_province
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let zip = self.postal_code_5digit();

        match (city, state) {
            (Some(c), Some(s)) => match zip {
                Some(z) => Some(format!("{c}, {s} {z}")),
                None => Some(format!("{c}, {s}")),
            },
            (Some(c), None) => Some(c.to_owned()),
            (None, Some(s)) => Some(s.to_owned()),
            (None, None) => None,
        }
    }

    // ── Geographic coordinates ────────────────────────────────────────────────

    /// Return `(latitude, longitude)` as a pair if both fields are present.
    pub fn coordinates(&self) -> Option<(f64, f64)> {
        match (self.latitude, self.longitude) {
            (Some(lat), Some(lon)) => Some((lat, lon)),
            _ => None,
        }
    }

    /// True if both `Latitude` and `Longitude` are present.
    pub fn has_coordinates(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }

    /// Validate and return `(latitude, longitude)`.
    ///
    /// WGS 84 bounds: latitude -90.0..=90.0, longitude -180.0..=180.0.
    /// Also rejects (0.0, 0.0) — the null island coordinate, which is never
    /// a valid US address and indicates a geocoding failure in the MLS feed.
    pub fn validated_coordinates(&self) -> ResoResult<(f64, f64)> {
        let (lat, lon) = self.coordinates().ok_or(ResoError::MissingField {
            field: "Latitude/Longitude",
        })?;

        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(ResoError::InvalidCoordinate { lat, lon });
        }

        // Reject null island — (0.0, 0.0) is never a valid US property
        if lat == 0.0 && lon == 0.0 {
            return Err(ResoError::InvalidCoordinate { lat, lon });
        }

        Ok((lat, lon))
    }

    /// True if the coordinates are present and valid for a US property.
    pub fn has_valid_coordinates(&self) -> bool {
        self.validated_coordinates().is_ok()
    }

    // ── FIPS derivation path ──────────────────────────────────────────────────

    /// Attempt to extract a 5-digit county FIPS code directly from RESO fields.
    ///
    /// Some MLS feeds populate `TaxTract` with the 11-digit census tract GEOID
    /// (e.g. "48209010905"). If so, the first 5 digits are the county FIPS.
    ///
    /// Returns `None` if no FIPS-derivable field is populated with a
    /// plausibly-valid value. The FCC API (Task 3.10) is the authoritative
    /// FIPS resolution path when coordinates are available.
    pub fn fips_from_fields(&self) -> Option<String> {
        // Check TaxTract for 11-digit GEOID (some feeds use this)
        if let Some(tract) = self.tax_tract.as_deref() {
            let digits: String = tract.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 11 {
                return Some(digits[..5].to_owned());
            }
        }

        None
    }

    /// True if a FIPS code can be derived from fields alone (without FCC API).
    pub fn has_field_derived_fips(&self) -> bool {
        self.fips_from_fields().is_some()
    }
}
