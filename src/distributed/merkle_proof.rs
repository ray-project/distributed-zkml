//! Merkle proof structure for distributed proving

use zkml::halo2_proofs::halo2curves::ff::PrimeField;

/// Merkle proof for connecting chunks
#[derive(Clone, Debug)]
pub struct MerkleProof {
    /// The Merkle root (public) - serialized as bytes
    pub root: Vec<u8>,
    /// The proof path (for verification) - serialized as bytes
    pub path: Vec<Vec<u8>>,
}

impl MerkleProof {
    pub fn new<F: PrimeField>(root: F) -> Self {
        Self {
            root: root.to_repr().as_ref().to_vec(),
            path: vec![],
        }
    }
}

