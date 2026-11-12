use super::*;
use jolt_field::{
    CanonicalBytes, FpExt4, FpExt8, Prime128Offset6A23801F as Wide, Prime32Offset99 as Small,
};
use std::sync::atomic::{AtomicUsize, Ordering};

// The previous receiver, deliberately independent of the new mode selection.
fn old_receive<T: Encoding + FromNarg + Copy>(
    state: &mut VerifierChannel<'_>,
    count: usize,
) -> Result<Vec<T>, AkitaError> {
    let mut atoms = Vec::new();
    if atoms.try_reserve_exact(count).is_err() {
        return Err(reject(state));
    }
    if count == 0 {
        return Ok(atoms);
    }
    state
        .prover_message_with(
            move |reader| {
                for _ in 0..count {
                    atoms.push(T::from_narg(reader)?);
                }
                Ok(atoms)
            },
            |atoms| word_padded(&encode_atoms(atoms.iter().copied())),
        )
        .map_err(invalid_proof)
}

fn old_fields<F: CanonicalEncoding>(
    state: &mut VerifierChannel<'_>,
    count: usize,
) -> Result<Vec<F>, AkitaError> {
    let record = group_record(
        ProtocolSiteId::default(),
        ProtocolMessageKind::ProofAtoms,
        Some(count),
        F::NUM_BYTES,
    );
    let record = state.guard(record)?;
    verifier_context(state, record);
    Ok(old_receive::<FieldAtom<F>>(state, count)?
        .into_iter()
        .map(FieldAtom::into_inner)
        .collect())
}

fn old_extensions<F: Field + CanonicalEncoding, E: ExtField<F>>(
    state: &mut VerifierChannel<'_>,
    values: &mut [E],
) -> Result<(), AkitaError> {
    let record = group_record(
        ProtocolSiteId::default(),
        ProtocolMessageKind::ProofAtoms,
        values.len().checked_mul(E::DEGREE),
        F::NUM_BYTES,
    );
    let record = state.guard(record)?;
    state.context(record);
    let mut atoms: Vec<ExtensionAtom<F, E>> = values
        .iter()
        .map(|&value| ExtensionAtom::new(value))
        .collect();
    let received = old_receive::<ExtensionAtom<F, E>>(state, atoms.len())?;
    for (slot, value) in atoms.iter_mut().zip(received) {
        *slot = value;
    }
    for (slot, atom) in values.iter_mut().zip(atoms) {
        *slot = atom.into_inner();
    }
    Ok(())
}

fn channel(proof: &[u8], squeezed: bool, poisoned: bool) -> VerifierChannel<'_> {
    let mut state = new_verifier_channel(b"canonical-receive-differential", proof);
    state.public_message_as(b"preceding message", |bytes| bytes);
    if squeezed {
        let _ = state.verifier_message::<[u8; 157]>();
    }
    if poisoned {
        let _ = reject(&mut state);
    }
    state
}

fn finish(mut state: VerifierChannel<'_>) -> (Vec<u8>, Option<Vec<u8>>) {
    // Directly inspect sponge continuation even on a rejected reader.
    let squeezed = state.verifier_message_as(401, <[u8]>::to_vec);
    let unread = state.into_narg_string().ok().map(<[u8]>::to_vec);
    (squeezed, unread)
}

fn compare_fields<F: CanonicalEncoding>(
    proof: &[u8],
    count: usize,
    squeezed: bool,
    poisoned: bool,
) {
    let mut old = channel(proof, squeezed, poisoned);
    let mut new = channel(proof, squeezed, poisoned);
    let expected = old_fields::<F>(&mut old, count);
    let actual = receive_field_group::<F>(&mut new, ProtocolSiteId::default(), count);
    assert_eq!(actual, expected);
    assert_eq!(finish(new), finish(old));
}

fn compare_extensions<F: Field + CanonicalEncoding, E: ExtField<F>>(
    proof: &[u8],
    count: usize,
    squeezed: bool,
    poisoned: bool,
) {
    let mut old = channel(proof, squeezed, poisoned);
    let mut new = channel(proof, squeezed, poisoned);
    let initial = vec![E::one(); count];
    let mut expected_values = initial.clone();
    let mut actual_values = initial.clone();
    let expected = old_extensions::<F, E>(&mut old, &mut expected_values);
    let actual = exchange_extension_group::<F, E, _>(
        &mut new,
        ProtocolSiteId::default(),
        &mut actual_values,
    );
    assert_eq!(actual, expected);
    assert_eq!(actual_values, expected_values);
    if actual.is_err() {
        assert_eq!(actual_values, initial);
    }
    assert_eq!(finish(new), finish(old));
}

fn next_random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn exercise_wire(
    wire: &[u8],
    coefficient_bytes: usize,
    mut compare: impl FnMut(&[u8], bool, bool),
) {
    for squeezed in [false, true] {
        for poisoned in [false, true] {
            compare(wire, squeezed, poisoned);
            let mut with_suffix = wire.to_vec();
            with_suffix.extend_from_slice(b"unread suffix");
            compare(&with_suffix, squeezed, poisoned);
        }
    }
    for cut in 0..wire.len() {
        compare(&wire[..cut], cut % 2 == 0, false);
    }
    for offset in (0..wire.len()).step_by(coefficient_bytes) {
        let mut noncanonical = wire.to_vec();
        noncanonical[offset..offset + coefficient_bytes].fill(0xff);
        compare(&noncanonical, true, false);
        // The first bad coefficient must still win over later truncation.
        compare(&noncanonical[..offset + coefficient_bytes], false, false);
    }
}

fn exercise_fields<F: Field + CanonicalEncoding>() {
    let mut seed = 0x84b1_529d_c670_3fa5;
    assert!(F::from_bytes_le_checked(&vec![0xff; F::NUM_BYTES]).is_none());
    for count in [0, 1, 2, 3, 7, 8, 9, 16, 17, 34, 35] {
        let values: Vec<F> = (0..count)
            .map(|_| {
                F::from_u128_reduced(
                    (u128::from(next_random(&mut seed)) << 64) | u128::from(next_random(&mut seed)),
                )
            })
            .collect();
        let wire = encode_atoms(values.iter().copied().map(FieldAtom::new));
        exercise_wire(&wire, F::NUM_BYTES, |proof, squeezed, poisoned| {
            compare_fields::<F>(proof, count, squeezed, poisoned);
        });
    }
}

fn exercise_extensions<F: Field + CanonicalEncoding, E: ExtField<F>>() {
    let mut seed = 0x976d_0132_8aef_54bc;
    for count in [0, 1, 2, 3, 7, 8, 9, 17] {
        let values: Vec<E> = (0..count)
            .map(|_| {
                E::from_base_fn(|_| {
                    F::from_u128_reduced(
                        (u128::from(next_random(&mut seed)) << 64)
                            | u128::from(next_random(&mut seed)),
                    )
                })
            })
            .collect();
        let wire = encode_atoms(values.iter().copied().map(ExtensionAtom::<F, E>::new));
        exercise_wire(&wire, F::NUM_BYTES, |proof, squeezed, poisoned| {
            compare_extensions::<F, E>(proof, count, squeezed, poisoned);
        });
        let mut reference = new_prover_channel(b"unchanged-extension-prover");
        send_atoms(
            &mut reference,
            values.iter().copied().map(ExtensionAtom::<F, E>::new),
        );
        let mut actual = new_prover_channel(b"unchanged-extension-prover");
        let mut slots = values.clone();
        exchange_extension_group::<F, E, _>(&mut actual, ProtocolSiteId::default(), &mut slots)
            .unwrap();
        assert_eq!(slots, values);
        assert_eq!(actual.narg_string(), reference.narg_string());
        assert_eq!(
            actual.verifier_message::<[u8; 401]>(),
            reference.verifier_message::<[u8; 401]>()
        );
    }
}

#[test]
fn base_receivers_match_for_valid_malformed_and_poisoned_inputs() {
    exercise_fields::<Small>();
    exercise_fields::<Wide>();
}

#[test]
fn extension_receivers_and_prover_match_across_widths() {
    exercise_extensions::<Small, Small>();
    exercise_extensions::<Small, FpExt4<Small>>();
    exercise_extensions::<Wide, Wide>();
    exercise_extensions::<Wide, FpExt8<Wide>>();
}

#[test]
fn overflow_and_reservation_errors_match_before_reading() {
    for squeezed in [false, true] {
        for poisoned in [false, true] {
            compare_fields::<Small>(&[0; 32], usize::MAX, squeezed, poisoned);
            // The byte count fits usize but exceeds Vec's isize::MAX limit.
            let count = isize::MAX as usize / std::mem::size_of::<Small>() + 1;
            compare_fields::<Small>(&[0; 32], count, squeezed, poisoned);
            let mut old = channel(&[0; 32], squeezed, poisoned);
            let mut new = channel(&[0; 32], squeezed, poisoned);
            let expected = old_receive::<ExtensionAtom<Small, Small>>(&mut old, count);
            let actual = receive_atoms::<ExtensionAtom<Small, Small>>(
                &mut new,
                count,
                count.checked_mul(Small::NUM_BYTES),
            );
            assert_eq!(actual, expected);
            assert_eq!(actual, Err(AkitaError::InvalidProof));
            assert_eq!(finish(new), finish(old));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Triple([u8; 3]);

static CUSTOM_ENCODINGS: AtomicUsize = AtomicUsize::new(0);

impl Encoding for Triple {
    fn encode(&self) -> impl AsRef<[u8]> {
        CUSTOM_ENCODINGS.fetch_add(1, Ordering::Relaxed);
        self.0
    }
}

impl FromNarg for Triple {
    fn from_narg(reader: &mut NargReader<'_>) -> Result<Self, VerificationError> {
        Ok(Self(reader.take_array::<3>()?))
    }
}

#[test]
fn custom_atoms_keep_generic_padded_exchange() {
    for count in [0, 1, 2, 3, 8, 9] {
        let wire: Vec<u8> = (0..count * 3).map(|index| index as u8).collect();
        for cut in 0..=wire.len() {
            let proof = &wire[..cut];
            let mut old = channel(proof, true, false);
            let mut new = channel(proof, true, false);
            let initial = vec![Triple([0xa5; 3]); count];
            let mut slots = initial.clone();
            CUSTOM_ENCODINGS.store(0, Ordering::Relaxed);
            let expected = old_receive::<Triple>(&mut old, count);
            let expected_encodings = CUSTOM_ENCODINGS.swap(0, Ordering::Relaxed);
            let actual = new.exchange(&mut slots);
            assert_eq!(
                CUSTOM_ENCODINGS.swap(0, Ordering::Relaxed),
                expected_encodings
            );
            assert_eq!(actual, expected.as_ref().map(|_| ()).map_err(Clone::clone));
            match expected {
                Ok(values) => assert_eq!(slots, values),
                Err(_) => assert_eq!(slots, initial),
            }
            assert_eq!(finish(new), finish(old));
        }
    }
}
