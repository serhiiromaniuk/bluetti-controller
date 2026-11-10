//! Warns when `config.toml` or the keys file is open to group writes or to
//! other users (ROADMAP.md §9). `0600` and the system unit's `0640
//! root:bluetti` pass. A warning, not an error: the owner fixes it with chmod.

use std::path::Path;

/// Group write, or any bit for other.
const LOOSE: u32 = 0o027;

/// Logs one `warn!` when `path` has a [`LOOSE`] mode bit. A file it cannot
/// stat stays silent here: its reader reports that error.
pub(super) fn warn_loose(path: &Path, key: &str) {
    if let Some(mode) = loose_mode(path) {
        // Bound first: a call inside `warn!`'s field list counts as an
        // uncovered line (the `COV_SECURITY` gate) even when the event fires.
        let (file, mode) = (path.display(), format!("{mode:o}"));
        tracing::warn!(
            %file,
            %mode,
            "{key} is group-writable or readable by other users; chmod 600 it"
        );
    }
}

// Unix only, like the crate (`bluer` needs BlueZ), so no non-unix stub.
fn loose_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path).ok()?.permissions().mode() & 0o777;
    (mode & LOOSE != 0).then_some(mode)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::loose_mode;

    #[test]
    fn flags_group_write_and_other_bits_only() {
        let path = std::env::temp_dir().join(format!("bluetti-mode-{}.toml", std::process::id()));
        std::fs::write(&path, "").unwrap();
        for (mode, loose) in [
            (0o600, false),
            (0o640, false),
            (0o400, false),
            (0o660, true),
            (0o604, true),
            (0o644, true),
            (0o601, true),
        ] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(loose_mode(&path).is_some(), loose, "{mode:o}");
        }
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loose_mode(&path), None, "missing file stays silent");
    }

    #[test]
    fn warns_once_naming_the_key_the_file_and_the_mode() {
        let path = std::env::temp_dir().join(format!("bluetti-warn-{}.toml", std::process::id()));
        std::fs::write(&path, "").unwrap();
        let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
        let capture = crate::web::tests::Capture::default();
        let sink = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || sink.clone())
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        super::warn_loose(&path, "config file");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        super::warn_loose(&path, "crypto.keys_file");
        std::fs::remove_file(&path).unwrap();
        super::warn_loose(&path, "missing");

        let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        assert_eq!(logs.lines().count(), 1, "{logs}");
        assert!(logs.contains("WARN"), "{logs}");
        assert!(
            logs.contains("config file is group-writable or readable by other users; chmod 600 it"),
            "{logs}"
        );
        assert!(logs.contains(&format!("file={}", path.display())), "{logs}");
        assert!(logs.contains("mode=644"), "{logs}");
    }
}
