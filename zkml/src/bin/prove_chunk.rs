//! CLI binary for generating proofs for model chunks.
//!
//! Usage:
//!   prove_chunk --config <path> --input <path> --start <n> --end <n> \
//!               [--use-merkle] [--prev-root <hex>] \
//!               [--params-dir <path>] [--output-dir <path>]
//!
//! Output files written to output-dir:
//!   - proof.bin: The proof bytes
//!   - public_vals.bin: Public values as concatenated 32-byte field elements
//!   - result.json: Metadata including timing and merkle root

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use clap::Parser;
use halo2_proofs::halo2curves::bn256::Fr;
use halo2_proofs::halo2curves::ff::PrimeField;
use serde_derive::Serialize;

use zkml::utils::proving_kzg::prove_chunk_kzg;

#[derive(Parser, Debug)]
#[command(name = "prove_chunk")]
#[command(about = "Generate ZK proof for a model chunk")]
struct Args {
    /// Path to model config/weights (msgpack)
    #[arg(long)]
    config: String,

    /// Path to input data (msgpack)
    #[arg(long)]
    input: String,

    /// Start layer index (inclusive)
    #[arg(long)]
    start: usize,

    /// End layer index (exclusive)
    #[arg(long)]
    end: usize,

    /// Enable Merkle tree for intermediate values
    #[arg(long, default_value = "false")]
    use_merkle: bool,

    /// Previous chunk's Merkle root (hex string, 64 chars)
    #[arg(long)]
    prev_root: Option<String>,

    /// Directory for KZG params (created if needed)
    #[arg(long, default_value = "./params_kzg")]
    params_dir: String,

    /// Output directory for proof files
    #[arg(long, default_value = "./proof_output")]
    output_dir: String,
}

#[derive(Serialize)]
struct ProofResult {
    chunk_start: usize,
    chunk_end: usize,
    use_merkle: bool,
    prev_merkle_root: Option<String>,
    merkle_root: Option<String>,
    proving_time_ms: u128,
    verify_time_ms: u128,
    proof_size_bytes: usize,
    public_vals_count: usize,
}

fn hex_to_fr(hex: &str) -> Option<Fr> {
    // Remove 0x prefix if present
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    
    if hex.len() != 64 {
        eprintln!("Error: prev-root must be 64 hex characters (32 bytes)");
        return None;
    }
    
    let bytes = hex::decode(hex).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    Fr::from_repr(arr).into()
}

fn fr_to_hex(fr: &Fr) -> String {
    let bytes = fr.to_repr();
    format!("0x{}", hex::encode(bytes))
}

fn main() {
    let args = Args::parse();

    // Create output directory
    fs::create_dir_all(&args.output_dir).expect("Failed to create output directory");
    fs::create_dir_all(&args.params_dir).expect("Failed to create params directory");

    // Parse previous merkle root if provided
    let prev_merkle_root = args.prev_root.as_ref().and_then(|h| hex_to_fr(h));
    
    if args.prev_root.is_some() && prev_merkle_root.is_none() {
        eprintln!("Error: Invalid prev-root hex string");
        std::process::exit(1);
    }

    println!(
        "Proving chunk [{}, {}) with merkle={}",
        args.start, args.end, args.use_merkle
    );

    // Generate proof
    let result = prove_chunk_kzg(
        &args.config,
        &args.input,
        args.start,
        args.end,
        args.use_merkle,
        prev_merkle_root,
        &args.params_dir,
    );

    // Write proof to file
    let proof_path = Path::new(&args.output_dir).join("proof.bin");
    let mut proof_file = File::create(&proof_path).expect("Failed to create proof file");
    proof_file
        .write_all(&result.proof)
        .expect("Failed to write proof");

    // Write public values to file (concatenated 32-byte field elements)
    let public_vals_path = Path::new(&args.output_dir).join("public_vals.bin");
    let mut pv_file = File::create(&public_vals_path).expect("Failed to create public_vals file");
    for val in &result.public_vals {
        pv_file
            .write_all(&val.to_repr())
            .expect("Failed to write public val");
    }

    // Write result metadata as JSON
    let json_result = ProofResult {
        chunk_start: args.start,
        chunk_end: args.end,
        use_merkle: args.use_merkle,
        prev_merkle_root: args.prev_root.clone(),
        merkle_root: result.merkle_root.as_ref().map(fr_to_hex),
        proving_time_ms: result.proving_time_ms,
        verify_time_ms: result.verify_time_ms,
        proof_size_bytes: result.proof.len(),
        public_vals_count: result.public_vals.len(),
    };

    let result_path = Path::new(&args.output_dir).join("result.json");
    let result_json = serde_json::to_string_pretty(&json_result).expect("Failed to serialize result");
    fs::write(&result_path, result_json).expect("Failed to write result.json");

    println!("Output written to {}/", args.output_dir);
    println!("  proof.bin: {} bytes", result.proof.len());
    println!("  public_vals.bin: {} values", result.public_vals.len());
    println!("  result.json: metadata");
    
    if let Some(root) = &json_result.merkle_root {
        println!("  merkle_root: {}", root);
    }
}

