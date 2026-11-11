use super::CenteredI16NttConverter;
use crate::ntt::prime::{MontCoeff, NttPrime};
use crate::ntt::tables::I32_RAW_PRIMES;
use crate::ring::crt_ntt_repr::{CrtNttParamSet, CyclotomicCrtNtt};
use jolt_inlines_ntt::forward_ntt64;

#[test]
fn prepared_converter_admits_six_primes_and_preserves_raw_inline_words() {
    let params = CrtNttParamSet::<i32, 6, 64>::new(I32_RAW_PRIMES.map(NttPrime::compute));
    for digits in [
        [i16::MIN; 64],
        [i16::MAX; 64],
        core::array::from_fn(|i| if i & 1 == 0 { i16::MIN } else { i16::MAX }),
    ] {
        let converter = CenteredI16NttConverter::new_inline(&params);
        assert_eq!(converter.prepared.as_ref().map(Vec::len), Some(6));
        let mut actual = CyclotomicCrtNtt::zero();
        converter.transform_into(&digits, &mut actual);
        for (index, (prime, tw)) in params.primes.iter().zip(params.twiddles.iter()).enumerate() {
            let mut expected = digits.map(i32::from);
            forward_ntt64(
                &mut expected,
                &tw.psi_pows_r2,
                &tw.fwd_twiddles.map(MontCoeff::raw),
                prime.p,
                prime.pinv,
            );
            assert_eq!(actual.limbs[index].map(MontCoeff::raw), expected);
        }
    }
}

#[test]
fn prepared_converter_rejection_keeps_each_raw_wrapping_fallback() {
    let original = CrtNttParamSet::<i32, 1, 64>::new([NttPrime::compute(I32_RAW_PRIMES[0])]);
    let digits = core::array::from_fn(|i| if i & 1 == 0 { i16::MIN } else { i16::MAX });
    for case in 0..9 {
        let mut params = original.clone();
        match case {
            0 => params.primes[0].p = 1 << 30,
            1 => params.primes[0].p -= 1,
            2 => params.primes[0].pinv ^= 1,
            3 => params.twiddles[0].psi_pows_r2[63] = params.primes[0].p,
            4 => params.twiddles[0].fwd_twiddles[63] = MontCoeff::from_raw(-params.primes[0].p),
            5 => params.twiddles[0].fwd_twiddles[0] = MontCoeff::from_raw(0),
            6 => params.primes[0].p = 0,
            7 => params.primes[0].p = -1,
            _ => params.primes[0].p = i32::MAX,
        }
        let converter = CenteredI16NttConverter::new_inline(&params);
        assert!(converter.prepared.is_none(), "case {case}");
        let mut actual = CyclotomicCrtNtt::zero();
        converter.transform_into(&digits, &mut actual);
        let mut expected = digits.map(i32::from);
        let prime = params.primes[0];
        let tw = &params.twiddles[0];
        forward_ntt64(
            &mut expected,
            &tw.psi_pows_r2,
            &tw.fwd_twiddles.map(MontCoeff::raw),
            prime.p,
            prime.pinv,
        );
        assert_eq!(actual.limbs[0].map(MontCoeff::raw), expected, "case {case}");
    }
}

#[test]
fn prepared_converter_keeps_raw_fallback_for_nonroot_tables() {
    let original = CrtNttParamSet::<i32, 1, 64>::new([NttPrime::compute(I32_RAW_PRIMES[0])]);
    let digits = core::array::from_fn(|i| if i & 1 == 0 { i16::MIN } else { i16::MAX });
    for case in 0..3 {
        let mut params = original.clone();
        match case {
            0 => params.twiddles[0].psi_pows_r2[0] = 0,
            1 => params.twiddles[0].psi_pows_r2[63] = 0,
            _ => params.twiddles[0].fwd_twiddles[62] = MontCoeff::from_raw(0),
        }
        let converter = CenteredI16NttConverter::new_inline(&params);
        assert!(converter.prepared.is_none());
        let mut actual = CyclotomicCrtNtt::zero();
        converter.transform_into(&digits, &mut actual);
        let mut expected = digits.map(i32::from);
        let prime = params.primes[0];
        let tw = &params.twiddles[0];
        forward_ntt64(
            &mut expected,
            &tw.psi_pows_r2,
            &tw.fwd_twiddles.map(MontCoeff::raw),
            prime.p,
            prime.pinv,
        );
        assert_eq!(actual.limbs[0].map(MontCoeff::raw), expected, "case {case}");
    }
}
