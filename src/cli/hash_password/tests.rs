use bluetti_controller::web::auth::password;

use super::read_password;

#[test]
fn takes_the_first_line_without_its_terminator() {
    assert_eq!(read_password(&b"pa ss\nnext\n"[..]).unwrap(), "pa ss");
    assert_eq!(read_password(&b"pa ss\r\n"[..]).unwrap(), "pa ss");
    assert_eq!(read_password(&b"no newline"[..]).unwrap(), "no newline");
}

#[test]
fn refuses_an_empty_password() {
    for input in [&b""[..], b"\n", b"\r\n"] {
        assert!(read_password(input).is_err(), "{input:?}");
    }
}

#[test]
fn prints_a_phc_string_that_verifies_and_the_line_to_paste() {
    let mut out = Vec::new();
    super::run(&b"pa ss\n"[..], &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    let phc = lines[0];
    assert!(phc.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{phc}");
    assert!(password::verify(phc, "pa ss"));
    assert!(
        !out.contains("pa ss"),
        "the password itself is never printed"
    );
    assert_eq!(lines[1], "# paste under [web] in config.toml:");
    assert_eq!(lines[2], format!("password_hash = \"{phc}\""));
}

#[test]
fn an_empty_password_prints_nothing() {
    let mut out = Vec::new();
    assert!(super::run(&b"\n"[..], &mut out).is_err());
    assert!(out.is_empty());
}
