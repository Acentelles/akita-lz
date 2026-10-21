//! Hiding-randomness sizing for zero-knowledge schedules.
//!
//! A zero-knowledge schedule appends fresh randomness columns to every outer
//! (B) slice and every opening (D) image made at a masked level. The
//! randomness hides the image by the leftover hash lemma once its entropy
//! exceeds the image's bit length by `2λ` plus a fixed slack. This module is
//! the one Akita caller of `jolt_crypto`'s sizing rule; planners and schedule
//! validation both size widths through [`hiding_randomness_columns`].

use akita_error::AkitaError;
use jolt_algebra::{Prime128Offset275, Prime128OffsetA7F7, Prime32Offset99, Prime64Offset59};
use jolt_crypto::lattice::required_randomness_columns;

use crate::sis::SisModulusProfileId;
use crate::{CommittedGroupParams, FoldSchedule, GroupOpenPhaseParams};

/// Slack bits added on top of `2λ` when sizing hiding randomness.
const HIDING_SLACK_BITS: usize = 8;

/// Shape of one outer-map image hidden by a randomness vector: one B slice or
/// one level's D image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageShape {
    /// Image rank (output ring elements).
    pub rows: usize,
    /// Ring dimension of the image and of each randomness column.
    pub ring_dim: usize,
    /// Field modulus profile of the image coefficients.
    pub modulus: SisModulusProfileId,
}

/// Balanced digit alphabet `[-2^(log_basis-1), 2^(log_basis-1) - 1]` of the
/// witness that holds the randomness. It is that witness's open basis, the one
/// alphabet the next level's range check proves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitAlphabet {
    /// Base-two logarithm of the digit base.
    pub log_basis: u32,
}

/// Number of randomness ring columns that hide one image of `image` shape
/// with digits drawn uniformly from `alphabet`.
///
/// # Errors
///
/// Returns an error for a zero ring dimension, an alphabet outside `1..=8`,
/// or an overflowing size.
pub fn hiding_randomness_columns(
    image: ImageShape,
    alphabet: DigitAlphabet,
) -> Result<usize, AkitaError> {
    let image_coeffs = image
        .rows
        .checked_mul(image.ring_dim)
        .ok_or_else(|| AkitaError::InvalidSetup("hidden image size overflows usize".into()))?;
    let columns = match image.modulus {
        SisModulusProfileId::Q32Offset99 => required_randomness_columns::<Prime32Offset99>(
            image_coeffs,
            0,
            image.ring_dim,
            alphabet.log_basis,
            HIDING_SLACK_BITS,
        ),
        SisModulusProfileId::Q64Offset59 => required_randomness_columns::<Prime64Offset59>(
            image_coeffs,
            0,
            image.ring_dim,
            alphabet.log_basis,
            HIDING_SLACK_BITS,
        ),
        SisModulusProfileId::Q128OffsetA7F7 => required_randomness_columns::<Prime128OffsetA7F7>(
            image_coeffs,
            0,
            image.ring_dim,
            alphabet.log_basis,
            HIDING_SLACK_BITS,
        ),
        SisModulusProfileId::Q128Offset275 => required_randomness_columns::<Prime128Offset275>(
            image_coeffs,
            0,
            image.ring_dim,
            alphabet.log_basis,
            HIDING_SLACK_BITS,
        ),
    }?;
    Ok(columns)
}

/// Hiding-randomness columns of every B slice of `group`'s commitment.
///
/// The randomness hides one slice image and is drawn from the open alphabet
/// of the opening that consumes the commitment, the alphabet that level's
/// range check proves.
///
/// # Errors
///
/// Returns the sizing errors of [`hiding_randomness_columns`].
pub fn b_randomness_columns(group: &GroupOpenPhaseParams) -> Result<usize, AkitaError> {
    let outer = &group.profile.outer.matrix;
    hiding_randomness_columns(
        ImageShape {
            rows: outer.output_rank(),
            ring_dim: outer.ring_dimension(),
            modulus: outer.sis_modulus_profile(),
        },
        DigitAlphabet {
            log_basis: group.opening.log_basis_open,
        },
    )
}

/// Hiding-randomness columns of the D image `params`' level makes, drawn from
/// that level's open alphabet.
///
/// # Errors
///
/// Returns the sizing errors of [`hiding_randomness_columns`].
pub fn d_randomness_columns(params: &CommittedGroupParams) -> Result<usize, AkitaError> {
    let open = params.open();
    hiding_randomness_columns(
        ImageShape {
            rows: open.matrix.output_rank(),
            ring_dim: open.matrix.ring_dimension(),
            modulus: open.matrix.sis_modulus_profile(),
        },
        DigitAlphabet {
            log_basis: open.digits.log_basis,
        },
    )
}

/// Recompute every hiding-randomness width of `schedule` and reject any
/// mismatch, in release builds.
///
/// Each level up to `cut_level` must carry exactly [`b_randomness_columns`]
/// for its own group's B slices and [`d_randomness_columns`] for its D
/// image. Both are sized at the level's open alphabet, the one its range
/// check proves. Preceding groups (setup prefixes, committed
/// programs, trusted advice) and every level after the cut carry none, and a
/// hiding level uses one witness chunk.
///
/// # Errors
///
/// Returns [`AkitaError::InvalidSetup`] naming the first violating level.
pub fn validate_hiding_randomness(
    schedule: &FoldSchedule,
    cut_level: usize,
) -> Result<(), AkitaError> {
    let levels = std::iter::once(&schedule.root.params)
        .chain(schedule.recursive_folds.iter().map(|fold| &fold.params));
    for (level, params) in levels.enumerate() {
        let invalid = |what: &str| {
            AkitaError::InvalidSetup(format!("level {level} {what} hiding randomness"))
        };
        let own = params.own_group();
        let (expected_b, expected_d) = if level <= cut_level {
            if params.witness_chunk.num_chunks != 1 {
                return Err(invalid("splits across witness chunks its"));
            }
            (b_randomness_columns(own)?, d_randomness_columns(params)?)
        } else {
            (0, 0)
        };
        if own.profile.randomness_columns != expected_b {
            return Err(invalid("has the wrong B"));
        }
        if params.open_randomness_columns != expected_d {
            return Err(invalid("has the wrong D"));
        }
        if params
            .preceding_group_iter()
            .any(|group| group.profile.randomness_columns != 0)
        {
            return Err(invalid("gives a precommitted group"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headline_slice_needs_forty_five_columns() {
        // d_b = 64, rank 1, b = 8 over the 128-bit field: 64 * 128 + 256 + 8
        // bits at 192 bits per column.
        let image = ImageShape {
            rows: 1,
            ring_dim: 64,
            modulus: SisModulusProfileId::Q128OffsetA7F7,
        };
        assert_eq!(
            hiding_randomness_columns(image, DigitAlphabet { log_basis: 3 }).unwrap(),
            45
        );
    }

    #[test]
    fn residual_commitment_needs_ninety_seven_columns() {
        // Three R_256 rows at b = 16: 768 * 128 + 264 bits at 1024 bits per
        // column.
        let image = ImageShape {
            rows: 3,
            ring_dim: 256,
            modulus: SisModulusProfileId::Q128OffsetA7F7,
        };
        assert_eq!(
            hiding_randomness_columns(image, DigitAlphabet { log_basis: 4 }).unwrap(),
            97
        );
    }

    #[test]
    fn rejects_unsupported_alphabets() {
        let image = ImageShape {
            rows: 1,
            ring_dim: 64,
            modulus: SisModulusProfileId::Q64Offset59,
        };
        assert!(hiding_randomness_columns(image, DigitAlphabet { log_basis: 0 }).is_err());
        assert!(hiding_randomness_columns(image, DigitAlphabet { log_basis: 9 }).is_err());
    }
}
