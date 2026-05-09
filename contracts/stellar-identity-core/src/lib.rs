#![no_std]

use soroban_sdk::{
    Address, BytesN, Env, Symbol, Vec, contract, contracterror, contractevent, contractimpl,
    contracttype,
    crypto::bn254::{Bn254Fr, Bn254G1Affine, Bn254G2Affine},
    vec,
};

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

    pub fn register_app(
        env: Env,
        app_id: Symbol,
        policy: AppPolicy,
        vk_hash: Option<BytesN<32>>,
    ) -> Result<AppPolicy, IdentityError> {
        if Self::get_policy(env.clone(), app_id.clone()).is_some() {
            return Err(IdentityError::AppAlreadyRegistered);
        }
        if Self::is_app_approval_required(&env) {
            if !Self::is_approved_app(env.clone(), app_id.clone()) {
                return Err(IdentityError::AppNotApproved);
            }
        }
        policy.owner.require_auth();
        let owner = policy.owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::AppPolicy(app_id.clone()), &policy);
        if let Some(hash) = vk_hash {
            env.storage()
                .persistent()
                .set(&DataKey::AppVkHash(app_id.clone()), &hash);
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
        if Self::is_app_approval_required(&env) {
            if !Self::is_approved_app(env.clone(), app_id.clone()) {
                return Err(IdentityError::AppNotApproved);
            }
        }
        let current = Self::get_policy_required(&env, app_id.clone())?;
        if current.owner != policy.owner {
            return Err(IdentityError::AppOwnerMismatch);
        }
        policy.owner.require_auth();
        let owner = policy.owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::AppPolicy(app_id.clone()), &policy);
        if let Some(hash) = vk_hash {
            env.storage()
                .persistent()
                .set(&DataKey::AppVkHash(app_id.clone()), &hash);
        }
        AppUpdated {
            app_id: app_id.clone(),
            owner,
        }
        .publish(&env);
        Ok(policy)
    }

    pub fn revoke_app(env: Env, app_id: Symbol) -> Result<AppPolicy, IdentityError> {
        let policy = Self::get_policy_required(&env, app_id.clone())?;
        policy.owner.require_auth();
        env.storage().persistent().remove(&DataKey::AppPolicy(app_id.clone()));
        env.storage().persistent().remove(&DataKey::AppVkHash(app_id.clone()));
        let owner = policy.owner.clone();
        AppRevoked {
            app_id: app_id.clone(),
            owner,
        }
        .publish(&env);
        Ok(policy)
    }

    pub fn get_policy(env: Env, app_id: Symbol) -> Option<AppPolicy> {
        env.storage().persistent().get(&DataKey::AppPolicy(app_id))
    }

    pub fn is_app_registered(env: Env, app_id: Symbol) -> bool {
        env.storage().persistent().has(&DataKey::AppPolicy(app_id))
    }

    pub fn get_vk_hash(env: Env, app_id: Symbol) -> Option<BytesN<32>> {
        env.storage().persistent().get(&DataKey::AppVkHash(app_id))
    }

    pub fn verify_and_record(
        env: Env,
        app_id: Symbol,
        subject: Address,
        nullifier: BytesN<32>,
        public_inputs_hash: BytesN<32>,
        vk: VerificationKey,
        proof: Proof,
        pub_signals: Vec<Bn254Fr>,
        claims: AttestedClaims,
    ) -> Result<VerificationRecord, IdentityError> {
        subject.require_auth();
        Self::ensure_unused_nullifier(&env, app_id.clone(), nullifier.clone())?;
        Self::ensure_subject_unverified(&env, app_id.clone(), subject.clone())?;

        let computed_pub_inputs_hash = Self::compute_pub_signals_hash(&env, &pub_signals);
        if public_inputs_hash != computed_pub_inputs_hash {
            return Err(IdentityError::PublicInputsHashMismatch);
        }

        let derived_claims = Self::derive_claims_from_signals(&pub_signals)?;
        Self::verify_claims_match(&derived_claims, &claims)?;

        let policy = Self::get_policy_required(&env, app_id.clone())?;
        Self::validate_policy(&policy, &derived_claims)?;
        if policy.sanctions_enabled {
            Self::check_sanctions(&env, &policy, &derived_claims)?;
        }

        let stored_vk_hash = Self::get_vk_hash(env.clone(), app_id.clone()).ok_or(IdentityError::VkNotRegistered)?;
        let computed_vk_hash = Self::compute_vk_hash(&env, &vk);
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
        Self::store_record(&env, record.clone())?;
        let source = VerificationSource::OnchainGroth16;
        let app_id = record.app_id.clone();
        let subject = record.subject.clone();
        let nullifier = record.nullifier.clone();
        VerificationRecorded {
            app_id,
            subject,
            nullifier,
            source,
        }
        .publish(&env);
        Ok(record)
    }

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
        prover.require_auth();
        let configured_prover = Self::read_prover(&env)?;
        if prover != configured_prover {
            return Err(IdentityError::Unauthorized);
        }

        let policy = Self::get_policy_required(&env, app_id.clone())?;
        Self::validate_policy(&policy, &claims)?;
        if policy.sanctions_enabled {
            Self::check_sanctions(&env, &policy, &claims)?;
        }
        Self::ensure_unused_nullifier(&env, app_id.clone(), nullifier.clone())?;
        Self::ensure_subject_unverified(&env, app_id.clone(), subject.clone())?;

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
        Self::store_record(&env, record.clone())?;
        let app_id = record.app_id.clone();
        let subject = record.subject.clone();
        let nullifier = record.nullifier.clone();
        VerificationRecorded {
            app_id,
            subject,
            nullifier,
            source,
        }
        .publish(&env);
        Ok(record)
    }

    pub fn is_verified(env: Env, app_id: Symbol, subject: Address) -> bool {
        let key = SubjectKey { app_id, subject };
        env.storage().persistent().has(&DataKey::Record(key))
    }

    pub fn get_record(env: Env, app_id: Symbol, subject: Address) -> Option<VerificationRecord> {
        let key = SubjectKey { app_id, subject };
        env.storage().persistent().get(&DataKey::Record(key))
    }

    pub fn has_nullifier(env: Env, app_id: Symbol, nullifier: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Nullifier(app_id, nullifier))
    }

    pub fn set_app_approval(env: Env, app_id: Symbol, approved: bool) -> Result<(), IdentityError> {
        let admin = Self::read_admin(&env)?;
        admin.require_auth();
        if approved {
            env.storage()
                .persistent()
                .set(&DataKey::AppApproved(app_id), &true);
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
            env.storage()
                .persistent()
                .set(&DataKey::AppApprovalRequired, &true);
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

        let mut vk_x = vk.ic.get(0).unwrap();
        for (s, v) in pub_signals.iter().zip(vk.ic.iter().skip(1)) {
            let prod = bn.g1_mul(&v, &s);
            vk_x = bn.g1_add(&vk_x, &prod);
        }

        let neg_a = -proof.a;
        let vp1 = vec![env, neg_a, vk.alpha, vk_x, proof.c];
        let vp2 = vec![env, proof.b, vk.beta, vk.gamma, vk.delta];
        Ok(bn.pairing_check(vp1, vp2))
    }

    fn store_record(env: &Env, record: VerificationRecord) -> Result<(), IdentityError> {
        let policy = Self::get_policy_required(env, record.app_id.clone())?;
        if policy.expiration_window > 0 {
            let current_ledger = env.ledger().sequence();
            let elapsed = current_ledger.saturating_sub(record.verified_ledger);
            if elapsed > policy.expiration_window {
                return Err(IdentityError::VerificationExpired);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::Nullifier(record.app_id.clone(), record.nullifier.clone()), &true);
        let key = SubjectKey {
            app_id: record.app_id.clone(),
            subject: record.subject.clone(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Record(key), &record);
        Ok(())
    }

    fn check_sanctions(env: &Env, policy: &AppPolicy, _claims: &AttestedClaims) -> Result<(), IdentityError> {
        let sanctions_root = policy.sanctions_root.clone();
        let zero = BytesN::<32>::from_array(env, &[0u8; 32]);
        if sanctions_root == zero {
            return Err(IdentityError::SanctionsCheckFailed);
        }
        Ok(())
    }

    fn get_policy_required(env: &Env, app_id: Symbol) -> Result<AppPolicy, IdentityError> {
        env.storage()
            .persistent()
            .get(&DataKey::AppPolicy(app_id))
            .ok_or(IdentityError::AppNotFound)
    }

    fn ensure_unused_nullifier(env: &Env, app_id: Symbol, nullifier: BytesN<32>) -> Result<(), IdentityError> {
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
    ) -> Result<(), IdentityError> {
        let key = SubjectKey { app_id, subject };
        if env.storage().persistent().has(&DataKey::Record(key)) {
            return Err(IdentityError::SubjectAlreadyVerified);
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

    fn compute_vk_hash(env: &Env, vk: &VerificationKey) -> BytesN<32> {
        use soroban_sdk::Bytes;
        let mut bytes = Bytes::new(env);
        bytes.append(&vk.alpha.to_bytes().into());
        bytes.append(&vk.beta.to_bytes().into());
        bytes.append(&vk.gamma.to_bytes().into());
        bytes.append(&vk.delta.to_bytes().into());
        for i in 0..vk.ic.len() {
            bytes.append(&vk.ic.get(i).unwrap().to_bytes().into());
        }
        let hash = env.crypto().sha256(&bytes);
        hash.into()
    }

    fn compute_pub_signals_hash(env: &Env, pub_signals: &Vec<Bn254Fr>) -> BytesN<32> {
        use soroban_sdk::Bytes;
        let mut bytes = Bytes::new(env);
        for i in 0..pub_signals.len() {
            bytes.append(&pub_signals.get(i).unwrap().to_bytes().into());
        }
        let hash = env.crypto().sha256(&bytes);
        hash.into()
    }

    fn derive_claims_from_signals(pub_signals: &Vec<Bn254Fr>) -> Result<AttestedClaims, IdentityError> {
        if pub_signals.len() < 3 {
            return Err(IdentityError::MalformedVerifyingKey);
        }
        let age_fr = pub_signals.get(0).unwrap();
        let country_fr = pub_signals.get(1).unwrap();
        let is_human_fr = pub_signals.get(2).unwrap();
        let age_bytes = age_fr.to_bytes();
        let country_bytes = country_fr.to_bytes();
        let is_human_bytes = is_human_fr.to_bytes();
        let age = u32::from_le_bytes([
            age_bytes.get(0).unwrap_or(0),
            age_bytes.get(1).unwrap_or(0),
            age_bytes.get(2).unwrap_or(0),
            age_bytes.get(3).unwrap_or(0),
        ]);
        let country_code = u32::from_le_bytes([
            country_bytes.get(0).unwrap_or(0),
            country_bytes.get(1).unwrap_or(0),
            country_bytes.get(2).unwrap_or(0),
            country_bytes.get(3).unwrap_or(0),
        ]);
        let is_human_val = u32::from_le_bytes([
            is_human_bytes.get(0).unwrap_or(0),
            is_human_bytes.get(1).unwrap_or(0),
            is_human_bytes.get(2).unwrap_or(0),
            is_human_bytes.get(3).unwrap_or(0),
        ]);
        Ok(AttestedClaims {
            age,
            country_code,
            is_human: is_human_val != 0,
        })
    }

    fn verify_claims_match(derived: &AttestedClaims, supplied: &AttestedClaims) -> Result<(), IdentityError> {
        if derived.age != supplied.age {
            return Err(IdentityError::PolicyViolation);
        }
        if derived.country_code != supplied.country_code {
            return Err(IdentityError::PolicyViolation);
        }
        if derived.is_human != supplied.is_human {
            return Err(IdentityError::PolicyViolation);
        }
        Ok(())
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

mod test;
