//! Test binary to verify Merkle root is added to public values
//!
//! Usage:
//!   cargo run --bin test_merkle_public -- examples/mnist/config.msgpack examples/mnist/inp.msgpack

use halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};
use zkml::{
    model::ModelCircuit,
    utils::{
        helpers::get_public_values,
        loader::load_model_msgpack,
    },
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <config_file> <input_file> [chunk_start] [chunk_end]", args[0]);
        eprintln!("Example: {} examples/mnist/config.msgpack examples/mnist/inp.msgpack 0 2", args[0]);
        std::process::exit(1);
    }

    let config_file = &args[1];
    let input_file = &args[2];
    let chunk_start = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
    let chunk_end = args.get(4).and_then(|s| s.parse::<usize>().ok());

    println!("==========================================");
    println!("Testing Merkle Root in Public Values");
    println!("==========================================");
    println!("Config file: {}", config_file);
    println!("Input file: {}", input_file);
    
    // Load model configuration
    let config = load_model_msgpack(config_file, input_file);
    println!("Model has {} layers", config.layers.len());
    
    // Create circuit
    let mut circuit = ModelCircuit::<Fr>::generate_from_file(config_file, input_file);
    
    if let Some(end) = chunk_end {
        println!("\n--- Testing Chunk Execution with Merkle ---");
        println!("Chunk: layers {} to {}", chunk_start, end);
        circuit.set_chunk_config(chunk_start, end, true);
    } else {
        println!("\n--- Testing Full Model Execution ---");
        println!("(No chunk configuration - default behavior)");
    }
    
    // First synthesis to populate public values
    println!("\n1. Synthesizing circuit...");
    let _prover1 = MockProver::run(config.k.try_into().unwrap(), &circuit, vec![vec![]])
        .expect("Failed to run MockProver");
    
    // Get public values
    println!("2. Getting public values...");
    let public_vals = get_public_values();
    println!("   Number of public values: {}", public_vals.len());
    
    if !public_vals.is_empty() {
        println!("   First 3 public values:");
        for (i, val) in public_vals.iter().take(3).enumerate() {
            println!("     [{}]: {:?}", i, val);
        }
        if public_vals.len() > 3 {
            println!("     ... ({} more)", public_vals.len() - 3);
        }
    }
    
    // Verify the circuit with public values
    println!("\n3. Verifying circuit with public values...");
    let prover2 = MockProver::run(config.k.try_into().unwrap(), &circuit, vec![public_vals])
        .expect("Failed to run MockProver with public values");
    
    match prover2.verify() {
        Ok(()) => {
            println!("\n✓ SUCCESS: Circuit verification passed!");
            if chunk_end.is_some() {
                println!("✓ Merkle root is successfully included in public values");
            }
        }
        Err(e) => {
            eprintln!("\n✗ FAILED: Circuit verification failed");
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
    
    println!("\n==========================================");
}

