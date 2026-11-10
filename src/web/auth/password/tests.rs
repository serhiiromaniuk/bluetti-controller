use super::{hash, verify};

#[test]
fn hash_then_verify_round_trips() {
    let phc = hash("test").unwrap();
    assert!(verify(&phc, "test"));
}

#[test]
fn wrong_password_is_refused() {
    let phc = hash("test").unwrap();
    assert!(!verify(&phc, "tesT"));
    assert!(!verify(&phc, ""));
}

#[test]
fn malformed_hash_is_refused_without_panic() {
    for phc in [
        "",
        "not a hash",
        "$argon2id$v=19$m=19456,t=2,p=1$",
        "$argon2id$$$$",
    ] {
        assert!(!verify(phc, "test"), "{phc:?}");
    }
}

#[test]
fn two_hashes_of_one_password_differ() {
    assert_ne!(hash("test").unwrap(), hash("test").unwrap());
}

#[test]
fn hash_carries_the_adr_011_parameters() {
    let phc = hash("test").unwrap();
    assert!(phc.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{phc}");
    // 16-byte salt and 32-byte tag, unpadded base64: 22 and 43 characters.
    let parts: Vec<&str> = phc.split('$').collect();
    assert_eq!((parts[4].len(), parts[5].len()), (22, 43), "{phc}");
}
