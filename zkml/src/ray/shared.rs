//! Shared resources for Ray workers
//!
//! This module manages shared resources like model configurations and proving keys
//! that can be reused across multiple Ray workers.

use std::sync::Arc;
use halo2_proofs::{
    halo2curves::bn256::{Bn256, Fr},
    plonk::ProvingKey,
    poly::kzg::commitment::ParamsKZG,
};
use serde::{Deserialize, Serialize};

use crate::model::ModelCircuit;
use crate::utils::proving_kzg::get_kzg_params;

/// Shared resources that can be reused across Ray workers
#[derive(Clone)]
pub struct SharedResources {
    /// Model circuit (without witness data)
    pub circuit: Arc<ModelCircuit<Fr>>,
    /// Proving parameters
    pub params: Arc<ParamsKZG<Bn256>>,
    /// Proving key (if available)
    pub proving_key: Option<Arc<ProvingKey<Bn256>>>,
    /// Circuit degree
    pub degree: u32,
}

impl SharedResources {
    /// Create shared resources from model configuration
    pub fn from_model_config(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Load model circuit (without input data)
        // Note: This is a placeholder - actual implementation would need to
        // load the model config separately from input data
        let circuit = ModelCircuit::<Fr>::generate_from_file(config_path, config_path)?;
        let degree = circuit.k as u32;
        
        // Load or generate parameters
        let params = Arc::new(get_kzg_params("./params_kzg", degree));
        
        Ok(SharedResources {
            circuit: Arc::new(circuit),
            params,
            proving_key: None,
            degree,
        })
    }
    
    /// Generate and cache the proving key
    pub fn generate_proving_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use halo2_proofs::plonk::{keygen_pk, keygen_vk};
        
        let vk = keygen_vk(&self.params, &*self.circuit)?;
        let pk = keygen_pk(&self.params, vk, &*self.circuit)?;
        
        self.proving_key = Some(Arc::new(pk));
        Ok(())
    }
}

/// Serializable input data for Ray workers
#[derive(Clone, Serialize, Deserialize)]
pub struct InferenceInput {
    /// Path to input msgpack file
    pub input_path: String,
    /// Input index (for tracking)
    pub index: usize,
}

/// Serializable proof result
#[derive(Clone, Serialize, Deserialize)]
pub struct ProofResult {
    /// Proof bytes
    pub proof: Vec<u8>,
    /// Public values
    pub public_vals: Vec<Vec<u8>>,
    /// Input index (for tracking)
    pub index: usize,
    /// Proof generation time in milliseconds
    pub time_ms: u64,
}

