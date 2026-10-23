//! Known answer of the Blake2b proof stream under the `terminal-quotient`
//! protocol identifier (`v7-terminal-quotient`). The `v7` known answer is
//! `tests::blake2b_transcript_has_a_cross_width_known_answer`; the transcript
//! steps are the same, only the identifier differs.

use super::*;

#[test]
fn blake2b_transcript_has_a_cross_width_known_answer() {
    let mut prover = new_prover_channel(b"cross-width/session", b"cross-width/instance").unwrap();
    prover.public_message(b"public-message");
    let challenge = prover.verifier_message::<[u8; 32]>();
    assert_eq!(
        challenge,
        [
            159, 36, 11, 15, 242, 123, 12, 223, 159, 245, 133, 118, 153, 39, 59, 112, 32, 94, 92,
            17, 240, 95, 32, 71, 241, 199, 90, 168, 172, 27, 226, 125,
        ]
    );
}
