use super::compression_cache::CompressionNttDomains;
use super::prepared_tests::F;
use super::CpuPreparedSetup;
use crate::cache::tests::{set_build_hook, watchdog};
use akita_error::AkitaError;
use akita_types::{NttCacheKey, NttTransformDomain};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::time::Duration;

const D: usize = 32;

fn prepared() -> CpuPreparedSetup<F> {
    let setup = crate::AkitaProverSetup::<F>::generate_with_capacity(
        8,
        1,
        akita_params::SetupMatrixCapacity {
            num_field_elements: 8 * D,
        },
    )
    .unwrap();
    CpuPreparedSetup::new(setup.expanded)
}

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
enum Kind {
    Compression(CompressionNttDomains),
    Shared,
}

fn key(width: usize) -> NttCacheKey {
    NttCacheKey {
        ring_d: D,
        num_ring_elements: width,
        domain: NttTransformDomain::Negacyclic,
    }
}

fn acquire(p: &CpuPreparedSetup<F>, kind: Kind, width: usize) -> Result<usize, AkitaError> {
    let address = |cache: &akita_types::PreparedNttCache<D>| Ok(cache as *const _ as usize);
    match kind {
        Kind::Compression(domain) => p.with_compression_ntt::<D, _>(width, domain, address),
        Kind::Shared => p.with_shared_ntt::<D, _>(key(width), address),
    }
}

fn assert_ready(p: &CpuPreparedSetup<F>, kind: Kind, width: usize) {
    let bytes = width * D * 3 * std::mem::size_of::<i32>();
    match kind {
        Kind::Compression(domain) => {
            assert_eq!(p.compression_ntt.slot_count(), 1);
            assert_eq!(
                p.compression_ntt_cache_bytes(),
                bytes
                    * if domain == CompressionNttDomains::Both {
                        2
                    } else {
                        1
                    }
            );
        }
        Kind::Shared => {
            let metrics = p.shared_ntt_cache_metrics().unwrap();
            assert_eq!(metrics.len(), 1);
            assert_eq!(metrics[0].key, key(width));
            assert_eq!(metrics[0].cache_bytes, bytes);
            assert_eq!(p.shared_ntt_cache_bytes(), bytes);
        }
    }
}

fn attempts(p: &CpuPreparedSetup<F>, kind: Kind) -> usize {
    match kind {
        Kind::Compression(_) => p.compression_ntt.slot_build_count(),
        Kind::Shared => p.ntt_slot_build_count(),
    }
}

fn reentry(kind: Kind) {
    watchdog(TIMEOUT, move || {
        let p = Arc::new(prepared());
        let inner = Arc::new(Mutex::new(None));
        let nested = Arc::clone(&p);
        let saved = Arc::clone(&inner);
        set_build_hook(move || {
            *saved.lock().unwrap() = Some(acquire(&nested, kind, 3)?);
            Ok(())
        });
        let outer = acquire(&p, kind, 3).unwrap();
        assert_eq!(Some(outer), *inner.lock().unwrap());
        assert_eq!(attempts(&p, kind), 2);
        assert_ready(&p, kind, 3);
    });
}

#[test]
fn compression_same_key_reentrant_build_completes() {
    reentry(Kind::Compression(CompressionNttDomains::Both));
}

#[test]
fn shared_ntt_same_key_reentrant_build_completes() {
    reentry(Kind::Shared);
}

fn concurrent_warm(kind: Kind) {
    watchdog(TIMEOUT, move || {
        let p = Arc::new(prepared());
        let gate = Arc::new(Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let p = Arc::clone(&p);
                let gate = Arc::clone(&gate);
                std::thread::spawn(move || {
                    set_build_hook(move || {
                        gate.wait();
                        Ok(())
                    });
                    acquire(&p, kind, 3).unwrap()
                })
            })
            .collect();
        let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert!(results.iter().all(|r| *r == results[0]));
        assert_eq!(attempts(&p, kind), 4);
        assert_ready(&p, kind, 3);
        assert_eq!(acquire(&p, kind, 3).unwrap(), results[0]);
        assert_eq!(attempts(&p, kind), 4);
    });
}

#[test]
fn concurrent_same_shape_warm_publishes_one_value() {
    for domain in [
        CompressionNttDomains::Both,
        CompressionNttDomains::Negacyclic,
    ] {
        concurrent_warm(Kind::Compression(domain));
    }
}

#[test]
fn concurrent_same_key_ntt_warm_publishes_one_value() {
    concurrent_warm(Kind::Shared);
}

fn failed_build(kind: Kind, panics: bool) {
    watchdog(TIMEOUT, move || {
        let p = Arc::new(prepared());
        let inject = move || {
            if panics {
                panic!("injected construction panic");
            }
            Err(AkitaError::Internal("injected construction failure".into()))
        };
        set_build_hook(inject);
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| acquire(&p, kind, 3)));
        if panics {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        assert_eq!(p.compression_ntt.slot_count(), 0);
        assert!(p.shared_ntt.lock().unwrap().is_empty());
        let retained = acquire(&p, kind, 3).unwrap();
        assert_ready(&p, kind, 3);
        assert_eq!(acquire(&p, kind, 3).unwrap(), retained);
        assert_eq!(attempts(&p, kind), 2);

        let p = Arc::new(prepared());
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let delayed = Arc::clone(&p);
        let failed = std::thread::spawn(move || {
            set_build_hook(move || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                inject()
            });
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| acquire(&delayed, kind, 3)))
        });
        entered_rx.recv().unwrap();
        let retained = acquire(&p, kind, 3).unwrap();
        release_tx.send(()).unwrap();
        let result = failed.join().unwrap();
        if panics {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        assert_eq!(acquire(&p, kind, 3).unwrap(), retained);
        assert_ready(&p, kind, 3);
        assert_eq!(attempts(&p, kind), 2);
    });
}

#[test]
fn failed_compression_build_is_not_cached() {
    failed_build(Kind::Compression(CompressionNttDomains::Both), false);
}

#[test]
fn failed_shared_ntt_build_is_not_cached() {
    failed_build(Kind::Shared, false);
}

#[test]
fn panicking_ntt_build_is_retriable() {
    failed_build(Kind::Compression(CompressionNttDomains::Both), true);
    failed_build(Kind::Shared, true);
}

#[test]
fn slow_small_ntt_build_returns_published_larger_prefix() {
    watchdog(TIMEOUT, || {
        let p = Arc::new(prepared());
        let delayed = Arc::clone(&p);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let small = std::thread::spawn(move || {
            set_build_hook(move || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            });
            acquire(&delayed, Kind::Shared, 3).unwrap()
        });
        entered_rx.recv().unwrap();
        let large = acquire(&p, Kind::Shared, 6).unwrap();
        release_tx.send(()).unwrap();
        assert_eq!(small.join().unwrap(), large);
        assert_ready(&p, Kind::Shared, 6);
        assert_eq!(attempts(&p, Kind::Shared), 2);

        let p = prepared();
        let small = acquire(&p, Kind::Shared, 3).unwrap();
        let held_small = p.shared_ntt.lock().unwrap()[&key(3)].clone();
        let large = acquire(&p, Kind::Shared, 6).unwrap();
        assert_ne!(small, large);
        assert_eq!(held_small.cache_bytes, 3 * D * 3 * 4);
        assert_ready(&p, Kind::Shared, 6);
        let cyclic = NttCacheKey {
            domain: NttTransformDomain::Cyclic,
            ..key(6)
        };
        let cyclic_address = p
            .with_shared_ntt::<D, _>(cyclic, |cache| Ok(cache as *const _ as usize))
            .unwrap();
        assert_ne!(large, cyclic_address);
        assert_eq!(p.shared_ntt_cache_metrics().unwrap().len(), 2);
    });
}

/// Checks completion and value agreement under cold contention. The reentry
/// tests are the deterministic deadlock regressions.
#[cfg(feature = "parallel")]
#[test]
fn cold_cache_rayon_acquisitions_complete() {
    use crate::opaque::{CompressionComputeBackend, CpuBackend, CyclicRowsComputeBackend};
    use rayon::prelude::*;
    watchdog(Duration::from_secs(120), || {
        let digits = vec![[-1i8; D]; 3];
        let backend = CpuBackend::<F, F>::for_arithmetic_tests();
        let reference = prepared();
        let expected_compression = backend
            .compression_rows_products::<D>(&reference, &[&digits])
            .unwrap();
        let expected_shared = backend
            .cyclic_digit_rows::<D>(&reference, 1, &digits, 2)
            .unwrap();
        for n in [4, 14] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build()
                .unwrap();
            for _ in 0..16 {
                let p = prepared();
                pool.install(|| {
                    (0..16).into_par_iter().for_each(|_| {
                        let compression = backend
                            .compression_rows_products::<D>(&p, &[&digits])
                            .unwrap();
                        assert_eq!(compression.len(), expected_compression.len());
                        for (actual, expected) in compression.iter().zip(&expected_compression) {
                            assert_eq!(actual.negacyclic, expected.negacyclic);
                            assert_eq!(actual.cyclic, expected.cyclic);
                        }
                        assert_eq!(
                            backend.cyclic_digit_rows::<D>(&p, 1, &digits, 2).unwrap(),
                            expected_shared
                        );
                    });
                });
                assert_eq!(p.compression_ntt.slot_count(), 1);
                assert_eq!(p.shared_ntt_cache_metrics().unwrap().len(), 1);
            }
        }
    });
}
