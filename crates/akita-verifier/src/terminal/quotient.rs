//! Terminal quotient certificate (feature `terminal-quotient`).
//!
//! Replaces the exact negacyclic terminal A-row product by a disclosed
//! quotient: for every row `i` the prover sends `Q_i` (exactly `D - 1`
//! canonical coefficients) after the `z` payload, the verifier then draws
//! `r_1, r_2` and checks
//!
//! `sum_j A_ij(r) z_j(r) - H_i(r) = (r^D + 1) Q_i(r)` at both points,
//!
//! where `H_i` is the sparse-challenge combination of the terminal `t` rows
//! and `A_ij` is read from the setup's own public matrix. With the unreduced
//! product `P_i = sum_j A_ij z_j`, for a false ring row
//! `P_i - H_i - (X^D + 1) Q_i` is a nonzero polynomial of degree at most
//! `2D - 2` for every sent `Q_i` (division by the monic `X^D + 1` is unique),
//! and the points are drawn after `z`, `t`, the folding challenges and `Q` are
//! absorbed. So a false row passes both points with probability at most
//! `((2D - 2) / p)^2`; one shared pair covers every row. The check never
//! divides by `r^D + 1`. All other terminal checks are unchanged.
//!
//! The points live in the base field `F`, not the extension, so the bound is
//! only adequate for a large base field. The pair is the typed grinding-plan
//! entry `GrindingSite::TerminalQuotientPair`, which the plan admits only when
//! `(2D - 2)^2 2^128 <= |F|^2` holds without grinding (the 128-bit field; no
//! 32- or 64-bit field). The points are drawn through that entry, so a
//! configuration that selects the certificate (`TerminalQuotientConfig`)
//! refuses to prove or verify over a smaller field.

use akita_algebra::CyclotomicRing;
use akita_error::AkitaError;
use akita_types::GrindingReplay;
use jolt_field::{CanonicalEncoding, Field, Ring};

/// The received quotient, the two evaluation points drawn after it, and the
/// setup coefficient view the A rows are read from.
pub(crate) struct TerminalQuotient<'a, F> {
    pub(crate) setup: &'a [F],
    pub(crate) coefficients: Vec<F>,
    pub(crate) points: [F; 2],
}

/// Receive the quotient of the `rows` terminal A rows of ring dimension
/// `ring_dimension` (after the `z` payload) and draw the two points through
/// the grinding plan's quotient-pair entry.
pub(crate) fn receive_terminal_quotient<'a, F>(
    grinding: &mut akita_types::VerifierGrinding<'_, '_>,
    level: u32,
    rows: usize,
    ring_dimension: usize,
    setup: &'a [F],
) -> Result<TerminalQuotient<'a, F>, AkitaError>
where
    F: Field + CanonicalEncoding,
{
    let quotient_len = akita_params::terminal_quotient_len(ring_dimension)?;
    let count = rows
        .checked_mul(quotient_len)
        .ok_or(AkitaError::InvalidProof)?;
    let coefficients = akita_transcript::receive_field_group::<F>(
        grinding.state_mut(),
        akita_types::FoldSite::TerminalQuotient { level }.id()?,
        count,
    )?;
    let points = grinding.terminal_quotient_points::<F>(level)?;
    Ok(TerminalQuotient {
        setup,
        coefficients,
        points,
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
    let quotient_len = akita_params::terminal_quotient_len(D)?;
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
    use akita_params::{ChallengeFieldOrder, GrindingPlan, GrindingRun, SisModulusProfileId};
    use akita_transcript::{
        new_prover_channel, new_verifier_channel, receive_field_group, send_field_group,
    };
    use akita_types::{FoldSite, ProverGrinding, VerifierGrinding};
    use jolt_field::{CanonicalBytes, One, Prime128OffsetA7F7 as F, Zero};

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

    // Transcript-level tests: the terminal tail `z`, `Q`, then the two points,
    // replayed through a real proof channel and the grinding plan's pair entry.

    const LEVEL: u32 = 2;
    const SESSION: &[u8] = b"terminal-quotient/transcript";

    fn fp128_order() -> ChallengeFieldOrder {
        ChallengeFieldOrder::from_field(128, 1, SisModulusProfileId::Q128OffsetA7F7.modulus())
            .unwrap()
    }

    fn plan() -> GrindingPlan {
        let order = fp128_order();
        GrindingPlan::new(
            vec![GrindingRun::terminal_quotient_pair(LEVEL, D, order).unwrap()],
            order,
        )
        .unwrap()
    }

    fn z_fields(z: &[[i16; D]]) -> Vec<F> {
        z.iter()
            .flatten()
            .map(|&value| F::from_i64(i64::from(value)))
            .collect()
    }

    /// The prover's terminal tail: `z`, then `Q`, then the two points.
    fn prove_tail(z: &[[i16; D]], quotient: &[F]) -> (Vec<u8>, [F; 2]) {
        let plan = plan();
        let mut prover = ProverGrinding::new(new_prover_channel(SESSION, b"tail").unwrap(), &plan);
        let z_site = FoldSite::TerminalZPayload { level: LEVEL }.id().unwrap();
        send_field_group(prover.state_mut(), z_site, &z_fields(z)).unwrap();
        let q_site = FoldSite::TerminalQuotient { level: LEVEL }.id().unwrap();
        send_field_group(prover.state_mut(), q_site, quotient).unwrap();
        let points = prover.terminal_quotient_points::<F>(LEVEL).unwrap();
        (prover.finish().unwrap(), points)
    }

    /// The verifier's side of the same tail, through end of stream.
    fn verify_tail<'a>(
        proof: &[u8],
        setup: &'a [F],
    ) -> Result<TerminalQuotient<'a, F>, AkitaError> {
        let plan = plan();
        let channel = new_verifier_channel(SESSION, b"tail", proof)?;
        let mut verifier = VerifierGrinding::new(channel, &plan);
        let z_site = FoldSite::TerminalZPayload { level: LEVEL }.id()?;
        receive_field_group::<F>(verifier.state_mut(), z_site, COLS * D)?;
        let quotient = receive_terminal_quotient::<F>(&mut verifier, LEVEL, ROWS, D, setup)?;
        verifier.finish()?;
        Ok(quotient)
    }

    fn power(base: F, exponent: usize) -> F {
        (0..exponent).fold(F::one(), |acc, _| acc * base)
    }

    fn monomial(degree: usize) -> CyclotomicRing<F, D> {
        CyclotomicRing::from_coefficients(std::array::from_fn(|k| {
            if k == degree {
                F::one()
            } else {
                F::zero()
            }
        }))
    }

    #[test]
    fn transcript_points_follow_the_quotient_and_match_the_prover() {
        let (setup, z) = fixture(21);
        let rows = reduced_rows(&setup, &z);
        let (proof, points) = prove_tail(&z, &honest(&setup, &z));
        assert_eq!(
            proof.len(),
            (COLS * D + ROWS * (D - 1)) * <F as CanonicalBytes>::NUM_BYTES
        );
        let quotient = verify_tail(&proof, &setup).expect("honest tail");
        assert_eq!(quotient.points, points);
        check_rows(&quotient, &rows, &z).expect("honest quotient at the transcript points");
    }

    #[test]
    fn noncanonical_quotient_atom_rejects_at_receipt() {
        let (setup, z) = fixture(22);
        let (proof, _) = prove_tail(&z, &honest(&setup, &z));
        let atom_bytes = <F as CanonicalBytes>::NUM_BYTES;
        let start = (COLS * D + 5) * atom_bytes;
        let mut noncanonical = proof.clone();
        noncanonical[start..start + atom_bytes].fill(0xff);
        assert!(verify_tail(&noncanonical, &setup).is_err());
        // A canonical change to the same atom parses, then fails the identity.
        let mut changed = proof;
        changed[start] ^= 1;
        let quotient = verify_tail(&changed, &setup).expect("canonical atom parses");
        assert!(check_rows(&quotient, &reduced_rows(&setup, &z), &z).is_err());
    }

    #[test]
    fn modified_response_with_recomputed_quotient_rejects_through_the_transcript() {
        let (setup, z) = fixture(23);
        let rows = reduced_rows(&setup, &z);
        let mut tampered = z.clone();
        tampered[1][17] = tampered[1][17].wrapping_add(3);
        let (proof, points) = prove_tail(&tampered, &honest(&setup, &tampered));
        let quotient = verify_tail(&proof, &setup).expect("well-formed tail");
        assert_eq!(quotient.points, points);
        assert!(check_rows(&quotient, &rows, &tampered).is_err());
    }

    #[test]
    fn quotient_is_bound_before_both_points() {
        let (setup, z) = fixture(24);
        let honest_quotient = honest(&setup, &z);
        let (_, points) = prove_tail(&z, &honest_quotient);

        // Any change to the quotient moves both points.
        let mut other = honest_quotient.clone();
        other[ROWS * (D - 1) - 1] += F::one();
        let (_, moved) = prove_tail(&z, &other);
        assert_ne!(moved[0], points[0]);
        assert_ne!(moved[1], points[1]);

        // A false row, and a quotient fitted to the honest transcript's points:
        // `Q* = Q + c0 + c1 X` with `Q*(r) = Q(r) - r^40 / (r^D + 1)` at both.
        let false_row = 1;
        let mut rows = reduced_rows(&setup, &z);
        rows[false_row] += monomial(40);
        let targets = points.map(|r| {
            let shift = power(r, D) + F::one();
            F::zero() - power(r, 40) * shift.inverse().expect("r^D + 1 is nonzero")
        });
        let slope =
            (targets[1] - targets[0]) * (points[1] - points[0]).inverse().expect("distinct points");
        let intercept = targets[0] - slope * points[0];
        let mut fitted = honest_quotient;
        fitted[false_row * (D - 1)] += intercept;
        fitted[false_row * (D - 1) + 1] += slope;

        // Were the quotient chosen after the points, it would pass.
        let after_points = TerminalQuotient {
            setup: &setup,
            coefficients: fitted.clone(),
            points,
        };
        check_rows(&after_points, &rows, &z).expect("a quotient fitted to known points");

        // Sent in its place before the points, it moves them and fails.
        let (proof, drawn) = prove_tail(&z, &fitted);
        assert_ne!(drawn, points);
        let quotient = verify_tail(&proof, &setup).expect("well-formed tail");
        assert_eq!(quotient.points, drawn);
        assert!(check_rows(&quotient, &rows, &z).is_err());
    }

    #[test]
    fn truncated_or_extended_quotient_tail_rejects() {
        let (setup, z) = fixture(25);
        let (proof, _) = prove_tail(&z, &honest(&setup, &z));
        let atom_bytes = <F as CanonicalBytes>::NUM_BYTES;
        assert!(verify_tail(&proof[..proof.len() - 1], &setup).is_err());
        assert!(verify_tail(&proof[..proof.len() - atom_bytes], &setup).is_err());
        let mut extra_byte = proof.clone();
        extra_byte.push(0);
        assert!(verify_tail(&extra_byte, &setup).is_err());
        let mut extra_atom = proof;
        extra_atom.resize(extra_atom.len() + atom_bytes, 0);
        assert!(verify_tail(&extra_atom, &setup).is_err());
    }

    #[test]
    fn points_need_the_plan_entry_which_small_fields_do_not_get() {
        for (profile, bits, extension_degree) in [
            (SisModulusProfileId::Q32Offset99, 32, 4),
            (SisModulusProfileId::Q64Offset59, 64, 2),
        ] {
            let order =
                ChallengeFieldOrder::from_field(bits, extension_degree, profile.modulus()).unwrap();
            assert!(matches!(
                GrindingRun::terminal_quotient_pair(LEVEL, D, order),
                Err(AkitaError::UnsupportedSchedule(_))
            ));
        }
        let plan = GrindingPlan::new(Vec::new(), fp128_order()).unwrap();
        let channel = new_verifier_channel(SESSION, b"tail", &[]).unwrap();
        let mut verifier = VerifierGrinding::new(channel, &plan);
        assert!(verifier.terminal_quotient_points::<F>(LEVEL).is_err());
    }
}
