//! Poison-tolerant locking for [`std::sync::Mutex`].
//!
//! A panic while a thread holds a std `Mutex` marks it *poisoned*; every later
//! `.lock().unwrap()` / `.expect(...)` on it then panics too. Since nearly every
//! command in this app locks a small set of shared mutexes (the workspace, the
//! open-session maps), a single transient panic would otherwise cascade into a
//! permanently unusable app until restart.
//!
//! These critical sections are short and structurally simple (map insert/remove,
//! clone, a scalar assignment), so recovering the guard and carrying on is far
//! better behaviour than that cascade. [`MutexExt::lock_recover`] returns the
//! guard whether or not the mutex was poisoned.
use std::sync::{Mutex, MutexGuard};

pub trait MutexExt<T> {
    /// Locks the mutex, transparently recovering the guard if it was poisoned by
    /// a panic in another thread instead of propagating the poison.
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for Mutex<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn recovers_a_poisoned_mutex_instead_of_panicking() {
        let m = Arc::new(Mutex::new(0u32));
        // Poison the mutex: panic while holding the guard.
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let mut g = m2.lock().unwrap();
            *g = 42;
            panic!("boom while holding the lock");
        })
        .join();

        assert!(m.lock().is_err(), "precondition: the mutex is now poisoned");
        // The recovering lock still hands back the (last-written) value.
        assert_eq!(*m.lock_recover(), 42);
    }
}
