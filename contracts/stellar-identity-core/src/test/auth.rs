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
fn not_initialized_blocks_stateful_operations() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let _admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "notinit");

    let uninitialized = env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner: owner.clone(),
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    });
    assert_eq!(uninitialized, Err(IdentityError::NotInitialized));

    let verify_uninitialized = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            bytes32(&env, 1),
            VerificationKey {
                alpha: zero_g1(&env),
                beta: zero_g2(&env),
                gamma: zero_g2(&env),
                delta: zero_g2(&env),
                ic: Vec::from_array(&env, [zero_g1(&env)]),
            },
            Proof {
                a: zero_g1(&env),
                b: zero_g2(&env),
                c: zero_g1(&env),
            },
            Vec::from_array(&env, [Bn254Fr::from_u256(U256::from_u32(&env, 25))]),
            0,
            claims(25, 840, true),
        )
    });
    assert_eq!(verify_uninitialized, Err(IdentityError::NotInitialized));

    let attested_uninitialized = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 1),
            bytes32(&env, 2),
            bytes32(&env, 3),
            claims(25, 840, true),
        )
    });
    assert_eq!(attested_uninitialized, Err(IdentityError::NotInitialized));

    let revoke_uninitialized = env.as_contract(&contract_id, || {
        StellarIdentityCore::revoke_app(env.clone(), app_id.clone())
    });
    assert_eq!(revoke_uninitialized, Err(IdentityError::NotInitialized));

    let update_uninitialized = env.as_contract(&contract_id, || {
        StellarIdentityCore::update_app_policy(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    });
    assert_eq!(update_uninitialized, Err(IdentityError::NotInitialized));
}

#[test]
fn unauthorized_prover_blocks_attested_path() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let configured_prover = Address::generate(&env);
    let wrong_prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "wrongprover");

    emit_init(&env, &contract_id, &admin, &configured_prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    })
    .unwrap();

    let wrong_prover_result = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            wrong_prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 1),
            bytes32(&env, 2),
            bytes32(&env, 3),
            claims(25, 840, true),
        )
    });
    assert_eq!(wrong_prover_result, Err(IdentityError::Unauthorized));
}

#[test]
fn app_owner_mismatch_blocks_policy_update() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let original_owner = Address::generate(&env);
    let wrong_owner = Address::generate(&env);
    let app_id = Symbol::new(&env, "ownerchange");

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner: original_owner.clone(),
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    })
    .unwrap();

    let owner_mismatch = env.as_contract(&contract_id, || {
        StellarIdentityCore::update_app_policy(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner: wrong_owner,
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    });
    assert_eq!(owner_mismatch, Err(IdentityError::AppOwnerMismatch));
}
