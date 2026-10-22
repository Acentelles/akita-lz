//! L2 terminal quotient certificate (experimental, `l2-terminal-quotient`).
//!
//! Replaces the exact negacyclic terminal A-row product by a disclosed
//! quotient: for every row `i` the prover sends `Q_i` (exactly `D - 1`
//! canonical coefficients) after the `z` payload, the verifier then draws
//! `r_1, r_2` and checks
//!
//! `sum_j A_ij(r) z_j(r) - H_i(r) = (r^D + 1) Q_i(r)` at both points,
//!
//! where `H_i` is the sparse-challenge combination of the terminal `t` rows
//! and `A_ij` is read from the setup's own public matrix. A false ring row
//! passes one point with probability at most `(2D - 2) / p`; see the lever's
//! SOUNDNESS note. All other terminal checks are unchanged.

use akita_algebra::CyclotomicRing;
use akita_error::AkitaError;
use akita_params::TerminalFoldParams;
use akita_types::GrindingReplay;
use jolt_field::{CanonicalEncoding, Field, Ring};

/// The received quotient, the two evaluation points drawn after it, and the
/// setup coefficient view the A rows are read from.
pub(crate) struct TerminalQuotient<'a, F> {
    pub(crate) setup: &'a [F],
    pub(crate) coefficients: Vec<F>,
    pub(crate) points: [F; 2],
}

/// Receive the quotient (after the `z` payload) and draw the two points.
pub(crate) fn receive_terminal_quotient<'a, F>(
    grinding: &mut akita_types::VerifierGrinding<'_, '_>,
    level: u32,
    scheduled: &TerminalFoldParams,
    setup: &'a [F],
) -> Result<TerminalQuotient<'a, F>, AkitaError>
where
    F: Field + CanonicalEncoding,
{
    let quotient_len = akita_types::terminal_quotient_len(scheduled.d_a())?;
    let count = scheduled
        .inner
        .matrix
        .output_rank()
        .checked_mul(quotient_len)
        .ok_or(AkitaError::InvalidProof)?;
    let coefficients = akita_transcript::receive_field_group::<F>(
        grinding.state_mut(),
        akita_types::FoldSite::TerminalQuotient { level }.id()?,
        count,
    )?;
    let first = akita_transcript::verifier_field_challenge::<F>(grinding.state_mut())?;
    let second = akita_transcript::verifier_field_challenge::<F>(grinding.state_mut())?;
    Ok(TerminalQuotient {
        setup,
        coefficients,
        points: [first, second],
    })
}

/// `[1, r, ..., r^(D-1)]`.
fn power_row<F: Field>(point: F, d: usize) -> Vec<F> {
    let mut powers = Vec::with_capacity(d);
    let mut power = F::one();
    for _ in 0..d {
        powers.push(power);
        power *= point;
    }
    powers
}

/// Check every terminal A row against its quotient at both points.
///
/// `rows` are the `H_i` (one per A row) and `z` the centered response rings.
pub(crate) fn check_rows<F, const D: usize>(
    quotient: &TerminalQuotient<'_, F>,
    rows: &[CyclotomicRing<F, D>],
    z: &[[i16; D]],
) -> Result<(), AkitaError>
where
    F: Field + Ring,
{
    let quotient_len = akita_types::terminal_quotient_len(D)?;
    let num_rows = rows.len();
    if quotient.coefficients.len()
        != num_rows
            .checked_mul(quotient_len)
            .ok_or(AkitaError::InvalidProof)?
    {
        return Err(AkitaError::InvalidProof);
    }
    let num_cols = z.len();
    let row_len = num_cols.checked_mul(D).ok_or(AkitaError::InvalidProof)?;
    let prefix = num_rows
        .checked_mul(row_len)
        .ok_or(AkitaError::InvalidProof)?;
    let setup = quotient.setup.get(..prefix).ok_or_else(|| {
        AkitaError::InvalidSetup("terminal quotient setup view is shorter than the A prefix".into())
    })?;
    let z_fields: Vec<F> = z
        .iter()
        .flat_map(|ring| ring.iter().map(|&value| F::from_i64(i64::from(value))))
        .collect();
    for &point in &quotient.points {
        let powers = power_row(point, D);
        let last = powers.last().copied().ok_or(AkitaError::InvalidProof)?;
        let shift = last * point + F::one();
        let z_evals: Vec<F> = z_fields
            .chunks_exact(D)
            .map(|ring| F::dot_product(ring, &powers))
            .collect();
        for (row_index, (rhs, row_quotient)) in rows
            .iter()
            .zip(quotient.coefficients.chunks_exact(quotient_len))
            .enumerate()
        {
            let start = row_index
                .checked_mul(row_len)
                .ok_or(AkitaError::InvalidProof)?;
            let row = setup
                .get(start..start + row_len)
                .ok_or(AkitaError::InvalidProof)?;
            let product = if num_cols == 0 {
                F::zero()
            } else {
                row.chunks_exact(D)
                    .zip(&z_evals)
                    .fold(F::zero(), |acc, (a, z_eval)| {
                        acc + F::dot_product(a, &powers) * *z_eval
                    })
            };
            let h = F::dot_product(rhs.coefficients(), &powers);
            let q = F::dot_product(row_quotient, &powers);
            if product - h != shift * q {
                return Err(AkitaError::InvalidProof);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jolt_field::{One, Prime128OffsetA7F7 as F, Zero};

    const D: usize = 64;
    const ROWS: usize = 3;
    const COLS: usize = 5;

    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state >> 11
    }

    fn fixture(seed: u64) -> (Vec<F>, Vec<[i16; D]>) {
        let mut state = seed;
        let setup = (0..ROWS * COLS * D)
            .map(|_| {
                F::from_u64(lcg(&mut state)) * F::from_u64(lcg(&mut state))
                    + F::from_u64(lcg(&mut state))
            })
            .collect();
        let z = (0..COLS)
            .map(|_| std::array::from_fn(|_| (lcg(&mut state) % 65_536) as u16 as i16))
            .collect();
        (setup, z)
    }

    fn reduced_rows(setup: &[F], z: &[[i16; D]]) -> Vec<CyclotomicRing<F, D>> {
        (0..ROWS)
            .map(|row| {
                (0..COLS).fold(CyclotomicRing::zero(), |acc, col| {
                    let start = (row * COLS + col) * D;
                    let a = CyclotomicRing::<F, D>::from_coefficients(std::array::from_fn(|k| {
                        setup[start + k]
                    }));
                    let zz = CyclotomicRing::<F, D>::from_coefficients(std::array::from_fn(|k| {
                        F::from_i64(i64::from(z[col][k]))
                    }));
                    acc + a * zz
                })
            })
            .collect()
    }

    fn honest(setup: &[F], z: &[[i16; D]]) -> Vec<F> {
        akita_types::terminal_a_row_quotients(setup, z.as_flattened(), D, ROWS).unwrap()
    }

    fn certificate(setup: &[F], coefficients: Vec<F>, seed: u64) -> TerminalQuotient<'_, F> {
        let mut state = seed;
        TerminalQuotient {
            setup,
            coefficients,
            points: [
                F::from_u64(lcg(&mut state)) * F::from_u64(lcg(&mut state)),
                F::from_u64(lcg(&mut state)) * F::from_u64(lcg(&mut state)),
            ],
        }
    }

    #[test]
    fn honest_quotient_accepts_at_random_points() {
        for seed in 1..4 {
            let (setup, z) = fixture(seed);
            let rows = reduced_rows(&setup, &z);
            let q = certificate(&setup, honest(&setup, &z), seed + 100);
            check_rows(&q, &rows, &z).expect("honest quotient");
        }
    }

    #[test]
    fn wrong_quotient_rejects() {
        let (setup, z) = fixture(7);
        let rows = reduced_rows(&setup, &z);
        let mut coefficients = honest(&setup, &z);
        coefficients[17] += F::one();
        assert!(check_rows(&certificate(&setup, coefficients, 9), &rows, &z).is_err());
    }

    #[test]
    fn wrong_quotient_length_rejects() {
        let (setup, z) = fixture(8);
        let rows = reduced_rows(&setup, &z);
        let mut long = honest(&setup, &z);
        long.push(F::zero());
        assert!(check_rows(&certificate(&setup, long, 9), &rows, &z).is_err());
        let mut short = honest(&setup, &z);
        short.pop();
        assert!(check_rows(&certificate(&setup, short, 9), &rows, &z).is_err());
    }

    #[test]
    fn wrong_setup_view_rejects() {
        let (setup, z) = fixture(10);
        let rows = reduced_rows(&setup, &z);
        let coefficients = honest(&setup, &z);
        let mut other = setup.clone();
        other[3 * D + 5] += F::one();
        assert!(check_rows(&certificate(&other, coefficients.clone(), 11), &rows, &z).is_err());
        assert!(matches!(
            check_rows(
                &certificate(&setup[..setup.len() - 1], coefficients, 11),
                &rows,
                &z
            ),
            Err(AkitaError::InvalidSetup(_))
        ));
    }

    #[test]
    fn wrong_terminal_response_rejects_even_with_its_own_quotient() {
        let (setup, z) = fixture(12);
        let rows = reduced_rows(&setup, &z);
        let mut tampered = z.clone();
        tampered[2][9] = tampered[2][9].wrapping_add(1);
        let coefficients = honest(&setup, &tampered);
        assert!(check_rows(&certificate(&setup, coefficients, 13), &rows, &tampered).is_err());
    }

    #[test]
    fn wrong_right_hand_side_rejects() {
        let (setup, z) = fixture(14);
        let mut rows = reduced_rows(&setup, &z);
        rows[1] += CyclotomicRing::from_coefficients(std::array::from_fn(|k| {
            if k == 40 {
                F::one()
            } else {
                F::zero()
            }
        }));
        let q = certificate(&setup, honest(&setup, &z), 15);
        assert!(check_rows(&q, &rows, &z).is_err());
    }

    #[test]
    fn quotient_identity_holds_as_polynomials() {
        let (setup, z) = fixture(16);
        let rows = reduced_rows(&setup, &z);
        let coefficients = honest(&setup, &z);
        // Points with r^D = -1 are allowed; at r = 0 the check reads the
        // constant terms, which must agree for an honest quotient.
        let at_zero = TerminalQuotient {
            setup: &setup,
            coefficients,
            points: [F::zero(), F::one()],
        };
        check_rows(&at_zero, &rows, &z).expect("honest quotient at 0 and 1");
    }
}
