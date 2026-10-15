use akita_algebra::CyclotomicRing;
use jolt_field::Field;

/// Internal dense CPU commit representation.
pub(crate) enum DenseCommitInput<'a, F: Field, const D: usize> {
    /// Ring coefficients need backend-side digit decomposition.
    CoeffBlocks {
        /// Per-block coefficient slices.
        block_slices: Vec<&'a [CyclotomicRing<F, D>]>,
        /// Number of balanced digits used for the A-side commit.
        num_digits_inner: usize,
        /// Logarithm of the gadget basis.
        log_basis_inner: u32,
    },
}

/// Named ring-switch relation rows returned by a backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RingSwitchRelationRows<F: Field, const D: usize> {
    /// D-side negacyclic reduced rows used as the transcript-visible `v`.
    pub d_negacyclic: Vec<CyclotomicRing<F, D>>,
    /// D-side cyclic rows.
    pub d_cyclic: Vec<CyclotomicRing<F, D>>,
    /// B-side cyclic rows.
    pub b_cyclic: Vec<CyclotomicRing<F, D>>,
    /// A-side quotient rows.
    pub a_quotients: Vec<CyclotomicRing<F, D>>,
}
