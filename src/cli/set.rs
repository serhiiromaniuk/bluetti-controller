//! `set <station> ac|dc on|off`: switch one output for the hardware checks
//! (`ROADMAP.md` §R3, HUMAN-CHECKLIST H14–H16). The gate comes first: a
//! station that is not a config name, `control.enabled = false` or an output
//! the model does not list exits non-zero before any BLE (golden rule #2).
//! With `serve` running on the same host, pause polling first.

use anyhow::bail;
use bluetti_controller::config::{ControlConfig, StationConfig};
use bluetti_controller::control::Command;
use bluetti_controller::protocol::control::Output;
use bluetti_controller::transport::Gatt;
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::transport::write::set_output;

use super::{Context, session};

pub async fn run(
    ctx: &Context<'_>,
    control: &ControlConfig,
    station: &str,
    output: Output,
    on: bool,
) -> anyhow::Result<()> {
    let command = command(ctx.stations, control, station, output, on)?;
    let policy = ReadPolicy::from_ble(ctx.ble);
    let state = session::run(ctx, command.station(), async |gatt, _| {
        apply(gatt, &command, &policy).await
    })
    .await?;
    println!("{} {} {}", command.station(), output.key(), on_off(state));
    Ok(())
}

/// The gate, without BLE: a config station name only, then [`Command::new`].
fn command(
    stations: &[StationConfig],
    control: &ControlConfig,
    station: &str,
    output: Output,
    on: bool,
) -> anyhow::Result<Command> {
    let Some(config) = stations.iter().find(|s| s.name == station) else {
        bail!("set takes a station name from the config, got {station:?}");
    };
    Ok(Command::new(control, config, output, on)?)
}

/// Writes the command's allow-listed switch; returns the read-back state.
pub(super) async fn apply(
    gatt: &mut impl Gatt,
    command: &Command,
    policy: &ReadPolicy,
) -> anyhow::Result<bool> {
    Ok(set_output(gatt, command.switch(), command.on(), policy).await?)
}

/// Clap parser for `ac|dc`.
pub(super) fn parse_output(text: &str) -> Result<Output, String> {
    Output::from_key(text).ok_or_else(|| format!("expected ac or dc, got {text:?}"))
}

/// Clap parser for `on|off`.
pub(super) fn parse_state(text: &str) -> Result<bool, String> {
    match text {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("expected on or off, got {text:?}")),
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

#[cfg(test)]
mod tests;
