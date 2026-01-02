//! GPU-accelerated NTT (FFT over a finite field) using ICICLE.
//!
//! Halo2 uses FFTs over finite fields for polynomial operations (evaluation/interpolation).
//! Over a finite field, this is commonly called an NTT (Number Theoretic Transform).
//!
//! This module provides an opt-in ICICLE-powered in-place NTT for BN256 scalar field (`Fr`).
//! Enable via `--features gpu` and set `HALO2_USE_GPU_NTT=1`.

use std::{
    env,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Once,
    },
};

use icicle_bn254::curve::ScalarField as IcicleScalar;
use icicle_core::ntt::{initialize_domain, ntt_inplace, NTTConfig, NTTDir, NTTInitDomainConfig};
use icicle_core::traits::FieldImpl;
use icicle_runtime::device::Device;
use icicle_runtime::memory::HostSlice;

use crate::halo2curves::bn256::Fr;
use group::ff::PrimeField;

static NTT_INIT: Once = Once::new();
static NTT_READY: AtomicBool = AtomicBool::new(false);
static NTT_MAX_SIZE: AtomicUsize = AtomicUsize::new(0);
static NTT_CALLS: AtomicUsize = AtomicUsize::new(0);

/// Number of successful GPU NTT calls (for tests/benchmarks).
pub fn gpu_ntt_call_count() -> usize {
    NTT_CALLS.load(Ordering::Relaxed)
}

/// Reset the GPU NTT call counter.
pub fn reset_gpu_ntt_call_count() {
    NTT_CALLS.store(0, Ordering::Relaxed);
}

fn enabled() -> bool {
    env::var("HALO2_USE_GPU_NTT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[inline]
fn fr_to_icicle(fr: &Fr) -> IcicleScalar {
    let repr = fr.to_repr();
    IcicleScalar::from_bytes_le(repr.as_ref())
}

#[inline]
fn icicle_to_fr(s: &IcicleScalar) -> Fr {
    let bytes = s.to_bytes_le();
    let arr: [u8; 32] = bytes.as_slice().try_into().expect("Fr bytes");
    Fr::from_repr(arr).expect("valid Fr")
}

fn ensure_initialized(max_size: usize, omega: Fr) -> bool {
    if !enabled() {
        return false;
    }

    // Initialize once; assumes the first max_size is large enough for subsequent calls.
    NTT_INIT.call_once(|| {
        if icicle_runtime::load_backend_from_env_or_default().is_err() {
            NTT_READY.store(false, Ordering::Relaxed);
            return;
        }

        let cuda_device = Device::new("CUDA", 0);
        if icicle_runtime::set_device(&cuda_device).is_err() {
            NTT_READY.store(false, Ordering::Relaxed);
            return;
        }

        // Initialize domain twiddles for this size using Halo2's omega.
        let init_cfg = NTTInitDomainConfig::default();
        let rou = fr_to_icicle(&omega);
        if initialize_domain::<IcicleScalar>(rou, &init_cfg).is_err() {
            NTT_READY.store(false, Ordering::Relaxed);
            return;
        }

        NTT_MAX_SIZE.store(max_size, Ordering::Relaxed);
        NTT_READY.store(true, Ordering::Relaxed);
    });

    NTT_READY.load(Ordering::Relaxed) && NTT_MAX_SIZE.load(Ordering::Relaxed) >= max_size
}

/// Try to run an in-place GPU NTT over BN256 `Fr` values.
///
/// Returns `true` if the GPU path ran successfully, `false` to indicate the caller
/// should fall back to the CPU FFT implementation.
pub fn try_gpu_ntt_inplace(values: &mut [Fr], omega: Fr) -> bool {
    if values.is_empty() {
        return true;
    }

    // Halo2 uses power-of-two FFT sizes.
    if !values.len().is_power_of_two() {
        return false;
    }

    // Heuristic: avoid paying conversion overhead on tiny transforms unless forced.
    if values.len() < (1 << 16) && env::var_os("HALO2_FORCE_GPU_NTT").is_none() {
        return false;
    }

    if !ensure_initialized(values.len(), omega) {
        return false;
    }

    let mut icicle_vals: Vec<IcicleScalar> = values.iter().map(fr_to_icicle).collect();

    // Halo2 encodes direction by passing either omega or omega^{-1}.
    // We match Halo2 by always running a forward transform using the provided omega.
    let dir = NTTDir::kForward;
    let cfg: NTTConfig<IcicleScalar> = NTTConfig::default();

    if ntt_inplace::<IcicleScalar, IcicleScalar>(HostSlice::from_mut_slice(&mut icicle_vals), dir, &cfg).is_err() {
        return false;
    }

    for (dst, src) in values.iter_mut().zip(icicle_vals.iter()) {
        *dst = icicle_to_fr(src);
    }

    NTT_CALLS.fetch_add(1, Ordering::Relaxed);
    true
}

