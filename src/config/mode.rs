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
        tracing::warn!(
            file = %path.display(),
            mode = %format_args!("{mode:o}"),
            "{key} is group-writable or readable by other users; chmod 600 it"
        );
    }
}

#[cfg(unix)]
fn loose_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path).ok()?.permissions().mode() & 0o777;
    (mode & LOOSE != 0).then_some(mode)
}

#[cfg(not(unix))]
fn loose_mode(_: &Path) -> Option<u32> {
    None
}

#[cfg(all(test, unix))]
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
}
