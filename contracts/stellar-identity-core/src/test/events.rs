#![allow(unused_imports)]

extern crate std;

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
fn init_emits_initialization_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);

    emit_init(&env, &contract_id, &admin, &prover);

    assert_eq!(
        event_vec(&env),
        std::vec![Initialized { admin, prover }.to_xdr(&env, &contract_id)]
    );
}

#[test]
fn register_app_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let app_id = Symbol::new(&env, "wave");

    emit_init(&env, &contract_id, &admin, &prover);

    let policy = AppPolicy {
        owner: owner.clone(),
        min_age: 18,
        require_humanity: true,
        sanctions_root: bytes32(&env, 9),
        excluded_countries: Vec::new(&env),
        expiration_window: 0,
        sanctions_enabled: false,
        claim_layout: ClaimLayout::Standard,
    };

    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(env.clone(), app_id.clone(), policy, None).unwrap()
    });

    assert_eq!(
        event_vec(&env),
        std::vec![
            AppRegistered {
                app_id: app_id.clone(),
                owner
            }
            .to_xdr(&env, &contract_id)
        ]
    );
}

#[test]
fn app_lifecycle_emits_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let app_id = Symbol::new(&env, "wave");

    emit_init(&env, &contract_id, &admin, &prover);

    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner: owner.clone(),
                min_age: 18,
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

    assert_eq!(
        event_vec(&env),
        std::vec![
            AppRegistered {
                app_id: app_id.clone(),
                owner: owner.clone()
            }
            .to_xdr(&env, &contract_id)
        ]
    );

    env.as_contract(&contract_id, || {
        StellarIdentityCore::update_app_policy(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner: owner.clone(),
                min_age: 21,
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

    assert_eq!(
        event_vec(&env),
        std::vec![
            AppUpdated {
                app_id: app_id.clone(),
                owner: owner.clone()
            }
            .to_xdr(&env, &contract_id)
        ]
    );

    env.as_contract(&contract_id, || {
        StellarIdentityCore::revoke_app(env.clone(), app_id.clone())
    })
    .unwrap();

    assert_eq!(
        event_vec(&env),
        std::vec![AppRevoked { app_id, owner }.to_xdr(&env, &contract_id)]
    );
}

#[test]
fn record_attested_result_emits_event_and_blocks_replay() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "drips");
    let nullifier = bytes32(&env, 7);

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 21,
                require_humanity: true,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::from_array(&env, [840]),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    })
    .unwrap();

    let claim_values = claims(22, 566, true);
    let (public_inputs_hash, attestation_hash) =
        attested_hashes(&env, &prover, &app_id, &subject, &nullifier, &claim_values);

    let record = env
        .as_contract(&contract_id, || {
            StellarIdentityCore::record_attested_result(
                env.clone(),
                prover.clone(),
                app_id.clone(),
                subject.clone(),
                nullifier.clone(),
                public_inputs_hash.clone(),
                attestation_hash.clone(),
                claim_values.clone(),
            )
        })
        .unwrap();

    assert_eq!(
        record.source,
        VerificationSource::AttestedProver(attestation_hash.clone())
    );
    assert_eq!(
        event_vec(&env),
        std::vec![
            VerificationRecorded {
                app_id: app_id.clone(),
                subject: subject.clone(),
                nullifier: nullifier.clone(),
                source: VerificationSource::AttestedProver(attestation_hash.clone()),
            }
            .to_xdr(&env, &contract_id)
        ]
    );

    let replay = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            nullifier.clone(),
            public_inputs_hash.clone(),
            attestation_hash.clone(),
            claim_values.clone(),
        )
    });
    assert_eq!(replay, Err(IdentityError::NullifierAlreadyUsed));
}

#[test]
fn set_prover_emits_update_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let next_prover = Address::generate(&env);

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::set_prover(env.clone(), next_prover.clone())
    })
    .unwrap();

    assert_eq!(
        event_vec(&env),
        std::vec![
            ProverUpdated {
                previous: prover,
                current: next_prover
            }
            .to_xdr(&env, &contract_id)
        ]
    );
}
