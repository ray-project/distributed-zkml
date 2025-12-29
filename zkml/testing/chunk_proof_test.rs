//! Tests for chunk proof generation
//!
//! These tests verify that:
//! 1. Real KZG proofs can be generated for model chunks
//! 2. Proofs verify correctly
//!
//! Note: These are slow tests (~50s each) as they generate real cryptographic proofs.
//! Run with: cargo test --test chunk_proof_test --release -- --nocapture

#[cfg(test)]
mod tests {
    use zkml::utils::proving_kzg::prove_chunk_kzg;
    use std::fs;

    /// Test: Generate and verify a real KZG proof for a chunk
    /// 
    /// This is the main test - it proves that we can generate real ZK proofs
    /// for a subset of model layers.
    #[test]
    fn test_chunk_proof_generation() {
        let config_file = "examples/mnist/model.msgpack";
        let input_file = "examples/mnist/inp.msgpack";
        
        if !std::path::Path::new(config_file).exists() {
            eprintln!("Skipping test: example files not found");
            return;
        }
        
        // Use unique params directory to avoid race conditions
        let params_dir = "./params_kzg_chunk_test";
        fs::create_dir_all(params_dir).ok();
        
        // Generate proof for first 2 layers WITHOUT Merkle
        // This tests the core proof generation functionality
        let result = prove_chunk_kzg(
            config_file,
            input_file,
            0,  // chunk_start
            2,  // chunk_end
            false,  // use_merkle = false (simpler case)
            params_dir,
        );
        
        // Verify we got a valid proof
        assert!(!result.proof.is_empty(), "Proof should not be empty");
        assert!(!result.public_vals.is_empty(), "Public values should not be empty");
        assert!(result.merkle_root.is_none(), "Merkle root should be None when use_merkle=false");
        assert!(result.proving_time_ms > 0, "Should have non-zero proving time");
        assert!(result.verify_time_ms > 0, "Should have non-zero verify time");
        
        println!("✓ Chunk proof generated and verified successfully");
        println!("  Proof size: {} bytes", result.proof.len());
        println!("  Public values: {}", result.public_vals.len());
        println!("  Proving time: {}ms", result.proving_time_ms);
        println!("  Verify time: {}ms", result.verify_time_ms);
    }
}

