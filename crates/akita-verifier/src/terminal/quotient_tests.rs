use super::*;
use akita_algebra::CyclotomicRing;
use jolt_field::{One, Prime128OffsetA7F7 as F, Zero};

const D: usize = 64;
const ROWS: usize = 3;
const COLS: usize = 5;

fn check_reference_rows(
    quotient: &TerminalQuotient<'_, F>,
    rows: &[CyclotomicRing<F, D>],
    z: &[[i16; D]],
) -> Result<(), AkitaError> {
    let evaluated: Vec<[F; 2]> = rows
        .iter()
        .map(|row| {
            quotient.points.map(|point| {
                row.coefficients()
                    .iter()
                    .rev()
                    .fold(F::zero(), |acc, coefficient| acc * point + *coefficient)
            })
        })
        .collect();
    super::check_rows(quotient, &evaluated, z)
}

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
        check_reference_rows(&q, &rows, &z).expect("honest quotient");
    }
}

#[test]
fn wrong_quotient_rejects() {
    let (setup, z) = fixture(7);
    let rows = reduced_rows(&setup, &z);
    let mut coefficients = honest(&setup, &z);
    coefficients[17] += F::one();
    assert!(check_reference_rows(&certificate(&setup, coefficients, 9), &rows, &z).is_err());
}

#[test]
fn wrong_quotient_length_rejects() {
    let (setup, z) = fixture(8);
    let rows = reduced_rows(&setup, &z);
    let mut long = honest(&setup, &z);
    long.push(F::zero());
    assert!(check_reference_rows(&certificate(&setup, long, 9), &rows, &z).is_err());
    let mut short = honest(&setup, &z);
    short.pop();
    assert!(check_reference_rows(&certificate(&setup, short, 9), &rows, &z).is_err());
}

#[test]
fn wrong_setup_view_rejects() {
    let (setup, z) = fixture(10);
    let rows = reduced_rows(&setup, &z);
    let coefficients = honest(&setup, &z);
    let mut other = setup.clone();
    other[3 * D + 5] += F::one();
    assert!(
        check_reference_rows(&certificate(&other, coefficients.clone(), 11), &rows, &z).is_err()
    );
    assert!(matches!(
        check_reference_rows(
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
    assert!(
        check_reference_rows(&certificate(&setup, coefficients, 13), &rows, &tampered).is_err()
    );
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
    assert!(check_reference_rows(&q, &rows, &z).is_err());
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
    check_reference_rows(&at_zero, &rows, &z).expect("honest quotient at 0 and 1");
}
