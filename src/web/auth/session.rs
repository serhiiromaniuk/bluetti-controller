//! The web login's in-memory state (ROADMAP.md §8.5 b–c): the session store,
//! the failed-login lockout and the one-permit verification slot. No I/O, no
//! HTTP. Every instant is `tokio::time::Instant`, so tests can pause time.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::Semaphore;
use tokio::time::Instant;

/// A session lasts 30 days from login; use does not extend it.
pub const SESSION_TTL: Duration = Duration::from_secs(30 * 24 * 3600);
/// At most this many live sessions; one more login evicts the oldest.
pub const MAX_SESSIONS: usize = 16;
/// Consecutive failed logins that start a lock.
pub const MAX_FAILURES: u32 = 5;
/// How long every login attempt is refused once locked.
pub const LOCK: Duration = Duration::from_secs(60);

/// The plain data behind each mutex stays consistent after a panic, so a
/// poisoned lock is used as is rather than taking the login down with it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Live session ids and their expiry instants.
#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, Instant>>);

impl Sessions {
    /// A new session id: 32 bytes from `getrandom`, as 64 lowercase hex chars.
    /// Drops expired ids, then evicts the oldest while the store is full.
    pub fn issue(&self) -> anyhow::Result<String> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|err| anyhow!("session id: {err}"))?;
        let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let now = Instant::now();
        let mut map = lock(&self.0);
        map.retain(|_, expiry| *expiry > now);
        while map.len() >= MAX_SESSIONS {
            // Every id has the same lifetime, so the earliest expiry is the oldest.
            let Some(oldest) = map.iter().min_by_key(|(_, e)| **e).map(|(k, _)| k.clone()) else {
                break;
            };
            map.remove(&oldest);
        }
        map.insert(id.clone(), now + SESSION_TTL);
        Ok(id)
    }

    /// Whether `id` is a live, unexpired session.
    pub fn check(&self, id: &str) -> bool {
        lock(&self.0)
            .get(id)
            .is_some_and(|expiry| *expiry > Instant::now())
    }

    /// Ends the session `id` (logout); an unknown id is a no-op.
    pub fn revoke(&self, id: &str) {
        lock(&self.0).remove(id);
    }
}

/// Whether a login attempt may go on to verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// Verify the password.
    Proceed,
    /// Answer `429` without verifying.
    Locked,
}

#[derive(Default)]
struct LockState {
    failures: u32,
    locked_until: Option<Instant>,
}

/// One global failed-login counter (single user, §8.5 c).
#[derive(Default)]
pub struct Lockout(Mutex<LockState>);

impl Lockout {
    /// Counts the attempt as a failure **before** verification. The attempt
    /// that reaches `MAX_FAILURES` still verifies; the lock refuses every later
    /// one for `LOCK`, then the count starts again.
    pub fn begin(&self) -> Attempt {
        let now = Instant::now();
        let mut state = lock(&self.0);
        if state.locked_until.is_some_and(|until| now < until) {
            return Attempt::Locked;
        }
        state.locked_until = None;
        state.failures += 1;
        if state.failures >= MAX_FAILURES {
            state.failures = 0;
            state.locked_until = Some(now + LOCK);
        }
        Attempt::Proceed
    }

    /// A verified login: clears the count and any lock.
    pub fn succeeded(&self) {
        *lock(&self.0) = LockState::default();
    }
}

/// The login state of a server with `web.password_hash` set. No `Debug`: it
/// holds the hash.
pub struct Auth {
    /// The argon2id PHC string from `web.password_hash`.
    pub hash: String,
    /// Live sessions.
    pub sessions: Sessions,
    /// The failed-login counter.
    pub lockout: Lockout,
    /// One permit: at most one argon2 verification (~19 MiB) runs at a time.
    /// The verification task owns the permit, so a client that hangs up
    /// mid-login cannot free it early (amendment A3).
    pub verify_slot: Arc<Semaphore>,
    /// Verifications started, so tests can prove the lock and the slot bound them.
    #[cfg(test)]
    pub verified: std::sync::atomic::AtomicU32,
    /// Verifications running now and the most ever at once (amendment A3 test).
    #[cfg(test)]
    pub running: Arc<(std::sync::atomic::AtomicU32, std::sync::atomic::AtomicU32)>,
}

impl Auth {
    /// Fresh state for `hash`: no sessions, no failures.
    pub fn new(hash: String) -> Self {
        Self {
            hash,
            sessions: Sessions::default(),
            lockout: Lockout::default(),
            verify_slot: Arc::new(Semaphore::new(1)),
            #[cfg(test)]
            verified: std::sync::atomic::AtomicU32::new(0),
            #[cfg(test)]
            running: Arc::default(),
        }
    }
}

#[cfg(test)]
mod tests;
