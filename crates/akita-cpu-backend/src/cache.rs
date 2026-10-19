//! Ready-value publication without waiting for in-flight construction.

use akita_error::AkitaError;
use std::sync::{Arc, Mutex};

/// Build outside the lock; overlapping misses may build duplicate candidates.
/// Successful callers select the first ready covering value at publication.
pub(crate) fn get_or_build<S, V>(
    state: &Mutex<S>,
    lock_error: &'static str,
    lookup: impl Fn(&S) -> Option<Arc<V>>,
    build: impl FnOnce() -> Result<V, AkitaError>,
    publish: impl FnOnce(&mut S, Arc<V>),
) -> Result<Arc<V>, AkitaError> {
    {
        let guard = state
            .lock()
            .map_err(|_| AkitaError::Internal(lock_error.into()))?;
        if let Some(value) = lookup(&guard) {
            return Ok(value);
        }
    }
    let candidate = Arc::new(build()?);
    let selected = {
        let mut guard = state
            .lock()
            .map_err(|_| AkitaError::Internal(lock_error.into()))?;
        if let Some(value) = lookup(&guard) {
            value
        } else {
            publish(&mut guard, Arc::clone(&candidate));
            Arc::clone(&candidate)
        }
    };
    drop(candidate);
    Ok(selected)
}

#[cfg(test)]
pub(crate) mod tests;
