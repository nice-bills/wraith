use std::{fs, path::PathBuf, str::FromStr};

use anyhow::{Context, Result, bail};
use clap::Parser;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod rarimo_transformer;
use rarimo_transformer::RarimoPublicSignals;

#[derive(Parser, Debug)]
#[command(
    name = "proof-adapter",
    about = "Convert snarkjs Groth16 JSON into Soroban BN254 payloads"
)]
struct Cli {
    #[arg(long)]
    proof: PathBuf,
    #[arg(long)]
    vk: PathBuf,
    #[arg(long)]
    public: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    age_index: Option<usize>,
    #[arg(long)]
    country_index: Option<usize>,
    #[arg(long)]
    humanity_index: Option<usize>,
    #[arg(long)]
    rarimo_mode: bool,
    #[arg(long)]
    current_date: Option<String>,
}

#[derive(Deserialize)]
struct SnarkProof {
    pi_a: Value,
    pi_b: Value,
    pi_c: Value,
}

#[derive(Deserialize)]
struct SnarkVerificationKey {
    vk_alpha_1: Value,
    vk_beta_2: Value,
    vk_gamma_2: Value,
    vk_delta_2: Value,
    #[serde(rename = "IC")]
    ic: Value,
}

#[derive(Serialize)]
struct AdapterOutput {
    proof: ProofPayload,
    verification_key: VerificationKeyPayload,
    public_signals_decimals: Vec<String>,
    claims: Option<ClaimsPayload>,
}

#[derive(Serialize)]
struct ProofPayload {
    a: String,
    b: String,
    c: String,
}

#[derive(Serialize)]
struct VerificationKeyPayload {
    alpha: String,
    beta: String,
    delta: String,
    gamma: String,
    ic: Vec<String>,
}

impl VerificationKeyPayload {
    fn ic_len(&self) -> usize {
        self.ic.len()
    }
}

#[derive(Serialize, PartialEq, Eq, Debug)]
struct ClaimsPayload {
    age: u32,
    country_code: u32,
    is_human: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let proof: SnarkProof = read_json_file(&cli.proof)?;
    let vk: SnarkVerificationKey = read_json_file(&cli.vk)?;
    let public_signals: Value = read_json_file(&cli.public)?;

    let public_signals_decimals = parse_public_signals(&public_signals)?;

    let (output_public_signals, claims) = if cli.rarimo_mode {
        let current_date = cli.current_date.unwrap_or_else(|| {
            chrono::Local::now().format("%y%m%d").to_string()
        });

        let rarimo_signals = RarimoPublicSignals::from_query_output(&public_signals_decimals)
            .context("failed to parse rarimo signals")?;

        let wraith_claims = rarimo_signals
            .to_wraith_claims(&current_date)
            .context("failed to derive wraith claims from rarimo signals")?;

        let claims_payload = ClaimsPayload {
            age: wraith_claims.age,
            country_code: wraith_claims.country_code,
            is_human: wraith_claims.is_human,
        };

        (wraith_claims.to_public_signals(), Some(claims_payload))
    } else {
        let claims = derive_claims(
            &public_signals_decimals,
            cli.age_index,
            cli.country_index,
            cli.humanity_index,
        )?;
        (public_signals_decimals.clone(), claims)
    };

    let output = AdapterOutput {
        proof: ProofPayload {
            a: encode_g1(&proof.pi_a).context("failed to parse pi_a")?,
            b: encode_g2(&proof.pi_b).context("failed to parse pi_b")?,
            c: encode_g1(&proof.pi_c).context("failed to parse pi_c")?,
        },
        verification_key: VerificationKeyPayload {
            alpha: encode_g1(&vk.vk_alpha_1).context("failed to parse vk_alpha_1")?,
            beta: encode_g2(&vk.vk_beta_2).context("failed to parse vk_beta_2")?,
            gamma: encode_g2(&vk.vk_gamma_2).context("failed to parse vk_gamma_2")?,
            delta: encode_g2(&vk.vk_delta_2).context("failed to parse vk_delta_2")?,
            ic: encode_ic(&vk.ic).context("failed to parse IC")?,
        },
        public_signals_decimals: output_public_signals,
        claims,
    };

    validate_output(&output)?;

    fs::write(&cli.out, serde_json::to_string_pretty(&output)?).with_context(|| {
        format!(
            "failed to write adapter output to {}",
            cli.out.to_string_lossy()
        )
    })?;

    Ok(())
}

fn read_json_file<T: for<'a> Deserialize<'a>>(path: &PathBuf) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.to_string_lossy()))?;
    let parsed: T = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse json in {}", path.to_string_lossy()))?;
    Ok(parsed)
}

fn parse_public_signals(v: &Value) -> Result<Vec<String>> {
    let arr = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("public input file must be a JSON array"))?;
    arr.iter()
        .enumerate()
        .map(|(i, x)| parse_decimal_value(x).with_context(|| format!("public[{i}]")))
        .collect()
}

fn encode_ic(v: &Value) -> Result<Vec<String>> {
    let arr = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("IC must be an array"))?;
    arr.iter()
        .enumerate()
        .map(|(i, g1)| encode_g1(g1).with_context(|| format!("IC[{i}]")))
        .collect()
}

fn encode_g1(v: &Value) -> Result<String> {
    let arr = as_array(v, "expected G1 array [x, y, ...]")?;
    if arr.len() < 2 {
        bail!("expected at least 2 values for G1 point, got {}", arr.len());
    }

    let x = decimal_to_be32(&parse_decimal_value(&arr[0])?)?;
    let y = decimal_to_be32(&parse_decimal_value(&arr[1])?)?;
    Ok(format!("0x{}{}", hex::encode(x), hex::encode(y)))
}

fn encode_g2(v: &Value) -> Result<String> {
    let arr = as_array(v, "expected G2 array [[x0,x1],[y0,y1],...]")?;
    if arr.len() < 2 {
        bail!("expected at least 2 values for G2 point, got {}", arr.len());
    }
    let x = encode_fp2_pair(&arr[0]).context("G2.x")?;
    let y = encode_fp2_pair(&arr[1]).context("G2.y")?;

    let mut out = [0u8; 128];
    out[..64].copy_from_slice(&x);
    out[64..].copy_from_slice(&y);
    Ok(format!("0x{}", hex::encode(out)))
}

fn encode_fp2_pair(v: &Value) -> Result<[u8; 64]> {
    let pair = as_array(v, "expected Fp2 pair [c0, c1]")?;
    if pair.len() < 2 {
        bail!(
            "expected 2 coefficients for Fp2 element, got {}",
            pair.len()
        );
    }
    let c0 = decimal_to_be32(&parse_decimal_value(&pair[0])?)?;
    let c1 = decimal_to_be32(&parse_decimal_value(&pair[1])?)?;

    // Soroban BN254 encoding uses c1 || c0 for each Fp2 element.
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&c1);
    out[32..].copy_from_slice(&c0);
    Ok(out)
}

fn decimal_to_be32(s: &str) -> Result<[u8; 32]> {
    let v = BigUint::from_str(s).with_context(|| format!("invalid decimal integer: {s}"))?;
    let bytes = v.to_bytes_be();
    if bytes.len() > 32 {
        bail!("integer does not fit in 32 bytes: {s}");
    }
    let mut out = [0u8; 32];
    let start = 32 - bytes.len();
    out[start..].copy_from_slice(&bytes);
    Ok(out)
}

fn parse_decimal_value(v: &Value) -> Result<String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => bail!("expected decimal string/number, got {v}"),
    }
}

fn as_array<'a>(v: &'a Value, msg: &str) -> Result<&'a Vec<Value>> {
    v.as_array().ok_or_else(|| anyhow::anyhow!("{msg}"))
}

fn derive_claims(
    public_signals_decimals: &[String],
    age_index: Option<usize>,
    country_index: Option<usize>,
    humanity_index: Option<usize>,
) -> Result<Option<ClaimsPayload>> {
    let all = [age_index, country_index, humanity_index];
    if all.iter().all(Option::is_none) {
        return Ok(None);
    }
    if all.iter().any(Option::is_none) {
        bail!(
            "age_index, country_index, and humanity_index must be provided together or omitted together"
        );
    }

    let age_raw = read_signal(public_signals_decimals, age_index.unwrap(), "age_index")?;
    let country_raw = read_signal(
        public_signals_decimals,
        country_index.unwrap(),
        "country_index",
    )?;
    let humanity_raw = read_signal(
        public_signals_decimals,
        humanity_index.unwrap(),
        "humanity_index",
    )?;

    let age = age_raw
        .parse::<u32>()
        .with_context(|| format!("age signal must fit u32, got {age_raw}"))?;
    let country_code = country_raw
        .parse::<u32>()
        .with_context(|| format!("country signal must fit u32, got {country_raw}"))?;
    let humanity_num = humanity_raw
        .parse::<u32>()
        .with_context(|| format!("humanity signal must be 0 or 1, got {humanity_raw}"))?;
    let is_human = match humanity_num {
        0 => false,
        1 => true,
        _ => bail!("humanity signal must be 0 or 1, got {humanity_num}"),
    };

    Ok(Some(ClaimsPayload {
        age,
        country_code,
        is_human,
    }))
}

fn validate_output(output: &AdapterOutput) -> Result<()> {
    if output.verification_key.ic_len() == 0 {
        bail!("verification key IC must not be empty");
    }
    if output.public_signals_decimals.is_empty() {
        bail!("public signals must not be empty");
    }
    if let Some(claims) = &output.claims {
        if claims.age == 0 {
            bail!("claims.age must be greater than zero");
        }
        if claims.country_code == 0 {
            bail!("claims.country_code must be greater than zero");
        }
    }
    Ok(())
}

fn read_signal<'a>(signals: &'a [String], idx: usize, name: &str) -> Result<&'a str> {
    signals
        .get(idx)
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("{name} out of bounds for {} signals", signals.len()))
}

#[cfg(test)]
mod tests {
    use super::{ClaimsPayload, decimal_to_be32, derive_claims, encode_fp2_pair, validate_output};
    use serde_json::json;

    #[test]
    fn decimal_encoding_is_left_padded() {
        let out = decimal_to_be32("1").unwrap();
        assert_eq!(out[31], 1);
        assert!(out[..31].iter().all(|x| *x == 0));
    }

    #[test]
    fn fp2_order_is_c1_then_c0() {
        let encoded = encode_fp2_pair(&json!(["1", "2"])).unwrap();
        assert_eq!(encoded[31], 2);
        assert_eq!(encoded[63], 1);
    }

    #[test]
    fn derives_claims_when_indexes_are_provided() {
        let signals = vec!["18".to_string(), "566".to_string(), "1".to_string()];
        let claims = derive_claims(&signals, Some(0), Some(1), Some(2)).unwrap();
        assert_eq!(
            claims,
            Some(ClaimsPayload {
                age: 18,
                country_code: 566,
                is_human: true
            })
        );
    }

    #[test]
    fn claims_indexes_must_be_all_or_none() {
        let signals = vec!["18".to_string(), "566".to_string(), "1".to_string()];
        assert!(derive_claims(&signals, Some(0), None, Some(2)).is_err());
    }

    #[test]
    fn validate_output_rejects_empty_ic() {
        let output = super::AdapterOutput {
            proof: super::ProofPayload {
                a: "0x1".into(),
                b: "0x2".into(),
                c: "0x3".into(),
            },
            verification_key: super::VerificationKeyPayload {
                alpha: "0x1".into(),
                beta: "0x2".into(),
                gamma: "0x3".into(),
                delta: "0x4".into(),
                ic: Vec::new(),
            },
            public_signals_decimals: vec!["1".into()],
            claims: None,
        };
        assert!(validate_output(&output).is_err());
    }
}
