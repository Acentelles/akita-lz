//! Manual benchmarks with explicit internal backend construction.
//!
//! Run on the benchmark VM with `cargo test --release -p akita-algebra
//! --features labinius-trinomial --lib limb_backend_benchmarks -- --ignored
//! --nocapture`. This module is excluded on non-x86 targets.

use super::*;
use criterion::Criterion;
use std::hint::black_box;
use std::time::Duration;

#[test]
#[ignore = "manual Criterion benchmark; run only on the benchmark VM"]
fn limb_backend_benchmarks() {
    let mut criterion = Criterion::default()
        .without_plots()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    let mut seed = 0x158a_e09b_23c7_4f61u64;
    let bits: Vec<[u64; 11]> = (0..128)
        .map(|_| {
            let mut bits = std::array::from_fn(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed
            });
            bits[10] &= 255;
            bits
        })
        .collect();
    let mut group = criterion.benchmark_group("limb_explicit_backend");
    for backend in available_backends() {
        // available_backends detects every feature required by each variant.
        // Constructing the domain explicitly also selects the accumulator's
        // backend, including its checkpoint and finish implementations.
        let mut domain = TrinomialLimbDomain::new(33_568_993).unwrap();
        domain.backend = backend;
        let inputs: Vec<_> = bits
            .iter()
            .map(|bits| {
                let mut slots = domain.zero_slots();
                domain.forward_interleaved_bits(bits, &mut slots).unwrap();
                slots
            })
            .collect();
        let mut output = domain.zero_slots();
        group.bench_function(format!("bits/{backend:?}"), |b| {
            b.iter(|| {
                for bits in black_box(&bits) {
                    domain.forward_interleaved_bits(bits, &mut output).unwrap();
                    black_box(&output);
                }
            });
        });
        let words: Vec<[u128; 4]> = bits
            .iter()
            .map(|bits| {
                std::array::from_fn(|i| {
                    u128::from(bits[2 * i]) | (u128::from(bits[2 * i + 1]) << 64)
                })
            })
            .collect();
        if domain
            .try_forward_interleaved_words(&words[0], &mut output)
            .unwrap()
        {
            group.bench_function(format!("direct_words/{backend:?}"), |b| {
                b.iter(|| {
                    for words in black_box(&words) {
                        assert!(domain
                            .try_forward_interleaved_words(words, &mut output)
                            .unwrap());
                        black_box(&output);
                    }
                });
            });
        }
        let mut accumulator = TrinomialLimbAccumulator::new(&domain);
        group.bench_function(format!("checkpoint_and_finish/{backend:?}"), |b| {
            b.iter(|| {
                accumulator
                    .add_product_batch(black_box(&inputs), black_box(&inputs))
                    .unwrap();
                accumulator.finish(&mut output).unwrap();
                black_box(&output);
            });
        });
    }
    group.finish();
    criterion.final_summary();
}
