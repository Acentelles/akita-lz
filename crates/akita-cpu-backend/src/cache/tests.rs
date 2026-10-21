use super::get_or_build;
use akita_error::AkitaError;
use std::cell::RefCell;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

type BuildHook = Box<dyn FnOnce() -> Result<(), AkitaError>>;
thread_local! {
    static BUILD_HOOK: RefCell<Option<BuildHook>> = RefCell::new(None);
}

pub(crate) fn set_build_hook(hook: impl FnOnce() -> Result<(), AkitaError> + 'static) {
    BUILD_HOOK.with(|slot| {
        assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
    });
}

pub(crate) fn run_build_hook() -> Result<(), AkitaError> {
    let hook = BUILD_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook()?;
    }
    Ok(())
}

pub(crate) fn watchdog<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
        let _ = tx.send(result);
    });
    let result = rx
        .recv_timeout(timeout)
        .expect("cache acquisition watchdog timed out");
    worker.join().expect("watchdog worker");
    result.unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

#[test]
fn losing_cache_candidate_is_dropped_after_unlock() {
    watchdog(Duration::from_secs(30), || {
        struct Probe {
            state: Arc<Mutex<Option<Arc<Probe>>>>,
            drops: Arc<Mutex<usize>>,
        }
        impl Drop for Probe {
            fn drop(&mut self) {
                assert!(self.state.try_lock().is_ok());
                *self.drops.lock().unwrap() += 1;
            }
        }
        let state = Arc::new(Mutex::new(None));
        let drops = Arc::new(Mutex::new(0));
        let value = get_or_build(
            &state,
            "probe lock",
            Clone::clone,
            || {
                let first = Arc::new(Probe {
                    state: Arc::clone(&state),
                    drops: Arc::clone(&drops),
                });
                *state.lock().unwrap() = Some(first);
                Ok(Probe {
                    state: Arc::clone(&state),
                    drops: Arc::clone(&drops),
                })
            },
            |state, value| *state = Some(value),
        )
        .unwrap();
        assert_eq!(*drops.lock().unwrap(), 1);
        assert!(Arc::ptr_eq(state.lock().unwrap().as_ref().unwrap(), &value));
        let warm = get_or_build(
            &state,
            "probe lock",
            Clone::clone,
            || panic!("warm hit built"),
            |state, value| *state = Some(value),
        )
        .unwrap();
        assert!(Arc::ptr_eq(&warm, &value));
        let retained = state.lock().unwrap().take();
        drop(retained);
        drop(warm);
        drop(value);
        assert_eq!(*drops.lock().unwrap(), 2);
    });
}
