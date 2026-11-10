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
