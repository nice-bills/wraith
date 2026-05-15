#![cfg(test)]
//! Contract unit tests.
//!
//! Test coverage notes:
//! - The Groth16 verification path (`verify_and_record`) is exercised through rejection tests
//!   that validate VK hash, claim derivation, policy enforcement, and sanctions checks.
//! - A positive end-to-end test with a real valid proof requires BN254 pairing support in the
//!   test environment, which is not available. Real deployment should be tested on Futurenet
//!   with actual snarkjs-generated proofs.
//! - The attested path (`record_attested_result`) is tested for replay protection and policy
//!   enforcement via `record_attested_result_emits_event_and_blocks_replay`.
//! - Negative auth tests cover NotInitialized, Unauthorized, and AppOwnerMismatch cases.

extern crate std;

use soroban_sdk::{
    Address, BytesN, Env, Event, Symbol, U256, Vec,
    crypto::bn254::{Bn254Fr, Bn254G1Affine, Bn254G2Affine},
    testutils::{Address as _, Events as _, Ledger as _},
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

fn ic_points(env: &Env, count: u32) -> Vec<Bn254G1Affine> {
    let z = zero_g1(env);
    match count {
        1 => Vec::from_array(env, [z.clone()]),
        2 => Vec::from_array(env, [z.clone(), z.clone()]),
        3 => Vec::from_array(env, [z.clone(), z.clone(), z.clone()]),
        4 => Vec::from_array(
            env,
            [z.clone(), z.clone(), z.clone(), z.clone()],
        ),
        5 => Vec::from_array(
            env,
            [
                z.clone(),
                z.clone(),
                z.clone(),
                z.clone(),
                z.clone(),
            ],
        ),
        _ => panic!("unsupported ic count in test helper"),
    }
}

fn claims(age: u32, country_code: u32, is_human: bool) -> AttestedClaims {
    AttestedClaims {
        age,
        country_code,
        is_human,
    }
}

fn attested_hashes(
    env: &Env,
    prover: &Address,
    app_id: &Symbol,
    subject: &Address,
    nullifier: &BytesN<32>,
    claim_values: &AttestedClaims,
) -> (BytesN<32>, BytesN<32>) {
    let public_inputs_hash = StellarIdentityCore::hash_attested_claims(env.clone(), claim_values.clone());
    let attestation_hash = StellarIdentityCore::hash_attestation(
        env.clone(),
        prover.clone(),
        app_id.clone(),
        subject.clone(),
        nullifier.clone(),
        claim_values.clone(),
    );
    (public_inputs_hash, attestation_hash)
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

fn set_ledger_sequence(env: &Env, sequence: u32) {
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number = sequence;
    });
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
    let pub_signals = Vec::from_array(
        &env,
        [
            Bn254Fr::from_u256(U256::from_u32(&env, 33)),
            Bn254Fr::from_u256(U256::from_u32(&env, 840)),
            Bn254Fr::from_u256(U256::from_u32(&env, 1)),
        ],
    );
    let pub_inputs_hash =
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals).unwrap();

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
            claims(33, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::MalformedVerifyingKey));
}

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
            },
            None,
        )
    });
    assert_eq!(owner_mismatch, Err(IdentityError::AppOwnerMismatch));
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

    let young_claims = claims(20, 840, true);
    let young_nullifier = bytes32(&env, 8);
    let (young_pub_hash, young_att_hash) =
        attested_hashes(&env, &prover, &app_id, &subject, &young_nullifier, &young_claims);
    let too_young = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            young_nullifier,
            young_pub_hash,
            young_att_hash,
            young_claims,
        )
    });
    assert_eq!(too_young, Err(IdentityError::PolicyViolation));

    let blocked_claims = claims(30, 566, true);
    let blocked_nullifier = bytes32(&env, 10);
    let (blocked_pub_hash, blocked_att_hash) =
        attested_hashes(&env, &prover, &app_id, &subject, &blocked_nullifier, &blocked_claims);
    let blocked_country = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            blocked_nullifier,
            blocked_pub_hash,
            blocked_att_hash,
            blocked_claims,
        )
    });
    assert_eq!(blocked_country, Err(IdentityError::PolicyViolation));

    let valid_claims = claims(30, 840, true);
    let valid_nullifier = bytes32(&env, 11);
    let (valid_pub_hash, valid_att_hash) =
        attested_hashes(&env, &prover, &app_id, &subject, &valid_nullifier, &valid_claims);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            valid_nullifier,
            valid_pub_hash,
            valid_att_hash,
            valid_claims,
        )
    })
    .unwrap();

    let dup_claims = claims(30, 840, true);
    let dup_nullifier = bytes32(&env, 12);
    let (dup_pub_hash, dup_att_hash) =
        attested_hashes(&env, &prover, &app_id, &subject, &dup_nullifier, &dup_claims);
    let duplicate_subject = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            dup_nullifier,
            dup_pub_hash,
            dup_att_hash,
            dup_claims,
        )
    });
    assert_eq!(
        duplicate_subject,
        Err(IdentityError::SubjectAlreadyVerified)
    );
}

fn garbage_claims() -> AttestedClaims {
    AttestedClaims {
        age: 99,
        country_code: 111,
        is_human: true,
    }
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
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals).unwrap();

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
            claims(25, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::VkNotRegistered));
}

#[test]
fn revoke_app_preserves_records_and_nullifiers() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "revival");
    let nullifier = bytes32(&env, 77);

    emit_init(&env, &contract_id, &admin, &prover);

    env.as_contract(&contract_id, || {
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
            },
            None,
        )
    })
    .unwrap();

    let revive_claims = claims(30, 840, true);
    let (revive_pub, revive_att) =
        attested_hashes(&env, &prover, &app_id, &subject, &nullifier, &revive_claims);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            nullifier.clone(),
            revive_pub,
            revive_att,
            revive_claims,
        )
    })
    .unwrap();

    let was_verified = env.as_contract(&contract_id, || {
        StellarIdentityCore::is_verified(env.clone(), app_id.clone(), subject.clone())
    });
    assert!(was_verified);

    let nullifier_still_used_before = env.as_contract(&contract_id, || {
        StellarIdentityCore::has_nullifier(env.clone(), app_id.clone(), nullifier.clone())
    });
    assert!(nullifier_still_used_before);

    env.as_contract(&contract_id, || {
        StellarIdentityCore::revoke_app(env.clone(), app_id.clone())
    })
    .unwrap();

    let nullifier_still_used_after = env.as_contract(&contract_id, || {
        StellarIdentityCore::has_nullifier(env.clone(), app_id.clone(), nullifier.clone())
    });
    assert!(nullifier_still_used_after);

    let record_inaccessible = env.as_contract(&contract_id, || {
        StellarIdentityCore::get_record(env.clone(), app_id.clone(), subject.clone())
    });
    assert!(record_inaccessible.is_none());

    env.as_contract(&contract_id, || {
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
            },
            None,
        )
    })
    .unwrap();

    let reverify_claims = claims(30, 840, true);
    let reverify_nullifier = bytes32(&env, 88);
    let (reverify_pub, reverify_att) =
        attested_hashes(&env, &prover, &app_id, &subject, &reverify_nullifier, &reverify_claims);
    let blocked = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            reverify_nullifier,
            reverify_pub,
            reverify_att,
            reverify_claims,
        )
    });
    assert_eq!(blocked, Err(IdentityError::SubjectAlreadyVerified));
}

#[test]
fn expired_records_can_refresh_with_fresh_nullifier() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "expires");
    let original_nullifier = bytes32(&env, 21);
    let refresh_nullifier = bytes32(&env, 22);

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
                expiration_window: 1,
                sanctions_enabled: false,
            },
            None,
        )
    })
    .unwrap();

    set_ledger_sequence(&env, 10);
    let initial_claims = claims(30, 840, true);
    let (initial_pub, initial_att) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &original_nullifier,
        &initial_claims,
    );
    env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            original_nullifier.clone(),
            initial_pub,
            initial_att,
            initial_claims,
        )
    })
    .unwrap();

    set_ledger_sequence(&env, 11);
    let still_verified = env.as_contract(&contract_id, || {
        StellarIdentityCore::is_verified(env.clone(), app_id.clone(), subject.clone())
    });
    assert!(still_verified);

    set_ledger_sequence(&env, 12);
    let expired = env.as_contract(&contract_id, || {
        StellarIdentityCore::is_verified(env.clone(), app_id.clone(), subject.clone())
    });
    assert!(!expired);

    let expired_record = env.as_contract(&contract_id, || {
        StellarIdentityCore::get_record(env.clone(), app_id.clone(), subject.clone())
    });
    assert!(expired_record.is_none());

    let original_nullifier_burned = env.as_contract(&contract_id, || {
        StellarIdentityCore::has_nullifier(env.clone(), app_id.clone(), original_nullifier.clone())
    });
    assert!(original_nullifier_burned);

    let reuse_claims = claims(30, 840, true);
    let (reuse_pub, reuse_att) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &original_nullifier,
        &reuse_claims,
    );
    let old_nullifier_reuse = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            original_nullifier.clone(),
            reuse_pub,
            reuse_att,
            reuse_claims,
        )
    });
    assert_eq!(
        old_nullifier_reuse,
        Err(IdentityError::NullifierAlreadyUsed)
    );

    let refresh_claims = claims(30, 840, true);
    let (refresh_pub, refresh_att) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &refresh_nullifier,
        &refresh_claims,
    );
    env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            refresh_nullifier.clone(),
            refresh_pub,
            refresh_att,
            refresh_claims,
        )
    })
    .unwrap();

    let refreshed = env.as_contract(&contract_id, || {
        StellarIdentityCore::is_verified(env.clone(), app_id.clone(), subject.clone())
    });
    assert!(refreshed);

    let refreshed_record = env
        .as_contract(&contract_id, || {
            StellarIdentityCore::get_record(env.clone(), app_id.clone(), subject.clone())
        })
        .unwrap();
    assert_eq!(refreshed_record.nullifier, refresh_nullifier);
}

#[test]
fn revoke_clears_approval_state() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let app_id = Symbol::new(&env, "approvals");

    emit_init(&env, &contract_id, &admin, &prover);

    env.as_contract(&contract_id, || {
        StellarIdentityCore::set_approval_mode(env.clone(), true).unwrap()
    });

    env.as_contract(&contract_id, || {
        StellarIdentityCore::set_app_approval(env.clone(), app_id.clone(), true).unwrap()
    });

    let approved_before = env.as_contract(&contract_id, || {
        StellarIdentityCore::is_approved_app(env.clone(), app_id.clone())
    });
    assert!(approved_before);

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
            },
            None,
        )
    })
    .unwrap();

    env.as_contract(&contract_id, || {
        StellarIdentityCore::revoke_app(env.clone(), app_id.clone())
    })
    .unwrap();

    let cleared = env.as_contract(&contract_id, || {
        StellarIdentityCore::is_approved_app(env.clone(), app_id.clone())
    });
    assert!(!cleared);
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
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals).unwrap();

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
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals).unwrap();

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
            claims(99, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::ClaimMismatch));
}
#[test]
fn app_approval_mode_blocks_registration() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let app_id = Symbol::new(&env, "needsapproval");

    emit_init(&env, &contract_id, &admin, &prover);

    env.as_contract(&contract_id, || {
        StellarIdentityCore::set_approval_mode(env.clone(), true).unwrap()
    });

    let policy = AppPolicy {
        owner,
        min_age: 18,
        require_humanity: false,
        sanctions_root: bytes32(&env, 1),
        excluded_countries: Vec::new(&env),
        expiration_window: 0,
        sanctions_enabled: false,
    };

    let unregistered = env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(env.clone(), app_id.clone(), policy.clone(), None)
    });
    assert_eq!(unregistered, Err(IdentityError::AppNotApproved));

    env.as_contract(&contract_id, || {
        StellarIdentityCore::set_app_approval(env.clone(), app_id.clone(), true).unwrap()
    });

    let registered = env
        .as_contract(&contract_id, || {
            StellarIdentityCore::register_app(env.clone(), app_id.clone(), policy.clone(), None)
        })
        .unwrap();
    assert_eq!(registered.owner, policy.owner);
}

#[test]
fn sanctions_stub_rejects_zero_root() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let _subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "sanctionszero");

    emit_init(&env, &contract_id, &admin, &prover);
    let register_result = env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 0),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: true,
            },
            Some(bytes32(&env, 1)),
        )
    });
    assert_eq!(register_result, Err(IdentityError::SanctionsCheckFailed));
}

#[test]
fn sanctions_failsafe_blocks_with_nonzero_root() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let _subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "sanctionsfail");

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
        StellarIdentityCore::register_app(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner: owner.clone(),
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 42),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: false,
            },
            Some(bytes32(&env, 99)),
        )
    })
    .unwrap();

    let update_result = env.as_contract(&contract_id, || {
        StellarIdentityCore::update_app_policy(
            env.clone(),
            app_id.clone(),
            AppPolicy {
                owner,
                min_age: 0,
                require_humanity: false,
                sanctions_root: bytes32(&env, 42),
                excluded_countries: Vec::new(&env),
                expiration_window: 0,
                sanctions_enabled: true,
            },
            None,
        )
    });
    assert_eq!(update_result, Err(IdentityError::SanctionsCheckFailed));
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
        StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals).unwrap();

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
            claims(25, 840, true),
        )
    });
    assert_eq!(result, Err(IdentityError::VkMismatch));
}
