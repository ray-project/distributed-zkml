//! GPU-accelerated MSM using ICICLE
//!
//! This module provides GPU acceleration for Multi-Scalar Multiplication (MSM)
//! operations using the ICICLE library with CUDA backend.

use std::sync::atomic::{AtomicBool, Ordering};

use icicle_bn254::curve::{CurveCfg, G1Projective, ScalarCfg};
use icicle_core::curve::Affine;
use icicle_core::msm::{msm, MSMConfig};
use icicle_core::traits::FieldImpl;
use icicle_runtime::device::Device;
use icicle_runtime::memory::HostSlice;

use crate::halo2curves::bn256::{Fr, G1Affine, G1};
use group::Curve;

/// Global flag indicating if GPU is available and initialized
static GPU_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GPU_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Minimum size for GPU MSM to be beneficial (below this, CPU is faster)
const GPU_MSM_MIN_SIZE: usize = 1 << 10; // 1024 points

/// Initialize ICICLE GPU backend
/// Returns true if GPU is available and initialized successfully
pub fn init_gpu() -> bool {
    // Only initialize once
    if GPU_INITIALIZED.load(Ordering::Relaxed) {
        return GPU_AVAILABLE.load(Ordering::Relaxed);
    }

    // Try to load backend
    if let Err(_) = icicle_runtime::load_backend_from_env_or_default() {
        GPU_INITIALIZED.store(true, Ordering::Relaxed);
        GPU_AVAILABLE.store(false, Ordering::Relaxed);
        return false;
    }

    // Try to set CUDA device
    let cuda_device = Device::new("CUDA", 0);
    let available = icicle_runtime::set_device(&cuda_device).is_ok();

    GPU_INITIALIZED.store(true, Ordering::Relaxed);
    GPU_AVAILABLE.store(available, Ordering::Relaxed);

    if available {
        eprintln!("[GPU] ICICLE CUDA backend initialized successfully");
    }

    available
}

/// Check if GPU is available for MSM
#[inline]
pub fn is_gpu_available() -> bool {
    if !GPU_INITIALIZED.load(Ordering::Relaxed) {
        init_gpu()
    } else {
        GPU_AVAILABLE.load(Ordering::Relaxed)
    }
}

/// Convert halo2curves Fr to ICICLE scalar
#[inline]
fn fr_to_icicle_scalar(fr: &Fr) -> <ScalarCfg as FieldImpl>::Scalar {
    let bytes = fr.to_bytes();
    <ScalarCfg as FieldImpl>::Scalar::from_bytes_le(&bytes)
}

/// Convert halo2curves G1Affine to ICICLE affine point
#[inline]
fn g1_to_icicle_affine(point: &G1Affine) -> Affine<CurveCfg> {
    let x_bytes = point.x.to_bytes();
    let y_bytes = point.y.to_bytes();

    Affine::<CurveCfg>::from_limbs(
        <CurveCfg as icicle_core::curve::Curve>::BaseField::from_bytes_le(&x_bytes).into(),
        <CurveCfg as icicle_core::curve::Curve>::BaseField::from_bytes_le(&y_bytes).into(),
    )
}

/// Convert ICICLE projective point back to halo2curves G1
#[inline]
fn icicle_projective_to_g1(point: &G1Projective) -> G1 {
    let affine = icicle_core::curve::Projective::to_affine(point);

    let x_bytes: [u8; 32] = affine.x.to_bytes_le().try_into().unwrap();
    let y_bytes: [u8; 32] = affine.y.to_bytes_le().try_into().unwrap();

    let x = crate::halo2curves::bn256::Fq::from_bytes(&x_bytes).unwrap();
    let y = crate::halo2curves::bn256::Fq::from_bytes(&y_bytes).unwrap();

    G1Affine::from_xy(x, y).unwrap().into()
}

/// Perform GPU-accelerated MSM
///
/// Returns None if GPU is not available or if the operation fails.
/// The caller should fall back to CPU MSM in that case.
pub fn gpu_msm(scalars: &[Fr], bases: &[G1Affine]) -> Option<G1> {
    if scalars.len() != bases.len() {
        return None;
    }

    // Don't use GPU for small MSMs
    if scalars.len() < GPU_MSM_MIN_SIZE {
        return None;
    }

    // Check GPU availability
    if !is_gpu_available() {
        return None;
    }

    // Convert scalars to ICICLE format
    let icicle_scalars: Vec<_> = scalars.iter().map(fr_to_icicle_scalar).collect();

    // Convert bases to ICICLE format
    let icicle_bases: Vec<_> = bases.iter().map(g1_to_icicle_affine).collect();

    // Perform MSM
    let mut result = vec![G1Projective::zero(); 1];
    let cfg = MSMConfig::default();

    match msm(
        HostSlice::from_slice(&icicle_scalars),
        HostSlice::from_slice(&icicle_bases),
        &cfg,
        HostSlice::from_mut_slice(&mut result),
    ) {
        Ok(_) => Some(icicle_projective_to_g1(&result[0])),
        Err(_) => None,
    }
}
