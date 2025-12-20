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
    commitments::{
        commit::Commit,
        poseidon_commit::{PoseidonCommitChip, WIDTH, RATE, L},
    },
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

    /// Build a proper binary Merkle tree from a list of values
    /// This builds a binary tree level by level:
    /// - Level 0: Hash each value individually (leaves)
    /// - Level 1: Hash pairs of level 0 hashes
    /// - Level 2: Hash pairs of level 1 hashes
    /// - Continue until we have a single root
    pub fn build_binary_tree(
        &self,
        mut layouter: impl Layouter<F>,
        gadget_config: Rc<GadgetConfig>,
        constants: &HashMap<i64, CellRc<F>>,
        values: &[CellRc<F>],
    ) -> Result<CellRc<F>, Error> {
        if values.is_empty() {
            return Err(Error::Synthesis);
        }

        let zero = constants.get(&0).unwrap().clone();
        
        // If only one value, hash it directly
        if values.len() == 1 {
            return self.hash_single(
                layouter.namespace(|| "single leaf"),
                gadget_config,
                constants,
                values[0].clone(),
            );
        }

        // Step 1: Hash all leaves (level 0)
        let mut current_level = Vec::new();
        for (i, value) in values.iter().enumerate() {
            let leaf_hash = self.hash_single(
                layouter.namespace(|| format!("leaf {}", i)),
                gadget_config.clone(),
                constants,
                value.clone(),
            )?;
            current_level.push(leaf_hash);
        }

        // Step 2: Build tree level by level
        let mut level = 0;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            // Hash pairs
            for i in 0..(current_level.len() / 2) {
                let left = current_level[i * 2].clone();
                let right = current_level[i * 2 + 1].clone();
                
                // Hash pair: [left, right]
                let pair_hash = self.poseidon.commit(
                    layouter.namespace(|| format!("level {} pair {}", level, i)),
                    gadget_config.clone(),
                    constants,
                    &vec![left, right],
                    zero.clone(),
                )?;
                next_level.push(pair_hash[0].clone());
            }
            
            // If odd number, hash the last element with itself
            if current_level.len() % 2 == 1 {
                let last = current_level.last().unwrap().clone();
                let pair_hash = self.poseidon.commit(
                    layouter.namespace(|| format!("level {} last", level)),
                    gadget_config.clone(),
                    constants,
                    &vec![last.clone(), last],
                    zero.clone(),
                )?;
                next_level.push(pair_hash[0].clone());
            }
            
            current_level = next_level;
            level += 1;
        }

        // Root is the single remaining element
        Ok(current_level[0].clone())
    }

    /// Build a simple Merkle tree from a list of values
    /// For simplicity, we'll just hash all values together
    /// In production, you'd build a proper binary tree
    /// NOTE: This is kept for backward compatibility, but build_binary_tree is preferred
    pub fn build_simple_tree(
        &self,
        mut layouter: impl Layouter<F>,
        gadget_config: Rc<GadgetConfig>,
        constants: &HashMap<i64, CellRc<F>>,
        values: &[CellRc<F>],
    ) -> Result<CellRc<F>, Error> {
        // Use binary tree implementation
        self.build_binary_tree(layouter, gadget_config, constants, values)
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
        layouter.assign_region(
            || "constrain merkle root equal",
            |mut region| {
                region.constrain_equal(computed_root.cell(), expected_root.cell())?;
                Ok(())
            },
        )?;

        Ok(())
    }
}

