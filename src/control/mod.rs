//! The control gate (`ROADMAP.md` §R3, ADR-010, golden rule #2). A
//! [`Command`] exists only when `control.enabled = true` and the station's
//! model lists the output in [`switches`]; CLI, web and scheduler all build
//! one before any write. No I/O here.

use crate::config::{ControlConfig, StationConfig};
use crate::protocol::control::{Output, OutputSwitch, switches};
use crate::protocol::device::Model;

/// Why no [`Command`] was built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("control is disabled")]
    Disabled,
    #[error("{model:?} has no {} output switch", output.key())]
    NotAllowed { model: Model, output: Output },
}

/// One allowed output switch request. Private fields, one constructor: the
/// gate cannot be skipped.
#[derive(Debug)]
pub struct Command {
    station: String,
    output: Output,
    on: bool,
    switch: &'static OutputSwitch,
}

impl Command {
    /// Refuses unless `control.enabled` and the station's model lists `output`.
    pub fn new(
        control: &ControlConfig,
        station: &StationConfig,
        output: Output,
        on: bool,
    ) -> Result<Command, Refused> {
        if !control.enabled {
            return Err(Refused::Disabled);
        }
        let model = station.model;
        let switch = switches(model)
            .iter()
            .find(|switch| switch.output() == output)
            .ok_or(Refused::NotAllowed { model, output })?;
        Ok(Command {
            station: station.name.clone(),
            output,
            on,
            switch,
        })
    }

    /// The config station name.
    pub fn station(&self) -> &str {
        &self.station
    }

    pub fn output(&self) -> Output {
        self.output
    }

    /// The wanted state: `true` = on.
    pub fn on(&self) -> bool {
        self.on
    }

    /// The allow-listed switch; the only one a caller may pass to
    /// [`set_output`](crate::transport::write::set_output).
    pub fn switch(&self) -> &'static OutputSwitch {
        self.switch
    }
}

/// Answers a [`Request`]: the read-back state, or the error text.
pub type Reply = tokio::sync::oneshot::Sender<Result<bool, String>>;

/// A [`Command`] sent to the collector, which runs it between poll rounds.
#[derive(Debug)]
pub struct Request {
    pub command: Command,
    pub reply: Reply,
}

#[cfg(test)]
mod tests;
