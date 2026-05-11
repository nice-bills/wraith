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
        StellarIdentityCore::register_app(env.clone(), app_id.clone(), policy, None).unwrap()
    });

    assert_eq!(
        event_vec(&env),
        std::vec![AppRegistered { app_id: app_id.clone(), owner }.to_xdr(&env, &contract_id)]
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

fn garbage_claims() -> AttestedClaims {
    AttestedClaims {
        age: 0,
        country_code: 0,
        is_human: false,
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
        ic: Vec::from_array(&env, [zero_g1(&env)]),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(&env, [
        Bn254Fr::from_u256(U256::from_u32(&env, 25)),
        Bn254Fr::from_u256(U256::from_u32(&env, 840)),
        Bn254Fr::from_u256(U256::from_u32(&env, 1)),
    ]);
    let pub_inputs_hash = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals);

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
            garbage_claims(),
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

    env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            nullifier.clone(),
            bytes32(&env, 8),
            bytes32(&env, 9),
            claims(30, 840, true),
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

    let blocked = env.as_contract(&contract_id, || {
        StellarIdentityCore::record_attested_result(
            env.clone(),
            prover.clone(),
            app_id.clone(),
            subject.clone(),
            bytes32(&env, 88),
            bytes32(&env, 8),
            bytes32(&env, 9),
            claims(30, 840, true),
        )
    });
    assert_eq!(blocked, Err(IdentityError::SubjectAlreadyVerified));
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
        ic: Vec::from_array(&env, [zero_g1(&env)]),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(&env, [
        Bn254Fr::from_u256(U256::from_u32(&env, 25)),
        Bn254Fr::from_u256(U256::from_u32(&env, 840)),
        Bn254Fr::from_u256(U256::from_u32(&env, 1)),
    ]);
    let pub_inputs_hash = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals);

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
            garbage_claims(),
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
        ic: Vec::from_array(&env, [zero_g1(&env)]),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(&env, [
        Bn254Fr::from_u256(U256::from_u32(&env, 25)),
        Bn254Fr::from_u256(U256::from_u32(&env, 840)),
        Bn254Fr::from_u256(U256::from_u32(&env, 1)),
    ]);
    let pub_inputs_hash = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals);

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
    assert_eq!(result, Err(IdentityError::PolicyViolation));
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

    let registered = env.as_contract(&contract_id, || {
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
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "sanctionszero");

    emit_init(&env, &contract_id, &admin, &prover);
    env.as_contract(&contract_id, || {
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
    let pub_signals = Vec::from_array(&env, [
        Bn254Fr::from_u256(U256::from_u32(&env, 25)),
        Bn254Fr::from_u256(U256::from_u32(&env, 840)),
        Bn254Fr::from_u256(U256::from_u32(&env, 1)),
    ]);
    let pub_inputs_hash = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals);

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
            garbage_claims(),
        )
    });
    assert_eq!(result, Err(IdentityError::SanctionsCheckFailed));
}

#[test]
fn sanctions_stub_allows_nonzero_root() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = create_contract(&env);

    let admin = Address::generate(&env);
    let prover = Address::generate(&env);
    let owner = Address::generate(&env);
    let subject = Address::generate(&env);
    let app_id = Symbol::new(&env, "sanctionsok");

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
                sanctions_enabled: true,
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
        ic: Vec::from_array(&env, [zero_g1(&env)]),
    };
    let proof = Proof {
        a: zero_g1(&env),
        b: zero_g2(&env),
        c: zero_g1(&env),
    };
    let pub_signals = Vec::from_array(&env, [
        Bn254Fr::from_u256(U256::from_u32(&env, 25)),
        Bn254Fr::from_u256(U256::from_u32(&env, 840)),
        Bn254Fr::from_u256(U256::from_u32(&env, 1)),
    ]);
    let pub_inputs_hash = StellarIdentityCore::compute_pub_signals_hash(&env, &pub_signals);

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
            garbage_claims(),
        )
    });
    assert_eq!(result, Err(IdentityError::VkMismatch));
}
