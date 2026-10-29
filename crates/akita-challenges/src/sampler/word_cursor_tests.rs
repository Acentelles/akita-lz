use super::*;
use shake::digest::{ExtendableOutput, Update, XofReader};
use shake::Shake256;

#[test]
fn word_reads_match_canonical_shake_at_every_lane_and_rate_boundary() {
    for seed in [0u8, 1, 0x5a, 0xff] {
        for index in [0, 1, 137, u64::MAX] {
            let prefix = IndexedXofPrefix::new(&[seed; 32]).unwrap();
            let mut xof = Shake256::default();
            xof.update(&[seed; 32]);
            xof.update(&index.to_le_bytes());
            let mut bytes = [0u8; 280];
            xof.finalize_xof().read(&mut bytes);
            for offset in 0..272 {
                let mut cursor = XofCursor::from_indexed_prefix(&prefix, index);
                cursor.fill_bytes(&mut vec![0u8; offset]);
                let got = cursor.next_u16();
                assert_eq!(got.to_le_bytes(), bytes[offset..offset + 2]);
                assert_eq!(cursor.next_u8(), bytes[offset + 2]);

                let mut cursor = XofCursor::from_indexed_prefix(&prefix, index);
                cursor.fill_bytes(&mut vec![0u8; offset]);
                let got = cursor.next_u32();
                assert_eq!(got.to_le_bytes(), bytes[offset..offset + 4]);
                assert_eq!(cursor.next_u8(), bytes[offset + 4]);
            }
        }
    }
}

#[test]
fn word_rejection_matches_byte_reference_and_preserves_stream_position() {
    for seed in 0..32u8 {
        let prefix = IndexedXofPrefix::new(&[seed; 32]).unwrap();
        let mut cursor = XofCursor::from_indexed_prefix(&prefix, u64::from(seed));
        let mut xof = Shake256::default();
        xof.update(&[seed; 32]);
        xof.update(&u64::from(seed).to_le_bytes());
        let mut reader = xof.finalize_xof();
        for _ in 0..16 {
            for modulus in [
                1usize, 2, 3, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1024, 1025,
                2048, 65535, 65536, 65537,
            ] {
                let expected = if modulus == 1 {
                    0
                } else {
                    let mask = modulus.next_power_of_two() - 1;
                    let width = if modulus <= 256 {
                        1
                    } else if modulus <= 65536 {
                        2
                    } else {
                        4
                    };
                    loop {
                        let mut bytes = [0u8; 4];
                        reader.read(&mut bytes[..width]);
                        let value = u32::from_le_bytes(bytes) as usize & mask;
                        if value < modulus {
                            break value;
                        }
                    }
                };
                assert_eq!(cursor.next_usize_mod(modulus), expected);
            }
        }
        let mut actual_tail = [0u8; 277];
        let mut expected_tail = [0u8; 277];
        cursor.fill_bytes(&mut actual_tail);
        reader.read(&mut expected_tail);
        assert_eq!(actual_tail, expected_tail);
    }
}

#[test]
fn word_signs_match_scalar_with_exact_consumption_at_all_boundaries() {
    for seed in 0..32u8 {
        let prefix = IndexedXofPrefix::new(&[seed; 32]).unwrap();
        for offset in [0, 1, 6, 7, 8, 127, 128, 129, 135, 136, 137] {
            for len in [0, 1, 2, 7, 8, 9, 31, 42, 63, 64, 65, 137, 257] {
                for split in [0, len / 2, len] {
                    let mut cursor = XofCursor::from_indexed_prefix(&prefix, u64::from(seed));
                    let mut reference = XofCursor::from_indexed_prefix(&prefix, u64::from(seed));
                    let mut skip = vec![0u8; offset];
                    cursor.fill_bytes(&mut skip);
                    reference.fill_bytes(&mut skip);
                    let mut bytes = vec![0u8; len];
                    reference.fill_bytes(&mut bytes);
                    let mut actual = vec![0i8; len];
                    let (ones, twos) = actual.split_at_mut(split);
                    cursor.fill_signed_bytes::<1>(ones);
                    cursor.fill_signed_bytes::<2>(twos);
                    let expected = bytes
                        .iter()
                        .enumerate()
                        .map(|(i, byte)| {
                            let magnitude = if i < split { 1 } else { 2 };
                            if byte & 1 == 0 {
                                magnitude
                            } else {
                                -magnitude
                            }
                        })
                        .collect::<Vec<i8>>();
                    assert_eq!(actual, expected);
                    let mut got_tail = [0u8; 17];
                    let mut ref_tail = [0u8; 17];
                    cursor.fill_bytes(&mut got_tail);
                    reference.fill_bytes(&mut ref_tail);
                    assert_eq!(got_tail, ref_tail);
                }
            }
        }
    }
    for pattern in 0..=255u64 {
        let word = (0..8).fold(0u64, |word, lane| {
            word | (((pattern >> lane) & 1) << (8 * lane))
        });
        for high in [0, 0xfefe_fefe_fefe_fefe] {
            let mut cursor = XofCursor {
                state: [word | high; 25],
                pos: 0,
            };
            let mut ones = [0i8; 8];
            let mut twos = [0i8; 8];
            cursor.fill_signed_bytes::<1>(&mut ones);
            cursor.fill_signed_bytes::<2>(&mut twos);
            for (lane, (one, two)) in ones.iter().zip(twos).enumerate() {
                let sign = if (pattern >> lane) & 1 == 0 { 1 } else { -1 };
                assert_eq!(*one, sign);
                assert_eq!(two, 2 * sign);
            }
        }
    }
}
