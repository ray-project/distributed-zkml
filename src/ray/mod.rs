//! Ray integration for distributed zkml inference
//!
//! This module provides utilities for running zkml inference and proof generation
//! in a distributed manner using Ray.

pub mod batch;
pub mod shared;
pub mod worker;

pub use batch::batch_inference;
pub use shared::SharedResources;

