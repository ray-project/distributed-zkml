//! Simple Merkle tree implementation for distributed proving
//! 
//! This is a minimal implementation for demonstrating the concept.
//! For production, you'd want a more optimized version.

use std::{collections::HashMap, rc::Rc};
use halo2_proofs::{
    circuit::Layouter,
    halo2curves::ff::{FromUniformBytes, PrimeField},
    plonk::Error,
};

use crate::{
    commitments::poseidon_commit::{PoseidonCommitChip, WIDTH, RATE, L},
    gadgets::gadget::GadgetConfig,
    layers::layer::CellRc,
};

/// Simple Merkle tree chip for hashing intermediate values
pub struct MerkleTreeChip<F: PrimeField + Ord + FromUniformBytes<64>> {
    poseidon: PoseidonCommitChip<F, WIDTH, RATE, L>,
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField + Ord + FromUniformBytes<64>> MerkleTreeChip<F> {
    /// Create a new Merkle tree chip
    pub fn new(poseidon: PoseidonCommitChip<F, WIDTH, RATE, L>) -> Self {
        Self {
            poseidon,
            _marker: std::marker::PhantomData,
        }
    }

    /// Hash a single value using Poseidon
    /// This is a simplified version - in production you'd want to handle multiple values
    pub fn hash_single(
        &self,
        mut layouter: impl Layouter<F>,
        gadget_config: Rc<GadgetConfig>,
        constants: &HashMap<i64, CellRc<F>>,
        value: CellRc<F>,
    ) -> Result<CellRc<F>, Error> {
        let zero = constants.get(&0).unwrap().clone();
        let hash = self.poseidon.commit(
            layouter.namespace(|| "hash single"),
            gadget_config,
            constants,
            &vec![value],
            zero,
        )?;
        Ok(hash[0].clone())
    }

    /// Build a simple Merkle tree from a list of values
    /// For simplicity, we'll just hash all values together
    /// In production, you'd build a proper binary tree
    pub fn build_simple_tree(
        &self,
        mut layouter: impl Layouter<F>,
        gadget_config: Rc<GadgetConfig>,
        constants: &HashMap<i64, CellRc<F>>,
        values: &[CellRc<F>],
    ) -> Result<CellRc<F>, Error> {
        // For minimal example: just hash all values together
        // In production, build proper binary Merkle tree
        let zero = constants.get(&0).unwrap().clone();
        let root = self.poseidon.commit(
            layouter.namespace(|| "merkle root"),
            gadget_config,
            constants,
            &values.to_vec(),
            zero,
        )?;
        Ok(root[0].clone())
    }

    /// Verify that values hash to a given root
    /// This proves the values are consistent without revealing them
    pub fn verify_root(
        &self,
        mut layouter: impl Layouter<F>,
        gadget_config: Rc<GadgetConfig>,
        constants: &HashMap<i64, CellRc<F>>,
        values: &[CellRc<F>],
        expected_root: CellRc<F>,
    ) -> Result<(), Error> {
        // Compute root from values
        let computed_root = self.build_simple_tree(
            layouter.namespace(|| "verify root"),
            gadget_config.clone(),
            constants,
            values,
        )?;

        // Constrain: computed_root == expected_root
        // This proves values hash to expected root without revealing them
        layouter.constrain_equal(computed_root.cell(), expected_root.cell())?;

        Ok(())
    }
}

