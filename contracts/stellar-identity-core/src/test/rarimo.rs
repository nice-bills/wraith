#[allow(unused_imports)]
use soroban_sdk::{
    Address, BytesN, Env, Event, Symbol, U256, Vec,
    crypto::bn254::{Bn254Fr, Bn254G1Affine, Bn254G2Affine},
    testutils::{Address as _, Events as _, Ledger as _},
};

use crate::{
    AppPolicy, AppRegistered, AppRevoked, AppUpdated, AttestedClaims, ClaimLayout, IdentityError,
    Initialized, Proof, ProverUpdated, StellarIdentityCore, VerificationKey, VerificationRecorded,
    VerificationSource,
};

use super::{
    attested_hashes, bytes32, claims, create_contract, emit_init, event_vec, ic_points,
    set_ledger_sequence, zero_g1, zero_g2,
};
#[test]
fn rarimo_query_layout_derives_age_and_country() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);
    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    emit_init(&env, &contract_id, &admin, &prover);

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

    let derived =
        crate::claims::derive_from_signals(&pub_signals, &ClaimLayout::RarimoQuery, 260_515)
            .unwrap();
    assert_eq!(derived.age, 31);
    assert_eq!(derived.country_code, 840);
    assert!(derived.is_human);
}

#[test]
fn rarimo_query_phase2_layout_derives_age_and_country() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);
    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    emit_init(&env, &contract_id, &admin, &prover);

    let z = || Bn254Fr::from_u256(U256::from_u32(&env, 0));
    let pub_signals = Vec::from_array(
        &env,
        [
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 260_515)),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 950_101)),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            z(),
            z(),
            z(),
        ],
    );

    let derived =
        crate::claims::derive_from_signals(&pub_signals, &ClaimLayout::RarimoQuery, 260_515)
            .unwrap();
    assert_eq!(derived.age, 31);
    assert_eq!(derived.country_code, 840);
    assert!(derived.is_human);
}

#[test]
fn rarimo_query_rejects_current_date_mismatch() {
    let env = Env::default();
    let z = || Bn254Fr::from_u256(U256::from_u32(&env, 0));
    let pub_signals = Vec::from_array(
        &env,
        [
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 260_515)),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 950_101)),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            z(),
            z(),
            z(),
        ],
    );

    let err = crate::claims::derive_from_signals(&pub_signals, &ClaimLayout::RarimoQuery, 250_101)
        .unwrap_err();
    assert_eq!(err, IdentityError::CurrentDateMismatch);
}

#[test]
fn rarimo_query_phase2_uses_citizenship_when_nationality_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);
    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    emit_init(&env, &contract_id, &admin, &prover);

    let z = || Bn254Fr::from_u256(U256::from_u32(&env, 0));
    let pub_signals = Vec::from_array(
        &env,
        [
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 260_515)),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 950_101)),
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 826)),
            z(),
            z(),
        ],
    );

    let derived =
        crate::claims::derive_from_signals(&pub_signals, &ClaimLayout::RarimoQuery, 260_515)
            .unwrap();
    assert_eq!(derived.country_code, 826);
}

#[test]
fn rarimo_groth16_hash_binds_current_date() {
    let env = Env::default();
    let z = || Bn254Fr::from_u256(U256::from_u32(&env, 0));
    let pub_signals = Vec::from_array(
        &env,
        [
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 260_515)),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 950_101)),
            z(),
            z(),
            z(),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            z(),
            z(),
            z(),
        ],
    );

    let h0 = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 0).unwrap();
    let h1 = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 260_515).unwrap();
    assert_ne!(h0, h1);
}
