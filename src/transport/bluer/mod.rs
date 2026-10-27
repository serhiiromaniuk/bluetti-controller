//! The BlueZ adapter: the only module that imports `bluer` (RULES.md
//! architecture rules). It turns BlueZ objects into our own [`Advert`] and a
//! [`Gatt`](super::Gatt) session, so core code never sees a `bluer` type.
//!
//! [`Advert`]: super::Advert

pub mod gatt;
pub mod scan;

static SESSION: tokio::sync::OnceCell<::bluer::Session> = tokio::sync::OnceCell::const_new();

/// The process's one D-Bus session to bluetoothd, opened on first use. A
/// session per call leaked its connection and tasks: 139 MB after 11 days on
/// the host (tasks/2026-10-27_ble-reliability/INVESTIGATION.md R1).
/// shortcut: never reopened; if the system bus drops it, every call fails
/// until the unit restarts — add a reopen if the journal ever shows that.
async fn session() -> ::bluer::Result<::bluer::Session> {
    SESSION
        .get_or_try_init(::bluer::Session::new)
        .await
        .cloned()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    /// Resident set size of this process in bytes.
    fn rss() -> u64 {
        let statm = std::fs::read_to_string("/proc/self/statm").unwrap();
        statm
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 4096
    }

    /// One test, since the shared session lives on the runtime of the test
    /// that opened it. Needs bluetoothd and a powered `hci0`, so `#[ignore]`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs bluetoothd and a powered hci0"]
    async fn the_shared_session_neither_grows_nor_jams_discovery() {
        // A poll takes 3–4 sessions; 2,000 is a few hours of polling.
        drop(super::session().await.unwrap());
        let before = rss();
        for _ in 0..2_000 {
            drop(super::session().await.unwrap());
        }
        let grown = rss().saturating_sub(before);
        assert!(grown < 2 << 20, "RSS grew {} KiB", grown >> 10);
        // A scan cancelled while its discovery starts must not leave the
        // session's BlueZ client discovering: later scans still start.
        let scan = |secs| super::scan::scan("hci0", Duration::from_secs(secs));
        for ms in 0..40 {
            let _ = tokio::time::timeout(Duration::from_millis(ms), scan(2)).await;
        }
        scan(2).await.unwrap();
        scan(2).await.unwrap();
    }
}
