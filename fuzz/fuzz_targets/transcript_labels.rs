#![no_main]

use akita_transcript::{ext_challenge, new_prover_channel, public_bytes, ProtocolSiteId};
use jolt_field::Prime128Offset275;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let split = data.len().min(255);
    let (label, bytes) = data.split_at(split);

    let mut transcript = new_prover_channel(label);
    let _ = public_bytes(&mut transcript, ProtocolSiteId::default(), bytes);
    let _: Prime128Offset275 =
        ext_challenge(&mut transcript, ProtocolSiteId::default()).unwrap();
});
