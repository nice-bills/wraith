use anyhow::{Context, Result};
use num_bigint::BigUint;
use std::str::FromStr;

/// Phase 2 queryIdentity: 14 public inputs + 9 outputs (23 signals).
pub const RARIMO_PHASE2_PUBLIC_SIGNALS: usize = 23;
pub const RARIMO_PHASE2_BIRTH_DATE_INDEX: usize = 15;
pub const RARIMO_PHASE2_NATIONALITY_INDEX: usize = 19;
pub const RARIMO_PHASE2_CITIZENSHIP_INDEX: usize = 20;

/// Layout stub: 6 public inputs (nullifier, birthDate, expirationDate, pad×2, nationality).
pub const RARIMO_LAYOUT_STUB_SIGNALS: usize = 6;
pub const RARIMO_LAYOUT_BIRTH_DATE_INDEX: usize = 1;
pub const RARIMO_LAYOUT_COUNTRY_INDEX: usize = 5;

#[derive(Debug, Clone)]
pub struct RarimoPublicSignals {
    pub birth_date: String,
    pub nationality: String,
}

impl RarimoPublicSignals {
    /// Parse birthDate and country from Rarimo query public signals (layout stub or Phase 2).
    pub fn from_query_output(signals: &[String]) -> Result<Self> {
        if signals.len() >= RARIMO_PHASE2_PUBLIC_SIGNALS {
            let nationality = &signals[RARIMO_PHASE2_NATIONALITY_INDEX];
            let country = if nationality == "0" {
                signals[RARIMO_PHASE2_CITIZENSHIP_INDEX].clone()
            } else {
                nationality.clone()
            };
            Ok(Self {
                birth_date: signals[RARIMO_PHASE2_BIRTH_DATE_INDEX].clone(),
                nationality: country,
            })
        } else if signals.len() >= RARIMO_LAYOUT_STUB_SIGNALS {
            Ok(Self {
                birth_date: signals[RARIMO_LAYOUT_BIRTH_DATE_INDEX].clone(),
                nationality: signals[RARIMO_LAYOUT_COUNTRY_INDEX].clone(),
            })
        } else {
            anyhow::bail!(
                "rarimo query circuit expects at least {} public signals (layout stub) or {} (Phase 2), got {}",
                RARIMO_LAYOUT_STUB_SIGNALS,
                RARIMO_PHASE2_PUBLIC_SIGNALS,
                signals.len()
            );
        }
    }

    pub fn parse_birth_date_yymmdd(&self) -> Result<(u16, u8, u8)> {
        let birth_str = self.birth_date.trim();

        if birth_str.is_empty() || birth_str == "0" {
            anyhow::bail!("birth_date is empty or zero");
        }

        let padded = if birth_str.len() < 6 {
            format!("{:0>6}", birth_str)
        } else {
            birth_str.to_string()
        };

        let year: u16 = padded[0..2]
            .parse()
            .with_context(|| format!("failed to parse year from '{}'", &padded[0..2]))?;
        let month: u8 = padded[2..4]
            .parse()
            .with_context(|| format!("failed to parse month from '{}'", &padded[2..4]))?;
        let day: u8 = padded[4..6]
            .parse()
            .with_context(|| format!("failed to parse day from '{}'", &padded[4..6]))?;

        Ok((year, month, day))
    }

    pub fn derive_age(&self, current_date_yymmdd: &str) -> Result<u32> {
        let (birth_yy, birth_mm, birth_dd) =
            self.parse_birth_date_yymmdd().context("parse birth date")?;

        if current_date_yymmdd.len() != 6 {
            anyhow::bail!(
                "current_date should be 6 digits (YYMMDD), got '{}'",
                current_date_yymmdd
            );
        }

        let current_yy: u16 = current_date_yymmdd[0..2].parse().with_context(|| {
            format!(
                "failed to parse current year from '{}'",
                &current_date_yymmdd[0..2]
            )
        })?;
        let current_mm: u8 = current_date_yymmdd[2..4].parse().with_context(|| {
            format!(
                "failed to parse current month from '{}'",
                &current_date_yymmdd[2..4]
            )
        })?;
        let current_dd: u8 = current_date_yymmdd[4..6].parse().with_context(|| {
            format!(
                "failed to parse current day from '{}'",
                &current_date_yymmdd[4..6]
            )
        })?;

        let birth_full_year = if birth_yy <= 50 {
            2000 + birth_yy
        } else {
            1900 + birth_yy
        };
        let current_full_year = if current_yy <= 50 {
            2000 + current_yy
        } else {
            1900 + current_yy
        };

        let mut age: u32 = (current_full_year as u32) - (birth_full_year as u32);

        if current_mm < birth_mm || (current_mm == birth_mm && current_dd < birth_dd) {
            age = age.saturating_sub(1);
        }

        Ok(age)
    }

    pub fn nationality_to_country_code(&self) -> Result<u32> {
        let nat = BigUint::from_str(&self.nationality)
            .with_context(|| format!("failed to parse nationality '{}'", self.nationality))?;

        let nat_bytes = nat.to_bytes_be();

        if nat_bytes.len() <= 4 {
            let mut padded = [0u8; 4];
            padded[4 - nat_bytes.len()..].copy_from_slice(&nat_bytes);
            Ok(u32::from_be_bytes(padded))
        } else {
            let start = nat_bytes.len() - 4;
            let mut country_bytes = [0u8; 4];
            country_bytes.copy_from_slice(&nat_bytes[start..]);
            Ok(u32::from_be_bytes(country_bytes))
        }
    }

    pub fn to_wraith_claims(&self, current_date_yymmdd: &str) -> Result<WraithClaims> {
        let age = self.derive_age(current_date_yymmdd).context("derive age")?;
        let country_code = self
            .nationality_to_country_code()
            .context("derive country code")?;

        Ok(WraithClaims {
            age,
            country_code,
            is_human: true,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WraithClaims {
    pub age: u32,
    pub country_code: u32,
    pub is_human: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_stub_signals() -> Vec<String> {
        vec![
            "42".into(),
            "950101".into(),
            "300101".into(),
            "0".into(),
            "0".into(),
            "840".into(),
        ]
    }

    fn phase2_signals() -> Vec<String> {
        let mut s = vec!["0".to_string(); RARIMO_PHASE2_PUBLIC_SIGNALS];
        s[15] = "950101".into();
        s[19] = "840".into();
        s
    }

    #[test]
    fn test_from_query_output_layout_stub() {
        let parsed = RarimoPublicSignals::from_query_output(&layout_stub_signals()).unwrap();
        assert_eq!(parsed.birth_date, "950101");
        assert_eq!(parsed.nationality, "840");
    }

    #[test]
    fn test_from_query_output_phase2() {
        let parsed = RarimoPublicSignals::from_query_output(&phase2_signals()).unwrap();
        assert_eq!(parsed.birth_date, "950101");
        assert_eq!(parsed.nationality, "840");
    }

    #[test]
    fn test_from_query_output_phase2_citizenship_fallback() {
        let mut s = phase2_signals();
        s[19] = "0".into();
        s[20] = "826".into();
        let parsed = RarimoPublicSignals::from_query_output(&s).unwrap();
        assert_eq!(parsed.nationality, "826");
    }

    #[test]
    fn test_parse_birth_date() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "0".to_string(),
        };

        let (yy, mm, dd) = signals.parse_birth_date_yymmdd().unwrap();
        assert_eq!((yy, mm, dd), (95, 1, 1));
    }

    #[test]
    fn test_parse_birth_date_rejects_empty() {
        let signals = RarimoPublicSignals {
            birth_date: "".to_string(),
            nationality: "840".to_string(),
        };
        assert!(signals.parse_birth_date_yymmdd().is_err());
    }

    #[test]
    fn test_parse_birth_date_with_leading_zeros() {
        let signals = RarimoPublicSignals {
            birth_date: "70101".to_string(),
            nationality: "0".to_string(),
        };

        let (yy, mm, dd) = signals.parse_birth_date_yymmdd().unwrap();
        assert_eq!((yy, mm, dd), (7, 1, 1));
    }

    #[test]
    fn test_derive_age_older_than_18() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "840".to_string(),
        };

        let age = signals.derive_age("250101").unwrap();
        assert!((29..=30).contains(&age));
    }

    #[test]
    fn test_derive_age_younger_than_18() {
        let signals = RarimoPublicSignals {
            birth_date: "200101".to_string(),
            nationality: "840".to_string(),
        };

        let age = signals.derive_age("250101").unwrap();
        assert!(age < 18);
    }

    #[test]
    fn test_derive_age_exact_boundary() {
        let signals = RarimoPublicSignals {
            birth_date: "070101".to_string(),
            nationality: "840".to_string(),
        };

        let age = signals.derive_age("250101").unwrap();
        assert_eq!(age, 18);
    }

    #[test]
    fn test_nationality_to_country_code() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "840".to_string(),
        };

        let country_code = signals.nationality_to_country_code().unwrap();
        assert_eq!(country_code, 840);
    }

    #[test]
    fn test_nationality_large_value() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "5589842".to_string(),
        };

        let country_code = signals.nationality_to_country_code().unwrap();
        assert_eq!(country_code, 5589842);
    }

    #[test]
    fn golden_spec_age_vector() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "840".to_string(),
        };
        let age = signals.derive_age("260515").unwrap();
        assert_eq!(age, 31);
    }

    #[test]
    fn test_to_wraith_claims() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "840".to_string(),
        };

        let claims = signals.to_wraith_claims("250101").unwrap();
        assert_eq!(claims.country_code, 840);
        assert!(claims.is_human);
    }
}
