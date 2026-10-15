//! What every handler shares (ROADMAP.md §8.4 a): the read-only connection, the
//! collector's `Control` sender, the SSE `Update` sender, the shutdown signal
//! and the config values the views need. No `Debug`: it would print the token.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use jiff::tz::TimeZone;
use rusqlite::Connection;
use tokio::sync::{broadcast, watch};

use crate::collector::Control;
use crate::config::Config;
use crate::store::Update;

/// The read-only connection, shared by handlers through `spawn_blocking` (§5.3).
pub type Db = Arc<Mutex<Connection>>;

/// The router's state, behind one `Arc` (a `watch::Sender` is not `Clone`).
pub struct AppState {
    /// From [`crate::store::open_reader`]; never the writer's connection.
    pub reader: Db,
    /// Pause/resume for `POST /api/polling/pause` (§8.4 h); `serve` sends
    /// `Shutdown` on it too.
    pub control: watch::Sender<Control>,
    /// The writer's committed rows; `/events` subscribes (§8.4 g).
    pub updates: broadcast::Sender<Update>,
    /// `general.poll_interval_secs`, for the staleness rule (§8.4 b).
    pub interval_s: i64,
    /// `metrics.usable_capacity_factor`, for the runtime forecast (§8.4 c).
    pub usable_factor: f64,
    /// `storage.raw_retention_days`, the cap of `/api/energy?days=` (§8.4 f).
    pub raw_retention_days: u32,
    /// `general.timezone`, for local days and "lasts until HH:MM" (ADR-009).
    pub tz: TimeZone,
    /// `web.token`; `None` = the POST is open (§9, §8.4 h).
    pub token: Option<String>,
    /// Set to `true` by `serve` before the server stops, so every `/events`
    /// stream ends and the graceful close does not wait on it (§8.4 g).
    pub shutdown: watch::Sender<bool>,
    /// Samples this process stored (at least: a lagged receiver counts what it
    /// missed). `/healthz` reports it, so `bluetti-update` sees the new build
    /// poll, never an older sample in the database (ROADMAP.md §10.6).
    pub samples_stored: AtomicU64,
}

impl AppState {
    /// The state `serve` builds from the loaded config.
    pub fn new(
        reader: Connection,
        config: &Config,
        control: watch::Sender<Control>,
        updates: broadcast::Sender<Update>,
    ) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            reader: Arc::new(Mutex::new(reader)),
            control,
            updates,
            interval_s: i64::try_from(config.general.poll_interval_secs.get())?,
            usable_factor: config.metrics.usable_capacity_factor,
            raw_retention_days: config.storage.raw_retention_days.get(),
            tz: config.general.timezone.clone(),
            token: config.web.token.clone(),
            shutdown: watch::channel(false).0,
            samples_stored: AtomicU64::new(0),
        }))
    }

    /// Counts the writer's `Sample` updates into `samples_stored` for the
    /// process lifetime (the state holds a sender, so the channel never
    /// closes under `serve`). `serve` starts it once.
    pub fn spawn_sample_counter(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let state = Arc::clone(self);
        let mut updates = self.updates.subscribe();
        tokio::spawn(async move {
            loop {
                let n = match updates.recv().await {
                    Ok(Update::Sample { .. }) => 1,
                    Ok(_) => 0,
                    Err(broadcast::error::RecvError::Lagged(missed)) => missed,
                    Err(broadcast::error::RecvError::Closed) => return,
                };
                state.samples_stored.fetch_add(n, Ordering::Relaxed);
            }
        })
    }

    /// A state over `reader` with test defaults: 60 s interval, factor 0.9,
    /// 30 days of raw retention, `Europe/Kyiv`, no token. Tests change fields,
    /// then wrap it in an `Arc`.
    #[cfg(test)]
    pub fn for_tests(reader: Connection) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            control: watch::channel(Control::Run).0,
            updates: broadcast::channel(16).0,
            interval_s: 60,
            usable_factor: 0.9,
            raw_retention_days: 30,
            tz: TimeZone::get("Europe/Kyiv").expect("tzdb has Europe/Kyiv"),
            token: None,
            shutdown: watch::channel(false).0,
            samples_stored: AtomicU64::new(0),
        }
    }
}
