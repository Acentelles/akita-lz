//! Proof-stream codec, channel and known-answer tests.

use super::*;
use jolt_field::{
    CanonicalBytes, Prime128OffsetA7F7, Prime32Offset99 as F, Prime48Offset59, Prime64Offset59,
    Ring,
};

#[test]
fn record_count_products_reject_argument_overflow_before_receipt() {
    let site = ProtocolSiteId::default();
    let kind = ProtocolMessageKind::ProofAtoms;
    assert!(matches!(
        extension_group_record::<F, jolt_field::FpExt4<F>>(site, kind, usize::MAX),
        Err(AkitaError::InvalidInput(message)) if message.contains("atom count overflows")
    ));
    assert!(matches!(
        extension_group_record::<F, jolt_field::FpExt4<F>>(
            site, kind, usize::MAX / 4,
        ),
        Err(AkitaError::InvalidInput(message)) if message.contains("extension record encoded")
    ));
    assert!(matches!(
        field_group_record::<F>(site, kind, usize::MAX),
        Err(AkitaError::InvalidInput(message)) if message.contains("field record encoded")
    ));
    let mut verifier = new_verifier_channel(b"count-overflow", b"fixture", &[]).unwrap();
    assert!(matches!(
        receive_field_group::<F>(&mut verifier, site, usize::MAX),
        Err(AkitaError::InvalidInput(message)) if message.contains("field record encoded")
    ));
}

#[test]
fn atom_bytes_encode_inline_and_spilled_lengths() {
    for len in [0, 16, 64, 65, 200] {
        let expected: Vec<u8> = (0..len).map(|index| (index * 7 + 1) as u8).collect();
        let encoded = AtomBytes::with_len(len, |out| out.copy_from_slice(&expected));
        assert_eq!(matches!(encoded, AtomBytes::Inline { .. }), len <= 64);
        assert_eq!(encoded.as_ref(), expected.as_slice());
    }
}

#[test]
fn public_absorption_never_extends_the_argument_string() {
    let mut prover = new_prover_channel(b"public-size", b"fixture").unwrap();
    prover.prover_message(&[7u8; 3]);
    let before = prover.narg_string().to_vec();
    prover.public_message(b"public fold payload");
    assert_eq!(prover.narg_string(), before);
}

#[cfg(all(feature = "transcript-blake2b", not(feature = "terminal-quotient")))]
#[test]
fn blake2b_transcript_has_a_cross_width_known_answer() {
    let mut prover = new_prover_channel(b"cross-width/session", b"cross-width/instance").unwrap();
    prover.public_message(b"public-message");
    let challenge = prover.verifier_message::<[u8; 32]>();
    assert_eq!(
        challenge,
        [
            174, 120, 165, 116, 108, 247, 146, 33, 182, 143, 40, 94, 150, 167, 244, 49, 71, 31, 91,
            26, 214, 49, 240, 75, 87, 142, 44, 99, 169, 37, 81, 104,
        ]
    );
}

#[test]
fn exact_rejection_sampling_covers_every_production_field() {
    assert!(field_sampling_is_certified(
        F::NUM_BYTES,
        F::MODULUS_BITS,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert!(field_sampling_is_certified(
        Prime64Offset59::NUM_BYTES,
        Prime64Offset59::MODULUS_BITS,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert!(field_sampling_is_certified(
        Prime128OffsetA7F7::NUM_BYTES,
        Prime128OffsetA7F7::MODULUS_BITS,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert_eq!(field_challenge_bytes::<F>(), 4);
    assert_eq!(field_challenge_bytes::<Prime64Offset59>(), 8);
    assert_eq!(field_challenge_bytes::<Prime128OffsetA7F7>(), 16);
}

#[test]
fn exact_rejection_sampling_rejects_unsupported_metadata() {
    assert!(!field_sampling_is_certified(
        0,
        1,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert!(!field_sampling_is_certified(
        FIELD_CHALLENGE_BYTES as usize + 1,
        1,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert!(!field_sampling_is_certified(
        8,
        48,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert!(!field_sampling_is_certified(
        8,
        65,
        FIELD_SAMPLING_QUERY_LIMIT,
    ));
    assert!(!field_sampling_is_certified(
        8,
        64,
        FIELD_SAMPLING_QUERY_LIMIT + 1,
    ));

    let mut prover = new_prover_channel(b"unsupported-field", b"fixture").unwrap();
    assert!(matches!(
        prover_field_challenge::<Prime48Offset59>(&mut prover),
        Err(AkitaError::InvalidSetup(_))
    ));

    let mut verifier = new_verifier_channel(b"unsupported-field", b"fixture", &[]).unwrap();
    assert!(matches!(
        verifier_field_challenge::<Prime48Offset59>(&mut verifier),
        Err(AkitaError::InvalidSetup(_))
    ));
    assert!(matches!(
        verifier_field_challenge::<F>(&mut verifier),
        Err(AkitaError::InvalidProof)
    ));
    assert!(matches!(
        verifier.check_eof(),
        Err(AkitaError::InvalidProof)
    ));
}

#[test]
fn field_roundtrip_and_eof() {
    let mut prover = new_prover_channel(b"session", b"instance").unwrap();
    send_field(&mut prover, F::from_u64(42));
    let proof = prover.narg_string().to_vec();

    let mut verifier = new_verifier_channel(b"session", b"instance", &proof).unwrap();
    assert_eq!(receive_field::<F>(&mut verifier).unwrap(), F::from_u64(42));
    assert!(verifier.check_eof().is_ok());
}

#[test]
fn chunked_bytes_match_bytewise_proof_and_transcript() {
    for len in [0, 1, 63, 64, 65, 1023, 1024, 1025, 21_066] {
        let bytes = (0..len)
            .map(|index| (index as u8).wrapping_mul(29).wrapping_add(7))
            .collect::<Vec<_>>();
        let site = ProtocolSiteId {
            family: SITE_FAMILY_TERMINAL,
            level: 6,
            round: 3,
            ..ProtocolSiteId::default()
        };

        let mut chunked = new_prover_channel(b"chunked-bytes", b"instance").unwrap();
        send_bytes(&mut chunked, &bytes);
        public_bytes(&mut chunked, site, &bytes).unwrap();
        let chunked_challenge = chunked.verifier_message::<[u8; 32]>();

        let mut bytewise = new_prover_channel(b"chunked-bytes", b"instance").unwrap();
        for &byte in &bytes {
            bytewise.prover_message(&[byte]);
        }
        prover_context(
            &mut bytewise,
            public_bytes_record(site, bytes.len()).unwrap(),
        );
        for &byte in &bytes {
            bytewise.public_message(&[byte]);
        }
        let bytewise_challenge = bytewise.verifier_message::<[u8; 32]>();

        assert_eq!(chunked.narg_string(), bytes);
        assert_eq!(chunked.narg_string(), bytewise.narg_string());
        assert_eq!(chunked_challenge, bytewise_challenge);

        let proof = chunked.narg_string().to_vec();
        let mut verifier = new_verifier_channel(b"chunked-bytes", b"instance", &proof).unwrap();
        assert_eq!(receive_bytes(&mut verifier, len).unwrap(), bytes);
        public_bytes(&mut verifier, site, &bytes).unwrap();
        assert_eq!(
            verifier.verifier_message::<[u8; 32]>().unwrap(),
            chunked_challenge
        );
        verifier.check_eof().unwrap();
    }
}

#[test]
fn extension_groups_share_context_and_fixed_shape() {
    type E = jolt_field::FpExt4<F>;

    let public = [E::from_u64(3), E::from_u64(5)];
    let private = [E::from_u64(8), E::from_u64(13)];
    let public_site = ProtocolSiteId {
        family: 27,
        stage: 1,
        ..ProtocolSiteId::default()
    };
    let private_site = ProtocolSiteId {
        family: 27,
        stage: 2,
        ..ProtocolSiteId::default()
    };
    let mut prover = new_prover_channel(b"groups", b"fixture").unwrap();
    public_extensions::<F, E, _>(&mut prover, public_site, &public).unwrap();
    let mut sent = private;
    exchange_extension_group::<F, E, _>(&mut prover, private_site, &mut sent).unwrap();
    let prover_challenge = ext_challenge::<F, E, _>(
        &mut prover,
        ProtocolSiteId {
            family: 27,
            stage: 3,
            ..ProtocolSiteId::default()
        },
    )
    .unwrap();
    let proof = prover.narg_string().to_vec();
    assert_eq!(proof.len(), private.len() * E::DEGREE * F::NUM_BYTES);

    let mut verifier = new_verifier_channel(b"groups", b"fixture", &proof).unwrap();
    public_extensions::<F, E, _>(&mut verifier, public_site, &public).unwrap();
    let mut received = [<E as jolt_field::Zero>::zero(); 2];
    exchange_extension_group::<F, E, _>(&mut verifier, private_site, &mut received).unwrap();
    assert_eq!(received, private);
    let verifier_challenge = ext_challenge::<F, E, _>(
        &mut verifier,
        ProtocolSiteId {
            family: 27,
            stage: 3,
            ..ProtocolSiteId::default()
        },
    )
    .unwrap();
    assert_eq!(verifier_challenge, prover_challenge);
    assert!(verifier.check_eof().is_ok());
}

#[test]
fn fold_preview_matches_live_prover_and_verifier() {
    let nonce_site = ProtocolSiteId {
        family: SITE_FAMILY_FOLD_CHALLENGE,
        level: 3,
        detail: 12,
        ..ProtocolSiteId::default()
    };
    let nonce_record = ProtocolContextRecord::new(
        nonce_site.to_bytes(),
        ProtocolMessageKind::FoldResponseNonce as u32,
        1,
        nonce_max_bytes(12) as u64,
        0,
    );
    let root_record = ProtocolContextRecord::new(
        ProtocolSiteId {
            family: SITE_FAMILY_FOLD_CHALLENGE,
            level: 3,
            group: 2,
            ..ProtocolSiteId::default()
        }
        .to_bytes(),
        ProtocolMessageKind::Challenge as u32,
        3,
        3,
        crate::FOLD_CHALLENGE_SEED_LEN as u64,
    );
    let payload = [5, 8, 13];
    let nonce = 7u32;
    let mut prover = new_prover_channel(b"fold-preview", b"fixture").unwrap();
    let preview = FoldPreview::new(&prover, nonce).fold_root(&payload);
    prover_context(&mut prover, nonce_record);
    prover.prover_message(&NonceAtom::new(nonce));
    let live = prover_fold_root(&mut prover, root_record, &payload);
    assert_eq!(preview, live);
    let proof = prover.narg_string().to_vec();
    assert_eq!(proof.len(), 1);

    let mut verifier = new_verifier_channel(b"fold-preview", b"fixture", &proof).unwrap();
    verifier_context(&mut verifier, nonce_record);
    assert_eq!(
        verifier.prover_message::<NonceAtom>().unwrap().into_inner(),
        nonce
    );
    assert_eq!(
        verifier_fold_root(&mut verifier, root_record, &payload).unwrap(),
        live
    );
    verifier.check_eof().unwrap();
}

#[test]
fn field_failure_does_not_consume_cursor() {
    let noncanonical = vec![u8::MAX; F::NUM_BYTES];
    let mut bytes = noncanonical.as_slice();
    let original = bytes;
    assert!(FieldAtom::<F>::deserialize_from_narg(&mut bytes).is_err());
    assert_eq!(bytes, original);
}

#[test]
fn extension_failure_does_not_consume_cursor() {
    type E = jolt_field::FpExt4<F>;

    let encoded = ExtensionAtom::<F, E>::new(E::from_u64(9))
        .encode()
        .as_ref()
        .to_vec();
    let truncated = &encoded[..encoded.len() - 1];
    let mut cursor = truncated;
    let original = cursor;
    assert!(ExtensionAtom::<F, E>::deserialize_from_narg(&mut cursor).is_err());
    assert_eq!(cursor, original);

    let mut noncanonical = encoded;
    noncanonical[F::NUM_BYTES..2 * F::NUM_BYTES].fill(u8::MAX);
    let mut cursor = noncanonical.as_slice();
    let original = cursor;
    assert!(ExtensionAtom::<F, E>::deserialize_from_narg(&mut cursor).is_err());
    assert_eq!(cursor, original);
}

#[test]
fn bounded_payload_emission_rejects_over_maximum_input() {
    let mut prover = new_prover_channel(b"bounded", b"fixture").unwrap();
    assert!(matches!(
        send_bounded_bytes(&mut prover, ProtocolSiteId::default(), &[1; 9], 8),
        Err(AkitaError::InvalidInput(_))
    ));
}

#[test]
fn ignored_bounded_receipt_failure_poisons_eof() {
    let site = ProtocolSiteId {
        family: SITE_FAMILY_TERMINAL,
        stage: 7,
        ..ProtocolSiteId::default()
    };
    let mut prover = new_prover_channel(b"bounded", b"fixture").unwrap();
    prover.prover_message(&9u32);
    let proof = prover.narg_string().to_vec();

    let mut verifier = new_verifier_channel(b"bounded", b"fixture", &proof).unwrap();
    assert!(receive_bounded_bytes(&mut verifier, site, 8).is_err());
    assert!(verifier.verifier_message::<[u8; 32]>().is_err());
    assert!(verifier.prover_message::<u32>().is_err());
    assert!(verifier.check_eof().is_err());
}

#[test]
fn session_and_instance_are_independent_domains() {
    let mut left = new_prover_channel(b"session-a", b"instance").unwrap();
    let mut right = new_prover_channel(b"session-b", b"instance").unwrap();
    assert_ne!(
        left.verifier_message::<[u8; 32]>(),
        right.verifier_message::<[u8; 32]>()
    );
}

#[cfg(feature = "transcript-keccak")]
#[test]
fn rejection_attempt_markers_prevent_keccak_reconvergence() {
    let mut short = new_prover_channel(b"width", b"substrate").unwrap();
    let mut long = new_prover_channel(b"width", b"substrate").unwrap();
    let _: [u8; 1] = short.verifier_message();
    let _: [u8; 2] = long.verifier_message();
    short.public_message(&[17u8]);
    long.public_message(&[17u8]);
    assert_eq!(
        short.verifier_message::<[u8; 32]>(),
        long.verifier_message::<[u8; 32]>(),
        "the Keccak substrate forgets distinct nonzero squeeze widths after absorb",
    );

    let mut framed_short = new_prover_channel(b"width", b"framed").unwrap();
    let mut framed_long = new_prover_channel(b"width", b"framed").unwrap();
    let _: [u8; 1] = framed_short.verifier_message();
    let _: [u8; 2] = framed_long.verifier_message();
    framed_short.public_message(&1u32);
    framed_long.public_message(&2u32);
    framed_short.public_message(&[17u8]);
    framed_long.public_message(&[17u8]);
    assert_ne!(
        framed_short.verifier_message::<[u8; 32]>(),
        framed_long.verifier_message::<[u8; 32]>(),
        "exact-sampling retry markers must bind the consumed squeeze path",
    );
}

#[cfg(feature = "transcript-blake2b")]
#[test]
fn blake2b_retains_distinct_squeeze_histories_across_absorb() {
    let mut short = new_prover_channel(b"width", b"substrate").unwrap();
    let mut long = new_prover_channel(b"width", b"substrate").unwrap();
    let _: [u8; 1] = short.verifier_message();
    let _: [u8; 2] = long.verifier_message();
    short.public_message(&[17u8]);
    long.public_message(&[17u8]);
    assert_ne!(
        short.verifier_message::<[u8; 32]>(),
        long.verifier_message::<[u8; 32]>(),
        "Blake2b must retain distinct rejection histories without a marker",
    );
}

#[test]
fn grinding_preview_matches_live_replay_without_mutation() {
    let nonce_record = ProtocolContextRecord::new(
        ProtocolSiteId {
            family: 9,
            invocation: 2,
            ..ProtocolSiteId::default()
        }
        .to_bytes(),
        ProtocolMessageKind::GrindingNonce as u32,
        1,
        nonce_max_bytes(12) as u64,
        0,
    );
    let predicate_record = ProtocolContextRecord::new(
        ProtocolSiteId {
            family: 9,
            invocation: 2,
            stage: 1,
            ..ProtocolSiteId::default()
        }
        .to_bytes(),
        ProtocolMessageKind::GrindingPredicate as u32,
        0,
        0,
        crate::GRINDING_PREDICATE_LEN as u64,
    );
    let mut prover = new_prover_channel(b"grinding", b"fixture").unwrap();
    let before = prover.narg_string().to_vec();
    let preview = preview_grinding_predicate(&prover, 17);
    assert_eq!(prover.narg_string(), before);
    let live = commit_grinding_nonce(&mut prover, nonce_record, 17, predicate_record);
    assert_eq!(preview, live);

    let proof = prover.narg_string().to_vec();
    let mut verifier = new_verifier_channel(b"grinding", b"fixture", &proof).unwrap();
    let (nonce, replay) =
        receive_grinding_nonce(&mut verifier, nonce_record, predicate_record).unwrap();
    assert_eq!(nonce, 17);
    assert_eq!(replay, live);
    assert!(verifier.check_eof().is_ok());
}
