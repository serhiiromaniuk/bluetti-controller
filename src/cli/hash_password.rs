//! `hash-password`: the argon2id PHC string for `web.password_hash`
//! (ROADMAP.md ADR-011, §9). The password comes from stdin, never argv
//! (shell history, `ps`).

use std::io::BufRead;

use anyhow::{Context, bail};
use bluetti_controller::web::auth::password;

/// The first line of `input`, without its line terminator; empty is refused.
fn read_password(mut input: impl BufRead) -> anyhow::Result<String> {
    let mut line = String::new();
    input.read_line(&mut line).context("reading stdin")?;
    let password = line.trim_end_matches(['\n', '\r']);
    if password.is_empty() {
        bail!("empty password refused");
    }
    Ok(password.to_owned())
}

pub fn run() -> anyhow::Result<()> {
    // shortcut: a terminal echoes the typed password; pipe it in (`read -rs`) to hide it.
    eprintln!("password (one line on stdin):");
    let phc = password::hash(&read_password(std::io::stdin().lock())?)?;
    println!("{phc}");
    println!("# paste under [web] in config.toml:");
    println!("password_hash = \"{phc}\"");
    Ok(())
}

#[cfg(test)]
mod tests;
