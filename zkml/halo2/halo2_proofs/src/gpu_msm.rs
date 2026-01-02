//! GPU-accelerated MSM using ICICLE
//!
//! This module provides GPU acceleration for Multi-Scalar Multiplication (MSM)
//! operations using the ICICLE library with CUDA backend.

use std::{
    env,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use icicle_bn254::curve::{
    G1Affine as IcicleAffine, 
    G1Projective as IcicleProjective,
    ScalarField as IcicleScalar,
    BaseField as IcicleBase,
};
use icicle_core::msm::{msm, MSMConfig};
use icicle_core::traits::FieldImpl;
use icicle_runtime::device::Device;
use icicle_runtime::memory::HostSlice;

use crate::halo2curves::bn256::{Fq, Fr, G1Affine, G1};
use crate::halo2curves::CurveAffine;
use group::ff::{Field, PrimeField};
use group::prime::PrimeCurveAffine;
use group::{Curve, Group};

/// Global flag indicating if GPU is available and initialized
static GPU_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GPU_AVAILABLE: AtomicBool = AtomicBool::new(false);
static GPU_MSM_CALLS: AtomicUsize = AtomicUsize::new(0);

/// Minimum size for GPU MSM to be beneficial (below this, CPU is faster)
pub const GPU_MSM_MIN_SIZE: usize = 1 << 10; // 1024 points

/// Number of successful GPU MSM calls (for tests/benchmarks).
pub fn gpu_msm_call_count() -> usize {
    GPU_MSM_CALLS.load(Ordering::Relaxed)
}

/// Reset the GPU MSM call counter (for tests).
pub fn reset_gpu_msm_call_count() {
    GPU_MSM_CALLS.store(0, Ordering::Relaxed);
}

/// Initialize ICICLE GPU backend
/// Returns true if GPU is available and initialized successfully
pub fn init_gpu() -> bool {
    // Only initialize once
    if GPU_INITIALIZED.load(Ordering::Relaxed) {
        return GPU_AVAILABLE.load(Ordering::Relaxed);
    }

    // Try to load backend
    if icicle_runtime::load_backend_from_env_or_default().is_err() {
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
fn fr_to_icicle(fr: &Fr) -> IcicleScalar {
    let repr = fr.to_repr();
    IcicleScalar::from_bytes_le(repr.as_ref())
}

/// Convert halo2curves Fq to ICICLE base field
#[inline]
fn fq_to_icicle(fq: &Fq) -> IcicleBase {
    let repr = fq.to_repr();
    IcicleBase::from_bytes_le(repr.as_ref())
}

/// Convert halo2curves G1Affine to ICICLE affine point
#[inline]
fn g1_to_icicle(point: &G1Affine) -> IcicleAffine {
    if bool::from(point.is_identity()) {
        return IcicleAffine::zero();
    }
    IcicleAffine {
        x: fq_to_icicle(&point.x),
        y: fq_to_icicle(&point.y),
    }
}

/// Convert ICICLE projective point back to halo2curves G1
fn icicle_to_g1(point: &IcicleProjective) -> G1 {
    if *point == IcicleProjective::zero() {
        return G1::identity();
    }
    
    // Get bytes from ICICLE coordinates
    let x_bytes = point.x.to_bytes_le();
    let y_bytes = point.y.to_bytes_le();
    let z_bytes = point.z.to_bytes_le();
    
    // Convert to halo2curves Fq - need to handle the 32-byte array
    let x_arr: [u8; 32] = x_bytes.as_slice().try_into().expect("x bytes");
    let y_arr: [u8; 32] = y_bytes.as_slice().try_into().expect("y bytes");  
    let z_arr: [u8; 32] = z_bytes.as_slice().try_into().expect("z bytes");
    
    let x = Fq::from_repr(x_arr).expect("valid x");
    let y = Fq::from_repr(y_arr).expect("valid y");
    let z = Fq::from_repr(z_arr).expect("valid z");
    
    // ICICLE uses standard projective: (X:Y:Z) where x=X/Z, y=Y/Z
    // Convert to affine: x = X/Z, y = Y/Z
    let z_inv = z.invert().expect("non-zero z");
    let affine_x = x * z_inv;
    let affine_y = y * z_inv;
    
    // Create affine point then convert to projective
    let affine = G1Affine::from_xy(affine_x, affine_y).expect("valid affine point");
    affine.to_curve()
}

/// Perform GPU-accelerated MSM
///
/// Returns None if GPU is not available or if the operation fails.
/// The caller should fall back to CPU MSM in that case.
pub fn gpu_msm(scalars: &[Fr], bases: &[G1Affine]) -> Option<G1> {
    if scalars.len() != bases.len() {
        return None;
    }

    // Don't use GPU for small MSMs unless explicitly forced.
    if scalars.len() < GPU_MSM_MIN_SIZE && env::var_os("HALO2_FORCE_GPU_MSM").is_none() {
        return None;
    }

    // Check GPU availability
    if !is_gpu_available() {
        return None;
    }

    // Convert scalars to ICICLE format
    let icicle_scalars: Vec<IcicleScalar> = scalars.iter().map(fr_to_icicle).collect();

    // Convert bases to ICICLE format  
    let icicle_bases: Vec<IcicleAffine> = bases.iter().map(g1_to_icicle).collect();

    // Perform MSM
    let mut result = vec![IcicleProjective::zero(); 1];
    let cfg = MSMConfig::default();

    match msm(
        HostSlice::from_slice(&icicle_scalars),
        HostSlice::from_slice(&icicle_bases),
        &cfg,
        HostSlice::from_mut_slice(&mut result),
    ) {
        Ok(_) => {
            GPU_MSM_CALLS.fetch_add(1, Ordering::Relaxed);
            Some(icicle_to_g1(&result[0]))
        }
        Err(_) => None,
    }
}
