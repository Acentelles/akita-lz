//! Precomputed lookup table for folding pairs of small integer values.
//!
//! Used by Akita stage sumchecks for compact-witness folding.

use akita_error::AkitaError;
use jolt_field::Unreduced;
use jolt_field::{Field, Ring};

/// Precomputed lookup table for folding pairs of small integer values at a
/// fixed challenge `r`.
///
/// This is useful for the round-0 compact tables in Akita's stage-1 and
/// stage-2 sumchecks: the table entries are small integers, the fold formula is
/// always `left + r * (right - left)`, and the set of possible `(left, right)`
/// pairs is tiny.
pub struct CompactPairFoldLut<E: Field> {
    min_value: i16,
    pair_values: Vec<E>,
    num_values: usize,
}

impl<E: Field + Ring + Unreduced> CompactPairFoldLut<E> {
    /// Build a lookup table for every integer in `min_value..=max_value`.
    ///
    /// # Errors
    ///
    /// Returns [`AkitaError::Internal`] if `min_value > max_value`.
    pub fn from_contiguous_range(min_value: i16, max_value: i16, r: E) -> Result<Self, AkitaError> {
        if min_value > max_value {
            return Err(AkitaError::Internal("invalid compact fold range".into()));
        }
        let num_values = (i32::from(max_value) - i32::from(min_value) + 1) as usize;
        let mut pair_values = Vec::with_capacity(num_values * num_values);
        for left in min_value..=max_value {
            let left_field = E::from_i64(i64::from(left));
            for right in min_value..=max_value {
                let delta = i64::from(right) - i64::from(left);
                let delta_abs = delta.unsigned_abs();
                let r_delta = E::reduce_small_product(r.mul_u64_unreduced(delta_abs));
                pair_values.push(if delta < 0 {
                    left_field - r_delta
                } else {
                    left_field + r_delta
                });
            }
        }
        Ok(Self {
            min_value,
            pair_values,
            num_values,
        })
    }
}

impl<E: Field> CompactPairFoldLut<E> {
    #[inline]
    fn index_of(&self, value: i16) -> usize {
        let index = (i32::from(value) - i32::from(self.min_value)) as usize;
        debug_assert!(
            index < self.num_values,
            "value missing from compact fold LUT"
        );
        index
    }

    /// Fold the pair `(left, right)` at the challenge used to construct this
    /// table.
    #[inline]
    pub fn fold(&self, left: i16, right: i16) -> E {
        let left_idx = self.index_of(left);
        let right_idx = self.index_of(right);
        self.pair_values[left_idx * self.num_values + right_idx]
    }
}
