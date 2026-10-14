//! Storage access for digit-based commitment and folding kernels.

use crate::sources::packed_digits::PackedSignedDigitView;
use crate::sources::poly_helpers::{
    I8DigitSpan, SignedDigitBasis, ValidatedSparseChallenge, WidePlan, WindowedChallenge,
};
use akita_algebra::ring::cyclotomic::BalancedDecomposePow2Params;
use akita_algebra::CyclotomicRing;
use akita_challenges::SparseChallenge;
use akita_error::AkitaError;
use akita_params::SignedDigitKernel;
use jolt_field::{CanonicalEncoding, Field};
use std::ops::Range;

#[derive(Clone, Copy)]
pub(crate) struct DigitShape {
    pub num_digits: usize,
    pub log_basis: u32,
}

pub(crate) trait RingValues<F: Field, const D: usize> {
    fn coeff(&self, k: usize) -> F;
}

impl<F: Field, const D: usize> RingValues<F, D> for &[F; D] {
    fn coeff(&self, k: usize) -> F {
        self[k]
    }
}

impl<F: Field, const D: usize> RingValues<F, D> for [i8; D] {
    fn coeff(&self, k: usize) -> F {
        F::from_i8(self[k])
    }
}

impl<F: Field, const D: usize> RingValues<F, D> for [i16; D] {
    fn coeff(&self, k: usize) -> F {
        F::from_i64(i64::from(self[k]))
    }
}

pub(crate) trait RingValueSource<F: Field, const D: usize>: Sync {
    type Values<'a>: RingValues<F, D> + Sync
    where
        Self: 'a;

    fn num_rings(&self) -> usize;
    fn values(&self, _ring: usize) -> Result<Self::Values<'_>, AkitaError> {
        Err(AkitaError::InvalidInput(
            "source does not supply ring values".into(),
        ))
    }
}

impl<F: Field, const D: usize> RingValueSource<F, D> for &[CyclotomicRing<F, D>] {
    type Values<'a>
        = &'a [F; D]
    where
        Self: 'a;

    fn num_rings(&self) -> usize {
        self.len()
    }
    fn values(&self, ring: usize) -> Result<Self::Values<'_>, AkitaError> {
        self.get(ring)
            .map(CyclotomicRing::coefficients)
            .ok_or_else(|| {
                AkitaError::InvalidInput("ring value index is outside the source".into())
            })
    }
}

pub(crate) trait BlockDigits<F: Field, const D: usize>: RingValueSource<F, D> {
    type Scratch: Send;
    type Planes<'a>: DigitPlaneSet<D>
    where
        Self: 'a;

    fn scratch(&self, shape: DigitShape, max_rings: usize) -> Self::Scratch;
    fn digit_abs_bound(&self, shape: DigitShape) -> u64;
    fn i8_digit_span(&self, shape: DigitShape) -> Option<I8DigitSpan>;
    fn digit_planes<'a>(
        &'a self,
        _ring: usize,
        _shape: DigitShape,
        _s: &'a mut Self::Scratch,
    ) -> Result<Self::Planes<'a>, AkitaError> {
        Err(AkitaError::InvalidInput(
            "source does not supply digit planes".into(),
        ))
    }
    fn i8_rows<'a>(
        &'a self,
        _rings: Range<usize>,
        _shape: DigitShape,
        _s: &'a mut Self::Scratch,
    ) -> Result<&'a [[i8; D]], AkitaError> {
        Err(AkitaError::InvalidInput(
            "source does not supply i8 rows".into(),
        ))
    }
    fn i16_rows<'a>(
        &'a self,
        _rings: Range<usize>,
        _shape: DigitShape,
        _s: &'a mut Self::Scratch,
    ) -> Result<&'a [[i16; D]], AkitaError> {
        Err(AkitaError::InvalidInput(
            "source does not supply i16 rows".into(),
        ))
    }
}

fn bounded_i8_digit_span(digit_abs_bound: u64) -> I8DigitSpan {
    if digit_abs_bound <= 63 {
        I8DigitSpan::Half
    } else {
        I8DigitSpan::Full
    }
}

pub(crate) struct CachedDigits<'a, const D: usize> {
    pub(crate) digit_planes: &'a [[i8; D]],
    pub(crate) num_rings: usize,
    pub(crate) digit_abs_bound: u64,
}

impl<'a, const D: usize> CachedDigits<'a, D> {
    pub(crate) fn new(
        digit_planes: &'a [[i8; D]],
        num_digits: usize,
        basis: SignedDigitBasis,
    ) -> Result<Self, AkitaError> {
        let num_rings = digit_planes.len().checked_div(num_digits).ok_or_else(|| {
            AkitaError::InvalidInput("cached fold digit count must be nonzero".into())
        })?;
        Ok(Self {
            digit_planes,
            num_rings,
            digit_abs_bound: basis.abs_bound.min(u64::from(i8::MIN.unsigned_abs())),
        })
    }
}

impl<F: Field, const D: usize> RingValueSource<F, D> for CachedDigits<'_, D> {
    type Values<'a>
        = [i8; D]
    where
        Self: 'a;
    fn num_rings(&self) -> usize {
        self.num_rings
    }
}

impl<F: Field, const D: usize> BlockDigits<F, D> for CachedDigits<'_, D> {
    type Scratch = ();
    type Planes<'a>
        = DigitPlanes<'a, D>
    where
        Self: 'a;
    fn digit_abs_bound(&self, _shape: DigitShape) -> u64 {
        self.digit_abs_bound
    }
    fn i8_digit_span(&self, _shape: DigitShape) -> Option<I8DigitSpan> {
        Some(bounded_i8_digit_span(self.digit_abs_bound))
    }
    fn scratch(&self, _shape: DigitShape, _max_rings: usize) {}
    fn digit_planes<'a>(
        &'a self,
        ring: usize,
        shape: DigitShape,
        _s: &'a mut (),
    ) -> Result<Self::Planes<'a>, AkitaError> {
        let start = ring * shape.num_digits;
        Ok(DigitPlanes::I8(
            &self.digit_planes[start..start + shape.num_digits],
        ))
    }
}

pub(crate) struct LiveRings<'a, F: Field + CanonicalEncoding, const D: usize> {
    pub(crate) coeffs: &'a [CyclotomicRing<F, D>],
    pub(crate) params: &'a BalancedDecomposePow2Params<F>,
    pub(crate) basis: SignedDigitBasis,
}

pub(crate) struct LiveDigitScratch<const D: usize> {
    i8: Vec<[i8; D]>,
    i16: Vec<[i16; D]>,
}

impl<F: Field + CanonicalEncoding, const D: usize> RingValueSource<F, D> for LiveRings<'_, F, D> {
    type Values<'a>
        = &'a [F; D]
    where
        Self: 'a;
    fn num_rings(&self) -> usize {
        self.coeffs.len()
    }
    fn values(&self, ring: usize) -> Result<Self::Values<'_>, AkitaError> {
        self.coeffs.values(ring)
    }
}

impl<F: Field + CanonicalEncoding, const D: usize> BlockDigits<F, D> for LiveRings<'_, F, D> {
    type Scratch = LiveDigitScratch<D>;
    type Planes<'a>
        = DigitPlanes<'a, D>
    where
        Self: 'a;
    fn digit_abs_bound(&self, _shape: DigitShape) -> u64 {
        self.basis.abs_bound
    }
    fn i8_digit_span(&self, _shape: DigitShape) -> Option<I8DigitSpan> {
        matches!(self.basis.kernel, SignedDigitKernel::I8).then(|| {
            if self.basis.abs_bound <= 64 {
                I8DigitSpan::Half
            } else {
                I8DigitSpan::Full
            }
        })
    }
    fn scratch(&self, shape: DigitShape, max_rings: usize) -> Self::Scratch {
        let width = shape.num_digits * max_rings;
        LiveDigitScratch {
            i8: if matches!(self.basis.kernel, SignedDigitKernel::I8) {
                vec![[0; D]; width]
            } else {
                Vec::new()
            },
            i16: vec![[0; D]; width],
        }
    }
    fn digit_planes<'a>(
        &'a self,
        ring: usize,
        shape: DigitShape,
        s: &'a mut Self::Scratch,
    ) -> Result<Self::Planes<'a>, AkitaError> {
        match self.basis.kernel {
            SignedDigitKernel::I8 => {
                let planes = &mut s.i8[..shape.num_digits];
                self.coeffs[ring].balanced_decompose_pow2_i8_into_with_params(planes, self.params);
                Ok(DigitPlanes::I8(planes))
            }
            SignedDigitKernel::I16 => {
                let planes = &mut s.i16[..shape.num_digits];
                self.coeffs[ring].balanced_decompose_pow2_i16_into(planes, self.params);
                Ok(DigitPlanes::I16(planes))
            }
        }
    }
    fn i8_rows<'a>(
        &'a self,
        rings: Range<usize>,
        shape: DigitShape,
        s: &'a mut Self::Scratch,
    ) -> Result<&'a [[i8; D]], AkitaError> {
        let width = rings.len() * shape.num_digits;
        for (ring, dst) in self.coeffs[rings]
            .iter()
            .zip(s.i8[..width].chunks_mut(shape.num_digits))
        {
            ring.balanced_decompose_pow2_i8_into_with_params(dst, self.params);
        }
        Ok(&s.i8[..width])
    }
    fn i16_rows<'a>(
        &'a self,
        rings: Range<usize>,
        shape: DigitShape,
        s: &'a mut Self::Scratch,
    ) -> Result<&'a [[i16; D]], AkitaError> {
        s.i16.fill([0; D]);
        for (ring, dst) in self.coeffs[rings]
            .iter()
            .zip(s.i16.chunks_mut(shape.num_digits))
        {
            ring.balanced_decompose_pow2_i16_into(dst, self.params);
        }
        Ok(&s.i16)
    }
}

pub(crate) struct PackedDigits<'a> {
    pub(crate) digits: PackedSignedDigitView<'a>,
    pub(crate) num_rings: usize,
    pub(crate) digit_abs_bound: u64,
}

impl<F: Field, const D: usize> RingValueSource<F, D> for PackedDigits<'_> {
    type Values<'a>
        = [i8; D]
    where
        Self: 'a;
    fn num_rings(&self) -> usize {
        self.num_rings
    }
}

impl<F: Field, const D: usize> BlockDigits<F, D> for PackedDigits<'_> {
    type Scratch = [i8; D];
    type Planes<'a>
        = &'a [i8; D]
    where
        Self: 'a;
    fn digit_abs_bound(&self, _shape: DigitShape) -> u64 {
        self.digit_abs_bound
    }
    fn i8_digit_span(&self, _shape: DigitShape) -> Option<I8DigitSpan> {
        Some(bounded_i8_digit_span(self.digit_abs_bound))
    }
    fn scratch(&self, _shape: DigitShape, _max_rings: usize) -> Self::Scratch {
        [0; D]
    }
    fn digit_planes<'a>(
        &'a self,
        ring: usize,
        shape: DigitShape,
        s: &'a mut Self::Scratch,
    ) -> Result<Self::Planes<'a>, AkitaError> {
        debug_assert_eq!(shape.num_digits, 1);
        self.digits.decode_range(ring * D, s)?;
        Ok(s)
    }
}

pub(crate) struct DigitRows<'a, const D: usize> {
    pub(crate) blocks: &'a [&'a [[i8; D]]],
    pub(crate) digit_abs_bound: u64,
}

impl<F: Field, const D: usize> RingValueSource<F, D> for DigitRows<'_, D> {
    type Values<'a>
        = [i8; D]
    where
        Self: 'a;
    fn num_rings(&self) -> usize {
        self.blocks.iter().map(|block| block.len()).sum()
    }
}

impl<F: Field, const D: usize> BlockDigits<F, D> for DigitRows<'_, D> {
    type Scratch = Vec<[i8; D]>;
    type Planes<'a>
        = DigitPlanes<'a, D>
    where
        Self: 'a;
    fn scratch(&self, shape: DigitShape, max_rings: usize) -> Self::Scratch {
        vec![[0; D]; shape.num_digits * max_rings]
    }
    fn digit_abs_bound(&self, _shape: DigitShape) -> u64 {
        self.digit_abs_bound
    }
    fn i8_digit_span(&self, _shape: DigitShape) -> Option<I8DigitSpan> {
        Some(bounded_i8_digit_span(self.digit_abs_bound))
    }
    fn i8_rows<'a>(
        &'a self,
        rings: Range<usize>,
        _shape: DigitShape,
        s: &'a mut Self::Scratch,
    ) -> Result<&'a [[i8; D]], AkitaError> {
        let mut offset = 0;
        let mut copied = 0;
        for block in self.blocks {
            let end = offset + block.len();
            if rings.start >= offset && rings.end <= end {
                return Ok(&block[rings.start - offset..rings.end - offset]);
            }
            let start = rings.start.max(offset);
            let stop = rings.end.min(end);
            if start < stop {
                let len = stop - start;
                s[copied..copied + len].copy_from_slice(&block[start - offset..stop - offset]);
                copied += len;
            }
            offset = end;
        }
        Ok(&s[..copied])
    }
}
/// A ring's digit planes, independent of how the source supplied them.
pub(crate) enum DigitPlanes<'a, const D: usize> {
    I8(&'a [[i8; D]]),
    I16(&'a [[i16; D]]),
}

pub(crate) trait DigitPlaneSet<const D: usize> {
    fn accumulate_wide(
        self,
        acc: &mut [[i32; D]],
        challenge: ValidatedSparseChallenge<'_, D>,
        plan: &WidePlan<D>,
    );
    fn accumulate_narrow(
        self,
        acc: &mut [[i16; D]],
        challenge: &SparseChallenge,
        windowed: Option<&WindowedChallenge>,
    );
    fn accumulate_chunked_narrow(
        self,
        narrow: &mut [[i16; D]],
        wide: &mut [[i32; D]],
        challenge: &SparseChallenge,
        term_ranges: &[Range<usize>],
    );
}
