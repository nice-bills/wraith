use anyhow::{Context, Result};
use num_bigint::BigUint;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct RarimoPublicSignals {
    pub birth_date: String,
    pub nationality: String,
}

impl RarimoPublicSignals {
    pub fn from_query_output(signals: &[String]) -> Result<Self> {
        if signals.len() < 7 {
            anyhow::bail!(
                "rarimo query circuit expects at least 7 public signals, got {}",
                signals.len()
            );
        }

        Ok(Self {
            birth_date: signals[1].clone(),
            nationality: signals[5].clone(),
        })
    }

    pub fn parse_birth_date_yymmdd(&self) -> Result<(u16, u8, u8)> {
        let birth_str = self.birth_date.trim();

        if birth_str.is_empty() {
            return Ok((0, 0, 0));
        }

        let padded = if birth_str.len() < 6 {
            format!("{:0>6}", birth_str)
        } else {
            birth_str.to_string()
        };

        let year: u16 = padded[0..2].parse()
            .with_context(|| format!("failed to parse year from '{}'", &padded[0..2]))?;
        let month: u8 = padded[2..4].parse()
            .with_context(|| format!("failed to parse month from '{}'", &padded[2..4]))?;
        let day: u8 = padded[4..6].parse()
            .with_context(|| format!("failed to parse day from '{}'", &padded[4..6]))?;

        Ok((year, month, day))
    }

    pub fn derive_age(&self, current_date_yymmdd: &str) -> Result<u32> {
        let (birth_yy, birth_mm, birth_dd) = self.parse_birth_date_yymmdd().context("parse birth date")?;

        if current_date_yymmdd.len() != 6 {
            anyhow::bail!("current_date should be 6 digits (YYMMDD), got '{}'", current_date_yymmdd);
        }

        let current_yy: u16 = current_date_yymmdd[0..2].parse()
            .with_context(|| format!("failed to parse current year from '{}'", &current_date_yymmdd[0..2]))?;
        let current_mm: u8 = current_date_yymmdd[2..4].parse()
            .with_context(|| format!("failed to parse current month from '{}'", &current_date_yymmdd[2..4]))?;
        let current_dd: u8 = current_date_yymmdd[4..6].parse()
            .with_context(|| format!("failed to parse current day from '{}'", &current_date_yymmdd[4..6]))?;

        let birth_full_year = if birth_yy <= 25 { 2000 + birth_yy } else { 1900 + birth_yy };
        let current_full_year = if current_yy <= 25 { 2000 + current_yy } else { 1900 + current_yy };

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
        let country_code = self.nationality_to_country_code().context("derive country code")?;

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

impl WraithClaims {
    pub fn to_public_signals(&self) -> Vec<String> {
        vec![
            self.age.to_string(),
            self.country_code.to_string(),
            if self.is_human { "1".to_string() } else { "0".to_string() },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_to_wraith_claims() {
        let signals = RarimoPublicSignals {
            birth_date: "950101".to_string(),
            nationality: "840".to_string(),
        };

        let claims = signals.to_wraith_claims("250101").unwrap();
        assert_eq!(claims.country_code, 840);
        assert!(claims.is_human);
    }

    #[test]
    fn test_public_signals_output() {
        let claims = WraithClaims {
            age: 30,
            country_code: 840,
            is_human: true,
        };

        let signals = claims.to_public_signals();
        assert_eq!(signals, vec!["30", "840", "1"]);
    }
}