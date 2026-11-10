//! `[web]`: the listener, the bearer token and the owner login (ROADMAP.md §9,
//! ADR-011, §8.5). Validation lives here so `config.rs` stays small.

use std::fmt;
use std::net::SocketAddr;

use anyhow::bail;
use serde::Deserialize;

/// `Debug` hides the token and the password hash (golden rule #3).
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebConfig {
    pub listen: SocketAddr,
    /// Bearer token for scripts. An empty token is rejected at load.
    pub token: Option<String>,
    /// argon2id PHC string from `bluetti-controller hash-password`. Unset =
    /// dev mode: loopback listener only, no login (§8.5 h).
    pub password_hash: Option<String>,
    /// DNS names the Host rule accepts; IP literals and `localhost` need none.
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

impl fmt::Debug for WebConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let set = |v: &Option<String>| if v.is_some() { "<set>" } else { "None" };
        f.debug_struct("WebConfig")
            .field("listen", &self.listen)
            .field("token", &format_args!("{}", set(&self.token)))
            .field(
                "password_hash",
                &format_args!("{}", set(&self.password_hash)),
            )
            .field("allowed_hosts", &self.allowed_hosts)
            .finish()
    }
}

impl WebConfig {
    /// The `[web]` rules, plus the two `control.enabled` prerequisites. No
    /// error quotes the token or the hash.
    pub(super) fn validate(&self, control_enabled: bool) -> anyhow::Result<()> {
        if self.token.as_deref().is_some_and(|t| t.trim().is_empty()) {
            bail!("web.token must not be empty; remove it if no script needs it");
        }
        if let Some(phc) = &self.password_hash
            && !is_argon2id(phc)
        {
            bail!(
                "web.password_hash is not an argon2id PHC string; run `bluetti-controller hash-password`"
            );
        }
        if !self.listen.ip().is_loopback() && self.password_hash.is_none() {
            bail!(
                "web.listen {} is not loopback: set web.password_hash (`bluetti-controller hash-password`)",
                self.listen
            );
        }
        // A write to a station is not a pause: no open write route (ROADMAP.md §9).
        if control_enabled && self.token.is_none() {
            bail!("control.enabled = true requires web.token to be set");
        }
        if control_enabled && self.password_hash.is_none() {
            bail!("control.enabled = true requires web.password_hash to be set");
        }
        for (i, host) in self.allowed_hosts.iter().enumerate() {
            if !is_host_name(host) {
                bail!(
                    "web.allowed_hosts[{i}] `{host}` is not a bare host name (no scheme, port or path)"
                );
            }
        }
        Ok(())
    }
}

/// An argon2id PHC string with valid params, a salt and a tag: anything else
/// never verifies at login.
fn is_argon2id(phc: &str) -> bool {
    argon2::password_hash::phc::PasswordHash::new(phc).is_ok_and(|hash| {
        hash.algorithm.as_str() == "argon2id"
            && hash.salt.is_some()
            && hash.hash.is_some()
            && argon2::Params::try_from(&hash).is_ok()
    })
}

fn is_host_name(host: &str) -> bool {
    !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
}

#[cfg(test)]
mod tests {
    use super::super::parse;

    const EXAMPLE: &str = include_str!("../../config.example.toml");
    /// `hash-password` of the throwaway string "test" (never an owner password).
    const HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$XYoC+IyxSMsuKhoGWisQFA$FYfUAw0rj87Ga04AeeadC8HcSCwM0Q++Dl9U0gVQ67w";
    const LISTEN: &str = "listen = \"127.0.0.1:8080\"";

    fn with(from: &str, to: &str) -> String {
        let text = EXAMPLE.replacen(from, to, 1);
        assert_ne!(text, EXAMPLE, "`{from}` not in config.example.toml");
        text
    }

    fn with_hash(text: &str) -> String {
        text.replacen(LISTEN, &format!("{LISTEN}\npassword_hash = \"{HASH}\""), 1)
    }

    fn error_of(text: &str) -> String {
        format!("{:#}", parse(text).unwrap_err())
    }

    #[test]
    fn example_is_loopback_dev_mode() {
        let web = parse(EXAMPLE).unwrap().web;
        assert!(web.listen.ip().is_loopback());
        assert_eq!(web.token, None);
        assert_eq!(web.password_hash, None);
        assert!(web.allowed_hosts.is_empty());
    }

    #[test]
    fn accepts_hash_and_hosts() {
        let text = with_hash(EXAMPLE).replacen(LISTEN, "listen = \"0.0.0.0:8080\"", 1);
        let text = text.replacen(
            "# allowed_hosts = [\"ideapad.lan\"]",
            "allowed_hosts = [\"ideapad.lan\", \"Bluetti-1.home.arpa\"]",
            1,
        );
        let web = parse(&text).unwrap().web;
        assert_eq!(web.password_hash.as_deref(), Some(HASH));
        assert_eq!(web.allowed_hosts, ["ideapad.lan", "Bluetti-1.home.arpa"]);
    }

    #[test]
    fn rejects_bad_hash_naming_the_key() {
        for bad in [
            "",
            "hunter2",
            "$argon2id$v=19$m=19456",
            "$pbkdf2-sha256$i=1000$c2FsdA$aGFzaA",
        ] {
            let text = with(LISTEN, &format!("{LISTEN}\npassword_hash = \"{bad}\""));
            let err = error_of(&text);
            assert!(err.contains("web.password_hash"), "{bad}: {err}");
            assert!(bad.is_empty() || !err.contains(bad), "{bad}: {err}");
        }
    }

    #[test]
    fn lan_listener_needs_hash() {
        for listen in ["0.0.0.0:8080", "192.168.0.192:8080", "[::]:8080"] {
            let err = error_of(&with(LISTEN, &format!("listen = \"{listen}\"")));
            assert!(
                err.contains("web.listen") && err.contains("web.password_hash"),
                "{listen}: {err}"
            );
        }
        for listen in ["127.0.0.2:8080", "[::1]:8080"] {
            assert!(parse(&with(LISTEN, &format!("listen = \"{listen}\""))).is_ok());
        }
    }

    #[test]
    fn rejects_bad_allowed_hosts() {
        for bad in [
            "\"\"",
            "\"http://ideapad.lan\"",
            "\"ideapad.lan:8080\"",
            "\"ideapad.lan/x\"",
            "\"a b\"",
        ] {
            let text = with(
                "# allowed_hosts = [\"ideapad.lan\"]",
                &format!("allowed_hosts = [{bad}]"),
            );
            assert!(error_of(&text).contains("web.allowed_hosts"), "{bad}");
        }
    }

    #[test]
    fn rejects_other_hash_kinds_hostile_hosts_and_unknown_keys() {
        // Security suite type 1 (step 10a): the wrong argon2 variant, params
        // argon2 refuses, a look-alike host name and a misspelt key.
        for bad in [
            "$argon2i$v=19$m=19456,t=2,p=1$XYoC+IyxSMsuKhoGWisQFA$FYfUAw0rj87Ga04AeeadC8HcSCwM0Q++Dl9U0gVQ67w",
            "$argon2d$v=19$m=19456,t=2,p=1$XYoC+IyxSMsuKhoGWisQFA$FYfUAw0rj87Ga04AeeadC8HcSCwM0Q++Dl9U0gVQ67w",
            "$argon2id$v=19$m=1,t=2,p=1$XYoC+IyxSMsuKhoGWisQFA$FYfUAw0rj87Ga04AeeadC8HcSCwM0Q++Dl9U0gVQ67w",
            "$argon2id$v=19$m=19456,t=2,p=1$XYoC+IyxSMsuKhoGWisQFA",
        ] {
            let text = with(LISTEN, &format!("{LISTEN}\npassword_hash = \"{bad}\""));
            assert!(error_of(&text).contains("web.password_hash"), "{bad}");
        }
        for bad in [
            "\"ideapad_lan\"",
            "\"\u{0456}deapad.lan\"",
            "\"ideapad.lan\\u0000\"",
        ] {
            let text = with(
                "# allowed_hosts = [\"ideapad.lan\"]",
                &format!("allowed_hosts = [{bad}]"),
            );
            assert!(error_of(&text).contains("web.allowed_hosts"), "{bad}");
        }
        let text = with(LISTEN, &format!("{LISTEN}\npasword_hash = \"{HASH}\""));
        assert!(error_of(&text).contains("pasword_hash"));
    }

    #[test]
    fn rejects_empty_token() {
        for token in ["\"\"", "\"  \""] {
            let text = with("# token = \"change-me\"", &format!("token = {token}"));
            assert!(error_of(&text).contains("web.token"), "{token}");
        }
        let text = with("# token = \"change-me\"", "token = \"t\"");
        assert_eq!(parse(&text).unwrap().web.token.as_deref(), Some("t"));
    }

    #[test]
    fn control_needs_token_and_hash() {
        let enabled = with("enabled = false ", "enabled = true ");
        let err = error_of(&with_hash(&enabled));
        assert!(
            err.contains("control.enabled") && err.contains("web.token"),
            "{err}"
        );
        let with_token = enabled.replacen("# token = \"change-me\"", "token = \"t\"", 1);
        let err = error_of(&with_token);
        assert!(
            err.contains("control.enabled") && err.contains("web.password_hash"),
            "{err}"
        );
        let config = parse(&with_hash(&with_token)).unwrap();
        assert!(config.control.enabled);
    }

    #[test]
    fn no_config_error_quotes_the_token_or_the_hash() {
        // Security suite type 5 (step 10a): a refused config never echoes a secret.
        let bad_hash = "$argon2i$v=19$m=19456,t=2,p=1$c2VjcmV0LXNhbHQ$SECRET-digest";
        let token = "token = \"tok-SECRET-1\"";
        let enabled = |text: &str| text.replacen("enabled = false", "enabled = true", 1);
        for text in [
            with(LISTEN, &format!("{LISTEN}\npassword_hash = \"{bad_hash}\"")),
            with(
                LISTEN,
                "listen = \"0.0.0.0:8080\"\ntoken = \"tok-SECRET-1\"",
            ),
            enabled(&with("# token = \"change-me\"", token)),
            enabled(&with_hash(EXAMPLE)),
            // TOML syntax errors (reviewer H1): `Display` would quote the line.
            with(
                LISTEN,
                &format!("{LISTEN}\npassword_hash = \"{bad_hash}\" junk"),
            ),
            with(LISTEN, &format!("{LISTEN}\ntoken = \"tok-SECRET-1")),
            with(LISTEN, &format!("{LISTEN}\n{token}\n{token}")),
            with(LISTEN, &format!("{LISTEN}\ntoken = [\"tok-SECRET-1\"]")),
            // An unquoted number: serde would quote it (second pass, H1b).
            with(LISTEN, &format!("{LISTEN}\ntoken = 987654321")),
            with(LISTEN, &format!("{LISTEN}\ntoken = 9876.54321")),
        ] {
            let err = error_of(&text);
            for secret in ["SECRET", "c2VjcmV0", "XYoC+", "FYfUAw0", "$argon2", "98765"] {
                assert!(!err.contains(secret), "{secret} in {err}");
            }
        }
    }

    #[test]
    fn debug_hides_token_and_hash() {
        let mut config = parse(&with_hash(EXAMPLE)).unwrap();
        config.web.token = Some("s3cret-token".into());
        let debug = format!("{config:?}");
        assert!(
            !debug.contains("s3cret-token") && !debug.contains("argon2"),
            "{debug}"
        );
        assert!(
            debug.contains("token: <set>") && debug.contains("password_hash: <set>"),
            "{debug}"
        );
        let plain = format!("{:?}", parse(EXAMPLE).unwrap().web);
        assert!(
            plain.contains("token: None") && plain.contains("password_hash: None"),
            "{plain}"
        );
    }
}
