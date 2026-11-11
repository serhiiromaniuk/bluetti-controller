//! One station open shared by polls and commands: a failed key exchange is
//! retried once on a fresh connection, unless the station is unreachable
//! (phase-4 PLAN A23 part 2; commands per ROADMAP.md §R3, A9). The caller's
//! deadline bounds both attempts.

use super::{Collector, Connect, Progress, Target};
use crate::store::Stage;

impl<C: Connect + Sync> Collector<C> {
    /// Opens `target` like [`Connect::open`], plus the one retry. A failed
    /// retry leaves `progress.stage` at [`Stage::Handshake`] and names both
    /// errors.
    pub(super) async fn open(
        &self,
        target: &Target,
        retry: bool,
        progress: &mut Progress,
    ) -> Result<(C::Gatt, bool), String> {
        match self.connect.open(&target.config, retry, progress).await {
            Err(first) if retry && progress.stage == Stage::Handshake => {
                tracing::info!(
                    station = target.config.name,
                    error = first,
                    "key exchange failed, retrying once on a fresh connection"
                );
                // Set before the release, so a deadline during it keeps `first` (A30).
                progress.retrying = Some(first);
                self.connect.release(&target.config).await;
                progress.cached = None;
                let opened = self.connect.open(&target.config, retry, progress).await;
                let first = progress.retrying.take().unwrap_or_default();
                opened.map_err(|second| {
                    progress.stage = Stage::Handshake;
                    format!("{first}; retry on a fresh connection: {second}")
                })
            }
            opened => opened,
        }
    }
}
