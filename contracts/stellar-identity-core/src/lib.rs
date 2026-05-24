#![no_std]
#![allow(clippy::too_many_arguments)]

mod claims;

use claims::{derive_from_signals, verify_match};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, IntoVal, Symbol, Vec, contract, contracterror, contractevent,
    contractimpl, contracttype,
    crypto::bn254::{Bn254Fr, Bn254G1Affine, Bn254G2Affine},
    vec,
    xdr::ToXdr,
};

/// Maximum excluded ISO country codes per app policy (gas griefing guard).
pub const MAX_EXCLUDED_COUNTRIES: u32 = 32;

/// Extend persistent TTL when below this ledger threshold.
const TTL_THRESHOLD: u32 = 50_000;
/// Target TTL after extension.
const TTL_EXTEND_TO: u32 = 200_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IdentityError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    AppNotFound = 4,
    MalformedVerifyingKey = 5,
    InvalidProof = 6,
    NullifierAlreadyUsed = 7,
    PolicyViolation = 8,
    SubjectAlreadyVerified = 9,
    AppAlreadyRegistered = 10,
    AppOwnerMismatch = 11,
    VkNotRegistered = 12,
    VkMismatch = 13,
    PublicInputsHashMismatch = 14,
    AppNotApproved = 15,
    VerificationExpired = 16,
    SanctionsCheckFailed = 17,
    ClaimMismatch = 18,
    AttestationHashMismatch = 19,
    ExcludedCountriesLimit = 20,
    InsufficientPublicSignals = 21,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ClaimLayout {
    /// Public signals `[0]=age, [1]=country_code, [2]=is_human` (e.g. e2e_claims).
    Standard,
    /// Rarimo query layout: `[1]=birthDate (YYMMDD)`, `[5]=nationality`; humanity is always true.
    RarimoQuery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AppPolicy {
    pub owner: Address,
    pub min_age: u32,
    pub require_humanity: bool,
    pub sanctions_root: BytesN<32>,
    pub excluded_countries: Vec<u32>,
    pub expiration_window: u32,
    pub sanctions_enabled: bool,
    pub claim_layout: ClaimLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AttestedClaims {
    pub age: u32,
    pub country_code: u32,
    pub is_human: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct VerificationKey {
    pub alpha: Bn254G1Affine,
    pub beta: Bn254G2Affine,
    pub gamma: Bn254G2Affine,
    pub delta: Bn254G2Affine,
    pub ic: Vec<Bn254G1Affine>,
}

#[derive(Clone)]
#[contracttype]
pub struct Proof {
    pub a: Bn254G1Affine,
    pub b: Bn254G2Affine,
    pub c: Bn254G1Affine,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VerificationSource {
    OnchainGroth16,
    AttestedProver(BytesN<32>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VerificationRecord {
    pub app_id: Symbol,
    pub subject: Address,
    pub nullifier: BytesN<32>,
    pub public_inputs_hash: BytesN<32>,
    pub verified_ledger: u32,
    pub source: VerificationSource,
    pub age: u32,
    pub country_code: u32,
    pub is_human: bool,
}

#[contractevent]
pub struct Initialized {
    #[topic]
    admin: Address,
    prover: Address,
}

#[contractevent]
pub struct AppRegistered {
    #[topic]
    app_id: Symbol,
    owner: Address,
}

#[contractevent]
pub struct AppUpdated {
    #[topic]
    app_id: Symbol,
    owner: Address,
}

#[contractevent]
pub struct AppRevoked {
    #[topic]
    app_id: Symbol,
    owner: Address,
}

#[contractevent]
pub struct ProverUpdated {
    #[topic]
    previous: Address,
    current: Address,
}

#[contractevent]
pub struct VerificationRecorded {
    #[topic]
    app_id: Symbol,
    #[topic]
    subject: Address,
    nullifier: BytesN<32>,
    source: VerificationSource,
}

#[derive(Clone)]
#[contracttype]
struct SubjectKey {
    app_id: Symbol,
    subject: Address,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    Prover,
    AppPolicy(Symbol),
    AppVkHash(Symbol),
    AppApproved(Symbol),
    AppApprovalRequired,
    Nullifier(Symbol, BytesN<32>),
    Record(SubjectKey),
}

#[contract]
pub struct StellarIdentityCore;

#[contractimpl]
impl StellarIdentityCore {
    pub fn init(env: Env, admin: Address, prover: Address) -> Result<(), IdentityError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(IdentityError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Prover, &prover);
        Initialized {
            admin: admin.clone(),
            prover: prover.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_prover(env: Env, new_prover: Address) -> Result<(), IdentityError> {
        let admin = Self::read_admin(&env)?;
        admin.require_auth();
        let previous = Self::read_prover(&env)?;
        env.storage().instance().set(&DataKey::Prover, &new_prover);
        ProverUpdated {
            previous,
            current: new_prover,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_admin(env: Env) -> Result<Address, IdentityError> {
        Self::read_admin(&env)
    }

    pub fn get_prover(env: Env) -> Result<Address, IdentityError> {
        Self::read_prover(&env)
    }

    /// SHA-256 of canonical attested claims (age, country, humanity). Used by the attested path.
    pub fn hash_attested_claims(env: Env, claims: AttestedClaims) -> BytesN<32> {
        Self::compute_attested_claims_hash(&env, &claims)
    }

    /// SHA-256 binding prover, app, subject, nullifier, and claims hash for the attested path.
    pub fn hash_attestation(
        env: Env,
        prover: Address,
        app_id: Symbol,
        subject: Address,
        nullifier: BytesN<32>,
        claims: AttestedClaims,
    ) -> BytesN<32> {
        let public_inputs_hash = Self::compute_attested_claims_hash(&env, &claims);
        Self::compute_attestation_hash(
            &env,
            &prover,
            &app_id,
            &subject,
            &nullifier,
            &public_inputs_hash,
        )
    }

    pub fn register_app(
        env: Env,
        app_id: Symbol,
        policy: AppPolicy,
        vk_hash: Option<BytesN<32>>,
    ) -> Result<AppPolicy, IdentityError> {
        Self::ensure_initialized(&env)?;
        if Self::get_policy(env.clone(), app_id.clone()).is_some() {
            return Err(IdentityError::AppAlreadyRegistered);
        }
        if Self::is_app_approval_required(&env)
            && !Self::is_approved_app(env.clone(), app_id.clone())
        {
            return Err(IdentityError::AppNotApproved);
        }
        Self::validate_policy_registration(&policy)?;
        policy.owner.require_auth();
        let owner = policy.owner.clone();
        Self::persist_set(&env, &DataKey::AppPolicy(app_id.clone()), &policy);
        if let Some(hash) = vk_hash {
            Self::persist_set(&env, &DataKey::AppVkHash(app_id.clone()), &hash);
        }
        AppRegistered {
            app_id: app_id.clone(),
            owner,
        }
        .publish(&env);
        Ok(policy)
    }

    pub fn update_app_policy(
        env: Env,
        app_id: Symbol,
        policy: AppPolicy,
        vk_hash: Option<BytesN<32>>,
    ) -> Result<AppPolicy, IdentityError> {
        Self::ensure_initialized(&env)?;
        if Self::is_app_approval_required(&env)
            && !Self::is_approved_app(env.clone(), app_id.clone())
        {
            return Err(IdentityError::AppNotApproved);
        }
        let current = Self::get_policy_required(&env, app_id.clone())?;
        if current.owner != policy.owner {
            return Err(IdentityError::AppOwnerMismatch);
        }
        Self::validate_policy_registration(&policy)?;
        policy.owner.require_auth();
        let owner = policy.owner.clone();
        Self::persist_set(&env, &DataKey::AppPolicy(app_id.clone()), &policy);
        if let Some(hash) = vk_hash {
            Self::persist_set(&env, &DataKey::AppVkHash(app_id.clone()), &hash);
        }
        AppUpdated {
            app_id: app_id.clone(),
            owner,
        }
        .publish(&env);
        Ok(policy)
    }

    pub fn revoke_app(env: Env, app_id: Symbol) -> Result<AppPolicy, IdentityError> {
        Self::ensure_initialized(&env)?;
        let policy = Self::get_policy_required(&env, app_id.clone())?;
        policy.owner.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::AppPolicy(app_id.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::AppVkHash(app_id.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::AppApproved(app_id.clone()));
        let owner = policy.owner.clone();
        AppRevoked {
            app_id: app_id.clone(),
            owner,
        }
        .publish(&env);
        Ok(policy)
    }

    pub fn get_policy(env: Env, app_id: Symbol) -> Option<AppPolicy> {
        let policy = env
            .storage()
            .persistent()
            .get(&DataKey::AppPolicy(app_id.clone()));
        if policy.is_some() {
            Self::extend_persistent(&env, &DataKey::AppPolicy(app_id));
        }
        policy
    }

    pub fn is_app_registered(env: Env, app_id: Symbol) -> bool {
        env.storage().persistent().has(&DataKey::AppPolicy(app_id))
    }

    pub fn get_vk_hash(env: Env, app_id: Symbol) -> Option<BytesN<32>> {
        let hash = env
            .storage()
            .persistent()
            .get(&DataKey::AppVkHash(app_id.clone()));
        if hash.is_some() {
            Self::extend_persistent(&env, &DataKey::AppVkHash(app_id));
        }
        hash
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify_and_record(
        env: Env,
        app_id: Symbol,
        subject: Address,
        nullifier: BytesN<32>,
        public_inputs_hash: BytesN<32>,
        vk: VerificationKey,
        proof: Proof,
        pub_signals: Vec<Bn254Fr>,
        current_date_ymd: u32,
        claims: AttestedClaims,
    ) -> Result<VerificationRecord, IdentityError> {
        Self::ensure_initialized(&env)?;
        subject.require_auth();
        let policy = Self::get_policy_required(&env, app_id.clone())?;
        Self::ensure_unused_nullifier(&env, app_id.clone(), nullifier.clone())?;
        Self::ensure_subject_unverified(
            &env,
            app_id.clone(),
            subject.clone(),
            policy.expiration_window,
        )?;

        if pub_signals.len() + 1 != vk.ic.len() {
            return Err(IdentityError::MalformedVerifyingKey);
        }
        if pub_signals.len() < 3 {
            return Err(IdentityError::InsufficientPublicSignals);
        }

        let computed_pub_inputs_hash = Self::compute_pub_signals_hash_internal(&env, &pub_signals)?;
        if public_inputs_hash != computed_pub_inputs_hash {
            return Err(IdentityError::PublicInputsHashMismatch);
        }

        let derived_claims =
            derive_from_signals(&pub_signals, &policy.claim_layout, current_date_ymd)?;
        verify_match(&derived_claims, &claims)?;

        Self::validate_policy(&policy, &derived_claims)?;
        if policy.sanctions_enabled {
            Self::check_sanctions(&env, &policy, &derived_claims)?;
        }

        let stored_vk_hash =
            Self::get_vk_hash(env.clone(), app_id.clone()).ok_or(IdentityError::VkNotRegistered)?;
        let computed_vk_hash = Self::compute_vk_hash(&env, &vk)?;
        if stored_vk_hash != computed_vk_hash {
            return Err(IdentityError::VkMismatch);
        }

        let verified = Self::verify_groth16(&env, vk, proof, pub_signals)?;
        if !verified {
            return Err(IdentityError::InvalidProof);
        }

        let record = VerificationRecord {
            app_id: app_id.clone(),
            subject: subject.clone(),
            nullifier: nullifier.clone(),
            public_inputs_hash,
            verified_ledger: env.ledger().sequence(),
            source: VerificationSource::OnchainGroth16,
            age: derived_claims.age,
            country_code: derived_claims.country_code,
            is_human: derived_claims.is_human,
        };
        Self::finalize_verification(&env, record)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_attested_result(
        env: Env,
        prover: Address,
        app_id: Symbol,
        subject: Address,
        nullifier: BytesN<32>,
        public_inputs_hash: BytesN<32>,
        attestation_hash: BytesN<32>,
        claims: AttestedClaims,
    ) -> Result<VerificationRecord, IdentityError> {
        Self::ensure_initialized(&env)?;
        prover.require_auth();
        subject.require_auth();
        let configured_prover = Self::read_prover(&env)?;
        if prover != configured_prover {
            return Err(IdentityError::Unauthorized);
        }

        let computed_claims_hash = Self::compute_attested_claims_hash(&env, &claims);
        if public_inputs_hash != computed_claims_hash {
            return Err(IdentityError::PublicInputsHashMismatch);
        }

        let expected_attestation = Self::compute_attestation_hash(
            &env,
            &prover,
            &app_id,
            &subject,
            &nullifier,
            &public_inputs_hash,
        );
        if attestation_hash != expected_attestation {
            return Err(IdentityError::AttestationHashMismatch);
        }

        let policy = Self::get_policy_required(&env, app_id.clone())?;
        Self::validate_policy(&policy, &claims)?;
        if policy.sanctions_enabled {
            Self::check_sanctions(&env, &policy, &claims)?;
        }
        Self::ensure_unused_nullifier(&env, app_id.clone(), nullifier.clone())?;
        Self::ensure_subject_unverified(
            &env,
            app_id.clone(),
            subject.clone(),
            policy.expiration_window,
        )?;

        let source = VerificationSource::AttestedProver(attestation_hash.clone());
        let record = VerificationRecord {
            app_id: app_id.clone(),
            subject: subject.clone(),
            nullifier: nullifier.clone(),
            public_inputs_hash,
            verified_ledger: env.ledger().sequence(),
            source: source.clone(),
            age: claims.age,
            country_code: claims.country_code,
            is_human: claims.is_human,
        };
        Self::finalize_verification(&env, record)
    }

    pub fn is_verified(env: Env, app_id: Symbol, subject: Address) -> bool {
        let key = SubjectKey {
            app_id: app_id.clone(),
            subject,
        };
        let record = match env
            .storage()
            .persistent()
            .get::<_, VerificationRecord>(&DataKey::Record(key.clone()))
        {
            Some(r) => r,
            None => return false,
        };
        Self::extend_persistent(&env, &DataKey::Record(key));
        let policy = match Self::get_policy(env.clone(), app_id.clone()) {
            Some(p) => p,
            None => return false,
        };
        if policy.expiration_window == 0 {
            return true;
        }
        let elapsed = env
            .ledger()
            .sequence()
            .saturating_sub(record.verified_ledger);
        elapsed <= policy.expiration_window
    }

    pub fn get_record(env: Env, app_id: Symbol, subject: Address) -> Option<VerificationRecord> {
        let key = SubjectKey {
            app_id: app_id.clone(),
            subject,
        };
        let record = env
            .storage()
            .persistent()
            .get::<_, VerificationRecord>(&DataKey::Record(key.clone()))?;
        Self::extend_persistent(&env, &DataKey::Record(key));
        let policy = Self::get_policy(env.clone(), app_id)?;
        if policy.expiration_window > 0 {
            let elapsed = env
                .ledger()
                .sequence()
                .saturating_sub(record.verified_ledger);
            if elapsed > policy.expiration_window {
                return None;
            }
        }
        Some(record)
    }

    pub fn has_nullifier(env: Env, app_id: Symbol, nullifier: BytesN<32>) -> bool {
        let key = DataKey::Nullifier(app_id.clone(), nullifier.clone());
        let exists = env.storage().persistent().has(&key);
        if exists {
            Self::extend_persistent(&env, &key);
        }
        exists
    }

    pub fn set_app_approval(env: Env, app_id: Symbol, approved: bool) -> Result<(), IdentityError> {
        let admin = Self::read_admin(&env)?;
        admin.require_auth();
        if approved {
            Self::persist_set(&env, &DataKey::AppApproved(app_id), &true);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::AppApproved(app_id));
        }
        Ok(())
    }

    pub fn is_approved_app(env: Env, app_id: Symbol) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::AppApproved(app_id))
    }

    pub fn set_approval_mode(env: Env, required: bool) -> Result<(), IdentityError> {
        let admin = Self::read_admin(&env)?;
        admin.require_auth();
        if required {
            Self::persist_set(&env, &DataKey::AppApprovalRequired, &true);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::AppApprovalRequired);
        }
        Ok(())
    }

    fn is_app_approval_required(env: &Env) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::AppApprovalRequired)
    }

    fn verify_groth16(
        env: &Env,
        vk: VerificationKey,
        proof: Proof,
        pub_signals: Vec<Bn254Fr>,
    ) -> Result<bool, IdentityError> {
        let bn = env.crypto().bn254();
        if pub_signals.len() + 1 != vk.ic.len() {
            return Err(IdentityError::MalformedVerifyingKey);
        }

        let vk_x = vk.ic.get(0).ok_or(IdentityError::MalformedVerifyingKey)?;
        let mut acc = vk_x;
        for (s, v) in pub_signals.iter().zip(vk.ic.iter().skip(1)) {
            let prod = bn.g1_mul(&v, &s);
            acc = bn.g1_add(&acc, &prod);
        }

        let neg_a = -proof.a;
        let vp1 = vec![env, neg_a, vk.alpha, acc, proof.c];
        let vp2 = vec![env, proof.b, vk.beta, vk.gamma, vk.delta];
        Ok(bn.pairing_check(vp1, vp2))
    }

    fn store_record(env: &Env, record: VerificationRecord) -> Result<(), IdentityError> {
        let nullifier_key = DataKey::Nullifier(record.app_id.clone(), record.nullifier.clone());
        Self::persist_set(env, &nullifier_key, &true);
        let key = SubjectKey {
            app_id: record.app_id.clone(),
            subject: record.subject.clone(),
        };
        Self::persist_set(env, &DataKey::Record(key), &record);
        Ok(())
    }

    fn persist_set<K, V>(env: &Env, key: &K, value: &V)
    where
        K: IntoVal<Env, soroban_sdk::Val>,
        V: IntoVal<Env, soroban_sdk::Val>,
    {
        env.storage().persistent().set(key, value);
        Self::extend_persistent(env, key);
    }

    fn extend_persistent<K>(env: &Env, key: &K)
    where
        K: IntoVal<Env, soroban_sdk::Val>,
    {
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
    }

    fn check_sanctions(
        _env: &Env,
        _policy: &AppPolicy,
        _claims: &AttestedClaims,
    ) -> Result<(), IdentityError> {
        Err(IdentityError::SanctionsCheckFailed)
    }

    fn get_policy_required(env: &Env, app_id: Symbol) -> Result<AppPolicy, IdentityError> {
        env.storage()
            .persistent()
            .get(&DataKey::AppPolicy(app_id))
            .ok_or(IdentityError::AppNotFound)
    }

    fn ensure_unused_nullifier(
        env: &Env,
        app_id: Symbol,
        nullifier: BytesN<32>,
    ) -> Result<(), IdentityError> {
        if env
            .storage()
            .persistent()
            .has(&DataKey::Nullifier(app_id, nullifier))
        {
            return Err(IdentityError::NullifierAlreadyUsed);
        }
        Ok(())
    }

    fn ensure_subject_unverified(
        env: &Env,
        app_id: Symbol,
        subject: Address,
        expiration_window: u32,
    ) -> Result<(), IdentityError> {
        let key = SubjectKey { app_id, subject };
        let record: VerificationRecord = match env.storage().persistent().get(&DataKey::Record(key))
        {
            Some(r) => r,
            None => return Ok(()),
        };
        if expiration_window == 0 {
            return Err(IdentityError::SubjectAlreadyVerified);
        }
        let elapsed = env
            .ledger()
            .sequence()
            .saturating_sub(record.verified_ledger);
        if elapsed <= expiration_window {
            return Err(IdentityError::SubjectAlreadyVerified);
        }
        Ok(())
    }

    fn validate_policy_registration(policy: &AppPolicy) -> Result<(), IdentityError> {
        if policy.sanctions_enabled {
            return Err(IdentityError::SanctionsCheckFailed);
        }
        if policy.excluded_countries.len() > MAX_EXCLUDED_COUNTRIES {
            return Err(IdentityError::ExcludedCountriesLimit);
        }
        Ok(())
    }

    fn validate_policy(policy: &AppPolicy, claims: &AttestedClaims) -> Result<(), IdentityError> {
        if claims.age < policy.min_age {
            return Err(IdentityError::PolicyViolation);
        }
        if policy.require_humanity && !claims.is_human {
            return Err(IdentityError::PolicyViolation);
        }
        for country in policy.excluded_countries.iter() {
            if country == claims.country_code {
                return Err(IdentityError::PolicyViolation);
            }
        }
        Ok(())
    }

    fn compute_vk_hash(env: &Env, vk: &VerificationKey) -> Result<BytesN<32>, IdentityError> {
        let mut bytes = Bytes::new(env);
        bytes.append(&vk.alpha.to_bytes().into());
        bytes.append(&vk.beta.to_bytes().into());
        bytes.append(&vk.gamma.to_bytes().into());
        bytes.append(&vk.delta.to_bytes().into());
        for i in 0..vk.ic.len() {
            let point = vk.ic.get(i).ok_or(IdentityError::MalformedVerifyingKey)?;
            bytes.append(&point.to_bytes().into());
        }
        Ok(env.crypto().sha256(&bytes).into())
    }

    fn compute_pub_signals_hash_internal(
        env: &Env,
        pub_signals: &Vec<Bn254Fr>,
    ) -> Result<BytesN<32>, IdentityError> {
        let mut bytes = Bytes::new(env);
        for i in 0..pub_signals.len() {
            let signal = pub_signals
                .get(i)
                .ok_or(IdentityError::InsufficientPublicSignals)?;
            bytes.append(&signal.to_bytes().into());
        }
        Ok(env.crypto().sha256(&bytes).into())
    }

    fn compute_attested_claims_hash(env: &Env, claims: &AttestedClaims) -> BytesN<32> {
        let mut bytes = Bytes::new(env);
        bytes.extend_from_slice(&claims.age.to_le_bytes());
        bytes.extend_from_slice(&claims.country_code.to_le_bytes());
        let humanity = if claims.is_human { 1u32 } else { 0u32 };
        bytes.extend_from_slice(&humanity.to_le_bytes());
        env.crypto().sha256(&bytes).into()
    }

    fn compute_attestation_hash(
        env: &Env,
        prover: &Address,
        app_id: &Symbol,
        subject: &Address,
        nullifier: &BytesN<32>,
        public_inputs_hash: &BytesN<32>,
    ) -> BytesN<32> {
        let mut bytes = Bytes::new(env);
        bytes.append(&prover.to_xdr(env));
        bytes.append(&app_id.to_xdr(env));
        bytes.append(&subject.to_xdr(env));
        bytes.append(&nullifier.to_xdr(env));
        bytes.append(&public_inputs_hash.to_xdr(env));
        env.crypto().sha256(&bytes).into()
    }

    fn finalize_verification(
        env: &Env,
        record: VerificationRecord,
    ) -> Result<VerificationRecord, IdentityError> {
        let source = record.source.clone();
        Self::store_record(env, record.clone())?;
        VerificationRecorded {
            app_id: record.app_id.clone(),
            subject: record.subject.clone(),
            nullifier: record.nullifier.clone(),
            source,
        }
        .publish(env);
        Ok(record)
    }

    fn ensure_initialized(env: &Env) -> Result<(), IdentityError> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(IdentityError::NotInitialized);
        }
        Ok(())
    }

    /// Used by unit tests and off-chain hash replication.
    pub fn compute_pub_signals_hash(
        env: &Env,
        pub_signals: &Vec<Bn254Fr>,
    ) -> Result<BytesN<32>, IdentityError> {
        Self::compute_pub_signals_hash_internal(env, pub_signals)
    }

    fn read_admin(env: &Env) -> Result<Address, IdentityError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(IdentityError::NotInitialized)
    }

    fn read_prover(env: &Env) -> Result<Address, IdentityError> {
        env.storage()
            .instance()
            .get(&DataKey::Prover)
            .ok_or(IdentityError::NotInitialized)
    }
}

#[cfg(test)]
mod test;
