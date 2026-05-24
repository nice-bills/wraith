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
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    })
    .unwrap();

    let young_claims = claims(20, 840, true);
    let young_nullifier = bytes32(&env, 8);
    let (young_pub_hash, young_att_hash) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &young_nullifier,
        &young_claims,
    );
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
    let (blocked_pub_hash, blocked_att_hash) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &blocked_nullifier,
        &blocked_claims,
    );
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
    let (valid_pub_hash, valid_att_hash) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &valid_nullifier,
        &valid_claims,
    );
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
    let (dup_pub_hash, dup_att_hash) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &dup_nullifier,
        &dup_claims,
    );
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
                claim_layout: ClaimLayout::Standard,
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
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    })
    .unwrap();

    let reverify_claims = claims(30, 840, true);
    let reverify_nullifier = bytes32(&env, 88);
    let (reverify_pub, reverify_att) = attested_hashes(
        &env,
        &prover,
        &app_id,
        &subject,
        &reverify_nullifier,
        &reverify_claims,
    );
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
                claim_layout: ClaimLayout::Standard,
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
                claim_layout: ClaimLayout::Standard,
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
        claim_layout: ClaimLayout::Standard,
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
                claim_layout: ClaimLayout::Standard,
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
                claim_layout: ClaimLayout::Standard,
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
                claim_layout: ClaimLayout::Standard,
            },
            None,
        )
    });
    assert_eq!(update_result, Err(IdentityError::SanctionsCheckFailed));
}
