//! This module provides common utilities, traits and structures for group,
//! field and polynomial arithmetic.

use super::multicore;
pub use ff::Field;
use group::{
    ff::{BatchInvert, PrimeField},
    Curve, Group, GroupOpsOwned, ScalarMulOwned,
};

pub use halo2curves::{CurveAffine, CurveExt};

use crate::{
    fft::{
        self, parallel,
        recursive::{self, FFTData},
    },
    plonk::{get_duration, get_time, log_info},
    poly::EvaluationDomain,
};
use std::{
    any::TypeId,
    env,
    mem,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Once,
    },
};

/// This represents an element of a group with basic operations that can be
/// performed. This allows an FFT implementation (for example) to operate
/// generically over either a field or elliptic curve group.
pub trait FftGroup<Scalar: Field>:
    Copy + Send + Sync + 'static + GroupOpsOwned + ScalarMulOwned<Scalar>
{
}

impl<T, Scalar> FftGroup<Scalar> for T
where
    Scalar: Field,
    T: Copy + Send + Sync + 'static + GroupOpsOwned + ScalarMulOwned<Scalar>,
{
}

/// TEMP
pub static mut MULTIEXP_TOTAL_TIME: usize = 0;

// -----------------------------------------------------------------------------
// Optional FFT stats instrumentation (enabled via HALO2_FFT_STATS=1)
// -----------------------------------------------------------------------------
static FFT_STATS_INIT: Once = Once::new();
static FFT_STATS_ENABLED: AtomicBool = AtomicBool::new(false);

static FFT_TOTAL_US: AtomicUsize = AtomicUsize::new(0);
static FFT_CALLS_TOTAL: AtomicUsize = AtomicUsize::new(0);
static FFT_CALLS_FIELD: AtomicUsize = AtomicUsize::new(0);
static FFT_CALLS_GROUP: AtomicUsize = AtomicUsize::new(0);

fn fft_stats_enabled() -> bool {
    FFT_STATS_INIT.call_once(|| {
        let enabled = env::var("HALO2_FFT_STATS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        FFT_STATS_ENABLED.store(enabled, Ordering::Relaxed);
    });
    FFT_STATS_ENABLED.load(Ordering::Relaxed)
}

/// Simple FFT stats (microseconds and call counts).
#[derive(Clone, Copy, Debug)]
pub struct FftStats {
    /// Total time spent in `best_fft` (microseconds).
    pub total_us: usize,
    /// Total number of `best_fft` calls.
    pub calls_total: usize,
    /// Number of `best_fft` calls where the FFT was over field elements (G == Scalar).
    pub calls_field: usize,
    /// Number of `best_fft` calls where the FFT was over group elements (G != Scalar).
    pub calls_group: usize,
}

/// Read current FFT stats counters.
pub fn fft_stats() -> FftStats {
    FftStats {
        total_us: FFT_TOTAL_US.load(Ordering::Relaxed),
        calls_total: FFT_CALLS_TOTAL.load(Ordering::Relaxed),
        calls_field: FFT_CALLS_FIELD.load(Ordering::Relaxed),
        calls_group: FFT_CALLS_GROUP.load(Ordering::Relaxed),
    }
}

/// Reset FFT stats counters.
pub fn reset_fft_stats() {
    FFT_TOTAL_US.store(0, Ordering::Relaxed);
    FFT_CALLS_TOTAL.store(0, Ordering::Relaxed);
    FFT_CALLS_FIELD.store(0, Ordering::Relaxed);
    FFT_CALLS_GROUP.store(0, Ordering::Relaxed);
}

fn multiexp_serial<C: CurveAffine>(coeffs: &[C::Scalar], bases: &[C], acc: &mut C::Curve) {
    let coeffs: Vec<_> = coeffs.iter().map(|a| a.to_repr()).collect();

    let c = if bases.len() < 4 {
        1
    } else if bases.len() < 32 {
        3
    } else {
        (f64::from(bases.len() as u32)).ln().ceil() as usize
    };

    fn get_at<F: PrimeField>(segment: usize, c: usize, bytes: &F::Repr) -> usize {
        let skip_bits = segment * c;
        let skip_bytes = skip_bits / 8;

        if skip_bytes >= 32 {
            return 0;
        }

        let mut v = [0; 8];
        for (v, o) in v.iter_mut().zip(bytes.as_ref()[skip_bytes..].iter()) {
            *v = *o;
        }

        let mut tmp = u64::from_le_bytes(v);
        tmp >>= skip_bits - (skip_bytes * 8);
        tmp = tmp % (1 << c);

        tmp as usize
    }

    let segments = (256 / c) + 1;

    for current_segment in (0..segments).rev() {
        for _ in 0..c {
            *acc = acc.double();
        }

        #[derive(Clone, Copy)]
        enum Bucket<C: CurveAffine> {
            None,
            Affine(C),
            Projective(C::Curve),
        }

        impl<C: CurveAffine> Bucket<C> {
            fn add_assign(&mut self, other: &C) {
                *self = match *self {
                    Bucket::None => Bucket::Affine(*other),
                    Bucket::Affine(a) => Bucket::Projective(a + *other),
                    Bucket::Projective(mut a) => {
                        a += *other;
                        Bucket::Projective(a)
                    }
                }
            }

            fn add(self, mut other: C::Curve) -> C::Curve {
                match self {
                    Bucket::None => other,
                    Bucket::Affine(a) => {
                        other += a;
                        other
                    }
                    Bucket::Projective(a) => other + &a,
                }
            }
        }

        let mut buckets: Vec<Bucket<C>> = vec![Bucket::None; (1 << c) - 1];

        for (coeff, base) in coeffs.iter().zip(bases.iter()) {
            let coeff = get_at::<C::Scalar>(current_segment, c, coeff);
            if coeff != 0 {
                buckets[coeff - 1].add_assign(base);
            }
        }

        // Summation by parts
        // e.g. 3a + 2b + 1c = a +
        //                    (a) + b +
        //                    ((a) + b) + c
        let mut running_sum = C::Curve::identity();
        for exp in buckets.into_iter().rev() {
            running_sum = exp.add(running_sum);
            *acc = *acc + &running_sum;
        }
    }
}

/// Performs a small multi-exponentiation operation.
/// Uses the double-and-add algorithm with doublings shared across points.
pub fn small_multiexp<C: CurveAffine>(coeffs: &[C::Scalar], bases: &[C]) -> C::Curve {
    let coeffs: Vec<_> = coeffs.iter().map(|a| a.to_repr()).collect();
    let mut acc = C::Curve::identity();

    // for byte idx
    for byte_idx in (0..32).rev() {
        // for bit idx
        for bit_idx in (0..8).rev() {
            acc = acc.double();
            // for each coeff
            for coeff_idx in 0..coeffs.len() {
                let byte = coeffs[coeff_idx].as_ref()[byte_idx];
                if ((byte >> bit_idx) & 1) != 0 {
                    acc += bases[coeff_idx];
                }
            }
        }
    }

    acc
}

/// Performs a multi-exponentiation operation.
///
/// This function will panic if coeffs and bases have a different length.
///
/// This will use multithreading if beneficial.
fn best_multiexp_cpu<C: CurveAffine>(coeffs: &[C::Scalar], bases: &[C]) -> C::Curve {
    assert_eq!(coeffs.len(), bases.len());

    log_info(format!("msm: {}", coeffs.len()));

    let start = get_time();
    let num_threads = multicore::current_num_threads();
    let res = if coeffs.len() > num_threads {
        let chunk = coeffs.len() / num_threads;
        let num_chunks = coeffs.chunks(chunk).len();
        let mut results = vec![C::Curve::identity(); num_chunks];
        multicore::scope(|scope| {
            let chunk = coeffs.len() / num_threads;

            for ((coeffs, bases), acc) in coeffs
                .chunks(chunk)
                .zip(bases.chunks(chunk))
                .zip(results.iter_mut())
            {
                scope.spawn(move |_| {
                    multiexp_serial(coeffs, bases, acc);
                });
            }
        });
        results.iter().fold(C::Curve::identity(), |a, b| a + b)
    } else {
        let mut acc = C::Curve::identity();
        multiexp_serial(coeffs, bases, &mut acc);
        acc
    };

    let duration = get_duration(start);
    #[allow(unsafe_code)]
    unsafe {
        crate::arithmetic::MULTIEXP_TOTAL_TIME += duration;
    }

    res
}

/// Performs a multi-exponentiation operation.
///
/// When `--features gpu` is enabled, BN256/G1Affine MSMs are dispatched to ICICLE (CUDA)
/// using safe specialization. All other curves fall back to the CPU implementation.
#[cfg(feature = "gpu")]
pub fn best_multiexp<C: CurveAffine>(coeffs: &[C::Scalar], bases: &[C]) -> C::Curve {
    <() as MultiexpBackend<C>>::multiexp(coeffs, bases)
}

/// Performs a multi-exponentiation operation (CPU-only build).
#[cfg(not(feature = "gpu"))]
pub fn best_multiexp<C: CurveAffine>(coeffs: &[C::Scalar], bases: &[C]) -> C::Curve {
    best_multiexp_cpu(coeffs, bases)
}

// -----------------------------------------------------------------------------
// Optional GPU MSM dispatch plumbing (BN256 specialization)
// -----------------------------------------------------------------------------
#[cfg(feature = "gpu")]
trait MultiexpBackend<C: CurveAffine> {
    fn multiexp(coeffs: &[C::Scalar], bases: &[C]) -> C::Curve;
}

#[cfg(feature = "gpu")]
impl<C: CurveAffine> MultiexpBackend<C> for () {
    default fn multiexp(coeffs: &[C::Scalar], bases: &[C]) -> C::Curve {
        best_multiexp_cpu(coeffs, bases)
    }
}

#[cfg(feature = "gpu")]
impl MultiexpBackend<crate::halo2curves::bn256::G1Affine> for () {
    fn multiexp(
        coeffs: &[crate::halo2curves::bn256::Fr],
        bases: &[crate::halo2curves::bn256::G1Affine],
    ) -> crate::halo2curves::bn256::G1 {
        if let Some(res) = crate::gpu_msm::gpu_msm(coeffs, bases) {
            return res;
        }
        best_multiexp_cpu(coeffs, bases)
    }
}

/// Dispatcher
pub fn best_fft<Scalar: Field + 'static, G: FftGroup<Scalar> + 'static>(
    a: &mut [G],
    omega: Scalar,
    log_n: u32,
    data: &FFTData<Scalar>,
    inverse: bool,
) {
    let stats = fft_stats_enabled();
    let start = if stats { Some(get_time()) } else { None };

    #[cfg(feature = "gpu")]
    {
        <() as FftBackend<Scalar, G>>::fft(a, omega, log_n, data, inverse);
    }
    #[cfg(not(feature = "gpu"))]
    {
        fft::fft(a, omega, log_n, data, inverse);
    }

    if let Some(start) = start {
        let dur = get_duration(start);
        FFT_TOTAL_US.fetch_add(dur, Ordering::Relaxed);
        FFT_CALLS_TOTAL.fetch_add(1, Ordering::Relaxed);

        let is_field_fft = TypeId::of::<G>() == TypeId::of::<Scalar>();
        if is_field_fft {
            FFT_CALLS_FIELD.fetch_add(1, Ordering::Relaxed);
        } else {
            FFT_CALLS_GROUP.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// -----------------------------------------------------------------------------
// Optional GPU FFT/NTT dispatch plumbing (BN256 Fr specialization)
// -----------------------------------------------------------------------------
#[cfg(feature = "gpu")]
trait FftBackend<Scalar: Field + 'static, G: FftGroup<Scalar> + 'static> {
    fn fft(a: &mut [G], omega: Scalar, log_n: u32, data: &FFTData<Scalar>, inverse: bool);
}

#[cfg(feature = "gpu")]
impl<Scalar: Field + 'static, G: FftGroup<Scalar> + 'static> FftBackend<Scalar, G> for () {
    default fn fft(a: &mut [G], omega: Scalar, log_n: u32, data: &FFTData<Scalar>, inverse: bool) {
        fft::fft(a, omega, log_n, data, inverse);
    }
}

#[cfg(feature = "gpu")]
impl FftBackend<crate::halo2curves::bn256::Fr, crate::halo2curves::bn256::Fr> for () {
    fn fft(
        a: &mut [crate::halo2curves::bn256::Fr],
        omega: crate::halo2curves::bn256::Fr,
        log_n: u32,
        data: &FFTData<crate::halo2curves::bn256::Fr>,
        inverse: bool,
    ) {
        // Opt-in: only use GPU NTT when explicitly enabled.
        if crate::gpu_ntt::try_gpu_ntt_inplace(a, omega) {
            return;
        }
        fft::fft(a, omega, log_n, data, inverse);
    }
}

/// Convert coefficient bases group elements to lagrange basis by inverse FFT.
pub fn g_to_lagrange<C: CurveAffine>(g_projective: Vec<C::Curve>, k: u32) -> Vec<C> {
    let n_inv = C::Scalar::TWO_INV.pow_vartime(&[k as u64, 0, 0, 0]);
    let omega = C::Scalar::ROOT_OF_UNITY;
    let mut omega_inv = C::Scalar::ROOT_OF_UNITY_INV;
    for _ in k..C::Scalar::S {
        omega_inv = omega_inv.square();
    }

    let mut g_lagrange_projective = g_projective;
    let n = g_lagrange_projective.len();
    let fft_data = FFTData::new(n, omega, omega_inv);

    best_fft(
        &mut g_lagrange_projective,
        omega_inv,
        k,
        &fft_data,
        false,
    );
    parallelize(&mut g_lagrange_projective, |g, _| {
        for g in g.iter_mut() {
            *g *= n_inv;
        }
    });

    let mut g_lagrange = vec![C::identity(); 1 << k];
    parallelize(&mut g_lagrange, |g_lagrange, starts| {
        C::Curve::batch_normalize(
            &g_lagrange_projective[starts..(starts + g_lagrange.len())],
            g_lagrange,
        );
    });

    g_lagrange
}

/// This evaluates a provided polynomial (in coefficient form) at `point`.
pub fn eval_polynomial<F: Field>(poly: &[F], point: F) -> F {
    fn evaluate<F: Field>(poly: &[F], point: F) -> F {
        poly.iter()
            .rev()
            .fold(F::ZERO, |acc, coeff| acc * point + coeff)
    }
    let n = poly.len();
    let num_threads = multicore::current_num_threads();
    if n * 2 < num_threads {
        evaluate(poly, point)
    } else {
        let chunk_size = (n + num_threads - 1) / num_threads;
        let mut parts = vec![F::ZERO; num_threads];
        multicore::scope(|scope| {
            for (chunk_idx, (out, poly)) in
                parts.chunks_mut(1).zip(poly.chunks(chunk_size)).enumerate()
            {
                scope.spawn(move |_| {
                    let start = chunk_idx * chunk_size;
                    out[0] = evaluate(poly, point) * point.pow_vartime(&[start as u64, 0, 0, 0]);
                });
            }
        });
        parts.iter().fold(F::ZERO, |acc, coeff| acc + coeff)
    }
}

/// This computes the inner product of two vectors `a` and `b`.
///
/// This function will panic if the two vectors are not the same size.
pub fn compute_inner_product<F: Field>(a: &[F], b: &[F]) -> F {
    // TODO: parallelize?
    assert_eq!(a.len(), b.len());

    let mut acc = F::ZERO;
    for (a, b) in a.iter().zip(b.iter()) {
        acc += (*a) * (*b);
    }

    acc
}

/// Divides polynomial `a` in `X` by `X - b` with
/// no remainder.
pub fn kate_division<'a, F: Field, I: IntoIterator<Item = &'a F>>(a: I, mut b: F) -> Vec<F>
where
    I::IntoIter: DoubleEndedIterator + ExactSizeIterator,
{
    b = -b;
    let a = a.into_iter();

    let mut q = vec![F::ZERO; a.len() - 1];

    let mut tmp = F::ZERO;
    for (q, r) in q.iter_mut().rev().zip(a.rev()) {
        let mut lead_coeff = *r;
        lead_coeff.sub_assign(&tmp);
        *q = lead_coeff;
        tmp = lead_coeff;
        tmp.mul_assign(&b);
    }

    q
}

/// This simple utility function will parallelize an operation that is to be
/// performed over a mutable slice.
pub fn parallelize<T: Send, F: Fn(&mut [T], usize) + Send + Sync + Clone>(v: &mut [T], f: F) {
    let n = v.len();
    let num_threads = multicore::current_num_threads();
    let mut chunk = (n as usize) / num_threads;
    if chunk < num_threads {
        chunk = 1;
    }

    multicore::scope(|scope| {
        for (chunk_num, v) in v.chunks_mut(chunk).enumerate() {
            let f = f.clone();
            scope.spawn(move |_| {
                let start = chunk_num * chunk;
                f(v, start);
            });
        }
    });
}

/// Compute the binary logarithm floored.
pub fn log2_floor(num: usize) -> u32 {
    assert!(num > 0);

    let mut pow = 0;

    while (1 << (pow + 1)) <= num {
        pow += 1;
    }

    pow
}

/// Returns coefficients of an n - 1 degree polynomial given a set of n points
/// and their evaluations. This function will panic if two values in `points`
/// are the same.
pub fn lagrange_interpolate<F: Field>(points: &[F], evals: &[F]) -> Vec<F> {
    assert_eq!(points.len(), evals.len());
    if points.len() == 1 {
        // Constant polynomial
        vec![evals[0]]
    } else {
        let mut denoms = Vec::with_capacity(points.len());
        for (j, x_j) in points.iter().enumerate() {
            let mut denom = Vec::with_capacity(points.len() - 1);
            for x_k in points
                .iter()
                .enumerate()
                .filter(|&(k, _)| k != j)
                .map(|a| a.1)
            {
                denom.push(*x_j - x_k);
            }
            denoms.push(denom);
        }
        // Compute (x_j - x_k)^(-1) for each j != i
        denoms.iter_mut().flat_map(|v| v.iter_mut()).batch_invert();

        let mut final_poly = vec![F::ZERO; points.len()];
        for (j, (denoms, eval)) in denoms.into_iter().zip(evals.iter()).enumerate() {
            let mut tmp: Vec<F> = Vec::with_capacity(points.len());
            let mut product = Vec::with_capacity(points.len() - 1);
            tmp.push(F::ONE);
            for (x_k, denom) in points
                .iter()
                .enumerate()
                .filter(|&(k, _)| k != j)
                .map(|a| a.1)
                .zip(denoms.into_iter())
            {
                product.resize(tmp.len() + 1, F::ZERO);
                for ((a, b), product) in tmp
                    .iter()
                    .chain(std::iter::once(&F::ZERO))
                    .zip(std::iter::once(&F::ZERO).chain(tmp.iter()))
                    .zip(product.iter_mut())
                {
                    *product = *a * (-denom * x_k) + *b * denom;
                }
                std::mem::swap(&mut tmp, &mut product);
            }
            assert_eq!(tmp.len(), points.len());
            assert_eq!(product.len(), points.len() - 1);
            for (final_coeff, interpolation_coeff) in final_poly.iter_mut().zip(tmp.into_iter()) {
                *final_coeff += interpolation_coeff * eval;
            }
        }
        final_poly
    }
}

pub(crate) fn evaluate_vanishing_polynomial<F: Field>(roots: &[F], z: F) -> F {
    fn evaluate<F: Field>(roots: &[F], z: F) -> F {
        roots.iter().fold(F::ONE, |acc, point| (z - point) * acc)
    }
    let n = roots.len();
    let num_threads = multicore::current_num_threads();
    if n * 2 < num_threads {
        evaluate(roots, z)
    } else {
        let chunk_size = (n + num_threads - 1) / num_threads;
        let mut parts = vec![F::ONE; num_threads];
        multicore::scope(|scope| {
            for (out, roots) in parts.chunks_mut(1).zip(roots.chunks(chunk_size)) {
                scope.spawn(move |_| out[0] = evaluate(roots, z));
            }
        });
        parts.iter().fold(F::ONE, |acc, part| acc * part)
    }
}

pub(crate) fn powers<F: Field>(base: F) -> impl Iterator<Item = F> {
    std::iter::successors(Some(F::ONE), move |power| Some(base * power))
}

/// Reverse `l` LSBs of bitvector `n`
pub fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

#[cfg(test)]
use crate::plonk::{start_measure, stop_measure};
use rand_core::OsRng;

#[cfg(test)]
use crate::halo2curves::pasta::Fp;

#[test]
fn test_lagrange_interpolate() {
    let rng = OsRng;

    let points = (0..5).map(|_| Fp::random(rng)).collect::<Vec<_>>();
    let evals = (0..5).map(|_| Fp::random(rng)).collect::<Vec<_>>();

    for coeffs in 0..5 {
        let points = &points[0..coeffs];
        let evals = &evals[0..coeffs];

        let poly = lagrange_interpolate(points, evals);
        assert_eq!(poly.len(), points.len());

        for (point, eval) in points.iter().zip(evals) {
            assert_eq!(eval_polynomial(&poly, *point), *eval);
        }
    }
}

/// GPU-accelerated MSM specifically for BN256 G1 curve.
/// Falls back to CPU if GPU is not available.
#[cfg(feature = "gpu")]
pub fn best_multiexp_bn256(
    coeffs: &[crate::halo2curves::bn256::Fr], 
    bases: &[crate::halo2curves::bn256::G1Affine]
) -> crate::halo2curves::bn256::G1 {
    use crate::halo2curves::bn256::{Fr, G1Affine, G1};
    use group::Group;
    
    assert_eq!(coeffs.len(), bases.len());
    
    // Try GPU MSM first
    if let Some(result) = crate::gpu_msm::gpu_msm(coeffs, bases) {
        return result;
    }
    
    // Fall back to CPU
    let cpu_result = best_multiexp(coeffs, bases);
    cpu_result
}
