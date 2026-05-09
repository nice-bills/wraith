#![cfg(test)]

extern crate std;

use soroban_sdk::{
    Address, BytesN, Env, Event, Symbol, U256, Vec,
    crypto::bn254::{Bn254Fr, Bn254G1Affine, Bn254G2Affine},
    testutils::{Address as _, Events as _},
};

use crate::{
    AppPolicy, AppRegistered, AppRevoked, AppUpdated, AttestedClaims, IdentityError, Initialized,
    Proof, ProverUpdated, StellarIdentityCore, VerificationKey, VerificationRecorded,
    VerificationSource,
};

fn create_contract(env: &Env) -> Address {
    env.register(StellarIdentityCore, ())
}

fn bytes32(env: &Env, v: u8) -> BytesN<32> {
    BytesN::from_array(env, &[v; 32])
}

fn zero_g1(env: &Env) -> Bn254G1Affine {
    Bn254G1Affine::from_array(env, &[0u8; 64])
}

fn zero_g2(env: &Env) -> Bn254G2Affine {
    Bn254G2Affine::from_array(env, &[0u8; 128])
}

fn claims(age: u32, country_code: u32, is_human: bool) -> AttestedClaims {
    AttestedClaims { age, country_code, is_human }
}

fn emit_init(env: &Env, contract_id: &Address, admin: &Address, prover: &Address) {
    env.as_contract(contract_id, || {
        StellarIdentityCore::init(env.clone(), admin.clone(), prover.clone())
    })
    .unwrap();
}

fn event_vec(env: &Env) -> std::vec::Vec<soroban_sdk::xdr::ContractEvent> {
    env.events().all().events().to_vec()
}

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
    };

    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(env.clone(), app_id.clone(), policy, None)
    })
    .unwrap();

    assert_eq!(
        event_vec(&env),
        std::vec![AppRegistered { app_id, owner }.to_xdr(&env, &contract_id)]
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
            },
            None,
        )
    })
    .unwrap();

    assert_eq!(
        event_vec(&env),
        std::vec![AppRegistered { app_id: app_id.clone(), owner: owner.clone() }.to_xdr(&env, &contract_id)]
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
            },
            None,
        )
    })
    .unwrap();

    assert_eq!(
        event_vec(&env),
        std::vec![AppUpdated { app_id: app_id.clone(), owner: owner.clone() }.to_xdr(&env, &contract_id)]
    );

    env.as_contract(&contract_id, || StellarIdentityCore::revoke_app(env.clone(), app_id.clone()))
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
            },
            None,
        )
    })
    .unwrap();

    let record = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            nullifier.clone(),
            bytes32(&env, 4),
            bytes32(&env, 5),
            claims(22, 566, true),
        )
    })
    .unwrap();

    assert_eq!(record.source, VerificationSource::AttestedProver(bytes32(&env, 5)));
    assert_eq!(
        event_vec(&env),
        std::vec![VerificationRecorded {
            app_id: app_id.clone(),
            subject: subject.clone(),
            nullifier: nullifier.clone(),
            source: VerificationSource::AttestedProver(bytes32(&env, 5)),
        }
        .to_xdr(&env, &contract_id)]
    );

    let replay = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            nullifier.clone(),
            bytes32(&env, 4),
            bytes32(&env, 5),
            claims(22, 566, true),
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
        std::vec![ProverUpdated { previous: prover, current: next_prover }.to_xdr(&env, &contract_id)]
    );
}

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
    let pub_signals = Vec::from_array(&env, [Bn254Fr::from_u256(U256::from_u32(&env, 33))]);

    let result = env.as_contract(&contract_id, || {
        StellarIdentityCore::verify_and_record(
            env.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 0),
            bytes32(&env, 6),
            vk.clone(),
            proof.clone(),
            pub_signals.clone(),
            claims(33, 566, true),
        )
    });
    assert_eq!(result, Err(IdentityError::PublicInputsHashMismatch));
}

#[test]
fn blocks_policy_violations_and_duplicate_subjects() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "gated");

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
                excluded_countries: Vec::from_array(&env, [566]),
                expiration_window: 0,
                sanctions_enabled: false,
            },
            None,
        )
    })
    .unwrap();

    let too_young = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 8),
            bytes32(&env, 4),
            bytes32(&env, 5),
            claims(20, 840, true),
        )
    });
    assert_eq!(too_young, Err(IdentityError::PolicyViolation));

    let blocked_country = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 10),
            bytes32(&env, 4),
            bytes32(&env, 5),
            claims(30, 566, true),
        )
    });
    assert_eq!(blocked_country, Err(IdentityError::PolicyViolation));

    env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 11),
            bytes32(&env, 4),
            bytes32(&env, 5),
            claims(30, 840, true),
        )
    })
    .unwrap();

    let duplicate_subject = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 12),
            bytes32(&env, 4),
            bytes32(&env, 5),
            claims(30, 840, true),
        )
    });
    assert_eq!(duplicate_subject, Err(IdentityError::SubjectAlreadyVerified));
}
