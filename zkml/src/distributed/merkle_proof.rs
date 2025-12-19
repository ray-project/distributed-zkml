//! Merkle proof structure for distributed proving

use halo2_proofs::halo2curves::ff::PrimeField;
use serde::{Deserialize, Serialize};

/// Merkle proof for connecting chunks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProof<F: PrimeField> {
    /// The Merkle root (public)
    pub root: Vec<u8>,  // Serialized field element
    /// The proof path (for verification)
    pub path: Vec<Vec<u8>>,
}

impl<F: PrimeField> MerkleProof<F> {
    pub fn new(root: F) -> Self {
        Self {
            root: root.to_repr().as_ref().to_vec(),
            path: vec![],
        }
    }
}

