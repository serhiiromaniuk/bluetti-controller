//! One station session for the connecting commands: find the station, open
//! the link (key exchange when encrypted), run the command, always
//! disconnect, and save the `--record` capture (`ROADMAP.md` §4.6, §11).

use std::path::{Component, Path};

use anyhow::{anyhow, bail};
use bluetti_controller::transport::record::RecordingGatt;
use bluetti_controller::transport::{self, Advert, Gatt, Link};
use tokio::signal::unix::{SignalKind, signal};

use super::{Context, detect};

/// Raw captures carry serial numbers, so they go only under the gitignored
/// `captures/` (RULES.md golden rule #3).
const CAPTURES: &str = "captures";

/// The plaintext layer the commands see: above `CryptoGatt`, teed for `--record`.
pub(super) type Session = RecordingGatt<Link>;

/// Runs `body` on a fresh link to `target`. SIGINT is caught from the start:
/// during the scan it aborts; during `open` (bounded by its timeouts) it is
/// held, then the body is skipped; during the body it cancels it. Every path
/// past `open` disconnects (golden rule #5) and saves the capture.
pub(super) async fn run<T>(
    ctx: &Context<'_>,
    target: &str,
    body: impl AsyncFnOnce(&mut Session, &Advert) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    if let Some(path) = ctx.record {
        prepare_capture(path)?;
    }
    let mut sigint = signal(SignalKind::interrupt())?;
    let address = detect::resolve(ctx.stations, target);
    let advert = tokio::select! {
        advert = transport::find(ctx.ble, address) => advert?,
        _ = sigint.recv() => bail!("interrupted"),
    };
    println!("{}", detect::line(&advert, ctx.anonymise));
    let link = transport::open(ctx.ble, &advert, ctx.keys).await?;
    match link {
        Link::Plain(_) => println!("connected"),
        Link::Encrypted(_) => println!("connected, session established"),
    }
    let mut session = RecordingGatt::new(link, &advert);
    let result = tokio::select! {
        result = body(&mut session, &advert) => result,
        _ = sigint.recv() => Err(anyhow!("interrupted")),
    };
    // Disconnect and save before acting on the result, so no path skips them.
    let closed = session.disconnect().await;
    if let Some(path) = ctx.record {
        match session.save(path) {
            Ok(()) => println!(
                "recorded {} frames to {}",
                session.fixture().frames.len(),
                path.display()
            ),
            Err(err) => tracing::error!(error = %err, "saving the capture failed"),
        }
    }
    closed?;
    println!("disconnected");
    result
}

/// Refuses a capture path outside `captures/` and creates its directory.
fn prepare_capture(path: &Path) -> anyhow::Result<()> {
    if !is_capture_path(path) {
        bail!("--record writes only under {CAPTURES}/ (gitignored): got {path:?}");
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

/// A relative path whose first component is `captures` and that never
/// climbs out with `..`.
fn is_capture_path(path: &Path) -> bool {
    let mut components = path.components();
    components.next() == Some(Component::Normal(CAPTURES.as_ref()))
        && components.clone().next().is_some()
        && components.all(|c| matches!(c, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_go_only_under_captures() {
        assert!(is_capture_path(Path::new("captures/eb3a.json")));
        assert!(is_capture_path(Path::new("captures/a/b.json")));
        assert!(!is_capture_path(Path::new("captures")));
        assert!(!is_capture_path(Path::new("tests/frames/eb3a.json")));
        assert!(!is_capture_path(Path::new("/tmp/captures/x.json")));
        assert!(!is_capture_path(Path::new("captures/../tests/x.json")));
        assert!(!is_capture_path(Path::new("./captures/x.json")));
    }
}
