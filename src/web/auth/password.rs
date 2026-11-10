//! argon2id password hashing for the web login (ROADMAP.md ADR-011, §8.5 c):
//! m = 19 MiB, t = 2, p = 1, a 16-byte salt from `getrandom`, a 32-byte tag.
//! Pure CPU work, ~100 ms: callers on the async runtime use `spawn_blocking`.

use anyhow::Context;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};

/// The ADR-011 hasher, stated explicitly so a new crate default cannot change it.
fn argon2id() -> anyhow::Result<Argon2<'static>> {
    let params = Params::new(19 * 1024, 2, 1, Some(32)).context("argon2 params")?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Hashes `password` with argon2id and a fresh 16-byte salt; returns the PHC
/// string (`$argon2id$v=19$m=19456,t=2,p=1$<salt>$<tag>`).
pub fn hash(password: &str) -> anyhow::Result<String> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).context("salt")?;
    let phc = argon2id()?
        .hash_password_with_salt(password.as_bytes(), &salt)
        .context("argon2")?;
    Ok(phc.to_string())
}

/// Whether `password` matches the PHC string `phc`, with the parameters the
/// string carries. A malformed string is `false`, never a panic.
pub fn verify(phc: &str, password: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), phc)
        .is_ok()
}

#[cfg(test)]
mod tests;
