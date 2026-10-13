//! `serve`: the long-running monitor (ROADMAP.md §5.3, §12 M3). It opens and
//! migrates the store, syncs the configured stations, then runs the writer, the
//! collector and the web server until SIGINT/SIGTERM, and shuts them down in
//! order: the collector stops (its open link disconnected), the writer drains
//! every queued record, the server closes gracefully.

use std::time::Duration;

use anyhow::{Context as _, bail};
use bluetti_controller::collector::{BluezConnect, Collector, Control, Target};
use bluetti_controller::config::Config;
use bluetti_controller::metrics::grid::Params;
use bluetti_controller::protocol::crypto::handshake::StaticKeys;
use bluetti_controller::store::{Station, Writer, open_reader, open_writer, sync_stations};
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::web;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::{mpsc, oneshot, watch};

/// Records queued between the collector and the writer: a few rounds' worth.
const QUEUE: usize = 256;

pub async fn run(config: &Config, keys: Option<StaticKeys>) -> anyhow::Result<()> {
    // Installed first, so a signal during startup is not lost.
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    let path = &config.storage.path;
    let mut conn = open_writer(path)?;
    let ids = sync_stations(&mut conn, &config.stations)?;
    let id = |name: &str| ids.get(name).copied().context("station missing after sync");
    let mut stations = Vec::new();
    let mut targets = Vec::new();
    for station in &config.stations {
        let id = id(&station.name)?;
        stations.push(Station {
            id,
            name: station.name.clone(),
            grid_sensor: station.grid_sensor,
        });
        targets.push(Target {
            id,
            config: station.clone(),
        });
    }
    let interval = Duration::from_secs(config.general.poll_interval_secs.get());
    let params = Params {
        voltage_min: config.metrics.grid_voltage_min,
        debounce: config.metrics.debounce_samples,
        interval_s: i64::try_from(interval.as_secs()).context("poll_interval_secs too large")?,
    };
    let writer = Writer::restore(conn, stations, params, config.storage.raw_retention_days)?;
    let router = web::router(open_reader(path)?);
    let listener = TcpListener::bind(config.web.listen)
        .await
        .with_context(|| format!("listen on {}", config.web.listen))?;
    tracing::info!(
        listen = %listener.local_addr()?,
        stations = config.stations.len(),
        interval_s = interval.as_secs(),
        "serving"
    );

    let (records, queue) = mpsc::channel(QUEUE);
    let writer = writer.spawn(queue);
    let (stop_server, server_stopped) = oneshot::channel::<()>();
    let mut server = tokio::spawn(web::serve(listener, router, async {
        let _ = server_stopped.await;
    }));
    let (control, control_rx) = watch::channel(Control::Run);
    let collector = Collector {
        connect: BluezConnect {
            ble: config.ble.clone(),
            keys,
        },
        stations: targets,
        interval,
        policy: ReadPolicy::from_ble(&config.ble),
        records,
    };
    let polling = collector.run(control_rx);
    tokio::pin!(polling);

    let mut failed = None;
    let mut collector_done = false;
    let mut server_done = false;
    tokio::select! {
        _ = sigint.recv() => tracing::info!("SIGINT, shutting down"),
        _ = sigterm.recv() => tracing::info!("SIGTERM, shutting down"),
        () = &mut polling => {
            failed = Some("collector stopped");
            collector_done = true;
        }
        result = &mut server => {
            failed = Some("web server stopped");
            server_done = true;
            log_server(result);
        }
    }
    // The collector ends its current poll with the disconnect and drops the
    // record sender, so the writer drains the queue and returns. A finished
    // future is never polled again (it would panic).
    let _ = control.send(Control::Shutdown);
    if !collector_done {
        polling.await;
    }
    // Kept until the server is stopped too, so a writer panic leaves no task.
    let written = writer.await;
    if !server_done {
        let _ = stop_server.send(());
        log_server(server.await);
    }
    written.context("store writer task")?;
    match failed {
        Some(what) => bail!("{what} unexpectedly"),
        None => {
            tracing::info!("stopped");
            Ok(())
        }
    }
}

fn log_server(result: Result<anyhow::Result<()>, tokio::task::JoinError>) {
    match result {
        Ok(Ok(())) => {}
        Ok(Err(err)) => tracing::error!(error = format!("{err:#}"), "web server failed"),
        Err(err) => tracing::error!(error = %err, "web server task failed"),
    }
}
