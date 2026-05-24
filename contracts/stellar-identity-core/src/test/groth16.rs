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
fn rejects_malformed_vk_before_pairing() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "mvp");

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 16,
                require_humanity: false,
                sanctions_root: bytes32(&env, 2),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    })
    .unwrap();

    let vk = VerificationKey {
        alpha: zero_g1(&env),
        beta: zero_g2(&env),
        gamma: zero_g2(&env),
        delta: zero_g2(&env),
        ic: Vec::from_array(&env, [zero_g1(&env)]),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(
        &env,
        [
            Bn254Fr::from_u256(U256::from_u32(&env, 33)),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            Bn254Fr::from_u256(U256::from_u32(&env, 1)),
        ],
    );
    let pub_inputs_hash =
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 0).unwrap();

    let result = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            pub_inputs_hash.clone(),
            vk.clone(),
            proof.clone(),
            pub_signals.clone(),
            0,
            claims(33, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::MalformedVerifyingKey));
}

#[test]
fn verify_and_record_requires_vk_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "vkreq");

    emit_init(&env, &contract_id, &admin, &prover);
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

    let vk = VerificationKey {
        alpha: zero_g1(&env),
        beta: zero_g2(&env),
        gamma: zero_g2(&env),
        delta: zero_g2(&env),
        ic: ic_points(&env, 4),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(
        &env,
        [
            Bn254Fr::from_u256(U256::from_u32(&env, 25)),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            Bn254Fr::from_u256(U256::from_u32(&env, 1)),
        ],
    );
    let pub_inputs_hash =
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 0).unwrap();

    let result = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            pub_inputs_hash.clone(),
            vk.clone(),
            proof.clone(),
            pub_signals.clone(),
            0,
            claims(25, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::VkNotRegistered));
}

#[test]
fn verify_and_record_rejects_vk_mismatch() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "vkmismatch");

    emit_init(&env, &contract_id, &admin, &prover);
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
            Some(bytes32(&env, 99)),
        )
    })
    .unwrap();

    let vk = VerificationKey {
        alpha: zero_g1(&env),
        beta: zero_g2(&env),
        gamma: zero_g2(&env),
        delta: zero_g2(&env),
        ic: ic_points(&env, 4),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(
        &env,
        [
            Bn254Fr::from_u256(U256::from_u32(&env, 25)),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            Bn254Fr::from_u256(U256::from_u32(&env, 1)),
        ],
    );
    let pub_inputs_hash =
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 0).unwrap();

    let result = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            pub_inputs_hash.clone(),
            vk.clone(),
            proof.clone(),
            pub_signals.clone(),
            0,
            claims(25, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::VkMismatch));
}

#[test]
fn verify_and_record_rejects_claim_mismatch() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "claimmismatch");

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 18,
                require_humanity: false,
                sanctions_root: bytes32(&env, 1),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            Some(bytes32(&env, 42)),
        )
    })
    .unwrap();

    let vk = VerificationKey {
        alpha: zero_g1(&env),
        beta: zero_g2(&env),
        gamma: zero_g2(&env),
        delta: zero_g2(&env),
        ic: ic_points(&env, 4),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(
        &env,
        [
            Bn254Fr::from_u256(U256::from_u32(&env, 25)),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            Bn254Fr::from_u256(U256::from_u32(&env, 1)),
        ],
    );
    let pub_inputs_hash =
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 0).unwrap();

    let result = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            pub_inputs_hash.clone(),
            vk.clone(),
            proof.clone(),
            pub_signals.clone(),
            0,
            claims(99, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::ClaimMismatch));
}

#[test]
fn with_sanctions_disabled_vk_mismatch_is_reached() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "sanctionsdisabled");

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 42),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
                claim_layout: ClaimLayout::Standard,
            },
            Some(bytes32(&env, 99)),
        )
    })
    .unwrap();

    let vk = VerificationKey {
        alpha: zero_g1(&env),
        beta: zero_g2(&env),
        gamma: zero_g2(&env),
        delta: zero_g2(&env),
        ic: ic_points(&env, 4),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(
        &env,
        [
            Bn254Fr::from_u256(U256::from_u32(&env, 25)),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            Bn254Fr::from_u256(U256::from_u32(&env, 1)),
        ],
    );
    let pub_inputs_hash =
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals, 0).unwrap();

    let result = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            pub_inputs_hash.clone(),
            vk.clone(),
            proof.clone(),
            pub_signals.clone(),
            0,
            claims(25, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::VkMismatch));
}
