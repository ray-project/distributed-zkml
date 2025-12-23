//! Ray worker functions for distributed inference
//!
//! These functions run on Ray workers to perform inference and proof generation.

use std::time::Instant;
use halo2_proofs::{
    halo2curves::bn256::Fr,
    plonk::{create_proof, ProvingKey},
    poly::kzg::commitment::ParamsKZG,
    transcript::{Blake2bWrite, Challenge255},
};
use rand::thread_rng;

use zkml::{
    model::ModelCircuit,
    utils::helpers::get_public_values,
};

use super::shared::{InferenceInput, ProofResult, SharedResources};

/// Generate a proof for a single input
///
/// This function is designed to be called on Ray workers.
/// It loads the input, creates a circuit with witness data, and generates a proof.
pub fn generate_proof_worker(
    shared: &SharedResources,
    input: InferenceInput,
) -> Result<ProofResult, Box<dyn std::error::Error>> {
    let start = Instant::now();
    
    // Load the input data and create circuit with witness
    let circuit = ModelCircuit::<Fr>::generate_from_file(
        "model.msgpack", // This would need to be passed or loaded differently
        &input.input_path,
    );
    
    // Get proving key (should be pre-generated and shared)
    let pk = shared.proving_key.as_ref()
        .ok_or("Proving key not generated")?;
    
    // Get public values (this is set during circuit synthesis)
    let public_vals = get_public_values();
    
    // Generate proof
    let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    create_proof::<
        _,
        _,
        _,
        _,
        Blake2bWrite<Vec<u8>, _, _>,
        ModelCircuit<Fr>,
    >(
        &shared.params,
        pk,
        &[circuit],
        &[&[&public_vals]],
        thread_rng(),
        &mut transcript,
    )?;
    
    let proof = transcript.finalize();
    
    // Serialize public values
    let public_vals_bytes: Vec<Vec<u8>> = public_vals
        .iter()
        .map(|v| v.to_bytes().to_vec())
        .collect();
    
    let time_ms = start.elapsed().as_millis() as u64;
    
    Ok(ProofResult {
        proof,
        public_vals: public_vals_bytes,
        index: input.index,
        time_ms,
    })
}

/// Simplified version that doesn't require Ray (for testing)
pub fn generate_proof_local(
    model_config: &str,
    input_path: &str,
    backend: &str,
) -> Result<ProofResult, Box<dyn std::error::Error>> {
    use zkml::utils::proving_kzg::time_circuit_kzg;
    use zkml::utils::proving_ipa::time_circuit_ipa;
    use halo2_proofs::halo2curves::pasta::Fp;
    
    let start = Instant::now();
    
    // This is a simplified version - in practice you'd want to
    // separate proof generation from timing
    if backend == "kzg" {
        let circuit = ModelCircuit::<Fr>::generate_from_file(model_config, input_path);
        // Note: time_circuit_kzg prints timing info but doesn't return proof
        // For actual implementation, we'd need a version that returns the proof
        time_circuit_kzg(circuit);
    } else {
        let circuit = ModelCircuit::<Fp>::generate_from_file(model_config, input_path);
        time_circuit_ipa(circuit);
    }
    
    // In a real implementation, we'd capture the proof here
    // For now, this is a placeholder
    Ok(ProofResult {
        proof: vec![], // Would contain actual proof
        public_vals: vec![], // Would contain actual public values
        index: 0,
        time_ms: start.elapsed().as_millis() as u64,
    })
}

