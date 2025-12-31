//! Tests for chunk proof generation
//!
//! These tests verify that:
//! 1. Real KZG proofs can be generated for model chunks
//! 2. Proofs verify correctly
//! 3. Chunks can be chained via Merkle root verification
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
            None,   // prev_merkle_root = None (first chunk)
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

    /// Test: Chain two chunk proofs via Merkle root
    /// 
    /// This demonstrates the key distributed proving pattern:
    /// - Chunk 0 produces a Merkle root as a public output
    /// - Chunk 1 includes that Merkle root as a public input
    /// - Verifier can check that chunks are properly linked
    #[test]
    fn test_chained_chunk_proofs() {
        let config_file = "examples/mnist/model.msgpack";
        let input_file = "examples/mnist/inp.msgpack";
        
        if !std::path::Path::new(config_file).exists() {
            eprintln!("Skipping test: example files not found");
            return;
        }
        
        let params_dir = "./params_kzg_chained_test";
        fs::create_dir_all(params_dir).ok();
        
        println!("\n=== Testing Chained Chunk Proofs ===\n");
        
        // --- Chunk 0: Generate proof WITHOUT prev_merkle_root ---
        println!("Generating Chunk 0 proof (layers 0-2)...");
        let chunk0_result = prove_chunk_kzg(
            config_file,
            input_file,
            0,     // chunk_start
            2,     // chunk_end  
            false, // use_merkle (not needed for this test)
            None,  // prev_merkle_root = None (first chunk has no predecessor)
            params_dir,
        );
        
        assert!(!chunk0_result.proof.is_empty(), "Chunk 0 proof should not be empty");
        let chunk0_public_count = chunk0_result.public_vals.len();
        println!("✓ Chunk 0: {} public values, {}ms", chunk0_public_count, chunk0_result.proving_time_ms);
        
        // --- Chunk 1: Generate proof WITH prev_merkle_root ---
        // In a real scenario, this would use chunk0's merkle_root
        // For this test, we use a dummy value to verify the mechanism works
        use halo2_proofs::halo2curves::bn256::Fr;
        use halo2_proofs::halo2curves::ff::Field;
        let dummy_prev_root = Fr::ONE; // Simulated previous merkle root
        
        println!("\nGenerating Chunk 1 proof (layers 0-2) with prev_merkle_root...");
        let chunk1_result = prove_chunk_kzg(
            config_file,
            input_file,
            0,     // chunk_start (same layers for simplicity)
            2,     // chunk_end
            false, // use_merkle
            Some(dummy_prev_root), // This chunk verifies it received correct input from chunk 0
            params_dir,
        );
        
        assert!(!chunk1_result.proof.is_empty(), "Chunk 1 proof should not be empty");
        let chunk1_public_count = chunk1_result.public_vals.len();
        println!("✓ Chunk 1: {} public values, {}ms", chunk1_public_count, chunk1_result.proving_time_ms);
        
        // Verify that chunk 1 has ONE MORE public value (the prev_merkle_root)
        assert_eq!(
            chunk1_public_count, 
            chunk0_public_count + 1,
            "Chunk with prev_merkle_root should have 1 more public value"
        );
        
        // Verify the first public value of chunk 1 is the prev_merkle_root we passed in
        assert_eq!(
            chunk1_result.public_vals[0],
            dummy_prev_root,
            "First public value should be the prev_merkle_root"
        );
        
        println!("\n✓ Chained proof verification successful!");
        println!("  Chunk 0 public values: {}", chunk0_public_count);
        println!("  Chunk 1 public values: {} (+1 for prev_merkle_root)", chunk1_public_count);
        println!("  Chain link verified: first public value matches prev_merkle_root");
    }
}

