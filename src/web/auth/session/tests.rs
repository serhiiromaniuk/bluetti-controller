use std::time::Duration;

use super::{Attempt, Auth, LOCK, Lockout, MAX_FAILURES, MAX_SESSIONS, SESSION_TTL, Sessions};

#[tokio::test(start_paused = true)]
async fn an_issued_id_checks_and_is_64_hex_chars() {
    let sessions = Sessions::default();
    let id = sessions.issue().unwrap();
    assert_eq!(id.len(), 64);
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(sessions.check(&id));
    assert!(!sessions.check("not-an-id"));
    assert_ne!(sessions.issue().unwrap(), id);
}

#[tokio::test(start_paused = true)]
async fn an_id_expires_after_30_days() {
    let sessions = Sessions::default();
    let id = sessions.issue().unwrap();
    assert_eq!(SESSION_TTL, Duration::from_secs(30 * 24 * 3600));
    tokio::time::advance(SESSION_TTL - Duration::from_secs(1)).await;
    assert!(
        sessions.check(&id),
        "use does not extend it, but it is still valid"
    );
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(!sessions.check(&id));
}

#[tokio::test(start_paused = true)]
async fn the_17th_id_evicts_the_oldest() {
    let sessions = Sessions::default();
    let mut ids = Vec::new();
    for _ in 0..=MAX_SESSIONS {
        ids.push(sessions.issue().unwrap());
        tokio::time::advance(Duration::from_secs(1)).await;
    }
    assert_eq!(MAX_SESSIONS, 16);
    assert!(!sessions.check(&ids[0]), "the oldest is evicted");
    assert!(ids[1..].iter().all(|id| sessions.check(id)));
}

#[tokio::test(start_paused = true)]
async fn a_revoked_id_no_longer_checks() {
    let sessions = Sessions::default();
    let id = sessions.issue().unwrap();
    let other = sessions.issue().unwrap();
    sessions.revoke(&id);
    assert!(!sessions.check(&id));
    assert!(sessions.check(&other));
    sessions.revoke("unknown");
}

#[tokio::test(start_paused = true)]
async fn the_6th_attempt_is_locked_for_60_seconds() {
    let lockout = Lockout::default();
    for _ in 0..MAX_FAILURES {
        assert_eq!(lockout.begin(), Attempt::Proceed);
    }
    assert_eq!(lockout.begin(), Attempt::Locked);
    tokio::time::advance(LOCK - Duration::from_secs(1)).await;
    assert_eq!(lockout.begin(), Attempt::Locked);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(lockout.begin(), Attempt::Proceed, "the lock window is over");
    for _ in 1..MAX_FAILURES {
        assert_eq!(lockout.begin(), Attempt::Proceed);
    }
    assert_eq!(
        lockout.begin(),
        Attempt::Locked,
        "5 more per window, never more"
    );
}

#[tokio::test(start_paused = true)]
async fn a_success_resets_the_count() {
    let lockout = Lockout::default();
    for _ in 0..MAX_FAILURES - 1 {
        assert_eq!(lockout.begin(), Attempt::Proceed);
    }
    lockout.succeeded();
    for _ in 0..MAX_FAILURES {
        assert_eq!(lockout.begin(), Attempt::Proceed);
    }
    lockout.succeeded();
    assert_eq!(
        lockout.begin(),
        Attempt::Proceed,
        "a success also lifts the lock"
    );
}

#[tokio::test(start_paused = true)]
async fn auth_allows_one_verification_at_a_time() {
    let auth = Auth::new("$argon2id$placeholder".to_owned());
    let permit = auth.verify_slot.try_acquire().unwrap();
    assert!(auth.verify_slot.try_acquire().is_err());
    drop(permit);
    assert!(auth.verify_slot.try_acquire().is_ok());
    assert_eq!(auth.hash, "$argon2id$placeholder");
}
