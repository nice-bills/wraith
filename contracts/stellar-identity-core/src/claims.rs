//! Claim derivation from Groth16 public signals (Standard + Rarimo layouts).
//! Indices must match `deployments/claim-layout-spec.json`.

use soroban_sdk::Vec;
use soroban_sdk::crypto::bn254::Bn254Fr;

use crate::{AttestedClaims, ClaimLayout, IdentityError};

/// Layout stub: 6 public signals.
pub const RARIMO_LAYOUT_MIN_SIGNALS: u32 = 6;
pub const RARIMO_LAYOUT_BIRTH_DATE_INDEX: u32 = 1;
pub const RARIMO_LAYOUT_COUNTRY_INDEX: u32 = 5;

/// Phase 2 queryIdentity: 14 inputs + 9 outputs.
pub const RARIMO_PHASE2_MIN_SIGNALS: u32 = 23;
pub const RARIMO_PHASE2_BIRTH_DATE_INDEX: u32 = 15;
pub const RARIMO_PHASE2_NATIONALITY_INDEX: u32 = 19;
pub const RARIMO_PHASE2_CITIZENSHIP_INDEX: u32 = 20;

pub fn fr_to_u32(fr: &Bn254Fr) -> Result<u32, IdentityError> {
    let bytes = fr.to_bytes();
    Ok(u32::from_be_bytes([
        bytes.get(28).unwrap_or(0),
        bytes.get(29).unwrap_or(0),
        bytes.get(30).unwrap_or(0),
        bytes.get(31).unwrap_or(0),
    ]))
}

pub fn age_from_birth_yymmdd(birth_yymmdd: u32, current_yymmdd: u32) -> Result<u32, IdentityError> {
    if birth_yymmdd == 0 || current_yymmdd == 0 {
        return Err(IdentityError::PolicyViolation);
    }
    let birth_yy = birth_yymmdd / 10_000;
    let birth_mm = (birth_yymmdd / 100) % 100;
    let birth_dd = birth_yymmdd % 100;
    let cur_yy = current_yymmdd / 10_000;
    let cur_mm = (current_yymmdd / 100) % 100;
    let cur_dd = current_yymmdd % 100;

    let birth_full_year = if birth_yy <= 50 {
        2000 + birth_yy
    } else {
        1900 + birth_yy
    };
    let current_full_year = if cur_yy <= 50 {
        2000 + cur_yy
    } else {
        1900 + cur_yy
    };

    let mut age = current_full_year.saturating_sub(birth_full_year);
    if cur_mm < birth_mm || (cur_mm == birth_mm && cur_dd < birth_dd) {
        age = age.saturating_sub(1);
    }
    Ok(age)
}

pub fn derive_from_signals(
    pub_signals: &Vec<Bn254Fr>,
    layout: &ClaimLayout,
    current_date_ymd: u32,
) -> Result<AttestedClaims, IdentityError> {
    match layout {
        ClaimLayout::Standard => derive_standard(pub_signals),
        ClaimLayout::RarimoQuery => derive_rarimo(pub_signals, current_date_ymd),
    }
}

fn derive_standard(pub_signals: &Vec<Bn254Fr>) -> Result<AttestedClaims, IdentityError> {
    if pub_signals.len() < 3 {
        return Err(IdentityError::InsufficientPublicSignals);
    }
    let age_fr = pub_signals
        .get(0)
        .ok_or(IdentityError::InsufficientPublicSignals)?;
    let country_fr = pub_signals
        .get(1)
        .ok_or(IdentityError::InsufficientPublicSignals)?;
    let is_human_fr = pub_signals
        .get(2)
        .ok_or(IdentityError::InsufficientPublicSignals)?;
    let age = fr_to_u32(&age_fr)?;
    let country_code = fr_to_u32(&country_fr)?;
    let is_human_val = fr_to_u32(&is_human_fr)?;
    Ok(AttestedClaims {
        age,
        country_code,
        is_human: is_human_val != 0,
    })
}

fn derive_rarimo(
    pub_signals: &Vec<Bn254Fr>,
    current_date_ymd: u32,
) -> Result<AttestedClaims, IdentityError> {
    if current_date_ymd == 0 {
        return Err(IdentityError::PolicyViolation);
    }
    let len = pub_signals.len();
    if len >= RARIMO_PHASE2_MIN_SIGNALS {
        let birth_fr = pub_signals
            .get(RARIMO_PHASE2_BIRTH_DATE_INDEX)
            .ok_or(IdentityError::InsufficientPublicSignals)?;
        let nationality_fr = pub_signals
            .get(RARIMO_PHASE2_NATIONALITY_INDEX)
            .ok_or(IdentityError::InsufficientPublicSignals)?;
        let citizenship_fr = pub_signals
            .get(RARIMO_PHASE2_CITIZENSHIP_INDEX)
            .ok_or(IdentityError::InsufficientPublicSignals)?;
        let birth_yymmdd = fr_to_u32(&birth_fr)?;
        let nationality = fr_to_u32(&nationality_fr)?;
        let citizenship = fr_to_u32(&citizenship_fr)?;
        let country_code = if nationality != 0 {
            nationality
        } else {
            citizenship
        };
        let age = age_from_birth_yymmdd(birth_yymmdd, current_date_ymd)?;
        return Ok(AttestedClaims {
            age,
            country_code,
            is_human: true,
        });
    }
    if len >= RARIMO_LAYOUT_MIN_SIGNALS {
        let birth_fr = pub_signals
            .get(RARIMO_LAYOUT_BIRTH_DATE_INDEX)
            .ok_or(IdentityError::InsufficientPublicSignals)?;
        let country_fr = pub_signals
            .get(RARIMO_LAYOUT_COUNTRY_INDEX)
            .ok_or(IdentityError::InsufficientPublicSignals)?;
        let birth_yymmdd = fr_to_u32(&birth_fr)?;
        let country_code = fr_to_u32(&country_fr)?;
        let age = age_from_birth_yymmdd(birth_yymmdd, current_date_ymd)?;
        return Ok(AttestedClaims {
            age,
            country_code,
            is_human: true,
        });
    }
    Err(IdentityError::InsufficientPublicSignals)
}

pub fn verify_match(
    derived: &AttestedClaims,
    supplied: &AttestedClaims,
) -> Result<(), IdentityError> {
    if derived.age != supplied.age {
        return Err(IdentityError::ClaimMismatch);
    }
    if derived.country_code != supplied.country_code {
        return Err(IdentityError::ClaimMismatch);
    }
    if derived.is_human != supplied.is_human {
        return Err(IdentityError::ClaimMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, U256, Vec};

    #[test]
    fn age_from_birth_yymmdd_golden() {
        assert_eq!(age_from_birth_yymmdd(950_101, 260_515).unwrap(), 31);
    }

    #[test]
    fn rarimo_layout_stub_from_fr() {
        let env = Env::default();
        let pub_signals = Vec::from_array(
            &env,
            [
                Bn254Fr::from_u256(U256::from_u32(&env, 1)),
                Bn254Fr::from_u256(U256::from_u32(&env, 950_101)),
                Bn254Fr::from_u256(U256::from_u32(&env, 300_101)),
                Bn254Fr::from_u256(U256::from_u32(&env, 0)),
                Bn254Fr::from_u256(U256::from_u32(&env, 0)),
                Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            ],
        );
        let c = derive_rarimo(&pub_signals, 260_515).unwrap();
        assert_eq!(c.age, 31);
        assert_eq!(c.country_code, 840);
        assert!(c.is_human);
    }
}
