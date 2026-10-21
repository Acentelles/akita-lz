//! Prover-side hiding randomness for schedules with randomness columns.
//!
//! Extension seam: Whiteout outer hiding randomness (akita#120). Clear
//! schedules declare zero columns and never reach the sampler, so their
//! commitments and proofs stay deterministic.

use akita_error::AkitaError;

/// Draw `len` digits uniformly from the balanced alphabet
/// `[-2^(log_basis-1), 2^(log_basis-1) - 1]` with the operating system CSPRNG.
///
/// # Errors
///
/// Returns [`AkitaError::UnsupportedSchedule`] when randomness is requested in
/// a build without the `zk` feature, and an input error for an alphabet the
/// sampler does not support.
pub(crate) fn sample_hiding_digits(len: usize, log_basis: u32) -> Result<Vec<i8>, AkitaError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    sample_nonempty(len, log_basis)
}

#[cfg(feature = "zk")]
fn sample_nonempty(len: usize, log_basis: u32) -> Result<Vec<i8>, AkitaError> {
    Ok(jolt_crypto::lattice::sample_box_digits(
        &mut rand_core::OsRng,
        len,
        log_basis,
    )?)
}

#[cfg(not(feature = "zk"))]
fn sample_nonempty(_len: usize, _log_basis: u32) -> Result<Vec<i8>, AkitaError> {
    Err(AkitaError::UnsupportedSchedule(
        "schedule declares hiding randomness, which needs the zk feature".into(),
    ))
}

/// The witness section one randomness vector fills.
#[cfg_attr(not(feature = "test-support"), allow(unreachable_pub))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomnessSection {
    /// The B randomness of the commitment a level consumes.
    Outer,
    /// The randomness of the level's own D image.
    Opening,
}

/// The digits written into `section` of the next-level witness: `digits`
/// itself, unless a test fault is pending for that section.
pub(crate) fn witness_randomness(
    section: RandomnessSection,
    digits: &[i8],
) -> std::borrow::Cow<'_, [i8]> {
    #[cfg(feature = "test-support")]
    if !digits.is_empty() && test_support::take(section) {
        let mut corrupted = digits.to_vec();
        corrupted[0] = if corrupted[0] < 0 {
            corrupted[0] + 1
        } else {
            corrupted[0] - 1
        };
        return std::borrow::Cow::Owned(corrupted);
    }
    #[cfg(not(feature = "test-support"))]
    let _ = section;
    std::borrow::Cow::Borrowed(digits)
}

/// Fault injection for tests that show the randomness is bound by the
/// relation of the level whose witness carries it.
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::RandomnessSection;
    use std::cell::Cell;

    std::thread_local! {
        static NEXT: Cell<Option<RandomnessSection>> = const { Cell::new(None) };
    }

    /// Shift the first digit of `section`, within its alphabet, in the next
    /// recursive witness this thread builds.
    pub fn corrupt_next_witness_randomness(section: RandomnessSection) {
        NEXT.set(Some(section));
    }

    pub(super) fn take(section: RandomnessSection) -> bool {
        let pending = NEXT.get() == Some(section);
        if pending {
            NEXT.set(None);
        }
        pending
    }
}

#[cfg(all(test, feature = "zk"))]
mod tests {
    use super::*;

    #[test]
    fn samples_cover_exactly_the_balanced_alphabet() {
        let digits = sample_hiding_digits(4096, 3).unwrap();
        assert_eq!(digits.len(), 4096);
        assert!(digits.iter().all(|digit| (-4..=3).contains(digit)));
        assert!(digits.contains(&-4) && digits.contains(&3));
        assert!(sample_hiding_digits(0, 3).unwrap().is_empty());
    }
}
