//! Negacyclic transform tables of one NTT prime, word for word as
//! `jolt_algebra::ntt::NttTwiddles::compute` builds them.
//!
//! Akita-local until `jolt-algebra` exposes the tables of an `NttTwiddles`:
//! the device kernels upload these instead of recomputing roots, so every
//! transform shares the CPU's root choice. The differential NTT tests pin the
//! two constructions together.

use jolt_algebra::{MontCoeff, NttPrime};

use crate::error::AkitaMetalError;

/// The forward stage twiddles, inverse stage twiddles, twist `psi^i`, and
/// fused untwist `D^{-1} psi^{-i}`, in Montgomery form. Stage twiddles use the
/// packed layout of `forward_ntt` and `inverse_ntt`.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "residues are below p < 2^31, so i64 products cannot overflow; \
              stage indices stay below D"
)]
pub(crate) fn negacyclic_tables<const D: usize>(
    prime: NttPrime<i32>,
) -> Result<[[MontCoeff<i32>; D]; 4], AkitaMetalError> {
    let p = i64::from(prime.p);
    let two_d = i64::try_from(D)
        .ok()
        .and_then(|d| d.checked_mul(2))
        .filter(|&two_d| D.is_power_of_two() && p > 1 && (p - 1) % two_d == 0)
        .ok_or_else(|| {
            AkitaMetalError::Shape(format!("2 x {D} does not divide p - 1 for p = {p}"))
        })?;
    let psi = primitive_root_2d(p, two_d)
        .ok_or_else(|| AkitaMetalError::Shape(format!("no primitive {two_d}-th root mod {p}")))?;
    let omega = psi * psi % p;
    let omega_inv = pow_mod(omega, p - 2, p);
    let psi_inv = pow_mod(psi, p - 2, p);
    let mont = |canonical: i64| prime.from_canonical(canonical as i32);
    let zero = MontCoeff::from_raw(0);

    let mut psi_pows = [zero; D];
    let mut d_inv_psi_inv = [zero; D];
    let d_inv = mont(pow_mod(two_d / 2, p - 2, p));
    let (mut cur, mut cur_inv) = (1i64, 1i64);
    for (twist, untwist) in psi_pows.iter_mut().zip(&mut d_inv_psi_inv) {
        *twist = mont(cur);
        *untwist = prime.mul(d_inv, mont(cur_inv));
        cur = cur * psi % p;
        cur_inv = cur_inv * psi_inv % p;
    }

    let mut fwd = [zero; D];
    let mut inv = [zero; D];
    let one = mont(1);
    let mut len = 1usize;
    while len < D {
        let exp = (D / (2 * len)) as i64;
        let (fwd_step, inv_step) = (
            mont(pow_mod(omega, exp, p)),
            mont(pow_mod(omega_inv, exp, p)),
        );
        let (mut w_fwd, mut w_inv) = (one, one);
        for j in 0..len {
            fwd[len - 1 + j] = w_fwd;
            inv[len - 1 + j] = w_inv;
            w_fwd = prime.mul(w_fwd, fwd_step);
            w_inv = prime.mul(w_inv, inv_step);
        }
        len *= 2;
    }
    Ok([fwd, inv, psi_pows, d_inv_psi_inv])
}

/// `a^((p - 1) / 2D)` for the least quadratic non-residue `a`, the root
/// `NttTwiddles::compute` selects.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "callers pass 1 < p < 2^31 with 2D | p - 1"
)]
fn primitive_root_2d(p: i64, two_d: i64) -> Option<i64> {
    let half = (p - 1) / 2;
    (2..p)
        .find(|&a| pow_mod(a, half, p) == p - 1)
        .map(|a| pow_mod(a, (p - 1) / two_d, p))
}

/// `base^exp mod modulus` for `0 <= base` and `modulus < 2^31`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "residues below 2^31 keep every product below 2^62"
)]
fn pow_mod(base: i64, mut exp: i64, modulus: i64) -> i64 {
    let mut result = 1;
    let mut base = base % modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % modulus;
        }
        base = base * base % modulus;
        exp >>= 1;
    }
    result
}
