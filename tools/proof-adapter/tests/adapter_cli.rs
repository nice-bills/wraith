use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn unique_output_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("proof-adapter-{nanos}.json"))
}

#[test]
fn cli_converts_fixtures_and_derives_claims() {
    let output_path = unique_output_path();

    let status = Command::new(env!("CARGO_BIN_EXE_proof-adapter"))
        .arg("--proof")
        .arg(fixture("proof.json"))
        .arg("--vk")
        .arg(fixture("verification_key.json"))
        .arg("--public")
        .arg(fixture("public.json"))
        .arg("--out")
        .arg(&output_path)
        .arg("--age-index")
        .arg("0")
        .arg("--country-index")
        .arg("1")
        .arg("--humanity-index")
        .arg("2")
        .status()
        .expect("failed to run proof-adapter binary");
    assert!(status.success());

    let raw = fs::read_to_string(&output_path).expect("failed to read output");
    let output: Value = serde_json::from_str(&raw).expect("invalid output json");

    let a = output["proof"]["a"].as_str().expect("missing proof.a");
    let b = output["proof"]["b"].as_str().expect("missing proof.b");
    let ic = output["verification_key"]["ic"]
        .as_array()
        .expect("missing verification_key.ic");

    assert_eq!(a.len(), 130); // 0x + 64-byte hex
    assert_eq!(b.len(), 258); // 0x + 128-byte hex
    assert_eq!(ic.len(), 2);
    assert_eq!(output["claims"]["age"], 25);
    assert_eq!(output["claims"]["country_code"], 840);
    assert_eq!(output["claims"]["is_human"], true);
    assert_eq!(
        output["public_signals_decimals"].as_array().unwrap().len(),
        4
    );

    let _ = fs::remove_file(output_path);
}
