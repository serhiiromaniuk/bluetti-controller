//! Output switch requests in the collector (`ROADMAP.md` §R3, ADR-010,
//! golden rule #5): run between poll rounds over the collector's own link —
//! connect, write once, read back, disconnect — under the poll deadline.

use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use super::{Collector, Connect, Control, Progress, Request, Target, UNREACHABLE_AFTER, shutdown};
use crate::control::Command;
use crate::transport::{self, write::set_output};

impl<C: Connect + Sync> Collector<C> {
    /// Runs one request and answers it. `failures` is the run loop's
    /// per-station miss count (an unreachable station gets one connect).
    /// A request whose caller gave up is dropped before any link opens.
    pub(super) async fn execute(
        &self,
        request: Request,
        failures: &[u32],
        deadline: Duration,
        control: &mut watch::Receiver<Control>,
    ) {
        let Request { command, reply } = request;
        if reply.is_closed() {
            tracing::debug!(station = command.station(), "control request abandoned");
            return;
        }
        let answer = self.switch(&command, failures, deadline, control).await;
        let (station, output) = (command.station(), command.output().key());
        match &answer {
            Ok(read_back) => tracing::info!(
                station,
                output,
                state = command.on(),
                read_back,
                "output switched"
            ),
            Err(error) => tracing::warn!(
                station,
                output,
                state = command.on(),
                error,
                "output switch failed"
            ),
        }
        let _ = reply.send(answer);
    }

    /// The switch itself: refused while paused or shutting down, else one
    /// open and an always-released link.
    async fn switch(
        &self,
        command: &Command,
        failures: &[u32],
        deadline: Duration,
        control: &mut watch::Receiver<Control>,
    ) -> Result<bool, String> {
        let (i, target) = self
            .stations
            .iter()
            .enumerate()
            .find(|(_, target)| target.config.name == command.station())
            .ok_or_else(|| format!("unknown station {}", command.station()))?;
        let state = *control.borrow();
        match state {
            Control::Shutdown => return Err("shutting down".into()),
            Control::Pause { until } if Instant::now() < until => {
                return Err("polling paused".into());
            }
            _ => {}
        }
        let retry = failures[i] < UNREACHABLE_AFTER;
        let mut progress = Progress::new();
        let outcome = tokio::select! {
            result = tokio::time::timeout(deadline, self.write(target, command, retry, &mut progress)) => Some(result),
            () = shutdown(control) => None,
        };
        if !progress.disconnected {
            self.connect.release(&target.config).await;
        }
        match outcome {
            None => Err("shutting down".into()),
            Some(Err(_)) => Err(format!("no result within the {deadline:?} poll deadline")),
            Some(Ok(result)) => result,
        }
    }

    async fn write(
        &self,
        target: &Target,
        command: &Command,
        retry: bool,
        progress: &mut Progress,
    ) -> Result<bool, String> {
        let (mut gatt, _) = self.connect.open(&target.config, retry, progress).await?;
        let (written, closed) = transport::with_disconnect(&mut gatt, async |gatt| {
            set_output(gatt, command.switch(), command.on(), &self.policy).await
        })
        .await;
        match closed {
            Ok(()) => progress.disconnected = true,
            Err(err) => {
                tracing::warn!(station = target.config.name, error = %err, "disconnect failed");
            }
        }
        written.map_err(|err| err.to_string())
    }
}
