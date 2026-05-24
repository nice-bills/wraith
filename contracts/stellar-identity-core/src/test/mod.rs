#![cfg(test)]
#![allow(unused_imports)]

//! Contract unit tests (split modules).
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
    AppPolicy, AppRegistered, AppRevoked, AppUpdated, AttestedClaims, ClaimLayout, IdentityError,
    Initialized, Proof, ProverUpdated, StellarIdentityCore, VerificationKey, VerificationRecorded,
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
    let mut out = Vec::new(env);
    for _ in 0..count {
        out.push_back(z.clone());
    }
    out
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
    let public_inputs_hash =
        StellarIdentityCore::hash_attested_claims(env.clone(), claim_values.clone());
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

mod auth;
mod events;
mod groth16;
mod policy_lifecycle;
mod rarimo;
