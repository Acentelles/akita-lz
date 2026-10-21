//! Prover-side hiding randomness for schedules with randomness columns.
//!
//! Extension seam: Whiteout outer hiding randomness (akita#120). Clear
//! schedules declare zero columns and never reach the sampler, so their
//! commitments and proofs stay deterministic.

use akita_error::AkitaError;
use rand_core::CryptoRngCore;
use std::sync::Mutex;

/// The CSPRNG a backend draws hiding randomness from.
///
/// A backend owns one source; commitments and folds draw from it in the
/// order the protocol runs, so a seeded source makes a hiding proof
/// reproducible.
pub(crate) struct HidingRng(Mutex<Option<Box<dyn CryptoRngCore + Send>>>);

impl HidingRng {
    /// The operating system CSPRNG, or no source in a build without the `zk`
    /// feature.
    pub(crate) fn os() -> Self {
        #[cfg(feature = "zk")]
        let rng: Option<Box<dyn CryptoRngCore + Send>> = Some(Box::new(rand_core::OsRng));
        #[cfg(not(feature = "zk"))]
        let rng = None;
        Self(Mutex::new(rng))
    }

    /// A caller-provided CSPRNG.
    #[cfg(feature = "zk")]
    pub(crate) fn new(rng: impl CryptoRngCore + Send + 'static) -> Self {
        Self(Mutex::new(Some(Box::new(rng))))
    }

    /// Draw `len` digits uniformly from the balanced alphabet
    /// `[-2^(log_basis-1), 2^(log_basis-1) - 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`AkitaError::UnsupportedSchedule`] when randomness is
    /// requested in a build without the `zk` feature, and an input error for
    /// an alphabet the sampler does not support.
    pub(crate) fn sample(&self, len: usize, log_basis: u32) -> Result<Vec<i8>, AkitaError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let mut rng = self
            .0
            .lock()
            .map_err(|_| AkitaError::Internal("hiding randomness source is poisoned".into()))?;
        let rng = rng.as_mut().ok_or_else(|| {
            AkitaError::UnsupportedSchedule(
                "schedule declares hiding randomness, which needs the zk feature".into(),
            )
        })?;
        sample_hiding_digits(rng.as_mut(), len, log_basis)
    }
}

#[cfg(feature = "zk")]
fn sample_hiding_digits(
    mut rng: &mut dyn CryptoRngCore,
    len: usize,
    log_basis: u32,
) -> Result<Vec<i8>, AkitaError> {
    Ok(jolt_crypto::lattice::sample_box_digits(
        &mut rng, len, log_basis,
    )?)
}

#[cfg(not(feature = "zk"))]
fn sample_hiding_digits(
    _rng: &mut dyn CryptoRngCore,
    _len: usize,
    _log_basis: u32,
) -> Result<Vec<i8>, AkitaError> {
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
        test_support::CORRUPTED.set(true);
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

/// Whether a test fault corrupted a witness this thread built since the
/// fault was armed, so the prover's self-checks must let the corrupt witness
/// reach the verifier. Always false outside `test-support` builds.
pub(crate) fn witness_corrupted_by_test() -> bool {
    #[cfg(feature = "test-support")]
    return test_support::CORRUPTED.get();
    #[cfg(not(feature = "test-support"))]
    false
}

/// Fault injection for tests that show the randomness is bound by the
/// relation of the level whose witness carries it.
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::RandomnessSection;
    use std::cell::Cell;

    std::thread_local! {
        static NEXT: Cell<Option<RandomnessSection>> = const { Cell::new(None) };
        pub(super) static CORRUPTED: Cell<bool> = const { Cell::new(false) };
    }

    /// Shift the first digit of `section`, within its alphabet, in the next
    /// recursive witness this thread builds.
    pub fn corrupt_next_witness_randomness(section: RandomnessSection) {
        CORRUPTED.set(false);
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
        let rng = HidingRng::os();
        let digits = rng.sample(4096, 3).unwrap();
        assert_eq!(digits.len(), 4096);
        assert!(digits.iter().all(|digit| (-4..=3).contains(digit)));
        assert!(digits.contains(&-4) && digits.contains(&3));
        assert!(rng.sample(0, 3).unwrap().is_empty());
    }
}
